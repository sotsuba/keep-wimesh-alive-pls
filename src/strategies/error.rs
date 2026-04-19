use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StrategyError {
    #[error("unknown SSID: {0}")]
    UnknownSsid(String),

    #[error("{endpoint} returned unexpected HTTP status {status}")]
    UnexpectedStatus {
        endpoint: &'static str,
        status: StatusCode,
    },

    #[error("missing required field '{field}' in {location}")]
    MissingField {
        field: String,
        location: &'static str,
    },

    #[error("parse error in {location}: {detail}")]
    Parse {
        location: &'static str,
        detail: String,
    },

    #[error("{context}: {detail}")]
    LoginRejected {
        context: &'static str,
        detail: String,
    },

    #[error("all servers failed for {operation}")]
    AllServersFailed { operation: &'static str },
}
