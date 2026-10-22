use crate::{Credentials, Decimal, ProviderId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Source {
    #[default]
    Auto,
    Api,
    OAuth,
    Web,
    LocalFile,
    LocalProbe,
}
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountScope {
    pub account_id: Option<String>,
    pub organization_id: Option<String>,
    pub workspace_id: Option<String>,
}
/// Provider-specific routing. Values are never read from the process environment implicitly.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum ProviderConfig {
    #[default]
    Default,
    Claude {
        organization_id: String,
    },
    Gemini {
        project_id: String,
    },
    Fireworks {
        account_id: String,
    },
    Exa {
        api_key_id: String,
    },
    Cosmic {
        project_id: String,
    },
    Flora {
        workspace_id: String,
    },
    Xai {
        team_id: String,
    },
    /// Explicitly trusted endpoint for self-hosted gateways (HTTPS or loopback HTTP).
    Gateway {
        base_url: String,
    },
}
#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub provider: ProviderId,
    pub credentials: Credentials,
    pub scope: AccountScope,
    pub source: Source,
    pub config: ProviderConfig,
    /// Fetch optional extra balances/profile data. A failure becomes a warning.
    pub include_optional: bool,
    pub(crate) credential_source: CredentialSource,
    pub(crate) deadline: Option<tokio::time::Instant>,
}
impl FetchRequest {
    pub fn new(provider: ProviderId, credentials: Credentials) -> Self {
        Self {
            provider,
            credentials,
            scope: AccountScope::default(),
            source: Source::Auto,
            config: ProviderConfig::Default,
            include_optional: false,
            credential_source: CredentialSource::Caller,
            deadline: None,
        }
    }
    pub fn source(mut self, source: Source) -> Self {
        self.source = source;
        self
    }
    pub fn scope(mut self, scope: AccountScope) -> Self {
        self.scope = scope;
        self
    }
    pub fn config(mut self, config: ProviderConfig) -> Self {
        self.config = config;
        self
    }
    pub fn include_optional(mut self, enabled: bool) -> Self {
        self.include_optional = enabled;
        self
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuotaLimit {
    Unknown,
    Unlimited,
    Known(Decimal),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Unit {
    Percent,
    Tokens,
    Requests,
    Characters,
    Slots,
    Bytes,
    Currency(String),
    Credits(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WindowKind {
    Rolling,
    Calendar,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaWindow {
    pub id: String,
    pub model: Option<String>,
    pub used: Option<Decimal>,
    pub remaining: Option<Decimal>,
    pub limit: QuotaLimit,
    pub unit: Unit,
    pub utilization_percent: Option<f64>,
    pub duration_seconds: Option<u64>,
    pub resets_at: Option<DateTime<Utc>>,
    pub kind: WindowKind,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub id: String,
    pub amount: Decimal,
    pub unit: Unit,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendSummary {
    pub id: String,
    pub amount: Decimal,
    pub currency: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Warning {
    OptionalFetchFailed,
    MalformedOptionalData,
    IncompleteData,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CredentialSource {
    Caller,
    LocalFile,
    LocalProbe,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub provider: ProviderId,
    pub scope: AccountScope,
    pub plan: Option<String>,
    pub fetched_at: DateTime<Utc>,
    pub source: Source,
    pub credential_source: CredentialSource,
    pub windows: Vec<QuotaWindow>,
    pub balances: Vec<Balance>,
    pub spend: Vec<SpendSummary>,
    pub warnings: Vec<Warning>,
}
impl UsageSnapshot {
    pub(crate) fn new(request: &FetchRequest, source: Source) -> Self {
        Self {
            provider: request.provider,
            scope: request.scope.clone(),
            plan: None,
            fetched_at: Utc::now(),
            source,
            credential_source: request.credential_source,
            windows: vec![],
            balances: vec![],
            spend: vec![],
            warnings: vec![],
        }
    }
}
