mod admin;
mod antigravity;
mod claude;
mod codex;
mod common;
mod cursor;
mod custom;
mod declarative;
mod opencode;
use crate::*;
pub(crate) async fn fetch(client: &Client, request: &FetchRequest) -> Result<UsageSnapshot> {
    if [
        &request.scope.account_id,
        &request.scope.organization_id,
        &request.scope.workspace_id,
    ]
    .into_iter()
    .flatten()
    .any(|id| id.trim().is_empty())
    {
        return Err(AgentStatError::Configuration(
            "account scope identifiers must not be empty",
        ));
    }
    // Scope is never silently ignored or forwarded to a different account source.
    let invalid_scope = match request.provider {
        ProviderId::Codex => {
            request.scope.organization_id.is_some() || request.scope.workspace_id.is_some()
        }
        ProviderId::Claude => {
            request.scope.account_id.is_some() || request.scope.workspace_id.is_some()
        }
        ProviderId::OpenCode => {
            request.scope.account_id.is_some() || request.scope.organization_id.is_some()
        }
        _ => {
            request.scope.account_id.is_some()
                || request.scope.organization_id.is_some()
                || request.scope.workspace_id.is_some()
        }
    };
    if invalid_scope {
        return Err(AgentStatError::Configuration(
            "this provider does not support the requested account scope; use its typed provider configuration",
        ));
    }
    let descriptor = request.provider.descriptor();
    if descriptor.status == CoverageStatus::Unsupported {
        return Err(AgentStatError::Unsupported {
            provider: request.provider,
            reason: descriptor.reason,
        });
    }
    if descriptor.status == CoverageStatus::Planned {
        return Err(AgentStatError::Unsupported {
            provider: request.provider,
            reason: "adapter has not been ported yet; see coverage manifest",
        });
    }
    if !descriptor.enabled {
        return Err(AgentStatError::FeatureDisabled(request.provider));
    }
    #[cfg(feature = "local-files")]
    if request.source == Source::LocalFile {
        return crate::local::fetch(client, request).await;
    }
    #[cfg(feature = "local-probes")]
    if request.source == Source::LocalProbe || request.provider == ProviderId::CodeRabbit {
        return crate::probe::fetch(client, request).await;
    }
    match request.provider {
        ProviderId::Codex => codex::fetch(client, request).await,
        ProviderId::Claude if matches!(request.credentials, Credentials::ApiKey(_)) => {
            admin::fetch(client, request).await
        }
        ProviderId::Claude => claude::fetch(client, request).await,
        ProviderId::OpenAI => admin::fetch(client, request).await,
        ProviderId::OpenCode => opencode::fetch(client, request).await,
        ProviderId::Cursor => cursor::fetch(client, request).await,
        ProviderId::Antigravity => antigravity::fetch(client, request).await,
        _ => {
            if let Some(spec) = declarative::spec(request.provider) {
                declarative::fetch(client, request, spec).await
            } else {
                custom::fetch(client, request).await
            }
        }
    }
}

#[cfg(all(feature = "local-probes", unix))]
pub(crate) fn parse_antigravity_probe(
    r: &FetchRequest,
    body: &serde_json::Value,
) -> Result<UsageSnapshot> {
    antigravity::parse(r, body, Source::LocalProbe)
}

#[cfg(feature = "local-files")]
pub(crate) async fn fetch_remote_cursor(c: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    cursor::fetch(c, r).await
}

#[cfg(feature = "local-files")]
pub(crate) async fn fetch_remote_codex(c: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    codex::fetch(c, r).await
}
#[cfg(feature = "local-files")]
pub(crate) async fn fetch_remote_claude(c: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    claude::fetch(c, r).await
}
#[cfg(all(feature = "local-probes", unix))]
pub(crate) fn probe_window(used: u64) -> QuotaWindow {
    common::window(
        "reviews",
        Unit::Requests,
        Some(Decimal::from(used)),
        None,
        None,
    )
}
