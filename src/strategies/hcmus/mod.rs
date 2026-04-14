use crate::strategies::LoginStrategy;
use anyhow::Result;
use reqwest::Client;
use tracing::info;

const PORTAL_LOGON_URL: &str = "http://10.232.0.1:8001/api/captiveportal/access/logon/";
const PORTAL_ORIGIN: &str = "http://10.232.0.1:8001";
const PORTAL_REFERER: &str =
    "http://10.232.0.1:8001/index.html?redirurl=detectportal.firefox.com/canonical.html";

pub struct HcmusStrategy;

#[async_trait::async_trait]
impl LoginStrategy for HcmusStrategy {
    async fn login(&self, client: &Client) -> Result<()> {
        let resp = client
            .post(PORTAL_LOGON_URL)
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Origin", PORTAL_ORIGIN)
            .header("Referer", PORTAL_REFERER)
            .form(&[("user", ""), ("password", "")])
            .send()
            .await?;
        info!("{}", resp.status());
        Ok(())
    }
}
