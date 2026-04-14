use anyhow::Result;
use std::process::Command;

/// Returned by `acquire_lock`. Releases the OS lock when dropped.
pub trait LockGuard: Send {}

pub trait Platform {
    fn detect_ssid(&self) -> Option<String>;
    fn default_gateway(&self) -> Option<String>;
    /// Ping the gateway once with a 1-second timeout. Returns true if reachable.
    fn ping_gateway(&self, addr: &str) -> bool;
    fn acquire_lock(&self) -> Result<Box<dyn LockGuard>>;
}

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Option<String>;
}

pub struct RealRunner;
impl CommandRunner for RealRunner {
    fn run(&self, program: &str, args: &[&str]) -> Option<String> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        // Suppress the console window that Windows creates by default when
        // spawning child processes (e.g. powershell, netsh).
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd.output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
    }
}
