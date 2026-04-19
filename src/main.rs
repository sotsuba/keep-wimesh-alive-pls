#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use anyhow::Result;
use clap::Parser;

use captive_portal::cli::{Cli, Command};
use captive_portal::platform::select_platform;
use captive_portal::strategies::client::LoginSession;
use captive_portal::watch;

fn setup_log() {
    use std::io::IsTerminal;
    tracing_subscriber::fmt()
        .with_ansi(std::io::stderr().is_terminal())
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_log();
    let cli = Cli::parse();
    let platform = select_platform()?;

    match cli.command {
        Command::Login(args) => {
            LoginSession::for_ssid(&args.ssid, platform.as_ref())?
                .login()
                .await?;
        }
        Command::Watch(args) => watch::run(platform.as_ref(), &args).await?,
    }
    Ok(())
}
