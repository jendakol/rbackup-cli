use std::path::PathBuf;

use err_context::AnyError;
use log::{debug, info};
use tokio::fs::File;
use tokio::prelude::*;
use url::Url;

use crate::config::ServerSession;

pub async fn register(url: Url, username: String) -> Result<(), AnyError> {
    let pass = rpassword::prompt_password_stdout("Password: ").unwrap();

    debug!("Registering to {} with username {}", url, username);

    super::connector::register(url, username, pass).await?;

    info!("Registered successfully!");

    Ok(())
}

pub async fn login(
    url: Url,
    device_id: String,
    username: String,
    config_file: &PathBuf,
) -> Result<(), AnyError> {
    let pass = rpassword::prompt_password_stdout("Password: ").unwrap();

    debug!(
        "Logging in at {} with username '{}' and device_id '{}'",
        url, username, device_id
    );

    let session_id = super::connector::login(url.clone(), device_id, username, pass).await?;

    debug!("Logged in, session ID: {}", session_id);

    let session = ServerSession { url, session_id };

    debug!("Saving session to {:?}: {:?}", config_file, session);

    if let Some(parent) = config_file.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut file = File::create(config_file).await?;

    file.write_all(toml::to_string_pretty(&session)?.as_bytes())
        .await?;

    info!("Logged in successfully, session ID: {}", session_id);

    Ok(())
}
