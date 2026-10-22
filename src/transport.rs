use crate::{AgentStatError, ProviderId, Result};
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use reqwest::{
    Method, Url,
    header::{HeaderMap, HeaderValue},
};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, time::Duration};

/// A response from a caller-supplied HTTP executor. Header/body values are never logged.
#[derive(Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}
impl std::fmt::Debug for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("body_bytes", &self.body.len())
            .finish_non_exhaustive()
    }
}
/// Optional integration point for custom networking and deterministic tests.
/// Executors must not follow redirects or retry requests and must honor future cancellation.
/// The default executor uses reqwest; custom executors own their network buffering limits.
pub trait HttpExecutor: Send + Sync + std::fmt::Debug {
    fn execute(&self, request: reqwest::Request) -> BoxFuture<'_, Result<HttpResponse>>;
}
#[derive(Debug, Clone)]
pub(crate) struct Transport {
    pub http: reqwest::Client,
    pub executor: Option<Arc<dyn HttpExecutor>>,
    pub endpoints: HashMap<ProviderId, Url>,
    pub max_bytes: usize,
}
impl Transport {
    pub async fn json(
        &self,
        provider: ProviderId,
        method: Method,
        url: &str,
        headers: HeaderMap,
        body: Option<Value>,
    ) -> Result<Value> {
        let mut url =
            Url::parse(url).map_err(|_| AgentStatError::Configuration("invalid provider URL"))?;
        if let Some(base) = self.endpoints.get(&provider) {
            let path = format!("{}{}", base.path().trim_end_matches('/'), url.path());
            let mut mapped = base.clone();
            mapped.set_path(&path);
            mapped.set_query(url.query());
            url = mapped;
        }
        validate_url(&url)?;
        let mut request = self
            .http
            .request(method, url)
            .headers(headers)
            .header("Accept", "application/json")
            .header("Cache-Control", "no-cache");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let request = request
            .build()
            .map_err(|_| AgentStatError::Configuration("invalid HTTP request"))?;
        if let Some(executor) = &self.executor {
            let response = executor.execute(request).await?;
            check_status(response.status, &response.headers)?;
            if response.body.len() > self.max_bytes {
                return Err(AgentStatError::ResponseTooLarge);
            }
            return serde_json::from_slice(&response.body)
                .map_err(|_| AgentStatError::InvalidResponse("expected JSON"));
        }
        let mut response = self.http.execute(request).await.map_err(|e| {
            if e.is_timeout() {
                AgentStatError::Timeout
            } else {
                AgentStatError::Transport
            }
        })?;
        let status = response.status().as_u16();
        check_status(status, response.headers())?;
        if response
            .content_length()
            .is_some_and(|n| n > self.max_bytes as u64)
        {
            return Err(AgentStatError::ResponseTooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| AgentStatError::Transport)?
        {
            if bytes.len().saturating_add(chunk.len()) > self.max_bytes {
                return Err(AgentStatError::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| AgentStatError::InvalidResponse("expected JSON"))
    }
}
pub(crate) fn validate_url(url: &Url) -> Result<()> {
    let loopback = url.host_str().is_some_and(|h| {
        h == "localhost"
            || h.parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    if !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https" || (url.scheme() == "http" && loopback))
    {
        return Err(AgentStatError::Configuration(
            "endpoint must be HTTPS or loopback HTTP, without URL credentials or fragments",
        ));
    }
    Ok(())
}
fn parse_retry_after(value: Option<&HeaderValue>) -> Option<DateTime<Utc>> {
    let raw = value?.to_str().ok()?.trim();
    if let Ok(seconds) = raw.parse::<u64>() {
        Utc::now()
            .checked_add_signed(chrono::Duration::from_std(Duration::from_secs(seconds)).ok()?)
    } else {
        httpdate::parse_http_date(raw)
            .ok()
            .map(DateTime::<Utc>::from)
    }
}
pub(crate) fn header(headers: &mut HeaderMap, name: &'static str, value: &str) -> Result<()> {
    let mut v = HeaderValue::from_str(value)
        .map_err(|_| AgentStatError::Configuration("invalid request header"))?;
    v.set_sensitive(true);
    headers.insert(name, v);
    Ok(())
}

fn check_status(status: u16, headers: &HeaderMap) -> Result<()> {
    match status {
        200..=299 => Ok(()),
        401 => Err(AgentStatError::Authentication),
        403 => Err(AgentStatError::PermissionDenied),
        429 => Err(AgentStatError::RateLimited {
            retry_after: parse_retry_after(headers.get("retry-after")),
        }),
        _ => Err(AgentStatError::Http { status }),
    }
}
