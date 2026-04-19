#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StrategyError {
    #[error("Unknown SSID: {0}")]
    UnknownSSID(String),
}
