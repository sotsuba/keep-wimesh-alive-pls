use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::HeaderMap;
use reqwest::Url;

#[derive(Debug, Clone)]
pub struct HotspotCookies {
    pub check_captive: Option<String>,
    pub hotspot_wm_device_id: String,
    pub hotspot_wm_token: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AwingCookies {
    pub ingresscookie: Option<String>,
    pub loading_index: Option<String>,
}

pub fn get_optional_cookie(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let prefix = format!("{cookie_name}=");
    for value in headers.get_all("set-cookie") {
        let Ok(raw) = value.to_str() else {
            continue;
        };
        if let Some(after_name) = raw.strip_prefix(&prefix) {
            let cookie_val = after_name.split(';').next().unwrap_or_default();
            if !cookie_val.is_empty() {
                return Some(cookie_val.to_string());
            }
        }
    }
    None
}

fn get_cookie_from_cookie_header(cookie_header: &str, cookie_name: &str) -> Option<String> {
    cookie_header.split(';').find_map(|part| {
        let trimmed = part.trim();
        let (name, value) = trimmed.split_once('=')?;
        if name == cookie_name {
            Some(value.to_string())
        } else {
            None
        }
    })
}

pub fn get_cookie_from_jar(jar: &Jar, url: &str, cookie_name: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let cookie_header = jar.cookies(&parsed)?;
    let header_str = cookie_header.to_str().ok()?;
    get_cookie_from_cookie_header(header_str, cookie_name)
}

pub fn cookie_header_hotspot(c: &HotspotCookies) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(v) = &c.check_captive {
        parts.push(format!("checkCaptive={v}"));
    }
    parts.push(format!("hotspot_wm_deviceId={}", c.hotspot_wm_device_id));
    if let Some(v) = &c.hotspot_wm_token {
        parts.push(format!("hotspot_wm_token={v}"));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

pub fn cookie_header_awing(c: &AwingCookies) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(v) = &c.ingresscookie {
        parts.push(format!("ingresscookie={v}"));
    }
    if let Some(v) = &c.loading_index {
        parts.push(format!("loadingIndex={v}"));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}
