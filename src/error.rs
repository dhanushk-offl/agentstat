use crate::ProviderId;
use chrono::{DateTime, Utc};
/// All failures deliberately omit remote response bodies, URLs, and credential values.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AgentStatError {
    #[error("provider {0} is not enabled in this build")]
    FeatureDisabled(ProviderId),
    #[error("provider {provider} is unavailable: {reason}")]
    Unsupported {
        provider: ProviderId,
        reason: &'static str,
    },
    #[error("selected source is unavailable for {0}")]
    UnsupportedSource(ProviderId),
    #[error("credentials are missing or incompatible for {0}")]
    InvalidCredentials(ProviderId),
    #[error("authentication rejected; renew the selected credentials outside agentstat")]
    Authentication,
    #[error("permission denied for the selected account")]
    PermissionDenied,
    #[error("rate limited (retry after {retry_after:?})")]
    RateLimited { retry_after: Option<DateTime<Utc>> },
    #[error("provider returned HTTP {status}")]
    Http { status: u16 },
    #[error("request transport failed")]
    Transport,
    #[error("fetch deadline exceeded")]
    Timeout,
    #[error("response exceeds the configured size limit")]
    ResponseTooLarge,
    #[error("invalid provider response: {0}")]
    InvalidResponse(&'static str),
    #[error("invalid configuration: {0}")]
    Configuration(&'static str),
    #[error("local access has not been explicitly enabled")]
    LocalAccessDisabled,
    #[error("local credentials could not be read")]
    LocalRead,
    #[error("selected source is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("local usage probe failed")]
    Probe,
}
pub type Result<T> = std::result::Result<T, AgentStatError>;
