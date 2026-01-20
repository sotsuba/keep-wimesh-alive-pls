//! Data models for Wi-MESH authentication

use serde::Deserialize;

/// Gateway configuration extracted from captive portal HTML
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub mac: String,
    pub ip: String,
    pub chap_id: String,
    pub chap_challenge: String,
    pub link_login_only: String,
}

/// Login credentials extracted from authentication form
#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// Response from /Home/VerifyUrl endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct VerifyResponse {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Response from /Content/GetCustomer endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct CustomerResponse {
    #[serde(rename = "captiveContext")]
    pub captive_context: Option<CaptiveContext>,
    
    #[serde(rename = "contentAuthenForm")]
    pub content_authen_form: Option<String>,
    
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CaptiveContext {
    #[serde(rename = "contentAuthenForm")]
    pub content_authen_form: Option<String>,
    
    #[serde(flatten)]
    pub extra: serde_json::Value,
}
