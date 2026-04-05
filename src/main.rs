mod config;
mod cookies;
mod parse;
mod types;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::Parser;
use reqwest::cookie::Jar;
use reqwest::header::{CONTENT_TYPE, COOKIE, HeaderMap, HeaderName, HeaderValue, ORIGIN, REFERER};
use reqwest::{Client, StatusCode, redirect::Policy};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};
use uuid::Uuid;

use config::{Cli, Config, USER_AGENT_FIREFOX};
use cookies::{AwingCookies, HotspotCookies, cookie_header_awing, cookie_header_hotspot,
              get_cookie_from_jar, get_optional_cookie};
use parse::{build_wifi_info_for_check, parse_login_form, parse_wifi_info, query_param,
            to_dash_mac};

async fn step_probe_and_get_login_cookies(
    client: &Client,
    config: &Config,
    jar: &Jar,
) -> Result<(HotspotCookies, String)> {
    info!("step1+2+3: probe, load hotspot, and request awing URL");

    let (probe_html, probe_headers) = match client.get(&config.probe_url).send().await {
        Ok(resp) => {
            let headers = resp.headers().clone();
            let body = resp.text().await.context("failed reading probe response")?;
            (body, Some(headers))
        }
        Err(err) => {
            warn!("probe failed ({err}); using logs/probe_sample_output.txt");
            (
                std::fs::read_to_string("logs/probe_sample_output.txt")
                    .context("failed to read logs/probe_sample_output.txt")?,
                None,
            )
        }
    };

    let first_wifi = parse_wifi_info(&probe_html)?;
    let login_url = first_wifi
        .raw
        .get("link-login")
        .and_then(Value::as_str)
        .unwrap_or(&first_wifi.link_login_only)
        .to_string();

    let response = client
        .get(&login_url)
        .send()
        .await
        .context("failed to load hotspot login page")?;

    let login_headers = response.headers().clone();

    let pick_cookie = |name: &str| {
        get_optional_cookie(&login_headers, name)
            .or_else(|| probe_headers.as_ref().and_then(|h| get_optional_cookie(h, name)))
            .or_else(|| get_cookie_from_jar(jar, "http://free.wi-mesh.vn/", name))
    };

    let derived_device_id = first_wifi
        .raw
        .get("mac-esc")
        .and_then(Value::as_str)
        .map(|m| m.replace("%3A", "").to_uppercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| first_wifi.mac.replace(':', "").to_uppercase());

    let check_captive = pick_cookie("checkCaptive");
    let hotspot_wm_device_id = pick_cookie("hotspot_wm_deviceId").unwrap_or_else(|| {
        warn!("hotspot_wm_deviceId cookie missing; using derived device id from MAC");
        derived_device_id.clone()
    });
    let hotspot_wm_token = pick_cookie("hotspot_wm_token");

    if check_captive.is_none() {
        warn!("checkCaptive cookie missing; continuing without it");
    }
    if hotspot_wm_token.is_none() {
        warn!("hotspot_wm_token cookie missing; continuing without it");
    }

    let cookies = HotspotCookies {
        check_captive,
        hotspot_wm_device_id,
        hotspot_wm_token,
    };

    let second_html = response
        .text()
        .await
        .context("failed reading hotspot page")?;
    let wifi = parse_wifi_info(&second_html).or_else(|_| parse_wifi_info(&probe_html))?;

    let body = json!({
        "device": {
            "deviceId": cookies.hotspot_wm_device_id,
            "userId": 0,
            "deviceName": config.device_name,
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

    let response = client
        .post("https://ex.login.net.vn/api-connect/check")
        .header(ORIGIN, HeaderValue::from_static("http://free.wi-mesh.vn"))
        .header(REFERER, HeaderValue::from_static("http://free.wi-mesh.vn/"))
        .json(&body)
        .send()
        .await
        .context("api-connect/check request failed")?;

    if response.status() != StatusCode::OK {
        bail!("api-connect/check returned status {}", response.status());
    }

    let payload: Value = response
        .json()
        .await
        .context("invalid JSON from api-connect/check")?;
    let awing_url = payload
        .get("data")
        .and_then(|d| d.get("url"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .context("api-connect/check missing data.url")?;

    Ok((cookies, awing_url))
}

async fn step_load_awing_portal(
    client: &Client,
    awing_url: &str,
    cookies1: &HotspotCookies,
) -> Result<AwingCookies> {
    info!("step4: load awing portal");

    let mut req = client
        .get(awing_url)
        .header(REFERER, HeaderValue::from_static("http://free.wi-mesh.vn/"));
    if let Some(cookie_header) = cookie_header_hotspot(cookies1) {
        req = req.header(COOKIE, cookie_header);
    }

    let response = req.send().await.context("failed to load awing portal")?;

    if !response.status().is_success() {
        bail!("awing portal returned status {}", response.status());
    }

    Ok(AwingCookies {
        ingresscookie: get_optional_cookie(response.headers(), "ingresscookie"),
        loading_index: get_optional_cookie(response.headers(), "loadingIndex"),
    })
}

/// Returns (token, Option<contentAuthenForm HTML>)
/// VerifyUrl already returns the pre-filled login form — GetCustomer is only needed as fallback.
async fn step_verify_url(
    client: &Client,
    awing_url: &str,
    cookies2: &AwingCookies,
) -> Result<(String, Option<String>)> {
    info!("step5: VerifyUrl -> token (+ optional pre-filled form)");

    let mut req = client
        .get("http://v1.awingconnect.vn/Home/VerifyUrl")
        .header(HeaderName::from_static("x-requested-with"), HeaderValue::from_static("XMLHttpRequest"))
        .header(ORIGIN, HeaderValue::from_static("http://v1.awingconnect.vn"))
        .header(REFERER, HeaderValue::from_str(awing_url).context("invalid awing URL for Referer")?)
        .header("Accept", "*/*");

    if let Some(cookie_header) = cookie_header_awing(cookies2) {
        req = req.header(COOKIE, cookie_header);
    }

    let response = req.send().await.context("VerifyUrl request failed")?;
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

    // VerifyUrl already embeds contentAuthenForm — use it to skip GetCustomer
    let form_html = payload
        .pointer("/captiveContext/contentAuthenForm")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    Ok((token, form_html))
}

async fn step_get_customer(
    client: &Client,
    token: &str,
    awing_url: &str,
    config: &Config,
    cookies2: &AwingCookies,
) -> Result<(String, String, String)> {
    info!("step6: GetCustomer -> username/password/dst");

    let serial = query_param(awing_url, "serial")?;
    let client_mac = query_param(awing_url, "client_mac")?;

    let body = json!({
        "token": token,
        "captiveContext": {
            "campaignData": {
                "sessionId": Uuid::new_v4().to_string(),
                "macAddress": to_dash_mac(&client_mac),
                "apMac": to_dash_mac(&serial),
                "placeId": config.place_id,
                "domainId": config.domain_id,
                "url": awing_url,
                "userAgent": USER_AGENT_FIREFOX,
                "campaignId": "0",
                "campaignGroupId": 0,
                "campaignAdId": 0,
                "campaignType": 0,
                "isNetworkCampaign": false,
                "loginId": "0",
                "loginHtml": null,
                "welcomeId": "0",
                "welcomeHtml": null
            },
            "customerActions": [0],
            "domain": null,
            "customer": null,
            "placeCustomerInfoCollections": [],
            "pageViewEvents": [],
            "customerRequiredFields": [],
            "contentAuthenForm": null,
            "createdDate": Utc::now().to_rfc3339()
        }
    });

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));
    headers.insert(
        HeaderName::from_static("x-requested-with"),
        HeaderValue::from_static("XMLHttpRequest"),
    );
    headers.insert(ORIGIN, HeaderValue::from_static("http://v1.awingconnect.vn"));
    headers.insert(
        REFERER,
        HeaderValue::from_str(awing_url).context("invalid awing URL for Referer")?,
    );
    if let Some(cookie_header) = cookie_header_awing(cookies2) {
        headers.insert(COOKIE, HeaderValue::from_str(&cookie_header).context("invalid awing cookies")?);
    }

    let response = client
        .post("http://v1.awingconnect.vn/Content/GetCustomer")
        .headers(headers)
        .json(&body)
        .send()
        .await
        .context("GetCustomer request failed")?;

    if !response.status().is_success() {
        bail!("GetCustomer returned status {}", response.status());
    }

    let payload: Value = response
        .json()
        .await
        .context("GetCustomer response is not JSON")?;

    let content_form = payload
        .pointer("/captiveContext/contentAuthenForm")
        .and_then(Value::as_str)
        .or_else(|| payload.get("contentAuthenForm").and_then(Value::as_str))
        .context("GetCustomer missing contentAuthenForm HTML")?;

    parse_login_form(content_form)
}

async fn step_post_login(
    client: &Client,
    username: &str,
    password: &str,
    dst: &str,
    cookies1: &HotspotCookies,
) -> Result<()> {
    info!("step7: POST final MikroTik login");

    let form = [
        ("username", username),
        ("password", password),
        ("dst", dst),
        ("popup", "false"),
    ];

    let response = client
        .post("http://free.wi-mesh.vn/login")
        .header(ORIGIN, HeaderValue::from_static("http://v1.awingconnect.vn"))
        .header(REFERER, HeaderValue::from_static("http://v1.awingconnect.vn/"))
        .form(&form);

    let response = if let Some(cookie_header) = cookie_header_hotspot(cookies1) {
        response
            .header(COOKIE, cookie_header)
            .send()
            .await
            .context("final hotspot login POST failed")?
    } else {
        response
            .send()
            .await
            .context("final hotspot login POST failed")?
    };

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

async fn login(client: &Client, config: &Config, jar: &Jar) -> Result<()> {
    let (cookies1, awing_url) = step_probe_and_get_login_cookies(client, config, jar).await?;
    let cookies2 = step_load_awing_portal(client, &awing_url, &cookies1).await?;
    let (token, verify_form) = step_verify_url(client, &awing_url, &cookies2).await?;

    // VerifyUrl already returns contentAuthenForm — use it directly to save one round trip.
    // Fall back to GetCustomer only if the form was absent.
    let (username, password, dst) = if let Some(form_html) = verify_form {
        info!("step6: credentials from VerifyUrl response (skipping GetCustomer)");
        parse_login_form(&form_html)?
    } else {
        step_get_customer(client, &token, &awing_url, config, &cookies2).await?
    };

    info!("waiting {} seconds before final login", config.timer);
    sleep(Duration::from_secs(config.timer)).await;

    if config.dry_run {
        info!("dry-run enabled: skipping final login POST");
        return Ok(());
    }

    step_post_login(client, &username, &password, &dst, &cookies1).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config: Config = Cli::parse().into();

    let cookie_jar = Arc::new(Jar::default());

    let client = Client::builder()
        .cookie_provider(cookie_jar.clone())
        .redirect(Policy::limited(10))
        .timeout(Duration::from_secs(20))
        .user_agent(USER_AGENT_FIREFOX)
        .build()
        .context("failed to construct HTTP client")?;

    login(&client, &config, cookie_jar.as_ref()).await
}
