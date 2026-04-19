use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "captive_portal")]
#[command(about = "Automate hotspot login flow")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the login flow once for the given SSID.
    Login(LoginArgs),
    /// Start the watchdog loop: monitors connectivity and re-logins automatically.
    Watch(WatchArgs),
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    pub ssid: String,
}

#[derive(Args, Debug)]
pub struct WatchArgs {
    /// URL to probe for connectivity (expects HTTP 204).
    #[arg(
        long,
        default_value = "http://connectivitycheck.gstatic.com/generate_204",
        env = "WIMESH_CHECK_URL"
    )]
    pub check_url: String,

    /// Seconds between connectivity checks.
    #[arg(long, default_value_t = 3, env = "WIMESH_CHECK_INTERVAL")]
    pub check_interval: u64,

    /// Seconds to wait after a successful login before the next check.
    #[arg(long, default_value_t = 3, env = "WIMESH_POST_LOGIN_WAIT")]
    pub post_login_wait: u64,

    /// Base retry backoff in seconds (doubles on each consecutive failure).
    #[arg(long, default_value_t = 10, env = "WIMESH_RETRY_BASE_SECONDS")]
    pub retry_base: u64,

    /// Maximum retry backoff in seconds.
    #[arg(long, default_value_t = 120, env = "WIMESH_RETRY_MAX_SECONDS")]
    pub retry_max: u64,
}
