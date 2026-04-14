use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde_json::{Value, json};
use std::sync::LazyLock;

static RE_WIFI_INFO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"const\s+wifiInfo\s*=\s*(\{.*?\})\s*;"#).unwrap());
static RE_OCTAL_ESCAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\\([0-7]{1,3})"#).unwrap());
static RE_JS_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"([\{,])\s*([A-Za-z0-9_-]+)\s*:"#).unwrap());
static RE_FIELD_USERNAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"name=["']username["']\s+value=["']([^"']+)["']"#).unwrap());
static RE_FIELD_PASSWORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"name=["']password["']\s+value=["']([^"']+)["']"#).unwrap());
static RE_FIELD_DST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"name=["']dst["']\s+value=["']([^"']+)["']"#).unwrap());

const HOTSPOT_HOSTNAME: &str = "free.wi-mesh.vn";
const HOTSPOT_SERVER_ADDR: &str = "172.172.0.1:80";

#[derive(Debug, Clone)]
pub struct WifiInfo {
    pub raw: Value,
    pub mac: String,
    pub ip: String,
    pub identity: String,
    pub link_login_only: String,
    pub link_orig: String,
    pub chap_id_raw: String,
    pub chap_challenge_raw: String,
    pub interface_name: String,
    pub server_name: String,
}

fn extract_wifi_info_json(html: &str) -> Result<String> {
    let object_literal = RE_WIFI_INFO
        .captures(html)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_owned()))
        .context("could not locate wifiInfo object in hotspot HTML")?;

    // wifiInfo contains JS octal escapes like \052 in CHAP fields. JSON does not allow
    // octal escapes, so preserve them as literal text by doubling the slash.
    let object_literal = RE_OCTAL_ESCAPE
        .replace_all(&object_literal, r#"\\$1"#)
        .into_owned();

    let json_like = RE_JS_KEY.replace_all(&object_literal, "$1\"$2\":");

    Ok(json_like.into_owned())
}

pub fn parse_wifi_info(html: &str) -> Result<WifiInfo> {
    let wifi_json = extract_wifi_info_json(html)?;
    let raw: Value = serde_json::from_str(&wifi_json).context("failed to parse wifiInfo JSON")?;

    let get = |k: &str, src: &Value| {
        src.get(k)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow!("wifiInfo missing key: {k}"))
    };

    Ok(WifiInfo {
        mac: get("mac", &raw)?,
        ip: get("ip", &raw)?,
        identity: get("identity", &raw)?,
        link_login_only: get("link-login-only", &raw)?,
        link_orig: get("link-orig", &raw)?,
        chap_id_raw: get("chap_id", &raw)?,
        chap_challenge_raw: get("chap_challenge", &raw)?,
        interface_name: get("interface-name", &raw)?,
        server_name: get("server-name", &raw)?,
        raw,
    })
}

fn chap_id_to_digits(chap_id_raw: &str) -> String {
    chap_id_raw.trim_start_matches('\\').to_string()
}

fn chap_challenge_to_csv(chap_raw: &str) -> String {
    chap_raw
        .split('\\')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

pub fn parse_login_form(html: &str) -> Result<(String, String, String)> {
    let extract = |re: &Regex, field: &str| -> Result<String> {
        re.captures(html)
            .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            .ok_or_else(|| anyhow!("contentAuthenForm missing input field: {field}"))
    };

    Ok((
        extract(&RE_FIELD_USERNAME, "username")?,
        extract(&RE_FIELD_PASSWORD, "password")?,
        extract(&RE_FIELD_DST, "dst")?,
    ))
}

pub fn build_wifi_info_for_check(w: &WifiInfo) -> Value {
    let chap_id = chap_id_to_digits(&w.chap_id_raw);
    let chap_challenge = chap_challenge_to_csv(&w.chap_challenge_raw);
    let link_login = format!(
        "{}?dst={}",
        w.link_login_only,
        urlencoding::encode(&w.link_orig)
    );

    json!({
        "identity": w.identity,
        "mac": w.mac,
        "ip": w.ip,
        "username": "",
        "link-login": link_login,
        "link-orig": w.link_orig,
        "error": "",
        "error-code": "",
        "link-login-only": w.link_login_only,
        "link-orig-esc": w.raw.get("link-orig-esc").and_then(Value::as_str).unwrap_or(""),
        "mac-esc": w.raw.get("mac-esc").and_then(Value::as_str).unwrap_or(""),
        "link-advert": w.raw.get("link-advert").and_then(Value::as_str).unwrap_or(""),
        "link-logout": w.raw.get("link-logout").and_then(Value::as_str).unwrap_or(""),
        "login-by": "",
        "bytes-in-nice": "0 B",
        "bytes-out-nice": "0 B",
        "istrial": "yes",
        "chap_id": chap_id,
        "chap_challenge": chap_challenge,
        "session-time-left": "",
        "uptime": "0s",
        "blocked": "no",
        "login-by-mac": "no",
        "interface-name": w.interface_name,
        "hostname": HOTSPOT_HOSTNAME,
        "server-address": HOTSPOT_SERVER_ADDR,
        "server-name": w.server_name,
        "domain": "",
        "session-id": "",
        "logged-in": "no",
        "host-ip": "0.0.0.0",
        "refresh-timeout": "",
        "refresh-timeout-secs": "0",
        "remain-bytes-in": "",
        "remain-bytes-out": "",
        "bytes-in": "0",
        "bytes-out": "0",
        "route": "/login",
        "is-captive": false
    })
}
