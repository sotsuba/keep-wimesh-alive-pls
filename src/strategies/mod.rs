pub mod wimesh;
pub mod hcmus;
pub mod highland;
pub mod traits;
pub mod utils;
pub mod error;

pub use traits::LoginStrategy;
use error::StrategyError;

pub fn select_strategy(ssid: &str) -> Result<Box<dyn LoginStrategy>, StrategyError> {
    match ssid {
        x if x.contains("Wi-MESH") 
            => Ok(Box::new(wimesh::KtxWiMeshStrategy::new()))
        ,
        x if x.contains("Highlands Coffee") 
            => Ok(Box::new(highland::HighlandStrategy)),
        x if x.contains("HCMUS-STUDENT") || x.contains("HCMUS-PUBLIC") 
            => Ok(Box::new(hcmus::HcmusStrategy)), 
        _ 
            => Err(StrategyError::UnknownSSID(ssid.to_string()))
    }
}