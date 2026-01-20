//! Utility functions for network checks

use anyhow::Result;
use std::process::Command;

/// Check if connected to any of the target WiFi SSIDs
/// Returns Some(ssid) if connected to one of the target SSIDs, None otherwise
pub fn is_connected_to_wifi(target_ssids: &[String]) -> Result<Option<String>> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "active,ssid", "dev", "wifi"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    for line in stdout.lines() {
        if line.starts_with("yes:") {
            let current_ssid = line.strip_prefix("yes:").unwrap_or("");
            // Check if current SSID matches any of the target SSIDs
            if target_ssids.iter().any(|ssid| ssid == current_ssid) {
                return Ok(Some(current_ssid.to_string()));
            }
        }
    }
    
    Ok(None)
}

/// Check internet connectivity by pinging Google
pub fn has_internet_connectivity() -> bool {
    Command::new("curl")
        .args([
            "-sf",
            "--head",
            "--max-time",
            "5",
            "https://www.google.com",
        ])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
