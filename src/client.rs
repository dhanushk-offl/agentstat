//! Client construction and bounded provider fetching.

#![expect(
    clippy::disallowed_types,
    reason = "clonable clients share one caller-supplied executor across concurrent requests"
)]

use crate::transport::{Transport, validate_url};
use crate::{
    AgentStatError, FetchRequest, HttpExecutor, ProviderDescriptor, ProviderId, Result,
    UsageSnapshot, provider_catalog, providers,
};
use futures::{StreamExt, stream};
use reqwest::Url;
use reqwest::redirect::Policy;
use std::collections::HashMap;
#[cfg(feature = "local-probes")]
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{self, Instant};

#[cfg(feature = "local-files")]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LocalAccess(u8);

#[cfg(feature = "local-files")]
impl LocalAccess {
    const FILES: u8 = 1;
    #[cfg(feature = "local-probes")]
    const PROBES: u8 = 2;

    pub(crate) fn files(self) -> bool {
        self.0 & Self::FILES != 0
    }

    fn with_files(mut self, enabled: bool) -> Self {
        if enabled {
            self.0 |= Self::FILES
        } else {
            self.0 &= !Self::FILES
        }
        self
    }

    #[cfg(feature = "local-probes")]
    pub(crate) fn probes(self) -> bool {
        self.0 & Self::PROBES != 0
    }

    #[cfg(feature = "local-probes")]
    fn with_probes(mut self, enabled: bool) -> Self {
        if enabled {
            self.0 |= Self::PROBES
        } else {
            self.0 &= !Self::PROBES
        }
        self
    }
}

#[derive(Debug, Clone)]
/// Fetches provider usage through the configured HTTP transport.
pub struct Client {
    pub(crate) transport: Transport,
    timeout: Duration,
    concurrency: usize,
    #[cfg(feature = "local-files")]
    pub(crate) local_access: LocalAccess,
    #[cfg(feature = "local-probes")]
    pub(crate) probe_executables: HashMap<ProviderId, PathBuf>,
}
#[derive(Debug)]
/// Configures network transport, safety limits, and optional local access.
pub struct ClientBuilder {
    timeout: Duration,
    concurrency: usize,
    max_bytes: usize,
    endpoints: HashMap<ProviderId, Url>,
    http: reqwest::ClientBuilder,
    executor: Option<Arc<dyn HttpExecutor>>,
    error: Option<AgentStatError>,
    #[cfg(feature = "local-files")]
    local_access: LocalAccess,
    #[cfg(feature = "local-probes")]
    probe_executables: HashMap<ProviderId, PathBuf>,
}
impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            concurrency: 4,
            max_bytes: 2 * 1024 * 1024,
            endpoints: HashMap::new(),
            http: reqwest::Client::builder(),
            executor: None,
            error: None,
            #[cfg(feature = "local-files")]
            local_access: LocalAccess::default(),
            #[cfg(feature = "local-probes")]
            probe_executables: HashMap::new(),
        }
    }
}
impl ClientBuilder {
    /// Uses a caller-supplied HTTP executor for requests.
    pub fn http_executor(mut self, executor: Arc<dyn HttpExecutor>) -> Self {
        self.executor = Some(executor);
        self
    }
    #[must_use]
    /// Sets the deadline for each fetch.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    #[must_use]
    /// Sets the maximum concurrent requests in a batch.
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }
    #[must_use]
    /// Limits the number of response bytes accepted.
    pub fn max_response_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }
    /// Configure proxies/TLS/pooling. Redirects are always disabled by agentstat.
    #[must_use]
    pub fn http_client_builder(mut self, builder: reqwest::ClientBuilder) -> Self {
        self.http = builder;
        self
    }
    /// Explicitly trust another origin for this provider, e.g. a test server or reverse proxy.
    /// The URL's path is prepended to the provider request path.
    #[must_use]
    pub fn endpoint_override(mut self, provider: ProviderId, url: &str) -> Self {
        match Url::parse(url)
            .map_err(|_| AgentStatError::Configuration("invalid endpoint URL"))
            .and_then(|u| {
                validate_url(&u)?;
                if u.query().is_some() {
                    return Err(AgentStatError::Configuration(
                        "override must not contain query parameters",
                    ));
                }
                Ok(u)
            }) {
            Ok(u) => {
                self.endpoints.insert(provider, u);
            }
            Err(e) => self.error = Some(e),
        }
        self
    }
    #[cfg(feature = "local-files")]
    /// Explicitly allows or denies reading known local credential files.
    #[must_use]
    pub fn allow_local_files(mut self, enabled: bool) -> Self {
        self.local_access = self.local_access.with_files(enabled);
        self
    }
    #[cfg(feature = "local-probes")]
    /// Explicitly allows or denies audited local usage probes.
    #[must_use]
    pub fn allow_local_probes(mut self, enabled: bool) -> Self {
        self.local_access = self.local_access.with_probes(enabled);
        self
    }
    /// Override the executable for an audited provider usage operation. No custom arguments.
    #[cfg(feature = "local-probes")]
    pub fn probe_executable(mut self, provider: ProviderId, path: impl Into<PathBuf>) -> Self {
        self.probe_executables.insert(provider, path.into());
        self
    }
    /// Validates settings and constructs a client.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for invalid limits or endpoints, or when the HTTP client
    /// cannot be constructed.
    pub fn build(self) -> Result<Client> {
        if let Some(e) = self.error {
            return Err(e);
        }
        if self.timeout.is_zero() || self.concurrency == 0 || self.max_bytes == 0 {
            return Err(AgentStatError::Configuration("limits must be positive"));
        }
        let http = self
            .http
            .redirect(Policy::none())
            .timeout(self.timeout)
            .user_agent(concat!("agentstat/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| AgentStatError::Configuration("HTTP client construction failed"))?;
        Ok(Client {
            transport: Transport {
                http,
                executor: self.executor,
                endpoints: self.endpoints,
                max_bytes: self.max_bytes,
            },
            timeout: self.timeout,
            concurrency: self.concurrency,
            #[cfg(feature = "local-files")]
            local_access: self.local_access,
            #[cfg(feature = "local-probes")]
            probe_executables: self.probe_executables,
        })
    }
}
impl Client {
    #[must_use]
    /// Creates a client builder with default limits.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }
    /// Includes disabled/planned providers, with explicit status and build availability.
    #[must_use]
    pub fn providers(&self) -> Vec<ProviderDescriptor> {
        provider_catalog()
    }
    /// Fetches one usage snapshot from the selected provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the request times out, credentials are rejected, transport fails,
    /// or the provider response cannot be interpreted.
    pub async fn fetch(&self, mut request: FetchRequest) -> Result<UsageSnapshot> {
        request.deadline = Instant::now().checked_add(self.timeout);
        time::timeout(self.timeout, providers::fetch(self, &request))
            .await
            .map_err(|_| AgentStatError::Timeout)?
    }
    /// Results remain in input order; a failure never discards another account's result.
    pub async fn fetch_many(
        &self,
        requests: impl IntoIterator<Item = FetchRequest>,
    ) -> Vec<Result<UsageSnapshot>> {
        stream::iter(requests)
            .map(|request| self.fetch(request))
            .buffered(self.concurrency)
            .collect()
            .await
    }
}
