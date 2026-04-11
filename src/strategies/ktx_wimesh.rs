use std::sync::Arc;
use reqwest::Client;
use reqwest::cookie::Jar;
use reqwest::header::{HeaderName, HeaderValue, ORIGIN, REFERER};
use reqwest::StatusCode;
use serde_json::{Value, json};
use anyhow::{Context, Result, bail};
use tracing::{info, warn};

use crate::parse::{build_wifi_info_for_check, parse_login_form, parse_wifi_info};
use crate::step::run_step;
use crate::strategies::traits::LoginStrategy;
use super::utils;

pub struct KtxWiMeshStrategy {
    jar: Arc<Jar>,
}

impl KtxWiMeshStrategy {
    pub fn new(jar: Arc<Jar>) -> Self {
        Self { jar }
    }
}

#[async_trait::async_trait]
impl LoginStrategy for KtxWiMeshStrategy {
    async fn login(&self, client: &Client) -> Result<()> {
        let awing_url = run_step(
            "step1+2+3: probe, load hotspot, and request awing URL",
            step_probe_and_get_login_cookies(client, &self.jar),
        ).await?;

        run_step(
            "step4: load awing portal",
            utils::load_awing_portal(client, &awing_url, Some("http://free.wi-mesh.vn/")),
        ).await?;

        let (_, form_html) = run_step(
            "step5: VerifyUrl -> token + pre-filled login form",
            step_verify_url(client, &awing_url),
        ).await?;

        let form_html = form_html.context("VerifyUrl did not return contentAuthenForm")?;
        info!("step6: credentials from VerifyUrl response");
        let (username, password, dst) = parse_login_form(&form_html)?;

        run_step(
            "step7: POST final MikroTik login",
            step_post_login(client, &username, &password, &dst),
        ).await?;

        Ok(())
    }
}


async fn fetch_server_list(client: &Client) -> Vec<String> {
    match try_fetch_server_list(client).await {
        Some(servers) => servers,
        None => {
            warn!("config.json unavailable, using hardcoded fallback server");
            vec!["https://ex.login.net.vn".to_string()]
        }
    }
}

async fn try_fetch_server_list(client: &Client) -> Option<Vec<String>> {
    let config: Value = client
        .get("http://free.wi-mesh.vn/config.json")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let list = config.get("listServer").and_then(Value::as_array)?;
    let servers: Vec<String> = list
        .iter()
        .filter_map(|v| v.as_str().map(ToOwned::to_owned))
        .collect();
    if servers.is_empty() { None } else { Some(servers) }
}

async fn step_probe_and_get_login_cookies(
    client: &Client,
    jar: &Jar,
) -> Result<String> {
    // The client follows the 302 redirect and the jar stores Set-Cookie headers automatically.
    let probe_html = client
        .get("http://login.net.vn/")
        .send()
        .await
        .context("probe request to login.net.vn failed")?
        .text()
        .await
        .context("failed reading probe response")?;

    let first_wifi = parse_wifi_info(&probe_html)?;
    let login_url = first_wifi
        .raw
        .get("link-login")
        .and_then(Value::as_str)
        .unwrap_or(&first_wifi.link_login_only)
        .to_string();

    let login_html = client
        .get(&login_url)
        .send()
        .await
        .context("failed to load hotspot login page")?
        .text()
        .await
        .context("failed reading hotspot page")?;

    let wifi = parse_wifi_info(&login_html).or_else(|_| parse_wifi_info(&probe_html))?;

    let device_id = first_wifi
        .raw
        .get("mac-esc")
        .and_then(Value::as_str)
        .map(|m| m.replace("%3A", "").to_uppercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| first_wifi.mac.replace(':', "").to_uppercase());

    let servers = fetch_server_list(client).await;

    let body = json!({
        "device": {
            "deviceId": device_id,
            "userId": 0,
            "deviceName": "MyDevice",
            "os": "",
            "osVersion": "",
            "appVersion": "",
            "network": "WiFi",
            "type": "",
            "status": "",
            "createdAt": null,
            "updatedAt": null,
            "deletedAt": null,
            "expiredAt": null
        },
        "wifiInfo": build_wifi_info_for_check(&wifi)
    });

    let mut check_payload: Option<Value> = None;
    for server in &servers {
        let url = format!("{}/api-connect/check", server);
        let result = client
            .post(&url)
            .header(ORIGIN, HeaderValue::from_static("http://free.wi-mesh.vn"))
            .header(REFERER, HeaderValue::from_static("http://free.wi-mesh.vn/"))
            .json(&body)
            .send()
            .await;
        match result {
            Ok(resp) if resp.status() == StatusCode::OK => {
                match resp.json::<Value>().await {
                    Ok(payload) if payload.pointer("/data/url").is_some() => {
                        check_payload = Some(payload);
                        break;
                    }
                    _ => warn!("api-connect/check at {} returned unexpected JSON", server),
                }
            }
            Ok(resp) => warn!("api-connect/check at {} returned status {}", server, resp.status()),
            Err(e) => warn!("api-connect/check at {} failed: {}", server, e),
        }
    }

    let payload = check_payload.context("api-connect/check failed on all servers")?;
    let awing_url = payload
        .pointer("/data/url")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .context("api-connect/check missing data.url")?;

    // Inject the fresh token into the jar so the final POST to free.wi-mesh.vn sends it automatically.
    if let Some(new_token) = payload.pointer("/data/token").and_then(Value::as_str) {
        let url = "http://free.wi-mesh.vn/".parse::<reqwest::Url>().unwrap();
        jar.add_cookie_str(&format!("hotspot_wm_token={}", new_token), &url);
    }

    Ok(awing_url)
}

/// Returns (token, Option<contentAuthenForm HTML>)
async fn step_verify_url(
    client: &Client,
    awing_url: &str,
) -> Result<(String, Option<String>)> {
    let response = client
        .get("http://v1.awingconnect.vn/Home/VerifyUrl")
        .header(HeaderName::from_static("x-requested-with"), HeaderValue::from_static("XMLHttpRequest"))
        .header(ORIGIN, HeaderValue::from_static("http://v1.awingconnect.vn"))
        .header(REFERER, HeaderValue::from_str(awing_url).context("invalid awing URL for Referer")?)
        .header("Accept", "*/*")
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

    let token = payload
        .get("token")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .context("VerifyUrl response missing token")?;

    let form_html = payload
        .pointer("/captiveContext/contentAuthenForm")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    Ok((token, form_html))
}

async fn step_post_login(
    client: &Client,
    username: &str,
    password: &str,
    dst: &str,
) -> Result<()> {
    let response = client
        .post("http://free.wi-mesh.vn/login")
        .header(ORIGIN, HeaderValue::from_static("http://v1.awingconnect.vn"))
        .header(REFERER, HeaderValue::from_static("http://v1.awingconnect.vn/"))
        .form(&[
            ("username", username),
            ("password", password),
            ("dst", dst),
            ("popup", "false"),
        ])
        .send()
        .await
        .context("final hotspot login POST failed")?;

    if !response.status().is_success() {
        bail!("hotspot login returned status {}", response.status());
    }

    let body = response
        .text()
        .await
        .context("failed reading hotspot login response body")?;
    if !body.to_ascii_lowercase().contains("you are logged in") {
        warn!("login POST succeeded but success phrase was not found in response body");
    }

    Ok(())
}
