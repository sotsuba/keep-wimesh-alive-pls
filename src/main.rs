//! Wimesh - Auto-login client for captive portals
//!
//! Supports multiple captive portal types through a trait-based plugin system.

mod config;
mod http;
mod models;
mod parser;
mod portal;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use portal::{AwingPortal, CaptivePortal, PortalRegistry};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "wimesh")]
#[command(about = "Captive Portal Auto Login Client", long_about = None)]
struct Args {
    /// Run in daemon mode (continuous monitoring)
    #[arg(short, long)]
    daemon: bool,

    /// Config file path (default: config.toml)
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let cfg = config::Config::load()?;

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&cfg.logging.level)),
        )
        .init();

    tracing::info!("Wimesh v0.2.0 - Captive Portal Auto Login");
    tracing::info!("==========================================");

    // Build portal registry from config
    let mut registry = build_portal_registry(&cfg)?;

    if args.daemon {
        run_daemon(cfg, registry).await
    } else {
        run_once(&mut registry).await
    }
}

/// Build a portal registry from configuration
fn build_portal_registry(cfg: &config::Config) -> Result<PortalRegistry> {
    let mut registry = PortalRegistry::new();

    for portal_cfg in &cfg.portals {
        match portal_cfg.portal_type.as_str() {
            "awing" => {
                let awing_config = portal::awing::AwingConfig {
                    name: portal_cfg.name.clone(),
                    ssids: portal_cfg.ssids.clone(),
                    mac_address: portal_cfg.mac_address.clone(),
                };
                let portal = AwingPortal::new(awing_config)?;
                registry.register(Box::new(portal));
            }
            unknown => {
                tracing::warn!("Unknown portal type '{}', skipping: {}", unknown, portal_cfg.name);
            }
        }
    }

    if registry.all_ssids().is_empty() {
        tracing::warn!("No portals configured! Add portal configurations to config.toml");
    }

    Ok(registry)
}

/// Run once - try to connect using the first available portal
async fn run_once(registry: &mut PortalRegistry) -> Result<()> {
    // Check current WiFi and find matching portal
    let all_ssids: Vec<String> = registry.all_ssids().iter().map(|s| s.to_string()).collect();
    
    match utils::is_connected_to_wifi(&all_ssids) {
        Ok(Some(connected_ssid)) => {
            tracing::info!("Connected to: {}", connected_ssid);
            
            if let Some(portal) = registry.find_for_ssid(&connected_ssid) {
                tracing::info!("Using portal: {}", portal.name());
                match portal.connect().await {
                    Ok(_) => {
                        tracing::info!("Connection established!");
                        Ok(())
                    }
                    Err(e) => {
                        tracing::error!("Connection failed: {:#}", e);
                        Err(e)
                    }
                }
            } else {
                anyhow::bail!("No portal configured for SSID: {}", connected_ssid)
            }
        }
        Ok(None) => {
            tracing::warn!("Not connected to any configured WiFi network");
            tracing::info!("Configured SSIDs: {}", all_ssids.join(", "));
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to check WiFi status: {}", e);
            Err(e)
        }
    }
}

/// Run in daemon mode - continuous monitoring
async fn run_daemon(cfg: config::Config, mut registry: PortalRegistry) -> Result<()> {
    let all_ssids: Vec<String> = registry.all_ssids().iter().map(|s| s.to_string()).collect();
    
    tracing::info!("Starting daemon mode...");
    tracing::info!("Monitoring SSIDs: {}", all_ssids.join(", "));
    tracing::info!("Check interval: {}s", cfg.global.check_interval);
    tracing::info!("---");

    let check_interval = std::time::Duration::from_secs(cfg.global.check_interval);
    let mut last_check = std::time::Instant::now();
    let mut consecutive_failures = 0;
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;

    loop {
        // Rate limiting
        let elapsed = last_check.elapsed();
        if elapsed < check_interval {
            tokio::time::sleep(check_interval - elapsed).await;
        }
        last_check = std::time::Instant::now();

        // Check if connected to any configured WiFi
        match utils::is_connected_to_wifi(&all_ssids) {
            Ok(Some(connected_ssid)) => {
                // Check internet connectivity
                if !utils::has_internet_connectivity() {
                    tracing::warn!(
                        "No internet on '{}', attempting login...",
                        connected_ssid
                    );

                    // Find the portal for this SSID
                    if let Some(portal) = registry.find_for_ssid(&connected_ssid) {
                        match portal.connect().await {
                            Ok(_) => {
                                tracing::info!("Login successful via '{}'", portal.name());
                                consecutive_failures = 0;

                                // Wait for connection to stabilize
                                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            }
                            Err(e) => {
                                consecutive_failures += 1;
                                tracing::error!(
                                    "Login failed via '{}' (attempt {}/{}): {:#}",
                                    portal.name(),
                                    consecutive_failures,
                                    MAX_CONSECUTIVE_FAILURES,
                                    e
                                );

                                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                                    tracing::error!("Too many failures, backing off...");
                                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                                    consecutive_failures = 0;
                                }
                            }
                        }
                    } else {
                        tracing::warn!("No portal configured for SSID: {}", connected_ssid);
                    }
                } else {
                    // Internet is working
                    if consecutive_failures > 0 {
                        tracing::debug!("Internet restored on '{}'", connected_ssid);
                        consecutive_failures = 0;
                    }
                }
            }
            Ok(None) => {
                tracing::debug!("Not connected to any configured WiFi");
                consecutive_failures = 0;
            }
            Err(e) => {
                tracing::warn!("Failed to check WiFi status: {}", e);
            }
        }
    }
}
