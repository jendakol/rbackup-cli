use std::str::FromStr;

use err_context::AnyError;
use log::debug;
use once_cell::sync::Lazy;
use reqwest::{Client, Response, StatusCode};
use url::Url;
use uuid::Uuid;

use crate::connector::structs::LoginResponse;
use crate::errors::HttpError::InvalidStatus;

mod structs;

static CLIENT: Lazy<Client> = Lazy::new(reqwest::Client::new);

mod paths {
    pub mod account {
        pub const REGISTER: &str = "account/register";
        pub const LOGIN: &str = "account/login";
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
