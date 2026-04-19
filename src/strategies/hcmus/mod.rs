use crate::strategies::LoginStrategy;
use anyhow::{Context, Result, bail};
use reqwest::Client;
use reqwest::header::{HeaderValue, ORIGIN};
use serde::Deserialize;

const CAPTIVE_PORT: u16 = 8001;

pub struct HcmusStrategy {
    base_url: String,
}

pub static REGISTRY_ENTRY: super::RegistryStrategy = super::RegistryStrategy {
    name: "HCMUS",
    predicate: |ssid| ssid.contains("HCMUS"),
    factory: |platform| {
        let ip = platform
            .get_wifi_ipv4_address()
            .context("could not determine Wi-Fi gateway IP address")?;
        Ok(Box::new(HcmusStrategy {
            base_url: format!("http://{}:{}", ip, CAPTIVE_PORT),
        }))
    },
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogonResponse {
    client_state: String,
}

#[async_trait::async_trait]
impl LoginStrategy for HcmusStrategy {
    async fn login(&self, client: &Client) -> Result<()> {
        let origin =
            HeaderValue::from_str(&self.base_url).context("invalid base URL for Origin header")?;

        let logon_url = format!("{}/api/captiveportal/access/logon/", self.base_url);
        let resp: LogonResponse = client
            .post(&logon_url)
            .header(ORIGIN, origin)
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&[("user", ""), ("password", "")])
            .send()
            .await
            .context("logon request failed")?
            .json()
            .await
            .context("logon response is not valid JSON")?;

        if resp.client_state == "AUTHORIZED" {
            Ok(())
        } else {
            bail!("logon failed: clientState = {}", resp.client_state);
        }
    }
}
