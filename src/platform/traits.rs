use anyhow::Result;
pub trait LockGuard: Send {}

pub trait Platform {
    fn name(&self) -> &'static str;
    fn detect_ssid(&self) -> Option<String>;
    fn get_wifi_ipv4_address(&self) -> Option<String>;
    fn ping_gateway(&self, addr: &str) -> bool;
    fn acquire_lock(&self) -> Result<Box<dyn LockGuard>>;
    fn run_command(&self, program: &str, args: &[&str]) -> Option<String> {
        let mut cmd = std::process::Command::new(program);
        cmd.args(args);

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
