use std::path::PathBuf;

use err_context::AnyError;
use log::info;
use structopt::StructOpt;
use url::Url;

use crate::Command::{Login, Register};

mod commands;
mod config;
mod connector;
mod errors;

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
        Register { url, username } => commands::register(url, username).await,
        Login {
            url,
            device_id,
            username,
        } => commands::login(url, device_id, username, &config_file).await,
    }
}
