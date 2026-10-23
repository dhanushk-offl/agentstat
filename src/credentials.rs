//! Credential wrappers that redact secrets in debug output.

use std::fmt;
use zeroize::Zeroizing;

/// Explicitly expose only when constructing a request. Never serialized; debug is redacted.
#[derive(Clone)]
pub struct Secret(Zeroizing<String>);
impl Secret {
    /// Creates a value with the supplied data.
    pub fn new(value: impl Into<String>) -> Self {
        Self(Zeroizing::new(value.into()))
    }
    #[must_use]
    /// Returns the secret value for use in an authorized request.
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}
#[derive(Clone, Debug)]
#[non_exhaustive]
/// Supported credential formats for provider requests.
pub enum Credentials {
    /// Uses a bearer token for authentication.
    Bearer(Secret),
    /// Uses an API key for authentication.
    ApiKey(Secret),
    /// Uses an HTTP Cookie header for authentication.
    Cookie(Secret),
    /// For sources that do not require authentication.
    None,
}
impl Credentials {
    /// Wraps a token for bearer authentication.
    pub fn bearer(value: impl Into<String>) -> Self {
        Self::Bearer(Secret::new(value))
    }
    /// Wraps a key for API key authentication.
    pub fn api_key(value: impl Into<String>) -> Self {
        Self::ApiKey(Secret::new(value))
    }
    /// A complete Cookie header, not a cURL command or browser export.
    pub fn cookie(value: impl Into<String>) -> Self {
        Self::Cookie(Secret::new(value))
    }
    pub(crate) fn value(&self) -> Option<&str> {
        match self {
            Self::Bearer(s) | Self::ApiKey(s) | Self::Cookie(s) => Some(s.expose()),
            Self::None => None,
        }
    }
}
