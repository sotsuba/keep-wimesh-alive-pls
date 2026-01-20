//! Awing captive portal implementation (Wi-MESH)
//!
//! This module handles authentication for Wi-MESH networks using the
//! Awing Connect portal (awingconnect.vn).

use crate::http::HttpClient;
use crate::models::{Credentials, CustomerResponse, GatewayConfig};
use crate::parser;
use crate::portal::CaptivePortal;
use anyhow::{Context, Result};
use async_trait::async_trait;

const GATEWAY_URL: &str = "http://login.net.vn";
const BASE_URL: &str = "http://v1.awingconnect.vn";

/// Configuration for the Awing portal
#[derive(Debug, Clone)]
pub struct AwingConfig {
    /// Human-readable name for this portal instance
    pub name: String,
    /// SSIDs that this portal handles
    pub ssids: Vec<String>,
    /// MAC address for authentication
    pub mac_address: String,
}

impl Default for AwingConfig {
    fn default() -> Self {
        Self {
            name: "Wi-MESH Awing".to_string(),
            ssids: vec!["1.Free Wi-MESH".to_string()],
            mac_address: String::new(),
        }
    }
}

/// Awing portal implementation for Wi-MESH networks
pub struct AwingPortal {
    config: AwingConfig,
    client: HttpClient,
    gateway: Option<GatewayConfig>,
    handshake_url: Option<String>,
}

impl AwingPortal {
    /// Create a new Awing portal instance
    pub fn new(config: AwingConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: HttpClient::new()?,
            gateway: None,
            handshake_url: None,
        })
    }

    /// Step 0: Scan Gateway - Fetch captive portal page and extract config
    async fn scan_gateway(&mut self) -> Result<()> {
        tracing::info!("[{}] Step 0: Scanning Gateway...", self.config.name);

        let resp = self.client.get(GATEWAY_URL).await?;
        let html = resp.text().await?;

        let gw = parser::parse_gateway_html(&html)?;
        tracing::info!("   -> Found gateway: {}", gw.ip);

        self.gateway = Some(gw);
        Ok(())
    }

    /// Step 1: Handshake - Register device with portal
    async fn handshake(&mut self) -> Result<()> {
        let gw = self.gateway.as_ref().context("Gateway not scanned")?;
        tracing::info!("[{}] Step 1: Handshaking...", self.config.name);
        tracing::info!("   -> Using MAC: {}", self.config.mac_address);

        let url = format!(
            "{}/login?serial={}&client_mac={}&client_ip={}&userurl=http://login.net.vn/&login_url={}&chap_id={}&chap_challenge={}",
            BASE_URL,
            self.config.mac_address,
            gw.mac,
            gw.ip,
            urlencoding::encode(&gw.link_login_only),
            gw.chap_id,
            gw.chap_challenge
        );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::REFERER,
            reqwest::header::HeaderValue::from_str(&url)?,
        );
        headers.insert(
            reqwest::header::ORIGIN,
            reqwest::header::HeaderValue::from_static(BASE_URL),
        );

        self.client.get_with_headers(&url, headers).await?;
        self.handshake_url = Some(url);
        Ok(())
    }

    /// Helper to create headers with Referer and Origin for API requests
    fn api_headers(&self) -> Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();

        if let Some(ref url) = self.handshake_url {
            headers.insert(
                reqwest::header::REFERER,
                reqwest::header::HeaderValue::from_str(url)?,
            );
        }

        headers.insert(
            reqwest::header::ORIGIN,
            reqwest::header::HeaderValue::from_static(BASE_URL),
        );

        Ok(headers)
    }

    /// Step 2: Verify Device - Get session context
    async fn verify_device(&self) -> Result<serde_json::Value> {
        tracing::info!("[{}] Step 2: Verifying Device...", self.config.name);

        let headers = self.api_headers()?;
        let resp = self
            .client
            .post_json_with_headers(
                &format!("{}/Home/VerifyUrl", BASE_URL),
                &serde_json::json!({}),
                headers,
            )
            .await?;

        let context: serde_json::Value = resp.json().await?;
        Ok(context)
    }

    /// Step 3: Get Credentials - Extract login credentials from form
    async fn get_credentials(&self, context: &serde_json::Value) -> Result<Credentials> {
        tracing::info!("[{}] Step 3: Getting Credentials...", self.config.name);

        let mut payload = serde_json::json!({
            "captiveContextDTO": context,
            "customer": {"gender": 1, "name": ""},
            "customerRequiredFields": []
        });

        // Merge all fields from context into payload
        if let Some(obj) = payload.as_object_mut() {
            if let Some(context_obj) = context.as_object() {
                for (key, value) in context_obj {
                    obj.insert(key.clone(), value.clone());
                }
            }
        }

        let headers = self.api_headers()?;
        let resp = self
            .client
            .post_json_with_headers(
                &format!("{}/Content/GetCustomer", BASE_URL),
                &payload,
                headers,
            )
            .await?;

        let data: CustomerResponse = resp.json().await?;

        let form_html = data
            .captive_context
            .as_ref()
            .and_then(|c| c.content_authen_form.as_ref())
            .or(data.content_authen_form.as_ref())
            .ok_or_else(|| anyhow::anyhow!("contentAuthenForm not found in response"))?;

        let creds = parser::parse_credentials(form_html)?;
        tracing::info!("   -> Got credentials for: {}", creds.username);
        Ok(creds)
    }

    /// Step 4: Send Analytics
    async fn send_analytics(&self, context: &serde_json::Value) -> Result<()> {
        tracing::info!("[{}] Step 4: Sending Analytics...", self.config.name);

        let payload = serde_json::json!({
            "captiveContextDTO": context,
            "analyticType": "Authentication",
            "viewIndex": 1
        });

        let headers = self.api_headers()?;
        self.client
            .post_json_with_headers(&format!("{}/Analytic/Send", BASE_URL), &payload, headers)
            .await?;

        Ok(())
    }

    /// Step 5: Login to Router - Submit credentials to gateway
    async fn login_router(&self, creds: &Credentials) -> Result<()> {
        let gw = self.gateway.as_ref().context("Gateway not scanned")?;
        tracing::info!("[{}] Step 5: Logging into Router...", self.config.name);

        let login_url = if gw.link_login_only.is_empty() {
            "http://free.wi-mesh.vn/login".to_string()
        } else {
            gw.link_login_only.clone()
        };

        let form = [
            ("username", creds.username.as_str()),
            ("password", creds.password.as_str()),
            ("dst", &format!("{}/Success", BASE_URL)),
            ("popup", "false"),
        ];

        self.client.post_form(&login_url, &form).await?;
        Ok(())
    }
}

#[async_trait]
impl CaptivePortal for AwingPortal {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn ssids(&self) -> &[String] {
        &self.config.ssids
    }

    async fn connect(&mut self) -> Result<()> {
        self.scan_gateway().await?;
        self.handshake().await?;
        let context = self.verify_device().await?;
        let creds = self.get_credentials(&context).await?;
        self.send_analytics(&context).await?;
        self.login_router(&creds).await?;

        tracing::info!("[{}] Connected successfully!", self.config.name);
        Ok(())
    }
}
