use crate::platform::traits::{LockGuard, Platform};
use anyhow::{Result, bail};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

const LOCK_FILE: &str = "/tmp/wimesh_watchdog.lock";

// File is held open to keep the flock active - dropped when lock is released.
#[allow(dead_code)]
struct FileLock(File);
impl LockGuard for FileLock {}

#[derive(Debug)]
pub struct LinuxPlatform;

impl Platform for LinuxPlatform {
    fn name(&self) -> &'static str {
        "Linux"
    }

    fn detect_ssid(&self) -> Option<String> {
        self.run_command("nmcli", &["-t", "-f", "active,ssid", "dev", "wifi"])
            .map(|output| {
                output
                    .lines()
                    .find(|l| l.starts_with("yes:"))
                    .map(|l| l[4..].to_string())
            })
            .unwrap_or(None)
    }

    fn get_wifi_ipv4_address(&self) -> Option<String> {
        self.run_command("ip", &["-4", "route", "show", "default"])
            .map(|output| {
                output
                    .lines()
                    .find(|l| l.starts_with("default"))
                    .and_then(|l| l.split_whitespace().nth(2))
                    .map(|s| s.to_string())
            })
            .unwrap_or(None)
    }

    fn ping_gateway(&self, addr: &str) -> bool {
        self.run_command("ping", &["-c", "1", "-W", "1", addr])
            .unwrap_or_else(|| "".into())
            .contains("1 received")
    }

    fn acquire_lock(&self) -> Result<Box<dyn LockGuard>> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(LOCK_FILE)?;
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            bail!("another watchdog instance is already running");
        }
        Ok(Box::new(FileLock(file)))
    }
}

pub static REGISTRY_ENTRY: crate::platform::PlatformRegistry = crate::platform::PlatformRegistry {
    factory: || Box::new(LinuxPlatform),
};
