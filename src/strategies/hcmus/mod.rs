use crate::strategies::LoginStrategy;
use anyhow::{Result, anyhow};
use reqwest::Client;
use tracing::info;

const PORTAL_DETECT_URL: &str = "http://detectportal.firefox.com/canonical.html";
const PORTAL_LOGON_PATH: &str = "/api/captiveportal/access/logon/";

async fn detect_portal_origin(client: &Client) -> Result<String> {
    let resp = client.get(PORTAL_DETECT_URL).send().await?;
    let url = resp.url();

    // Captive portal redirects probe URL away from original host
    if url.host_str() == Some("detectportal.firefox.com") {
        return Err(anyhow!(
            "No captive portal detected or already authenticated"
        ));
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("No host in redirect URL"))?;
    let mut origin = format!("{}://{}", url.scheme(), host);
    if let Some(port) = url.port() {
        origin.push_str(&format!(":{}", port));
    }
    Ok(origin)
}

pub struct HcmusStrategy;

pub static REGISTRY_ENTRY: super::RegistryStrategy = super::RegistryStrategy {
    name: "HCMUS",
    predicate: |ssid| ssid.contains("HCMUS"),
    factory: || Box::new(HcmusStrategy),
};

#[async_trait::async_trait]
impl LoginStrategy for HcmusStrategy {
    async fn login(&self, client: &Client) -> Result<()> {
        let origin = detect_portal_origin(client).await?;
        let logon_url = format!("{}{}", origin, PORTAL_LOGON_PATH);
        let referer = format!(
            "{}/index.html?redirurl=detectportal.firefox.com/canonical.html",
            origin
        );

        let resp = client
            .post(&logon_url)
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Origin", &origin)
            .header("Referer", &referer)
            .form(&[("user", ""), ("password", "")])
            .send()
            .await?;
        info!("{}", resp.status());
        Ok(())
    }
}
