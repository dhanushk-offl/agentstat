use std::fmt;
use zeroize::Zeroizing;

/// Explicitly expose only when constructing a request. Never serialized; debug is redacted.
#[derive(Clone)]
pub struct Secret(Zeroizing<String>);
impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Zeroizing::new(value.into()))
    }
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
pub enum Credentials {
    Bearer(Secret),
    ApiKey(Secret),
    Cookie(Secret),
    /// For sources that do not require authentication.
    None,
}
impl Credentials {
    pub fn bearer(value: impl Into<String>) -> Self {
        Self::Bearer(Secret::new(value))
    }
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
