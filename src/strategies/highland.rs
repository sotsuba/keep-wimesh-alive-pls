use reqwest::Client;
use reqwest::header::{HeaderName, HeaderValue, ORIGIN, REFERER};
use serde_json::Value;
use anyhow::{Context, Result, bail};
use tracing::info;

use crate::parse::{extract_qv_param, parse_highland_script_params, query_param};
use crate::step::run_step;
use crate::strategies::LoginStrategy;
use super::utils;

pub struct HighlandStrategy;

#[async_trait::async_trait]
impl LoginStrategy for HighlandStrategy {
    async fn login(&self, client: &Client) -> Result<()> {
        let awing_url = run_step("step1: probe redirect to awing URL",
            step_probe_redirect(client)).await?;

        run_step("step2: load awing portal",
            utils::load_awing_portal(client, &awing_url, None)).await?;

        let form_html = run_step("step3: VerifyUrl -> contentAuthenForm",
            step_verify_url(client, &awing_url)).await?;

        info!("step4: extract login params");
        let (login_url, hs_server, qv) = step_extract_login_params(&awing_url, &form_html)?;

        run_step("step5: POST hslogin",
            step_post_hslogin(client, &login_url, &hs_server, &qv)).await?;

        Ok(())
    }
}

/// Step 1: GET http://login.net.vn/ and capture the final redirect URL.
///
/// Highland Coffee redirects directly (307) to `v1.awingconnect.vn/login?...`
/// without going through the MikroTik hotspot page.  The final URL is the
/// awing URL and contains all parameters needed for the rest of the flow.
async fn step_probe_redirect(client: &Client) -> Result<String> {
    let response = client
        .get("http://login.net.vn/")
        .send()
        .await
        .context("probe request to login.net.vn failed")?;

    let awing_url = response.url().to_string();
    info!("awing URL: {}", awing_url);
    Ok(awing_url)
}

/// Step 3: POST /Home/VerifyUrl and return the `contentAuthenForm` HTML.
async fn step_verify_url(client: &Client, awing_url: &str) -> Result<String> {
    let response = client
        .post("http://v1.awingconnect.vn/Home/VerifyUrl")
        .header(
            HeaderName::from_static("x-requested-with"),
            HeaderValue::from_static("XMLHttpRequest"),
        )
        .header(ORIGIN, HeaderValue::from_static("http://v1.awingconnect.vn"))
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
        .map(ToOwned::to_owned)
        .context("VerifyUrl response missing contentAuthenForm")?;

    Ok(form_html)
}

/// Step 4: Extract login URL, `hs_server`, and `Qv` from the awing URL and form.
///
/// - `hs_server` and `Qv` come from the awing URL query parameters (the form
///   HTML only defines the fields; JS fills their values at runtime).
/// - `port` and `postToUrl` come from the `<script>` in the form HTML, with
///   sane defaults (880 / "/cgi-bin/hslogin.cgi") if parsing fails.
fn step_extract_login_params(awing_url: &str, form_html: &str) -> Result<(String, String, String)> {
    let hs_server = query_param(awing_url, "hs_server")
        .context("awing URL missing hs_server")?;
    let qv = extract_qv_param(awing_url)
        .context("awing URL missing Qv")?;

    let (port, post_path) = parse_highland_script_params(form_html);
    let login_url = format!("http://{}:{}{}", hs_server, port, post_path);
    info!("login URL: {}", login_url);

    Ok((login_url, hs_server, qv))
}

/// Step 5: POST to the Extreme Networks gateway to complete authentication.
async fn step_post_hslogin(
    client: &Client,
    login_url: &str,
    hs_server: &str,
    qv: &str,
) -> Result<()> {
    let response = client
        .post(login_url)
        .header(ORIGIN, HeaderValue::from_static("http://v1.awingconnect.vn"))
        .header(REFERER, HeaderValue::from_static("http://v1.awingconnect.vn/"))
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
        bail!("hslogin returned status {}", response.status());
    }

    info!("hslogin succeeded (status {})", response.status());
    Ok(())
}
