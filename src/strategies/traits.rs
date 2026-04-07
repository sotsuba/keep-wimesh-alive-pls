use reqwest::cookie::Jar;
use reqwest::Client;
use anyhow::Result;

#[async_trait::async_trait]
pub trait LoginStrategy: Send + Sync {
    /// Execute the login flow for this hostspot.
    async fn login(&self, client: &Client, jar: &Jar) -> Result<()>;
}