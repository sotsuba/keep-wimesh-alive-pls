use anyhow::{Context, Result};
use reqwest::cookie::Jar;
use reqwest::{Client, redirect::Policy};
use std::sync::Arc;
use std::time::Duration;
use clap::Parser;

use keep_wimesh_session::config::{Cli, Config, USER_AGENT_FIREFOX};
use keep_wimesh_session::strategies::select_strategy;
use keep_wimesh_session::core::setup_log;

#[tokio::main]
async fn main() -> Result<()> {
    setup_log();    
    let config: Config = Cli::parse().into();
    let cookie_jar = Arc::new(Jar::default());

    let strategy = select_strategy(&config, cookie_jar.clone());
    match strategy {
        Some(s) => {
            let client = Client::builder()
                .cookie_provider(cookie_jar.clone())
                .redirect(Policy::limited(10))
                .timeout(Duration::from_secs(20))
                .user_agent(USER_AGENT_FIREFOX)
                .danger_accept_invalid_certs(true) 
                .build()
                .context("failed to construct HTTP client")?;
            s.login(&client).await?;
        },
        None => println!("No strategy found")
    }
    Ok(())
}
