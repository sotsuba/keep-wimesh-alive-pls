pub mod cli;
pub mod platform;
pub mod strategies;
pub mod watcher;

#[cfg(target_os = "linux")]
pub const USER_AGENT_FIREFOX: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:149.0) Gecko/20100101 Firefox/149.0";
#[cfg(target_os = "windows")]
pub const USER_AGENT_FIREFOX: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:149.0) Gecko/20100101 Firefox/149.0";

pub fn build_client(strategy: &dyn strategies::LoginStrategy) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(USER_AGENT_FIREFOX)
        .danger_accept_invalid_certs(true);

    if let Some(jar) = strategy.cookie_jar() {
        builder = builder.cookie_provider(jar);
    }

    let client = builder.build()?;
    Ok(client)
}
