use reqwest::Client;
use reqwest::header::{HeaderValue, REFERER};
use anyhow::{Context, Result, bail};

/// Load the AWING portal page so the server's state machine advances.
/// Pass `referer` as `Some(url)` if the hotspot expects a specific Referer header.
pub async fn load_awing_portal(client: &Client, awing_url: &str, referer: Option<&str>) -> Result<()> {
    let mut req = client.get(awing_url);
    if let Some(r) = referer {
        req = req.header(
            REFERER,
            HeaderValue::from_str(r).context("invalid referer for load_awing_portal")?,
        );
    }
    let response = req.send().await.context("failed to load awing portal")?;
    if !response.status().is_success() {
        bail!("awing portal returned status {}", response.status());
    }
    Ok(())
}

/// Run an async step, logging its name before execution and enriching any error with context.
pub async fn run_step<T>(
    name: &str,
    fut: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tracing::info!("{}", name);
    fut.await.with_context(|| format!("{} failed", name))
}
