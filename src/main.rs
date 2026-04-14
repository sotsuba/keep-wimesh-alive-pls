use anyhow::{Context, Result};
use clap::Parser;
use reqwest::{Client, redirect::Policy};
use std::time::Duration;
use tracing::info; 

use keep_wimesh_session::cli::{Cli, USER_AGENT_FIREFOX};
use keep_wimesh_session::strategies::select_strategy;

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
    let config = Cli::parse();

    let strategy = select_strategy(&config.ssid);
    match &strategy {
        Ok(s) => {
            info!("Selected strategy for SSID '{}'", config.ssid);
            
            let mut builder = Client::builder()
                .redirect(Policy::limited(10))
                .timeout(Duration::from_secs(20))
                .user_agent(USER_AGENT_FIREFOX)
                .danger_accept_invalid_certs(true);
            if let Some(jar) = s.cookie_jar() {
                builder = builder.cookie_provider(jar.clone());
            }
            let client = builder.build().context("failed to construct HTTP client")?;
            s.login(&client).await?;
        },
        Err(e) => eprintln!("Failed to select strategy for SSID '{}': {}", config.ssid, e),
    };
    Ok(())
}
