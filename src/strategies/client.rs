use anyhow::Result;
use reqwest::Client;

use crate::platform::Platform;
use crate::strategies::{LoginStrategy, select_login_strategy};

#[cfg(target_os = "linux")]
pub const USER_AGENT_FIREFOX: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:149.0) Gecko/20100101 Firefox/149.0";

#[cfg(target_os = "windows")]
pub const USER_AGENT_FIREFOX: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:149.0) Gecko/20100101 Firefox/149.0";

pub fn build_client(strategy: Option<&dyn LoginStrategy>) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(USER_AGENT_FIREFOX)
        .danger_accept_invalid_certs(true);

    if let Some(s) = strategy
        && let Some(jar) = s.cookie_jar()
    {
        builder = builder.cookie_provider(jar);
    }

    let client = builder.build()?;
    Ok(client)
}

pub struct LoginSession {
    strategy: Box<dyn LoginStrategy>,
    client: Client,
}

impl LoginSession {
    pub fn for_ssid(ssid: &str, platform: &dyn Platform) -> Result<Self> {
        let strategy = select_login_strategy(ssid, platform)?;
        let client = build_client(Some(strategy.as_ref()))?;
        Ok(Self { strategy, client })
    }

    pub async fn login(&self) -> Result<()> {
        self.strategy.login(&self.client).await
    }
}
