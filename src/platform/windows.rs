use super::traits::{LockGuard, Platform};

use anyhow::{Result, bail};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

const MUTEX_NAME: &str = "Global\\wimesh_watchdog";

// HANDLE is held to keep the mutex owned — released on drop.
#[allow(dead_code)]
struct MutexLock(HANDLE);
// HANDLE is a raw pointer - safe to Send here because we own it exclusively.
unsafe impl Send for MutexLock {}
impl LockGuard for MutexLock {}
impl Drop for MutexLock {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}

#[derive(Debug)]
pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn name(&self) -> &'static str {
        "Windows"
    }

    fn detect_ssid(&self) -> Option<String> {
        self.run_command("netsh", &["wlan", "show", "interfaces"])?
            .lines()
            .find(|l| l.trim().starts_with("SSID"))
            .and_then(|l| l.split_once(':').map(|x| x.1))
            .map(|s| s.trim().to_string())
    }

    fn get_wifi_ipv4_address(&self) -> Option<String> {
        self.run_command("ipconfig", &[])?.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;

            if key.contains("Default Gateway") {
                let ip = value.trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
            None
        })
    }

    fn ping_gateway(&self, addr: &str) -> bool {
        self.run_command("ping", &["-n", "1", "-w", "1000", addr])
            .map(|o| o.contains("TTL=")) // "TTL=" is present in successful ping replies on Windows.
            .unwrap_or(false)
    }

    fn acquire_lock(&self) -> Result<Box<dyn LockGuard>> {
        let name: Vec<u16> = MUTEX_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            bail!("failed to create mutex");
        }
        // WaitForSingleObject with timeout=0: non-blocking try-acquire.
        let result = unsafe { WaitForSingleObject(handle, 0) };
        match result {
            // Normal acquisition.
            WAIT_OBJECT_0 => {}
            // Previous owner crashed without releasing; we still own the mutex now.
            WAIT_ABANDONED => {
                tracing::warn!("watchdog mutex was abandoned by a crashed process; proceeding");
            }
            // WAIT_TIMEOUT (another live instance holds it) or WAIT_FAILED.
            _ => {
                unsafe { CloseHandle(handle) };
                bail!("another watchdog instance is already running");
            }
        }
        Ok(Box::new(MutexLock(handle)))
    }
}

pub static REGISTRY_ENTRY: super::PlatformRegistry = crate::platform::PlatformRegistry {
    factory: || Box::new(WindowsPlatform),
};
