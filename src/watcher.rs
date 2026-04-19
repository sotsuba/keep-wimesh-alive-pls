use anyhow::Result;
use reqwest::{Client, redirect::Policy};
use std::time::Duration;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::USER_AGENT_FIREFOX;
use crate::cli::WatchArgs;
use crate::platform::Platform;
use crate::strategies::session::LoginSession;

pub async fn run(platform: &dyn Platform, args: &WatchArgs) -> Result<()> {
    let mut shutdown = make_shutdown_channel();
    let _lock = platform.acquire_lock()?;
    info!(check_url = %args.check_url, "watchdog started");

    let probe_client = Client::builder()
        .timeout(Duration::from_secs(4))
        .redirect(Policy::none())
        .user_agent(USER_AGENT_FIREFOX)
        .build()?;

    let mut login_fail_count: u32 = 0;

    loop {
        if check_204(&probe_client, &args.check_url).await {
            login_fail_count = 0;
            sleep_or_shutdown(Duration::from_secs(args.check_interval), &mut shutdown).await;
            if *shutdown.borrow() {
                break;
            }
            continue;
        }

        let ssid = tokio::task::block_in_place(|| platform.detect_ssid());

        let Some(ssid) = ssid else {
            warn!("connectivity lost; no active SSID detected, skipping login");
            sleep_or_shutdown(Duration::from_secs(args.check_interval), &mut shutdown).await;
            if *shutdown.borrow() {
                break;
            }
            continue;
        };

        info!(ssid, "connectivity lost; re-authenticating");
        match do_login(platform, &ssid).await {
            Ok(_) => {
                login_fail_count = 0;
                info!(ssid, "login succeeded");
                sleep_or_shutdown(Duration::from_secs(args.post_login_wait), &mut shutdown).await;
                if *shutdown.borrow() {
                    break;
                }
                continue;
            }
            Err(e) => {
                login_fail_count += 1;
                let backoff = next_backoff(login_fail_count, args.retry_base, args.retry_max);
                warn!(
                    ssid,
                    fail_count = login_fail_count,
                    backoff_secs = backoff.as_secs(),
                    error = %e,
                    "login failed; backing off"
                );
                sleep_or_shutdown(backoff, &mut shutdown).await;
                if *shutdown.borrow() {
                    break;
                }
                continue;
            }
        }
    }

    info!("watchdog stopped");
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

/// Sleep for `duration`, but return early if a shutdown signal arrives.
async fn sleep_or_shutdown(duration: Duration, shutdown: &mut watch::Receiver<bool>) {
    tokio::select! {
        _ = shutdown.changed() => {}
        _ = tokio::time::sleep(duration) => {}
    }
}

async fn check_204(client: &Client, url: &str) -> bool {
    client
        .get(url)
        .send()
        .await
        .map(|r| r.status().as_u16() == 204)
        .unwrap_or(false)
}

async fn do_login(platform: &dyn Platform, ssid: &str) -> Result<()> {
    LoginSession::for_ssid(ssid, platform)?.login().await
}

fn next_backoff(fail_count: u32, base_secs: u64, max_secs: u64) -> Duration {
    let shift = fail_count.saturating_sub(1);
    let delay = base_secs
        .checked_shl(shift)
        .unwrap_or(u64::MAX)
        .min(max_secs);
    Duration::from_secs(delay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_each_failure() {
        assert_eq!(next_backoff(1, 10, 120), Duration::from_secs(10));
        assert_eq!(next_backoff(2, 10, 120), Duration::from_secs(20));
        assert_eq!(next_backoff(3, 10, 120), Duration::from_secs(40));
        assert_eq!(next_backoff(4, 10, 120), Duration::from_secs(80));
    }

    #[test]
    fn backoff_caps_at_max() {
        assert_eq!(next_backoff(5, 10, 120), Duration::from_secs(120));
        assert_eq!(next_backoff(100, 10, 120), Duration::from_secs(120));
    }

    #[test]
    fn backoff_no_overflow() {
        assert_eq!(next_backoff(u32::MAX, 10, 120), Duration::from_secs(120));
    }
}
