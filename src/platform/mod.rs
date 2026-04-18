pub mod traits;
pub mod error; 

pub use error::PlatformError;
use traits::Platform;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Copy, Clone, Debug)]
pub struct PlatformRegistry {
    pub factory: fn() -> Box<dyn Platform>,
}

pub static PLATFORM_REGISTRY: &[PlatformRegistry] = &[
    #[cfg(target_os = "linux")]
    linux::REGISTRY_ENTRY, 
    #[cfg(target_os = "windows")]
    windows::REGISTRY_ENTRY,
];

pub fn select_platform() -> Result<Box<dyn Platform>, PlatformError> {
    if PLATFORM_REGISTRY.is_empty() {
        return Err(PlatformError::UnsupportedPlatform);
    }
    Ok((PLATFORM_REGISTRY[0].factory)())
}

pub fn run_command(program: &str, args: &[&str]) -> Option<String> {
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
