use std::path::PathBuf;

use err_context::AnyError;
use log::{debug, info};
use structopt::StructOpt;
use url::Url;

use crate::config::ServerSession;
use crate::Command::*;

mod commands;
mod config;
mod connector;
mod errors;
mod utils;

#[derive(Debug, StructOpt)]
#[structopt(about = "the stupid content tracker")]
struct Opts {
    #[structopt(short, long)]
    config: Option<PathBuf>,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    Register {
        url: Url,
        username: String,
    },
    Login {
        url: Url,
        device_id: String,
        username: String,
    },
    ListDevices,
    Upload {
        #[structopt(short, long)]
        recursive: bool,
        #[structopt(short, long, default_value = "4")]
        parallelism: usize,
        filenames: Vec<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    env_logger::init();

    let opts = Opts::from_args();

    let config_file = opts
        .config
        .or_else(|| dirs::home_dir().map(|p| p.join(".rbackup/config.toml")))
        .expect("Could not get home dir!");

    info!("Using config file: {:?}", config_file);

    match opts.cmd {
        Register { url, username } => commands::register(&url, username).await,
        Login {
            url,
            device_id,
            username,
        } => commands::login(&url, device_id, username, &config_file).await,
        ListDevices => {
            let session = load_session(&config_file).await?;
            commands::list_devices(session).await
        }
        Upload {
            recursive,
            parallelism,
            filenames,
        } => {
            if filenames.is_empty() {
                return Err(AnyError::from("You must provide at least one filename!"));
            }

            for path in filenames.iter() {
                if path.is_dir() && !recursive {
                    return Err(AnyError::from(format!(
                        "{:?} is a dir but you didn't enable dirs recursion!",
                        path
                    )));
                }
            }

            debug!(
                "Upload: recursive: {}, parallelism: {}, filenames: {:?}",
                recursive, parallelism, filenames
            );

            let session = load_session(&config_file).await?;
            commands::upload_files(session, parallelism, filenames).await
        }
    }
}

async fn load_session(path: &PathBuf) -> Result<ServerSession, AnyError> {
    let content = tokio::fs::read_to_string(path).await?;
    let session: ServerSession = toml::from_str(&content)?;

    debug!("Loaded stored session: {:?}", session);
    info!("Configured server: {:?}", session.url);

    Ok(session)
}
