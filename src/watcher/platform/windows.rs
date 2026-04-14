use crate::watcher::platform::traits::{CommandRunner, LockGuard, Platform, RealRunner};
use anyhow::{Result, bail};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
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

pub struct WindowsPlatform<R: CommandRunner = RealRunner> {
    runner: R,
}

impl<R: CommandRunner> WindowsPlatform<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R: CommandRunner> Platform for WindowsPlatform<R> {
    fn detect_ssid(&self) -> Option<String> {
        let output = self.runner.run("netsh", &["wlan", "show", "interfaces"])?;
        parse_netsh_output(&output)
    }

    fn default_gateway(&self) -> Option<String> {
        // PowerShell gives a clean single-line IP output.
        let output = self.runner.run("powershell", &[
            "-NoProfile", "-Command",
            "(Get-NetRoute -DestinationPrefix '0.0.0.0/0' | Sort-Object RouteMetric | Select-Object -First 1).NextHop",
        ])?;
        parse_powershell_gateway(&output)
    }

    fn ping_gateway(&self, addr: &str) -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("ping")
            .args(["-n", "1", "-w", "1000", addr])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn acquire_lock(&self) -> Result<Box<dyn LockGuard>> {
        let name: Vec<u16> = MUTEX_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle == 0 {
            bail!("failed to create mutex");
        }
        // WaitForSingleObject with timeout=0: non-blocking try-acquire.
        let result = unsafe { WaitForSingleObject(handle, 0) };
        if result != WAIT_OBJECT_0 {
            unsafe { CloseHandle(handle) };
            bail!("another watchdog instance is already running");
        }
        Ok(Box::new(MutexLock(handle)))
    }
}

// Pure parsing functions - testable without spawning real commands.

pub(super) fn parse_netsh_output(output: &str) -> Option<String> {
    // Output contains "    SSID                   : KTX Wi-MESH"
    // Must match "SSID" but NOT "BSSID".
    output
        .lines()
        .find(|l| {
            let t = l.trim();
            t.starts_with("SSID") && !t.starts_with("BSSID")
        })
        .and_then(|l| l.splitn(2, ':').nth(1))
        .map(|s| s.trim().to_string())
}

pub(super) fn parse_powershell_gateway(output: &str) -> Option<String> {
    let gw = output.trim().to_string();
    if gw.is_empty() { None } else { Some(gw) }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeRunner(&'static str);
    impl CommandRunner for FakeRunner {
        fn run(&self, _: &str, _: &[&str]) -> Option<String> {
            Some(self.0.to_string())
        }
    }

    // --- parse_netsh_output ---

    #[test]
    fn netsh_finds_ssid() {
        let out = "    Name                   : Wi-Fi\n    SSID                   : KTX Wi-MESH\n    BSSID                  : aa:bb:cc:dd:ee:ff\n";
        assert_eq!(parse_netsh_output(out), Some("KTX Wi-MESH".to_string()));
    }

    #[test]
    fn netsh_does_not_match_bssid_line() {
        let out = "    BSSID                  : aa:bb:cc:dd:ee:ff\n";
        assert_eq!(parse_netsh_output(out), None);
    }

    #[test]
    fn netsh_ssid_with_colon() {
        // SSID containing ':' - splitn(2) ensures only first colon is used as delimiter.
        let out = "    SSID                   : My:Network\n";
        assert_eq!(parse_netsh_output(out), Some("My:Network".to_string()));
    }

    #[test]
    fn netsh_empty_output() {
        assert_eq!(parse_netsh_output(""), None);
    }

    // --- parse_powershell_gateway ---

    #[test]
    fn gateway_parses_ip() {
        assert_eq!(
            parse_powershell_gateway("192.168.1.1\r\n"),
            Some("192.168.1.1".to_string())
        );
    }

    #[test]
    fn gateway_returns_none_on_empty() {
        assert_eq!(parse_powershell_gateway("   \n"), None);
    }

    // --- via FakeRunner ---

    #[test]
    fn detect_ssid_via_fake_runner() {
        let out = "    Name                   : Wi-Fi\n    SSID                   : KTX Wi-MESH\n    BSSID                  : aa:bb:cc\n";
        let p = WindowsPlatform::new(FakeRunner(out));
        // FakeRunner returns same output regardless of command
        assert_eq!(p.detect_ssid(), Some("KTX Wi-MESH".to_string()));
    }

    // --- integration (manual only) ---

    #[test]
    #[ignore = "requires Windows with netsh"]
    fn detect_ssid_real() {
        let p = WindowsPlatform::new(RealRunner);
        println!("SSID: {:?}", p.detect_ssid());
    }

    #[test]
    #[ignore = "requires Windows with PowerShell"]
    fn default_gateway_real() {
        let p = WindowsPlatform::new(RealRunner);
        println!("Gateway: {:?}", p.default_gateway());
    }
}
