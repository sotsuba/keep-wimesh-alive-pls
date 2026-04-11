use reqwest::Client;
use anyhow::Result;

#[async_trait::async_trait]
pub trait LoginStrategy: Send + Sync {
    /// Execute the login flow for this hotspot.
    async fn login(&self, client: &Client) -> Result<()>;
}
