use crate::{
    AgentStatError, FetchRequest, ProviderDescriptor, ProviderId, Result, UsageSnapshot,
    provider_catalog, providers,
    transport::{Transport, validate_url},
};
use futures::{StreamExt, stream};
use reqwest::Url;
use std::{collections::HashMap, time::Duration};

#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) transport: Transport,
    timeout: Duration,
    concurrency: usize,
    #[cfg(feature = "local-files")]
    pub(crate) local_files: bool,
    #[cfg(feature = "local-probes")]
    pub(crate) local_probes: bool,
    #[cfg(feature = "local-probes")]
    pub(crate) probe_executables: HashMap<ProviderId, std::path::PathBuf>,
}
#[derive(Debug)]
pub struct ClientBuilder {
    timeout: Duration,
    concurrency: usize,
    max_bytes: usize,
    endpoints: HashMap<ProviderId, Url>,
    http: reqwest::ClientBuilder,
    executor: Option<std::sync::Arc<dyn crate::HttpExecutor>>,
    error: Option<AgentStatError>,
    #[cfg(feature = "local-files")]
    local_files: bool,
    #[cfg(feature = "local-probes")]
    local_probes: bool,
    #[cfg(feature = "local-probes")]
    probe_executables: HashMap<ProviderId, std::path::PathBuf>,
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
            local_files: false,
            #[cfg(feature = "local-probes")]
            local_probes: false,
            #[cfg(feature = "local-probes")]
            probe_executables: HashMap::new(),
        }
    }
}
impl ClientBuilder {
    pub fn http_executor(mut self, executor: std::sync::Arc<dyn crate::HttpExecutor>) -> Self {
        self.executor = Some(executor);
        self
    }
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }
    pub fn max_response_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }
    /// Configure proxies/TLS/pooling. Redirects are always disabled by agentstat.
    pub fn http_client_builder(mut self, builder: reqwest::ClientBuilder) -> Self {
        self.http = builder;
        self
    }
    /// Explicitly trust another origin for this provider, e.g. a test server or reverse proxy.
    /// The URL's path is prepended to the provider request path.
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
    pub fn allow_local_files(mut self, enabled: bool) -> Self {
        self.local_files = enabled;
        self
    }
    #[cfg(feature = "local-probes")]
    pub fn allow_local_probes(mut self, enabled: bool) -> Self {
        self.local_probes = enabled;
        self
    }
    /// Override the executable for an audited provider usage operation. No custom arguments.
    #[cfg(feature = "local-probes")]
    pub fn probe_executable(
        mut self,
        provider: ProviderId,
        path: impl Into<std::path::PathBuf>,
    ) -> Self {
        self.probe_executables.insert(provider, path.into());
        self
    }
    pub fn build(self) -> Result<Client> {
        if let Some(e) = self.error {
            return Err(e);
        }
        if self.timeout.is_zero() || self.concurrency == 0 || self.max_bytes == 0 {
            return Err(AgentStatError::Configuration("limits must be positive"));
        }
        let http = self
            .http
            .redirect(reqwest::redirect::Policy::none())
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
            local_files: self.local_files,
            #[cfg(feature = "local-probes")]
            local_probes: self.local_probes,
            #[cfg(feature = "local-probes")]
            probe_executables: self.probe_executables,
        })
    }
}
impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }
    /// Includes disabled/planned providers, with explicit status and build availability.
    pub fn providers(&self) -> Vec<ProviderDescriptor> {
        provider_catalog()
    }
    pub async fn fetch(&self, mut request: FetchRequest) -> Result<UsageSnapshot> {
        request.deadline = tokio::time::Instant::now().checked_add(self.timeout);
        tokio::time::timeout(self.timeout, providers::fetch(self, &request))
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
