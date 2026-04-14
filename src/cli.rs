use clap::Parser;

pub const USER_AGENT_FIREFOX: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:149.0) Gecko/20100101 Firefox/149.0";

#[derive(Parser, Debug, Clone)]
#[command(name = "keep_wimesh_session")]
#[command(about = "Automate hotspot login flow")]
pub struct Cli {
    pub ssid: String,
}