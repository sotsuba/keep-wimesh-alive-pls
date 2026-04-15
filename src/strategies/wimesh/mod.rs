use super::LoginStrategy;
pub mod parse;

const HOTSPOT_ORIGIN: &str = "http://free.wi-mesh.vn";
const HOTSPOT_BASE: &str = "http://free.wi-mesh.vn/";
const HOTSPOT_CONFIG_URL: &str = "http://free.wi-mesh.vn/config.json";
const HOTSPOT_LOGIN_URL: &str = "http://free.wi-mesh.vn/login";
const AWING_VERIFY_URL: &str = "http://v1.awingconnect.vn/Home/VerifyUrl";
const AWING_ORIGIN: &str = "http://v1.awingconnect.vn";
const AWING_REFERER: &str = "http://v1.awingconnect.vn/";
const FALLBACK_SERVER: &str = "https://ex.login.net.vn";
use super::utils::{load_awing_portal, run_step};
use crate::strategies::wimesh::parse::{
    build_wifi_info_for_check, parse_login_form, parse_wifi_info,
};
use anyhow::{Context, Result, bail};
use reqwest::cookie::Jar;
use reqwest::header::{HeaderName, HeaderValue, ORIGIN, REFERER};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::info;
use tracing::warn;

pub struct KtxWiMeshStrategy {
    jar: Arc<Jar>,
}

pub static REGISTRY_ENTRY: super::RegistryStrategy = crate::strategies::RegistryStrategy {
    name: "KTX Wi-MESH",
    predicate: |ssid| ssid.contains("Free Wi-MESH"),
    factory: || Box::new(KtxWiMeshStrategy::new()),
};

impl KtxWiMeshStrategy {
    pub fn new() -> Self {
        Self {
            jar: Arc::new(Jar::default()),
        }
    }
}

impl Default for KtxWiMeshStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LoginStrategy for KtxWiMeshStrategy {
    async fn login(&self, client: &Client) -> Result<()> {
        let (first_wifi, wifi) = run_step(
            "step1+2: probe hotspot and load login page",
            probe_hotspot(client),
        )
        .await?;

        let body = build_check_body(&first_wifi, &wifi);

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

        let (_, form_html) = run_step(
            "step5: VerifyUrl -> token + pre-filled login form",
            step_verify_url(client, &awing_url),
        )
        .await?;

        let form_html = form_html.context("VerifyUrl did not return contentAuthenForm")?;
        info!("step6: credentials from VerifyUrl response");
        let (username, password, dst) = parse_login_form(&form_html)?;

        run_step(
            "step7: POST final MikroTik login",
            step_post_login(client, &username, &password, &dst),
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

/// Step 1+2: Load the hotspot login page directly.
/// Returns `(first_wifi, wifi)` — both parsed from the same page;
/// first_wifi.mac-esc is used for device_id, wifi for the api-connect/check body.
async fn probe_hotspot(client: &Client) -> Result<(parse::WifiInfo, parse::WifiInfo)> {
    let html = client
        .get(HOTSPOT_BASE)
        .send()
        .await
        .context("failed to load hotspot login page")?
        .text()
        .await
        .context("failed reading hotspot page")?;

    let wifi = parse_wifi_info(&html)?;
    Ok((wifi.clone(), wifi))
}

/// Build the JSON body for api-connect/check from probed wifi info.
fn build_check_body(first_wifi: &parse::WifiInfo, wifi: &parse::WifiInfo) -> Value {
    let device_id = first_wifi
        .raw
        .get("mac-esc")
        .and_then(Value::as_str)
        .map(|m| m.replace("%3A", "").to_uppercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| first_wifi.mac.replace(':', "").to_uppercase());

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

    let payload = check_payload.context("api-connect/check failed on all servers")?;
    let awing_url = payload
        .pointer("/data/url")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .context("api-connect/check missing data.url")?;

    // Inject the fresh token so the final POST to free.wi-mesh.vn sends it automatically.
    if let Some(token) = payload.pointer("/data/token").and_then(Value::as_str) {
        let url = HOTSPOT_BASE.parse::<reqwest::Url>().unwrap();
        jar.add_cookie_str(&format!("hotspot_wm_token={token}"), &url);
    }

    Ok(awing_url)
}

/// Returns (token, Option<contentAuthenForm HTML>)
async fn step_verify_url(client: &Client, awing_url: &str) -> Result<(String, Option<String>)> {
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

async fn step_post_login(client: &Client, username: &str, password: &str, dst: &str) -> Result<()> {
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
