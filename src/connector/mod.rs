use std::str::FromStr;

use err_context::AnyError;
use log::debug;
use once_cell::sync::Lazy;
use reqwest::{Client, Response, StatusCode};
use url::Url;
use uuid::Uuid;

use crate::config::ServerSession;
use crate::connector::structs::{DevicesListResponse, LoginResponse};
use crate::errors::HttpError::InvalidStatus;
use reqwest::header::{HeaderMap, HeaderValue};

mod structs;

static CLIENT: Lazy<Client> = Lazy::new(reqwest::Client::new);

const SESSION_HEADER: &str = "RBackup-Session-Pass";

mod paths {
    pub mod account {
        pub const REGISTER: &str = "account/register";
        pub const LOGIN: &str = "account/login";
    }
    pub mod list {
        pub const DEVICES: &str = "list/devices";
    }
}

pub async fn register(url: Url, username: String, password: String) -> Result<(), AnyError> {
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
    url: Url,
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

pub async fn list_devices(session: &ServerSession) -> Result<DevicesListResponse, AnyError> {
    let response = get_authenticated(session, paths::list::DEVICES, &[]).await?;

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

async fn get_authenticated(
    session: &ServerSession,
    path: &str,
    args: &[(&str, &str)],
) -> Result<Response, AnyError> {
    let url = reqwest::Url::from_str(session.url.to_string().as_str())
        .expect("Incompatible URLs are pain!");
    let url = url.join(path)?;

    let resp = CLIENT
        .get(url)
        .query(args)
        .headers(session.into())
        .send()
        .await
        .map_err(AnyError::from);

    debug!("Received response: {:?}", resp);

    resp
}

async fn get(base_url: Url, path: &str, args: &[(&str, &str)]) -> Result<Response, AnyError> {
    let url =
        reqwest::Url::from_str(base_url.to_string().as_str()).expect("Incompatible URLs are pain!");
    let url = url.join(path)?;

    let resp = CLIENT
        .get(url)
        .query(args)
        .send()
        .await
        .map_err(AnyError::from);

    debug!("Received response: {:?}", resp);

    resp
}

// async fn post<Req>(base_url: Url, path: &str, req: Req) -> Result<Response, AnyError>
// where
//     Req: Serialize + Sized,
// {
//     let url =
//         reqwest::Url::from_str(base_url.to_string().as_str()).expect("Incompatible URLs are pain!");
//     let url = url.join(path)?;
//
//     let resp = CLIENT.post(url).json(&req).send().await?;
//
//     debug!("Received response: {:?}", resp);
//
//     Ok(resp)
// }

impl From<&ServerSession> for HeaderMap {
    fn from(session: &ServerSession) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            SESSION_HEADER,
            HeaderValue::from_str(format!("{}", session.session_id.to_hyphenated()).as_str())
                .expect("UUID is not convertible to header value?"),
        );

        headers
    }
}
