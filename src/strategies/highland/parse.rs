use anyhow::{Context, Result, anyhow};
use regex::Regex;
use reqwest::Url;
use std::sync::LazyLock;

static RE_PORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"var\s+port\s*=\s*(\d+)").unwrap());
static RE_POST_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"var\s+postToUrl\s*=\s*["']([^"']+)["']"#).unwrap());

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
    let port = RE_PORT
        .captures(html)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u16>().ok())
        .unwrap_or(880);

    let path = RE_POST_URL
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_owned())
        .unwrap_or_else(|| "/cgi-bin/hslogin.cgi".to_string());

    (port, path)
}
