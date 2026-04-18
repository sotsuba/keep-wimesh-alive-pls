use std::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum PlatformError {
    UnsupportedPlatform,
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::UnsupportedPlatform => write!(f, "Unsupported platform"),
        }
    }
}

impl std::error::Error for PlatformError {}