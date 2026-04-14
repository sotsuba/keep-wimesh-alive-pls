use anyhow::{Context, Result};
use clap::Parser;
use reqwest::{Client, redirect::Policy};
use std::time::Duration;
use tracing::info;

use keep_wimesh_session::cli::{Cli, Command, USER_AGENT_FIREFOX, WatchArgs};
use keep_wimesh_session::strategies::select_strategy;
use keep_wimesh_session::watcher;

fn setup_log() {
    tracing_subscriber::fmt()
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

    match cli.command {
        Command::Login(args) => {
            let strategy = select_strategy(&args.ssid)?;
            info!("selected strategy for SSID '{}'", args.ssid);
            let mut builder = Client::builder()
                .redirect(Policy::limited(10))
                .timeout(Duration::from_secs(20))
                .user_agent(USER_AGENT_FIREFOX)
                .danger_accept_invalid_certs(true);
            if let Some(jar) = strategy.cookie_jar() {
                builder = builder.cookie_provider(jar);
            }
            let client = builder.build().context("failed to construct HTTP client")?;
            strategy.login(&client).await?;
        }
        Command::Watch(args) => run_watch(args).await?,
    }

    Ok(())
}

#[cfg(target_os = "linux")]
async fn run_watch(args: WatchArgs) -> Result<()> {
    use keep_wimesh_session::watcher::platform::linux::LinuxPlatform;
    use keep_wimesh_session::watcher::platform::traits::RealRunner;
    watcher::run(&LinuxPlatform::new(RealRunner), &args).await
}

#[cfg(target_os = "windows")]
async fn run_watch(args: WatchArgs) -> Result<()> {
    use keep_wimesh_session::watcher::platform::traits::RealRunner;
    use keep_wimesh_session::watcher::platform::windows::WindowsPlatform;
    watcher::run(&WindowsPlatform::new(RealRunner), &args).await
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
async fn run_watch(_args: WatchArgs) -> Result<()> {
    anyhow::bail!("watch subcommand is not supported on this platform")
}
