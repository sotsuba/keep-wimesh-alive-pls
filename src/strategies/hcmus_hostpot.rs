use reqwest::Client; 
use reqwest::cookie::Jar;
use anyhow::Result;
use crate::strategies::LoginStrategy;
use tracing::info;

pub struct HcmusStudentStrategy;

#[async_trait::async_trait]
impl LoginStrategy for HcmusStudentStrategy {
    async fn login(&self, client: &Client, jar: &Jar) -> Result<()> {
        let resp = client.post("http://10.232.0.1:8001/api/captiveportal/access/logon/")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Origin", "http://10.232.0.1:8001")
            .header(
                "Referer",
                "http://10.232.0.1:8001/index.html?redirurl=detectportal.firefox.com/canonical.html",
            )
            .form(&[("user", ""), ("password", "")])
            .send()
            .await?;    
        info!("{}", resp.status());
        Ok(())        
    }
}