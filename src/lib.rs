//! Read-only AI account usage. Fetches happen only when requested by the caller.
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use agentstat::{Client, Credentials, FetchRequest, ProviderId};
//! let client = Client::builder().build()?;
//! let request = FetchRequest::new(ProviderId::Codex, Credentials::bearer("existing-token"));
//! let snapshot = client.fetch(request).await?;
//! println!("{}", serde_json::to_string_pretty(&snapshot)?);
//! # Ok(()) }
//! ```
#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]
mod catalog;
mod client;
mod credentials;
mod decimal;
mod error;
#[cfg(feature = "local-files")]
pub mod local;
mod models;
#[cfg(all(feature = "opencode-local", unix))]
mod opencode_local;
#[cfg(feature = "local-probes")]
mod probe;
mod providers;
mod transport;

pub use catalog::{CoverageStatus, ProviderDescriptor, ProviderId, provider_catalog};
pub use client::{Client, ClientBuilder};
pub use credentials::{Credentials, Secret};
pub use decimal::Decimal;
pub use error::{AgentStatError, Result};
pub use models::*;
pub use transport::{HttpExecutor, HttpResponse};
