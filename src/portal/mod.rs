//! Captive Portal abstraction layer
//!
//! This module provides a trait-based system for handling different types of
//! captive portals. Each portal type implements the `CaptivePortal` trait,
//! allowing the main daemon to work with any supported portal transparently.

pub mod awing;

pub use awing::AwingPortal;

use anyhow::Result;
use async_trait::async_trait;

/// Trait defining the interface for captive portal handlers
///
/// Each captive portal type (Awing, FPT, etc.) implements this trait
/// to provide its specific authentication logic.
#[async_trait]
pub trait CaptivePortal: Send + Sync {
    /// Returns the human-readable name of this portal type
    fn name(&self) -> &str;

    /// Returns the list of SSIDs this portal handles
    fn ssids(&self) -> &[String];

    /// Check if this portal handles the given SSID
    fn matches_ssid(&self, ssid: &str) -> bool {
        self.ssids().iter().any(|s| s == ssid)
    }

    /// Execute the full authentication flow for this portal
    async fn connect(&mut self) -> Result<()>;

    /// Optional: Check if already authenticated (for portals that support this)
    async fn is_authenticated(&self) -> Result<bool> {
        // Default implementation: try to reach the internet
        Ok(crate::utils::has_internet_connectivity())
    }
}

/// Registry of all available portal implementations
pub struct PortalRegistry {
    portals: Vec<Box<dyn CaptivePortal>>,
}

impl PortalRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            portals: Vec::new(),
        }
    }

    /// Register a portal implementation
    pub fn register(&mut self, portal: Box<dyn CaptivePortal>) {
        tracing::debug!("Registered portal: {} (SSIDs: {})", 
            portal.name(), 
            portal.ssids().join(", "));
        self.portals.push(portal);
    }

    /// Find a portal that handles the given SSID
    pub fn find_for_ssid(&mut self, ssid: &str) -> Option<&mut Box<dyn CaptivePortal>> {
        self.portals.iter_mut().find(|p| p.matches_ssid(ssid))
    }

    /// Get all registered SSIDs across all portals
    pub fn all_ssids(&self) -> Vec<&str> {
        self.portals
            .iter()
            .flat_map(|p| p.ssids().iter().map(|s| s.as_str()))
            .collect()
    }

    /// Check if any portal handles the given SSID
    pub fn has_ssid(&self, ssid: &str) -> bool {
        self.portals.iter().any(|p| p.matches_ssid(ssid))
    }
}

impl Default for PortalRegistry {
    fn default() -> Self {
        Self::new()
    }
}
