use anyhow::Result;
use reqwest::Client;

use crate::build_client;
use crate::platform::Platform;
use crate::strategies::{LoginStrategy, select_strategy};

/// A ready-to-use login session: a strategy paired with its configured HTTP client.
pub struct LoginSession {
    strategy: Box<dyn LoginStrategy>,
    client: Client,
}

impl LoginSession {
    pub fn for_ssid(ssid: &str, platform: &dyn Platform) -> Result<Self> {
        let strategy = select_strategy(ssid, platform)?;
        let client = build_client(strategy.as_ref())?;
        Ok(Self { strategy, client })
    }

    pub async fn login(&self) -> Result<()> {
        self.strategy.login(&self.client).await
    }
}
