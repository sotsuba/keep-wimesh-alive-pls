use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum StrategyError {
    UnknownSSID(String),
}

impl fmt::Display for StrategyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StrategyError::UnknownSSID(ssid) => write!(f, "Unknown SSID: {}", ssid),
        }
    }
}

impl Error for StrategyError {}
