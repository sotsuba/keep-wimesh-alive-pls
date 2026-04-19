pub mod error;
pub mod hcmus;
pub mod highland;
pub mod session;
pub mod traits;
pub mod utils;
pub mod wimesh;

use anyhow::Result;
use error::StrategyError;
pub use traits::LoginStrategy;

use crate::platform::Platform;

#[derive(Clone, Copy, Debug)]
pub struct RegistryStrategy {
    pub name: &'static str,
    pub predicate: fn(&str) -> bool,
    pub factory: fn(&dyn Platform) -> Result<Box<dyn LoginStrategy>>,
}

static REGISTRY: &[RegistryStrategy] = &[
    hcmus::REGISTRY_ENTRY,
    highland::REGISTRY_ENTRY,
    wimesh::REGISTRY_ENTRY,
];

pub fn select_strategy(ssid: &str, platform: &dyn Platform) -> Result<Box<dyn LoginStrategy>> {
    let entry = REGISTRY
        .iter()
        .find(|entry| (entry.predicate)(ssid))
        .ok_or_else(|| StrategyError::UnknownSSID(ssid.to_string()))?;
    (entry.factory)(platform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::traits::LockGuard;

    struct NullPlatform;
    impl Platform for NullPlatform {
        fn name(&self) -> &'static str {
            "null"
        }
        fn detect_ssid(&self) -> Option<String> {
            None
        }
        fn get_wifi_ipv4_address(&self) -> Option<String> {
            Some("127.0.0.1".to_string())
        }
        fn ping_gateway(&self, _addr: &str) -> bool {
            false
        }
        fn acquire_lock(&self) -> anyhow::Result<Box<dyn LockGuard>> {
            anyhow::bail!("no-op")
        }
    }

    static NULL: NullPlatform = NullPlatform;

    #[test]
    fn hcmus_student_ssid() {
        assert!(select_strategy("HCMUS-STUDENT", &NULL).is_ok());
    }

    #[test]
    fn hcmus_public_ssid() {
        assert!(select_strategy("HCMUS-PUBLIC", &NULL).is_ok());
    }

    #[test]
    fn highland_ssid() {
        assert!(select_strategy("Highlands Coffee", &NULL).is_ok());
    }

    #[test]
    fn wimesh_ssid() {
        assert!(select_strategy("1.Free Wi-MESH", &NULL).is_ok());
        assert!(select_strategy("Free Wi-MESH rescuse", &NULL).is_ok());
    }

    #[test]
    fn unknown_ssid_returns_error() {
        let result = select_strategy("Starbucks-Guest", &NULL);
        assert!(result.is_err());
    }

    #[test]
    fn empty_ssid_returns_error() {
        assert!(select_strategy("", &NULL).is_err());
    }

    #[test]
    fn registry_names_are_unique() {
        let mut names: Vec<&str> = REGISTRY.iter().map(|e| e.name).collect();
        let original_len = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), original_len, "duplicate registry entry names");
    }

    #[test]
    fn first_match_wins() {
        let r = select_strategy("HCMUS-PUBLIC", &NULL);
        assert!(r.is_ok());
    }
}
