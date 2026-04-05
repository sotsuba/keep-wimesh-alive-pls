use clap::Parser;

pub const USER_AGENT_FIREFOX: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:149.0) Gecko/20100101 Firefox/149.0";

#[derive(Parser, Debug)]
#[command(name = "keep_wimesh_session")]
#[command(about = "Automate wi-mesh.com captive portal login flow")]
pub struct Cli {
    #[arg(long, default_value = "http://login.net.vn/")]
    pub probe_url: String,

    #[arg(long, default_value = "MyDevice")]
    pub device_name: String,

    #[arg(long, default_value_t = 7)]
    pub ad_wait_seconds: u64,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long, default_value = "4673982451984183424")]
    pub place_id: String,

    #[arg(long, default_value = "5476089696408447131")]
    pub domain_id: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub probe_url: String,
    pub device_name: String,
    pub timer: u64,
    pub dry_run: bool,
    pub place_id: String,
    pub domain_id: String,
}

impl From<Cli> for Config {
    fn from(value: Cli) -> Self {
        Self {
            probe_url: value.probe_url,
            device_name: value.device_name,
            timer: value.ad_wait_seconds,
            dry_run: value.dry_run,
            place_id: value.place_id,
            domain_id: value.domain_id,
        }
    }
}
