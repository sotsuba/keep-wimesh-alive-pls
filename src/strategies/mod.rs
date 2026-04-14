pub mod error;
pub mod hcmus;
pub mod highland;
pub mod traits;
pub mod utils;
pub mod wimesh;

use error::StrategyError;
pub use traits::LoginStrategy;

#[derive(Clone, Copy, Debug)]
pub struct RegistryStrategy {
    pub name: &'static str,
    pub predicate: fn(&str) -> bool,
    pub factory: fn() -> Box<dyn LoginStrategy>,
}

static REGISTRY: &[RegistryStrategy] = &[
    hcmus::REGISTRY_ENTRY,
    highland::REGISTRY_ENTRY,
    wimesh::REGISTRY_ENTRY,
];

pub fn select_strategy(ssid: &str) -> Result<Box<dyn LoginStrategy>, StrategyError> {
    REGISTRY
        .iter()
        .find(|entry| (entry.predicate)(ssid))
        .map(|entry| (entry.factory)())
        .ok_or_else(|| StrategyError::UnknownSSID(ssid.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hcmus_student_ssid() {
        assert!(select_strategy("HCMUS-STUDENT").is_ok());
    }

    #[test]
    fn hcmus_public_ssid() {
        assert!(select_strategy("HCMUS-PUBLIC").is_ok());
    }

    #[test]
    fn highland_ssid() {
        assert!(select_strategy("Highlands Coffee").is_ok());
    }

    #[test]
    fn wimesh_ssid() {
        assert!(select_strategy("1.Free Wi-MESH").is_ok());
        assert!(select_strategy("Free Wi-MESH rescuse").is_ok());
    }

    #[test]
    fn unknown_ssid_returns_error() {
        let result = select_strategy("Starbucks-Guest");
        assert!(matches!(result, Err(StrategyError::UnknownSSID(_))));
    }

    #[test]
    fn empty_ssid_returns_error() {
        assert!(matches!(
            select_strategy(""),
            Err(StrategyError::UnknownSSID(_))
        ));
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
        // "HCMUS-PUBLIC" should match hcmus entry, not trigger unknown
        let r = select_strategy("HCMUS-PUBLIC");
        assert!(r.is_ok());
    }
}
