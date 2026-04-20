pub mod parse;

const PROBE_URL: &str = "http://login.net.vn/";

use super::LoginStrategy;
use super::awing_utils::{
    AWING_ORIGIN, AWING_REFERER, call_verify_url, load_awing_portal, run_step,
};
use crate::strategies::error::StrategyError;
use crate::strategies::highland::parse::{
    extract_qv_param, parse_highland_script_params, query_param,
};

use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::header::{HeaderValue, ORIGIN, REFERER};
use tracing::info;

pub struct HighlandStrategy;

pub static REGISTRY_ENTRY: super::RegistryStrategy = crate::strategies::RegistryStrategy {
    name: "Highlands Coffee",
    predicate: |ssid| ssid.contains("Highlands Coffee"),
    factory: |_platform| Ok(Box::new(HighlandStrategy)),
};

#[async_trait::async_trait]
impl LoginStrategy for HighlandStrategy {
    async fn login(&self, client: &Client) -> Result<()> {
        let awing_url =
            run_step("step1: probe redirect to awing URL", probe_redirect(client)).await?;

        run_step(
            "step2: load awing portal",
            load_awing_portal(client, &awing_url, None),
        )
        .await?;

        let form_html = run_step(
            "step3: VerifyUrl -> contentAuthenForm",
            call_verify_url(client, &awing_url),
        )
        .await?
        .form_html
        .context("VerifyUrl response missing contentAuthenForm")?;

        info!("step4: extract login params");
        let (login_url, hs_server, qv) = extract_login_params(&awing_url, &form_html)?;

        run_step(
            "step5: POST hslogin",
            post_hslogin(client, &login_url, &hs_server, &qv),
        )
        .await?;

        Ok(())
    }
}

/// Step 1: GET http://login.net.vn/ and capture the final redirect URL.
///
/// Highland Coffee redirects directly (307) to `v1.awingconnect.vn/login?...`
/// without going through the MikroTik hotspot page. The final URL is the
/// awing URL and contains all parameters needed for the rest of the flow.
async fn probe_redirect(client: &Client) -> Result<String> {
    let resp = client
        .get(PROBE_URL)
        .send()
        .await
        .context("probe request to login.net.vn failed")?;

    let awing_url = resp.url().to_string();
    info!("awing URL: {}", awing_url);
    Ok(awing_url)
}

/// Step 4: Extract login URL, `hs_server`, and `Qv` from the awing URL and form.
///
/// - `hs_server` and `Qv` come from the awing URL query parameters (the form
///   HTML only defines the fields; JS fills their values at runtime).
/// - `port` and `postToUrl` come from the `<script>` in the form HTML, with
///   sane defaults (880 / "/cgi-bin/hslogin.cgi") if parsing fails.
fn extract_login_params(awing_url: &str, form_html: &str) -> Result<(String, String, String)> {
    let hs_server = query_param(awing_url, "hs_server")?;
    let qv = extract_qv_param(awing_url)?;

    let (port, post_path) = parse_highland_script_params(form_html);
    let login_url = format!("http://{}:{}{}", hs_server, port, post_path);
    info!("login URL: {}", login_url);

    Ok((login_url, hs_server, qv))
}

/// Step 5: POST to the Extreme Networks gateway to complete authentication.
async fn post_hslogin(client: &Client, login_url: &str, hs_server: &str, qv: &str) -> Result<()> {
    let response = client
        .post(login_url)
        .header(ORIGIN, HeaderValue::from_static(AWING_ORIGIN))
        .header(REFERER, HeaderValue::from_static(AWING_REFERER))
        .form(&[
            ("f_flex", ""),
            ("f_flex_type", "log"),
            ("f_hs_server", hs_server),
            ("f_Qv", qv),
        ])
        .send()
        .await
        .context("hslogin POST failed")?;

    if !response.status().is_success() {
        return Err(StrategyError::UnexpectedStatus {
            endpoint: "hslogin",
            status: response.status(),
        }
        .into());
    }

    info!("hslogin succeeded (status {})", response.status());
    Ok(())
}
