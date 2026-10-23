//! Errors returned by provider fetches and local sources.

use crate::ProviderId;
use chrono::{DateTime, Utc};
use std::result;
/// All failures deliberately omit remote response bodies, URLs, and credential values.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AgentStatError {
    #[error("provider {0} is not enabled in this build")]
    /// The requested provider was excluded by Cargo features.
    FeatureDisabled(ProviderId),
    #[error("provider {provider} is unavailable: {reason}")]
    /// The provider has no usable adapter in this release.
    Unsupported {
        /// Provider whose adapter is unavailable.
        provider: ProviderId,
        /// Why this adapter is unavailable.
        reason: &'static str,
    },
    #[error("selected source is unavailable for {0}")]
    /// The selected provider cannot use the requested data source.
    UnsupportedSource(ProviderId),
    #[error("credentials are missing or incompatible for {0}")]
    /// The provided credential format is unavailable or incompatible.
    InvalidCredentials(ProviderId),
    #[error("authentication rejected; renew the selected credentials outside agentstat")]
    /// Authentication failed; credentials may need renewal.
    Authentication,
    #[error("permission denied for the selected account")]
    /// The credentials cannot access the selected account.
    PermissionDenied,
    #[error("rate limited (retry after {retry_after:?})")]
    /// The provider rejected the request because of a rate limit.
    RateLimited {
        /// Suggested retry time, if supplied by the provider.
        retry_after: Option<DateTime<Utc>>,
    },
    #[error("provider returned HTTP {status}")]
    /// The provider returned an unsuccessful HTTP response.
    Http {
        /// HTTP response status code.
        status: u16,
    },
    #[error("request transport failed")]
    /// The network request failed before a usable response arrived.
    Transport,
    #[error("fetch deadline exceeded")]
    /// The fetch exceeded its configured deadline.
    Timeout,
    #[error("response exceeds the configured size limit")]
    /// The response exceeded the configured byte limit.
    ResponseTooLarge,
    #[error("invalid provider response: {0}")]
    /// The provider response could not be parsed or validated.
    InvalidResponse(&'static str),
    #[error("invalid configuration: {0}")]
    /// A client or request setting failed validation.
    Configuration(&'static str),
    #[error("local access has not been explicitly enabled")]
    /// A local source was requested without permission in the client configuration.
    LocalAccessDisabled,
    #[error("local credentials could not be read")]
    /// Local credential storage could not be read.
    LocalRead,
    #[error("selected source is unsupported on this platform")]
    /// The requested local source is unavailable on this platform.
    UnsupportedPlatform,
    #[error("local usage probe failed")]
    /// The audited local usage probe failed.
    Probe,
}
/// A crate result with an `AgentStatError` on failure.
pub type Result<T> = result::Result<T, AgentStatError>;
