//! HTML and JSON parsing utilities

use crate::models::{Credentials, GatewayConfig};
use anyhow::{anyhow, Result};
use regex::Regex;

/// Parse gateway configuration from captive portal HTML
pub fn parse_gateway_html(html: &str) -> Result<GatewayConfig> {
    fn extract_value(html: &str, key: &str) -> Option<String> {
        let pattern = format!(r#"["']?{}["']?\s*[:=]\s*["']([^"']+)["']"#, key);
        Regex::new(&pattern)
            .ok()?
            .captures(html)?
            .get(1)
            .map(|m| m.as_str().to_string())
    }

    let chap_challenge =
        extract_value(html, "chap_challenge").ok_or_else(|| anyhow!("chap_challenge not found"))?;

    Ok(GatewayConfig {
        mac: extract_value(html, "mac").unwrap_or_default(),
        ip: extract_value(html, "ip").unwrap_or_default(),
        chap_id: extract_value(html, "chap_id").unwrap_or_default(),
        chap_challenge,
        link_login_only: extract_value(html, "link-login-only").unwrap_or_default(),
    })
}

/// Parse credentials from authentication form HTML
pub fn parse_credentials(html: &str) -> Result<Credentials> {
    fn extract_input_value(html: &str, name: &str) -> Option<String> {
        // Try: <input ... name="xxx" ... value="yyy" ...>
        let pattern1 = format!(
            r#"<input[^>]*name=["']{}["'][^>]*value=["']([^"']*)["']"#,
            name
        );
        if let Some(caps) = Regex::new(&pattern1).ok()?.captures(html) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }

        // Try reverse: <input ... value="yyy" ... name="xxx" ...>
        let pattern2 = format!(
            r#"<input[^>]*value=["']([^"']*)["'][^>]*name=["']{}["']"#,
            name
        );
        Regex::new(&pattern2)
            .ok()?
            .captures(html)?
            .get(1)
            .map(|m| m.as_str().to_string())
    }

    let username =
        extract_input_value(html, "username").ok_or_else(|| anyhow!("username not found in form"))?;
    let password =
        extract_input_value(html, "password").ok_or_else(|| anyhow!("password not found in form"))?;

    Ok(Credentials { username, password })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gateway() {
        let html = r#"
            var mac = "AA:BB:CC:DD:EE:FF";
            var ip = "192.168.1.1";
            var chap_id = "12345";
            var chap_challenge = "abcdef123456";
            var link_login_only = "http://portal.local/login";
        "#;

        let gw = parse_gateway_html(html).unwrap();
        assert_eq!(gw.mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(gw.ip, "192.168.1.1");
        assert_eq!(gw.chap_challenge, "abcdef123456");
    }

    #[test]
    fn test_parse_credentials() {
        let html = r#"
            <form>
                <input type="hidden" name="username" value="user123">
                <input type="hidden" name="password" value="pass456">
            </form>
        "#;

        let creds = parse_credentials(html).unwrap();
        assert_eq!(creds.username, "user123");
        assert_eq!(creds.password, "pass456");
    }
}
