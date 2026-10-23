//! Request settings and provider usage snapshot types.

use crate::{Credentials, Decimal, ProviderId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Ways to obtain usage data from a provider.
pub enum Source {
    #[default]
    /// Selects the default available source.
    Auto,
    /// Selects the provider API.
    Api,
    /// Selects an OAuth-backed source.
    OAuth,
    /// Selects a web endpoint.
    Web,
    /// Selects locally stored data.
    LocalFile,
    /// Selects an audited local process.
    LocalProbe,
}
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
/// Optional account identifiers used to route a request.
#[expect(
    clippy::exhaustive_structs,
    reason = "public struct literals are part of the existing crate API"
)]
pub struct AccountScope {
    /// Provider account identifier.
    pub account_id: Option<String>,
    /// Provider organization identifier.
    pub organization_id: Option<String>,
    /// Provider workspace identifier.
    pub workspace_id: Option<String>,
}
/// Provider-specific routing. Values are never read from the process environment implicitly.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum ProviderConfig {
    #[default]
    /// Uses default provider routing.
    Default,
    /// Routes Claude requests through a specified organization.
    Claude {
        /// Organization used for billing and quota lookup.
        organization_id: String,
    },
    /// Routes Gemini requests through a specified project.
    Gemini {
        /// Project used for quota lookup.
        project_id: String,
    },
    /// Routes Antigravity requests through a specified project.
    Antigravity {
        /// Project used for quota lookup.
        project_id: String,
    },
    /// Routes Fireworks requests through a specified account.
    Fireworks {
        /// Account used for quota lookup.
        account_id: String,
    },
    /// Routes Exa requests using a specified key identifier.
    Exa {
        /// Key identifier used for quota lookup.
        api_key_id: String,
    },
    /// Routes Cosmic requests through a specified project.
    Cosmic {
        /// Project used for quota lookup.
        project_id: String,
    },
    /// Routes Flora requests through a specified workspace.
    Flora {
        /// Workspace used for quota lookup.
        workspace_id: String,
    },
    /// Routes xAI requests through a specified team.
    Xai {
        /// Team used for quota lookup.
        team_id: String,
    },
    /// Explicitly trusted endpoint for self-hosted gateways (HTTPS or loopback HTTP).
    Gateway {
        /// HTTPS or loopback HTTP endpoint for the gateway.
        base_url: String,
    },
}
#[derive(Debug, Clone)]
/// Credentials, routing, and source preferences for one fetch.
pub struct FetchRequest {
    /// Provider associated with this value.
    pub provider: ProviderId,
    /// Credentials used for the request.
    pub credentials: Credentials,
    /// Account scope for the request.
    pub scope: AccountScope,
    /// Requested data source.
    pub source: Source,
    /// Provider-specific routing configuration.
    pub config: ProviderConfig,
    /// Fetch optional extra balances/profile data. A failure becomes a warning.
    pub include_optional: bool,
    pub(crate) credential_source: CredentialSource,
    pub(crate) deadline: Option<Instant>,
}
impl FetchRequest {
    #[must_use]
    /// Creates a request using caller-supplied credentials and default routing.
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
    #[must_use]
    /// Selects the source for this request.
    pub fn source(mut self, source: Source) -> Self {
        self.source = source;
        self
    }
    #[must_use]
    /// Sets account routing identifiers for this request.
    pub fn scope(mut self, scope: AccountScope) -> Self {
        self.scope = scope;
        self
    }
    #[must_use]
    /// Sets provider-specific routing for this request.
    pub fn config(mut self, config: ProviderConfig) -> Self {
        self.config = config;
        self
    }
    #[must_use]
    /// Requests optional balance and profile data when enabled.
    pub fn include_optional(mut self, enabled: bool) -> Self {
        self.include_optional = enabled;
        self
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[non_exhaustive]
/// Whether a quota has a known finite limit.
pub enum QuotaLimit {
    /// Indicates that the value is unavailable.
    Unknown,
    /// Indicates that no quota limit applies.
    Unlimited,
    /// Contains the numeric quota limit.
    Known(Decimal),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
#[non_exhaustive]
/// Units used by provider quota and balance values.
pub enum Unit {
    /// Measures a percentage.
    Percent,
    /// Measures token consumption.
    Tokens,
    /// Counts requests.
    Requests,
    /// Counts characters.
    Characters,
    /// Counts available slots.
    Slots,
    /// Measures bytes.
    Bytes,
    /// Measures an amount in the named currency.
    Currency(String),
    /// Measures provider credits.
    Credits(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// How a quota window is bounded in time.
pub enum WindowKind {
    /// Uses a rolling time window.
    Rolling,
    /// Uses a calendar-aligned window.
    Calendar,
    /// Indicates that the value is unavailable.
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Usage and reset information for one quota window.
#[expect(
    clippy::exhaustive_structs,
    reason = "public struct literals are part of the existing crate API"
)]
pub struct QuotaWindow {
    /// Stable identifier for this entry.
    pub id: String,
    /// Model associated with the quota window.
    pub model: Option<String>,
    /// Amount used in the window.
    pub used: Option<Decimal>,
    /// Amount remaining in the window.
    pub remaining: Option<Decimal>,
    /// Configured quota limit.
    pub limit: QuotaLimit,
    /// Unit of the reported quantity.
    pub unit: Unit,
    /// Share of the limit already used, in percent.
    pub utilization_percent: Option<f64>,
    /// Length of the window in seconds.
    pub duration_seconds: Option<u64>,
    /// Time at which the window resets.
    pub resets_at: Option<DateTime<Utc>>,
    /// Window calculation type.
    pub kind: WindowKind,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// A provider-reported credit or currency balance.
#[expect(
    clippy::exhaustive_structs,
    reason = "public struct literals are part of the existing crate API"
)]
pub struct Balance {
    /// Stable identifier for this entry.
    pub id: String,
    /// Reported amount.
    pub amount: Decimal,
    /// Unit of the reported quantity.
    pub unit: Unit,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Spending over a provider-reported period.
#[expect(
    clippy::exhaustive_structs,
    reason = "public struct literals are part of the existing crate API"
)]
pub struct SpendSummary {
    /// Stable identifier for this entry.
    pub id: String,
    /// Reported amount.
    pub amount: Decimal,
    /// Currency code for the amount.
    pub currency: String,
    /// Start of the spending period.
    pub starts_at: Option<DateTime<Utc>>,
    /// End of the spending period.
    pub ends_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Non-fatal issues that left the primary snapshot available.
pub enum Warning {
    /// An optional request failed after the primary fetch succeeded.
    OptionalFetchFailed,
    /// Optional response data could not be parsed.
    MalformedOptionalData,
    /// The provider returned only part of the expected data.
    IncompleteData,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Where the credentials for a fetch originated.
pub enum CredentialSource {
    /// Credentials were supplied by the caller.
    Caller,
    /// Selects locally stored data.
    LocalFile,
    /// Selects an audited local process.
    LocalProbe,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Usage, balances, and warnings returned by a provider.
#[expect(
    clippy::exhaustive_structs,
    reason = "public struct literals are part of the existing crate API"
)]
pub struct UsageSnapshot {
    /// Provider associated with this value.
    pub provider: ProviderId,
    /// Account scope for the request.
    pub scope: AccountScope,
    /// Plan name reported by the provider.
    pub plan: Option<String>,
    /// Time the snapshot was fetched.
    pub fetched_at: DateTime<Utc>,
    /// Requested data source.
    pub source: Source,
    /// Origin of the credentials.
    pub credential_source: CredentialSource,
    /// Quota windows reported by the provider.
    pub windows: Vec<QuotaWindow>,
    /// Balances reported by the provider.
    pub balances: Vec<Balance>,
    /// Spending summaries reported by the provider.
    pub spend: Vec<SpendSummary>,
    /// Non-fatal problems encountered while fetching.
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
