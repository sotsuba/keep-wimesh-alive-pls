use anyhow::{Context, Result, bail};
use reqwest::Client;
use reqwest::header::{HeaderName, HeaderValue, ORIGIN, REFERER};
use serde_json::Value;

pub const AWING_VERIFY_URL: &str = "http://v1.awingconnect.vn/Home/VerifyUrl";
pub const AWING_ORIGIN: &str = "http://v1.awingconnect.vn";
pub const AWING_REFERER: &str = "http://v1.awingconnect.vn/";

/// Response from the AWING VerifyUrl endpoint.
pub struct VerifyUrlResponse {
    pub form_html: Option<String>,
}

/// POST to /Home/VerifyUrl and return the optional `contentAuthenForm` HTML.
/// Both Wi-MESH and Highland use this endpoint with identical headers.
pub async fn call_verify_url(client: &Client, awing_url: &str) -> Result<VerifyUrlResponse> {
    let response = client
        .post(AWING_VERIFY_URL)
        .header(
            HeaderName::from_static("x-requested-with"),
            HeaderValue::from_static("XMLHttpRequest"),
        )
        .header(ORIGIN, HeaderValue::from_static(AWING_ORIGIN))
        .header(
            REFERER,
            HeaderValue::from_str(awing_url).context("invalid awing URL for Referer")?,
        )
        .header("Content-Length", "0")
        .send()
        .await
        .context("VerifyUrl request failed")?;

    if !response.status().is_success() {
        bail!("VerifyUrl returned status {}", response.status());
    }

    let payload: Value = response
        .json()
        .await
        .context("VerifyUrl response is not JSON")?;

    let form_html = payload
        .pointer("/captiveContext/contentAuthenForm")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    Ok(VerifyUrlResponse { form_html })
}

/// Load the AWING portal page so the server's state machine advances.
/// Pass `referer` as `Some(url)` if the hotspot expects a specific Referer header.
pub async fn load_awing_portal(
    client: &Client,
    awing_url: &str,
    referer: Option<&str>,
) -> Result<()> {
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
