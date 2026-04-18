use anyhow::Result;
pub trait LockGuard: Send {}

pub trait Platform {
    fn name(&self) -> &'static str;
    fn detect_ssid(&self) -> Option<String>;
    fn get_wifi_ipv4_address(&self) -> Option<String>;
    fn ping_gateway(&self, addr: &str) -> bool;
    fn acquire_lock(&self) -> Result<Box<dyn LockGuard>>;
}