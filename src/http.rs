//! HTTP client with retry logic, timeouts, and cookie support

use anyhow::{bail, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use reqwest::{Client, Response};
use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RETRIES: u32 = 3;

pub struct HttpClient {
    inner: Client,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0"),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US,en;q=0.9,vi;q=0.8"),
        );

        let client = Client::builder()
            .cookie_store(true)
            .timeout(DEFAULT_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .default_headers(headers)
            .build()?;

        Ok(Self { inner: client })
    }

    pub async fn get(&self, url: &str) -> Result<Response> {
        self.with_retry(|| self.inner.get(url).send()).await
    }

    pub async fn get_with_headers(&self, url: &str, headers: HeaderMap) -> Result<Response> {
        self.with_retry(|| self.inner.get(url).headers(headers.clone()).send())
            .await
    }

    pub async fn post_json<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<Response> {
        self.with_retry(|| {
            self.inner
                .post(url)
                .header("Content-Type", "application/json")
                .header("X-Requested-With", "XMLHttpRequest")
                .json(body)
                .send()
        })
        .await
    }

    pub async fn post_json_with_headers<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        body: &T,
        headers: HeaderMap,
    ) -> Result<Response> {
        self.with_retry(|| {
            self.inner
                .post(url)
                .header("Content-Type", "application/json")
                .header("X-Requested-With", "XMLHttpRequest")
                .headers(headers.clone())
                .json(body)
                .send()
        })
        .await
    }

    pub async fn post_form<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        form: &T,
    ) -> Result<Response> {
        self.with_retry(|| self.inner.post(url).form(form).send())
            .await
    }

    /// Retry up to MAX_RETRIES times with exponential backoff
    async fn with_retry<F, Fut>(&self, request_fn: F) -> Result<Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = reqwest::Result<Response>>,
    {
        let mut last_err = None;

        for attempt in 0..MAX_RETRIES {
            match request_fn().await {
                Ok(resp) if resp.status().is_success() => return Ok(resp),
                Ok(resp) if resp.status().is_server_error() && attempt < MAX_RETRIES - 1 => {
                    let delay = Duration::from_secs(1 << attempt);
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    tracing::warn!(
                        "Server error {}, body: '{}', retrying in {:?}... (attempt {}/{})",
                        status,
                        &body[..body.len().min(200)],
                        delay,
                        attempt + 1,
                        MAX_RETRIES
                    );
                    tokio::time::sleep(delay).await;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    bail!(
                        "Request failed: {} - {}",
                        status,
                        &text[..50.min(text.len())]
                    );
                }
                Err(e) if attempt < MAX_RETRIES - 1 => {
                    let delay = Duration::from_secs(1 << attempt);
                    tracing::warn!(
                        "Request error: {}, retrying in {:?}... (attempt {}/{})",
                        e,
                        delay,
                        attempt + 1,
                        MAX_RETRIES
                    );
                    last_err = Some(e);
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Err(last_err
            .map(Into::into)
            .unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded")))
    }
}
