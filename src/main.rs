#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use anyhow::{Context, Result};
use clap::Parser;
use keep_wimesh_session::platform::select_platform;
use tracing::info;

use keep_wimesh_session::build_client;
use keep_wimesh_session::cli::{Cli, Command, WatchArgs};
use keep_wimesh_session::strategies::select_strategy;
use keep_wimesh_session::watcher;

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

    match cli.command {
        Command::Login(args) => {
            let strategy = select_strategy(&args.ssid)?;
            info!("selected strategy for SSID '{}'", args.ssid);
            let client = build_client(strategy.as_ref()).context("failed to build HTTP client")?;
            strategy.login(&client).await?;
        }
        Command::Watch(args) => run_watch(args).await?,
    }

    Ok(())
}

fn make_shutdown_channel() -> tokio::sync::watch::Receiver<bool> {
    let (tx, rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = tx.send(true);
    });
    rx
}

#[cfg(target_os = "linux")]
async fn run_watch(args: WatchArgs) -> Result<()> {
    use keep_wimesh_session::watcher::platform::linux::LinuxPlatform;
    use keep_wimesh_session::watcher::platform::traits::RealRunner;
    watcher::run(
        &LinuxPlatform::new(RealRunner),
        &args,
        make_shutdown_channel(),
    )
    .await
}

#[cfg(target_os = "windows")]
async fn run_watch(args: WatchArgs) -> Result<()> {
    use keep_wimesh_session::platform::windows::WindowsPlatform;
    watcher::run(
        &WindowsPlatform,
        &args,
        make_shutdown_channel(),
    )
    .await
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
async fn run_watch(_args: WatchArgs) -> Result<()> {
    anyhow::bail!("watch subcommand is not supported on this platform")
}
