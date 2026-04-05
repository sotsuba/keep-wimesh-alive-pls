use serde_json::Value;

#[derive(Debug, Clone)]
pub struct WifiInfo {
    pub raw: Value,
    pub mac: String,
    pub ip: String,
    pub identity: String,
    pub link_login_only: String,
    pub link_orig: String,
    pub chap_id_raw: String,
    pub chap_challenge_raw: String,
    pub interface_name: String,
    pub server_name: String,
}
