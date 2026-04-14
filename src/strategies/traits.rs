use anyhow::Result;
use reqwest::Client;

#[async_trait::async_trait]
pub trait LoginStrategy: Send + Sync {
    /// Execute the login flow for this hotspot.
    async fn login(&self, client: &Client) -> Result<()>;

    /// Return a cookie jar if this strategy needs to maintain session state.
    fn cookie_jar(&self) -> Option<std::sync::Arc<reqwest::cookie::Jar>> {
        None
    }
    /// Check if the session is still alive by making a request to a known URL and checking for expected content.
    async fn is_session_alive(&self, client: &Client) -> bool {
        let probe_url = "http://neverssl.com";
        let response = client.get(probe_url).send().await;

        match response {
            Ok(res) => {
                let final_url = res.url().as_str();
                // If final URL is still the probe URL, session is alive. If it redirects to a login page, session is likely dead.
                if !final_url.contains("neverssl.com") {
                    tracing::info!(
                        "Session appears to be dead; final URL after probe was: {}",
                        final_url
                    );
                    false
                } else {
                    tracing::info!(
                        "Session appears to be alive; final URL after probe was: {}",
                        final_url
                    );
                    true
                }
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeLoginStrategy;
    #[async_trait::async_trait]
    impl LoginStrategy for FakeLoginStrategy {
        async fn login(&self, _client: &Client) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    #[ignore = "This test is meant to verify the is_session_alive logic in isolation and does not require a real captive portal environment"]
    async fn test_is_session_alive() {
        let strategy = FakeLoginStrategy;
        let client = Client::new();
        assert!(strategy.is_session_alive(&client).await);
    }

    #[tokio::test]
    #[ignore = "This test requires an actual captive portal environment to be meaningful"]
    async fn test_real_environment_session_alive() {
        let strategy = FakeLoginStrategy;

        // Khởi tạo client thật
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();

        let is_alive = strategy.is_session_alive(&client).await;

        println!("Real environment check: is_session_alive = {}", is_alive);
        assert!(
            is_alive,
            "Session should be alive if you have internet access and are authenticated"
        );
    }
}
