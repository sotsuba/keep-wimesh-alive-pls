pub mod platform;

use anyhow::Result;
use reqwest::{Client, redirect::Policy};
use std::time::Duration;
use tracing::{info, warn};

use crate::cli::{USER_AGENT_FIREFOX, WatchArgs};
use crate::strategies::select_strategy;
use platform::traits::Platform;

pub async fn run(platform: &dyn Platform, args: &WatchArgs) -> Result<()> {
    let _lock = platform.acquire_lock()?;
    info!("watchdog started; check_url={}", args.check_url);

    let probe_client = Client::builder()
        .timeout(Duration::from_secs(4))
        .redirect(Policy::none())
        .user_agent(USER_AGENT_FIREFOX)
        .build()?;

    let mut login_fail_count: u32 = 0;

    loop {
        let online = tokio::task::block_in_place(|| {
            let Some(gateway) = platform.default_gateway() else {
                return false;
            };
            platform.ping_gateway(&gateway)
        });

        if online && check_204(&probe_client, &args.check_url).await {
            login_fail_count = 0;
            tokio::time::sleep(Duration::from_secs(args.check_interval)).await;
            continue;
        }

        let Some(ssid) = platform.detect_ssid() else {
            warn!("connectivity lost; cannot determine active SSID, skipping login");
            tokio::time::sleep(Duration::from_secs(args.check_interval)).await;
            continue;
        };

        info!("connectivity lost; running login for ssid={}", ssid);
        match do_login(&ssid).await {
            Ok(_) => {
                login_fail_count = 0;
                info!("login succeeded");
                tokio::time::sleep(Duration::from_secs(args.post_login_wait)).await;
            }
            Err(e) => {
                login_fail_count += 1;
                let backoff = next_backoff(login_fail_count, args.retry_base, args.retry_max);
                warn!(
                    "login failed (count={}): {}; backing off {}s",
                    login_fail_count,
                    e,
                    backoff.as_secs()
                );
                tokio::time::sleep(backoff).await;
            }
        }

        tokio::time::sleep(Duration::from_secs(args.check_interval)).await;
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

async fn do_login(ssid: &str) -> Result<()> {
    let strategy = select_strategy(ssid)?;
    let mut builder = Client::builder()
        .redirect(Policy::limited(10))
        .timeout(Duration::from_secs(20))
        .user_agent(USER_AGENT_FIREFOX)
        .danger_accept_invalid_certs(true);
    if let Some(jar) = strategy.cookie_jar() {
        builder = builder.cookie_provider(jar);
    }
    let client = builder.build()?;
    strategy.login(&client).await
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
