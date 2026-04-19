pub mod error;
pub mod traits;

pub use error::PlatformError;
pub use traits::Platform;

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
