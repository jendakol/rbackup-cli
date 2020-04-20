use std::fs::canonicalize;
use std::path::PathBuf;
use std::time::Duration;

use err_context::AnyError;
use futures::StreamExt;
use futures_retry::{ErrorHandler, FutureRetry, RetryPolicy};
use log::{debug, info, warn};
use tokio::fs::File;
use tokio::prelude::*;
use url::Url;
use walkdir::WalkDir;

use crate::config::ServerSession;
use crate::connector;
use crate::utils::IterUtils;

const MAX_ATTEMPTS: usize = 3;

pub async fn register(url: &Url, username: String) -> Result<(), AnyError> {
    let pass = rpassword::prompt_password_stdout("Password: ").unwrap();

    debug!("Registering to {} with username {}", url, username);

    connector::register(url, username, pass).await?;

    info!("Registered successfully!");

    Ok(())
}

pub async fn login(
    url: &Url,
    device_id: String,
    username: String,
    config_file: &PathBuf,
) -> Result<(), AnyError> {
    let pass = rpassword::prompt_password_stdout("Password: ").unwrap();

    debug!(
        "Logging in at {} with username '{}' and device_id '{}'",
        url, username, device_id
    );

    let session_id = connector::login(url, device_id, username, pass).await?;

    debug!("Logged in, session ID: {}", session_id);

    let session = ServerSession {
        url: url.clone(),
        session_id,
    };

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

pub async fn upload_files(
    session: ServerSession,
    parallelism: usize,
    filenames: Vec<PathBuf>,
) -> Result<(), AnyError> {
    let filenames: Vec<_> = filenames
        .into_iter()
        .flat_map(|path| {
            if path.is_dir() {
                WalkDir::new(path)
                    .follow_links(false)
                    .same_file_system(true)
                    .into_iter()
                    .filter_map(|e| match e {
                        Ok(e) => Some(e.path().to_path_buf()),
                        Err(e) => {
                            warn!("Could not open {:?}: {}", e.path(), e);
                            None
                        }
                    })
                    .filter(|p| p.is_file())
                    .collect()
            } else {
                vec![path]
            }
        })
        .collect();

    let c = filenames.len();

    let futures = futures::stream::iter(
        filenames
            .into_iter()
            .map(move |path| upload_file(session.clone(), path)),
    );

    let results = futures
        .buffer_unordered(parallelism)
        .collect::<Vec<_>>()
        .await
        .collect_errors();

    match results {
        Ok(_) => {
            info!("Upload of {} files was successful!", c);
            Ok(())
        }
        Err(errs) => {
            debug!("Could not upload all files, errors: {:?}", errs);

            for err in errs {
                warn!("Error while uploading: {:?}", err)
            }

            Err(AnyError::from("Could not upload all files"))
        }
    }
}

async fn upload_file(session: ServerSession, path: PathBuf) -> Result<(), AnyError> {
    let path = canonicalize(path)?;
    debug!("Uploading {:?}", path);

    FutureRetry::new(
        || connector::upload_file(session.clone(), path.clone()),
        RetryHandler,
    )
    .await
    .map(|(r, _)| {
        use connector::UploadFileResponse::*;

        match r {
            Success(_) => info!("File {:?} was uploaded", path),
            HashMismatch(err) => warn!("Upload of {:?} was not successful: {}", path, err),
            BadRequest(err) => warn!("Upload of {:?} was not successful: {}", path, err),
        }
    })
    .map_err(|(e, _)| e)
}

pub async fn list_devices(session: ServerSession) -> Result<(), AnyError> {
    let list = connector::list_devices(session).await?.0;

    info!("Remote devices list: {:?}", list);

    Ok(())
}

struct RetryHandler;

impl ErrorHandler<AnyError> for RetryHandler {
    type OutError = AnyError;

    fn handle(&mut self, attempt: usize, e: AnyError) -> RetryPolicy<Self::OutError> {
        if attempt < MAX_ATTEMPTS {
            debug!(
                "Error while downloading, {} attempts rest: {:?}",
                MAX_ATTEMPTS - attempt,
                e
            );
            RetryPolicy::WaitRetry(Duration::from_secs(2u64.pow(attempt as u32)))
        } else {
            RetryPolicy::ForwardError(e)
        }
    }
}
