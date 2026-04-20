use super::LoginStrategy;
pub mod parse;

// All Wi-MESH hotspot URLs share the same origin.
const HOTSPOT_ORIGIN: &str = "http://free.wi-mesh.vn";
const HOTSPOT_BASE: &str = "http://free.wi-mesh.vn/";
const HOTSPOT_CONFIG_URL: &str = "http://free.wi-mesh.vn/config.json";
const HOTSPOT_LOGIN_URL: &str = "http://free.wi-mesh.vn/login";
const FALLBACK_SERVER: &str = "https://ex.login.net.vn";

use super::awing_utils::{
    AWING_ORIGIN, AWING_REFERER, call_verify_url, load_awing_portal, run_step,
};
use crate::strategies::error::StrategyError;
use crate::strategies::wimesh::parse::{
    build_wifi_info_for_check, parse_login_form, parse_wifi_info,
};
use anyhow::{Context, Result};
use reqwest::cookie::Jar;
use reqwest::header::{HeaderValue, ORIGIN, REFERER};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Default)]
pub struct WiMeshStrategy {
    jar: Arc<Jar>,
}

pub static REGISTRY_ENTRY: super::RegistryStrategy = crate::strategies::RegistryStrategy {
    name: "VNU-HCM Dormitory Zone B Wi-MESH",
    predicate: |ssid| ssid.contains("Free Wi-MESH"),
    factory: |_platform| Ok(Box::new(WiMeshStrategy::default())),
};

#[async_trait::async_trait]
impl LoginStrategy for WiMeshStrategy {
    async fn login(&self, client: &Client) -> Result<()> {
        let wifi = run_step(
            "step1+2: probe hotspot and load login page",
            probe_hotspot(client),
        )
        .await?;

        let body = build_check_body(&wifi);

        let awing_url = run_step(
            "step3: api-connect/check → awing URL",
            call_api_connect_check(client, &body, &self.jar),
        )
        .await?;

        run_step(
            "step4: load awing portal",
            load_awing_portal(client, &awing_url, Some(HOTSPOT_BASE)),
        )
        .await?;

        let form_html = run_step(
            "step5: VerifyUrl -> pre-filled login form",
            call_verify_url(client, &awing_url),
        )
        .await?
        .form_html
        .context("VerifyUrl did not return contentAuthenForm")?;

        info!("step6: credentials from VerifyUrl response");
        let (username, password, dst) = parse_login_form(&form_html)?;

        run_step(
            "step7: POST final MikroTik login",
            post_login(client, &username, &password, &dst),
        )
        .await?;

        Ok(())
    }

    fn cookie_jar(&self) -> Option<std::sync::Arc<reqwest::cookie::Jar>> {
        Some(self.jar.clone())
    }
}

async fn fetch_server_list(client: &Client) -> Vec<String> {
    match try_fetch_server_list(client).await {
        Some(servers) => servers,
        None => {
            warn!("config.json unavailable, using hardcoded fallback server");
            vec![FALLBACK_SERVER.to_string()]
        }
    }
}

async fn try_fetch_server_list(client: &Client) -> Option<Vec<String>> {
    let config: Value = client
        .get(HOTSPOT_CONFIG_URL)
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
    if servers.is_empty() {
        None
    } else {
        Some(servers)
    }
}

/// Step 1+2: Load the hotspot login page directly and parse Wi-Fi info.
async fn probe_hotspot(client: &Client) -> Result<parse::WifiInfo> {
    let html = client
        .get(HOTSPOT_BASE)
        .send()
        .await
        .context("failed to load hotspot login page")?
        .text()
        .await
        .context("failed reading hotspot page")?;

    parse_wifi_info(&html)
}

/// Build the JSON body for api-connect/check from probed wifi info.
fn build_check_body(wifi: &parse::WifiInfo) -> Value {
    let device_id = wifi
        .raw
        .get("mac-esc")
        .and_then(Value::as_str)
        .map(|m| m.replace("%3A", "").to_uppercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| wifi.mac.replace(':', "").to_uppercase());

    json!({
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
        "wifiInfo": build_wifi_info_for_check(wifi)
    })
}

/// POST body to each server's api-connect/check, inject the returned token into the jar,
/// and return the awing redirect URL.
async fn call_api_connect_check(client: &Client, body: &Value, jar: &Jar) -> Result<String> {
    let servers = fetch_server_list(client).await;
    let mut check_payload: Option<Value> = None;

    for server in &servers {
        let url = format!("{}/api-connect/check", server);
        let result = client
            .post(&url)
            .header(ORIGIN, HeaderValue::from_static(HOTSPOT_ORIGIN))
            .header(REFERER, HeaderValue::from_static(HOTSPOT_BASE))
            .json(body)
            .send()
            .await;
        match result {
            Ok(resp) if resp.status() == StatusCode::OK => match resp.json::<Value>().await {
                Ok(payload) if payload.pointer("/data/url").is_some() => {
                    check_payload = Some(payload);
                    break;
                }
                _ => warn!("api-connect/check at {} returned unexpected JSON", server),
            },
            Ok(resp) => warn!(
                "api-connect/check at {} returned status {}",
                server,
                resp.status()
            ),
            Err(e) => warn!("api-connect/check at {} failed: {}", server, e),
        }
    }

    let payload = check_payload.ok_or(StrategyError::AllServersFailed {
        operation: "api-connect/check",
    })?;

    let awing_url = payload
        .pointer("/data/url")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| StrategyError::MissingField {
            field: "data.url".to_string(),
            location: "api-connect/check response",
        })?;

    // Inject the fresh token so the final POST to free.wi-mesh.vn sends it automatically.
    if let Some(token) = payload.pointer("/data/token").and_then(Value::as_str) {
        let url = HOTSPOT_BASE
            .parse::<reqwest::Url>()
            .expect("HOTSPOT_BASE is a valid URL");
        jar.add_cookie_str(&format!("hotspot_wm_token={token}"), &url);
    }

    Ok(awing_url)
}

async fn post_login(client: &Client, username: &str, password: &str, dst: &str) -> Result<()> {
    let response = client
        .post(HOTSPOT_LOGIN_URL)
        .header(ORIGIN, HeaderValue::from_static(AWING_ORIGIN))
        .header(REFERER, HeaderValue::from_static(AWING_REFERER))
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
        return Err(StrategyError::UnexpectedStatus {
            endpoint: "hotspot login",
            status: response.status(),
        }
        .into());
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
