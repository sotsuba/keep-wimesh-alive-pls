use crate::strategies::LoginStrategy;
use anyhow::Result;
use reqwest::Client;

pub struct HcmusStrategy;

pub static REGISTRY_ENTRY: super::RegistryStrategy = super::RegistryStrategy {
    name: "HCMUS",
    predicate: |ssid| ssid.contains("HCMUS"),
    factory: || Box::new(HcmusStrategy),
};

#[async_trait::async_trait]
impl LoginStrategy for HcmusStrategy {
    async fn login(&self, client: &Client) -> Result<()> {
        let ip = crate::platform::select_platform().unwrap().get_wifi_ipv4_address().unwrap();
        let url = format!("http://{}/api/captiveportal/access/status/", ip);
        let resp = client.post(&url)
            .send()
            .await?;
        
        if resp.status().is_success() {
            return Ok(());
        }
        else {
            anyhow::bail!("unexpected response from captive portal: {}", resp.status());
        }
    }
}
