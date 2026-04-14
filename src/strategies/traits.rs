use anyhow::Result;
use reqwest::Client;

#[async_trait::async_trait]
pub trait LoginStrategy: Send + Sync {
    /// Execute the login flow for this hotspot.
    async fn login(&self, client: &Client) -> Result<()>;

    /// Return a cookie jar if this strategy needs to maintain session state.
    fn cookie_jar(&self) -> Option<std::sync::Arc<reqwest::cookie::Jar>> { None }
}
