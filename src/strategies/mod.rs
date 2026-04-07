pub mod ktx_wimesh;
pub mod hcmus_hostpot;
pub mod traits;
pub use traits::LoginStrategy;
use tracing::{info, warn};

pub fn select_strategy(config: &super::config::Config) -> Option<Box<dyn LoginStrategy>> {
    match config.ssid.as_str() {
        x if x.contains("Wi-MESH") => {
            info!("Selected strategy: KTX Wi-Mesh");
            Some(Box::new(ktx_wimesh::KtxWiMeshStrategy))
        },
        x if x == "HCMUS-STUDENT" || x == "HCMUS-PUBLIC" => {
            info!("Selected strategy: HCMUS. This may not work because of fixed IP address. (TODO)");  
            Some(Box::new(hcmus_hostpot::HcmusStudentStrategy))
        }, 
        _ => {
            warn!("Unknown SSID '{}'", config.ssid);
            None
        }
    }
}