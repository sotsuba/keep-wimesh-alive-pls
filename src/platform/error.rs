#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlatformError {
    #[error("unsupported platform")]
    UnsupportedPlatform,

    #[error("could not determine gateway IP")]
    NoGateway,

    #[error("failed to acquire instance lock: {0}")]
    LockFailed(String),
}
