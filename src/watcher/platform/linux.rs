use crate::watcher::platform::traits::{CommandRunner, LockGuard, Platform, RealRunner};
use anyhow::{Result, bail};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

const LOCK_FILE: &str = "/tmp/wimesh_watchdog.lock";

// File is held open to keep the flock active - dropped when lock is released.
#[allow(dead_code)]
struct FileLock(File);
impl LockGuard for FileLock {}

pub struct LinuxPlatform<R: CommandRunner = RealRunner> {
    runner: R,
}

impl<R: CommandRunner> LinuxPlatform<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R: CommandRunner> Platform for LinuxPlatform<R> {
    fn detect_ssid(&self) -> Option<String> {
        let output = self
            .runner
            .run("nmcli", &["-t", "-f", "active,ssid", "dev", "wifi"])?;
        parse_nmcli_output(&output)
    }

    fn default_gateway(&self) -> Option<String> {
        let output = self.runner.run("ip", &["-4", "route", "show", "default"])?;
        parse_ip_route_output(&output)
    }

    fn ping_gateway(&self, addr: &str) -> bool {
        self.runner.run("ping", &["-c", "1", "-W", "1", addr])
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

pub(super) fn parse_nmcli_output(output: &str) -> Option<String> {
    // nmcli -t output: "yes:SSID" for active, "no:SSID" for inactive
    output
        .lines()
        .find(|l| l.starts_with("yes:"))
        .map(|l| l[4..].to_string())
}

pub(super) fn parse_ip_route_output(output: &str) -> Option<String> {
    // "default via 192.168.1.1 dev wlan0 proto dhcp ..."
    output
        .lines()
        .find(|l| l.starts_with("default"))
        .and_then(|l| l.split_whitespace().nth(2))
        .map(|s| s.to_string())
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

    // --- parse_nmcli_output ---

    #[test]
    fn nmcli_finds_active_ssid() {
        assert_eq!(
            parse_nmcli_output("no:OtherNet\nyes:KTX Wi-MESH\nno:Another\n"),
            Some("KTX Wi-MESH".to_string())
        );
    }

    #[test]
    fn nmcli_returns_none_when_disconnected() {
        assert_eq!(parse_nmcli_output("no:OtherNet\nno:Another\n"), None);
    }

    #[test]
    fn nmcli_empty_output() {
        assert_eq!(parse_nmcli_output(""), None);
    }

    #[test]
    fn nmcli_first_active_wins() {
        assert_eq!(
            parse_nmcli_output("yes:First\nyes:Second\n"),
            Some("First".to_string())
        );
    }

    // --- parse_ip_route_output ---

    #[test]
    fn ip_route_extracts_gateway() {
        let out = "default via 192.168.1.1 dev wlan0 proto dhcp src 192.168.1.100 metric 600\n";
        assert_eq!(parse_ip_route_output(out), Some("192.168.1.1".to_string()));
    }

    #[test]
    fn ip_route_none_when_no_default_route() {
        let out = "192.168.1.0/24 dev wlan0 proto kernel scope link\n";
        assert_eq!(parse_ip_route_output(out), None);
    }

    #[test]
    fn ip_route_empty_output() {
        assert_eq!(parse_ip_route_output(""), None);
    }

    // --- via FakeRunner ---

    #[test]
    fn detect_ssid_via_fake_runner() {
        let p = LinuxPlatform::new(FakeRunner("no:OtherNet\nyes:KTX Wi-MESH\n"));
        assert_eq!(p.detect_ssid(), Some("KTX Wi-MESH".to_string()));
    }

    #[test]
    fn default_gateway_via_fake_runner() {
        let p = LinuxPlatform::new(FakeRunner("default via 10.0.0.1 dev eth0\n"));
        assert_eq!(p.default_gateway(), Some("10.0.0.1".to_string()));
    }

    // --- integration (manual only) ---

    #[test]
    #[ignore = "requires Linux with nmcli installed"]
    fn detect_ssid_real() {
        let p = LinuxPlatform::new(RealRunner);
        println!("SSID: {:?}", p.detect_ssid());
    }

    #[test]
    #[ignore = "requires Linux with ip command"]
    fn default_gateway_real() {
        let p = LinuxPlatform::new(RealRunner);
        println!("Gateway: {:?}", p.default_gateway());
    }
}
