use anyhow::{Context, Result, anyhow};
use regex::Regex;
use reqwest::Url;
use serde_json::{Value, json};

use crate::types::WifiInfo;

fn compile_regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("regex pattern must compile")
}

fn extract_wifi_info_json(html: &str) -> Result<String> {
    let re = compile_regex(r#"const\s+wifiInfo\s*=\s*(\{.*?\})\s*;"#);
    let object_literal = re
        .captures(html)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_owned()))
        .context("could not locate wifiInfo object in hotspot HTML")?;

    // wifiInfo contains JS octal escapes like \052 in CHAP fields. JSON does not allow
    // octal escapes, so preserve them as literal text by doubling the slash.
    let octal_escape_re = compile_regex(r#"\\([0-7]{1,3})"#);
    let object_literal = octal_escape_re
        .replace_all(&object_literal, r#"\\$1"#)
        .into_owned();

    let key_re = compile_regex(r#"([\{,])\s*([A-Za-z0-9_-]+)\s*:"#);
    let json_like = key_re.replace_all(&object_literal, "$1\"$2\":");

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
    let extract_field = |field: &str| -> Result<String> {
        let pattern = format!(r#"name=[\"']{field}[\"']\s+value=[\"']([^\"']+)[\"']"#);
        let re = compile_regex(&pattern);
        re.captures(html)
            .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            .ok_or_else(|| anyhow!("contentAuthenForm missing input field: {field}"))
    };

    Ok((
        extract_field("username")?,
        extract_field("password")?,
        extract_field("dst")?,
    ))
}

pub fn query_param(url: &str, key: &str) -> Result<String> {
    let parsed = Url::parse(url).context("invalid URL")?;
    parsed
        .query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| anyhow!("missing query param: {key}"))
}

/// Extract the `Qv` parameter from a Highland Coffee awing URL.
///
/// The `Qv` value uses a non-standard format containing unencoded `=` and `@`
/// characters (e.g. `Qv=it_qpmjdz=Dbqujwf.ID@bbb_qpmjdz=...`), so standard
/// URL query-pair parsing truncates it incorrectly.  This function finds `Qv=`
/// in the raw URL string and returns everything after it.
pub fn extract_qv_param(url: &str) -> Result<String> {
    // Work on the query-string portion only to avoid false positives in the host/path.
    let query = url.find('?').map(|i| &url[i + 1..]).unwrap_or(url);
    let marker = "Qv=";
    let pos = query
        .find(marker)
        .ok_or_else(|| anyhow!("missing Qv parameter in URL"))?;
    let after = &query[pos + marker.len()..];
    // Qv is always the last query parameter in the Highland URL; no '&' follows.
    // If one ever does appear, stop there so we don't bleed into the next param.
    let value = after.split('&').next().unwrap_or(after);
    Ok(value.to_string())
}

/// Parse `port` and `postToUrl` from the JS embedded in a Highland Coffee
/// `contentAuthenForm` HTML snippet.
///
/// Returns `(port, path)`, defaulting to `(880, "/cgi-bin/hslogin.cgi")` if
/// the values cannot be found in the script block.
pub fn parse_highland_script_params(html: &str) -> (u16, String) {
    let port = compile_regex(r"var\s+port\s*=\s*(\d+)")
        .captures(html)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u16>().ok())
        .unwrap_or(880);

    let path = compile_regex(r#"var\s+postToUrl\s*=\s*["']([^"']+)["']"#)
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_owned())
        .unwrap_or_else(|| "/cgi-bin/hslogin.cgi".to_string());

    (port, path)
}

pub fn build_wifi_info_for_check(w: &WifiInfo) -> Value {
    let chap_id = chap_id_to_digits(&w.chap_id_raw);
    let chap_challenge = chap_challenge_to_csv(&w.chap_challenge_raw);
    let link_login = format!("{}?dst={}", w.link_login_only, urlencoding::encode(&w.link_orig));

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
        "hostname": "free.wi-mesh.vn",
        "server-address": "172.172.0.1:80",
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
