pub mod ktx_wimesh;
pub mod hcmus;
pub mod highland;
pub mod traits;
pub mod utils;
pub use traits::LoginStrategy;
use std::sync::Arc;
use reqwest::cookie::Jar;
use tracing::{info, warn};

pub fn select_strategy(config: &super::config::Config, jar: Arc<Jar>) -> Option<Box<dyn LoginStrategy>> {
    match config.ssid.as_str() {
        x if x.contains("Wi-MESH") => {
            info!("Selected strategy: KTX Wi-Mesh");
            Some(Box::new(ktx_wimesh::KtxWiMeshStrategy::new(jar)))
        },
        "Highlands Coffee" => {
            info!("Selected strategy: Highland Coffee");
            Some(Box::new(highland::HighlandStrategy))
        },
        x if x == "HCMUS-STUDENT" || x == "HCMUS-PUBLIC" => {
            info!("Selected strategy: HCMUS. This may not work because of fixed IP address. (TODO)");  
            Some(Box::new(hcmus::HcmusStrategy))
        }, 
        _ => {
            warn!("Unknown SSID '{}'", config.ssid);
            None
        }
    }
}