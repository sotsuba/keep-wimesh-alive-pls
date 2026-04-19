#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlatformError {
    #[error("Unsupported platform")]
    UnsupportedPlatform,
}
