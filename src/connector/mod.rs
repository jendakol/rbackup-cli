use std::path::PathBuf;
use std::str::FromStr;

use err_context::AnyError;
use log::debug;
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Response, StatusCode};
use serde::Serialize;
use url::Url;
use uuid::Uuid;

use crate::config::ServerSession;
use crate::connector::structs::{DevicesListResponse, LoginResponse};
use crate::connector::upload::UploadedFile;
use crate::errors::HttpError::InvalidStatus;

mod structs;
mod upload;

pub use crate::connector::structs::UploadFileResponse;

static CLIENT: Lazy<Client> = Lazy::new(reqwest::Client::new);

const SESSION_HEADER: &str = "RBackup-Session-Pass";
// const FILE_HASH_HEADER: &str = "RBackup-File-Hash";

mod paths {
    pub mod account {
        pub const REGISTER: &str = "account/register";
        pub const LOGIN: &str = "account/login";
    }

    pub mod list {
        pub const DEVICES: &str = "list/devices";
    }

    pub const UPLOAD: &str = "upload";
}

pub async fn register(url: &Url, username: String, password: String) -> Result<(), AnyError> {
    let response = get(
        url,
        paths::account::REGISTER,
        &[("username", &username), ("password", &password)],
    )
    .await?;

    match response.status() {
        StatusCode::CONFLICT | StatusCode::CREATED => Ok(()),
        status => Err(Box::from(InvalidStatus {
            expected: 201,
            found: status.as_u16(),
        })),
    }
}

pub async fn login(
    url: &Url,
    name: String,
    username: String,
    password: String,
) -> Result<Uuid, AnyError> {
    let response = get(
        url,
        paths::account::LOGIN,
        &[
            ("device_id", &name),
            ("username", &username),
            ("password", &password),
        ],
    )
    .await?;

    match response.status() {
        StatusCode::OK | StatusCode::CREATED => response
            .json::<LoginResponse>()
            .await
            .map(|r| r.session_id)
            .map_err(AnyError::from),
        status => Err(Box::from(InvalidStatus {
            expected: 201,
            found: status.as_u16(),
        })),
    }
}

pub async fn upload_file(
    session: ServerSession,
    file: PathBuf,
) -> Result<UploadFileResponse, AnyError> {
    let url = create_url(&session.url, paths::UPLOAD)?;

    let file = UploadedFile::open(file)?;

    let resp = CLIENT
        .put(url)
        .query(&file.as_query()?)
        .headers(session.into())
        .multipart(file.into_multipart_form().await?)
        .send()
        .await?;

    debug!("Received response: {:?}", resp);

    match resp.status() {
        StatusCode::OK => Ok(UploadFileResponse::Success(resp.json().await?)),
        StatusCode::PRECONDITION_FAILED => Ok(UploadFileResponse::HashMismatch(resp.text().await?)),
        StatusCode::BAD_REQUEST => Ok(UploadFileResponse::BadRequest(resp.text().await?)),
        status => Err(Box::from(InvalidStatus {
            expected: 200,
            found: status.as_u16(),
        })),
    }
}

// fn get_header(resp: &Response, name: &str) -> Option<String> {
//     let x: Option<&HeaderValue> = resp.headers().get(name);
//
//     x.and_then(|v| v.to_str().ok().map(String::from))
// }

pub async fn list_devices(session: ServerSession) -> Result<DevicesListResponse, AnyError> {
    let response = get_authenticated(session, paths::list::DEVICES, &Vec::<&str>::new()).await?;

    match response.status() {
        StatusCode::OK => response
            .json::<DevicesListResponse>()
            .await
            .map_err(AnyError::from),
        status => Err(Box::from(InvalidStatus {
            expected: 200,
            found: status.as_u16(),
        })),
    }
}

async fn get_authenticated<Q: Serialize + ?Sized>(
    session: ServerSession,
    path: &str,
    query: &Q,
) -> Result<Response, AnyError> {
    let url = create_url(&session.url, path)?;

    let resp = CLIENT
        .get(url)
        .query(query)
        .headers(session.into())
        .send()
        .await
        .map_err(AnyError::from);

    debug!("Received response: {:?}", resp);

    resp
}

async fn get<Q: Serialize + ?Sized>(
    base_url: &Url,
    path: &str,
    query: &Q,
) -> Result<Response, AnyError> {
    let url = create_url(base_url, path)?;

    let resp = CLIENT
        .get(url)
        .query(query)
        .send()
        .await
        .map_err(AnyError::from);

    debug!("Received response: {:?}", resp);

    resp
}

fn create_url(base_url: &url::Url, path: &str) -> Result<reqwest::Url, AnyError> {
    let url = reqwest::Url::from_str(base_url.as_str()).expect("Incompatible URLs are pain!");

    url.join(path).map_err(AnyError::from)
}

impl From<ServerSession> for HeaderMap {
    fn from(session: ServerSession) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            SESSION_HEADER,
            HeaderValue::from_str(format!("{}", session.session_id.to_hyphenated()).as_str())
                .expect("UUID is not convertible to header value?"),
        );

        headers
    }
}
