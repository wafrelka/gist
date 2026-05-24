use std::process::ExitCode;

use clap::Parser as _;
use tracing::error;
use tracing_subscriber::EnvFilter;

use gist::{Cli, run};

fn main() -> ExitCode {
    init_logging();

    let cli = Cli::parse();
    match run(cli, chrono::Utc::now(), &mut std::io::stdout()) {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{err:?}");
            ExitCode::FAILURE
        }
    }
}

fn init_logging() {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or(EnvFilter::from("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).init();
}
