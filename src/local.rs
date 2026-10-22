//! Explicit local credential access. No browser, keychain, login, or token refresh operations.
use crate::*;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Default)]
pub enum CredentialLocation {
    #[default]
    Native,
    Directory(PathBuf),
}
impl Client {
    /// Read a known provider credential file. Native resolution happens only on this explicit call.
    pub async fn load_local_credentials(
        &self,
        provider: ProviderId,
        location: CredentialLocation,
    ) -> Result<FetchRequest> {
        if !self.local_files {
            return Err(AgentStatError::LocalAccessDisabled);
        }
        let path = match location {
            CredentialLocation::Directory(p) => credential_path(provider, &p)?,
            CredentialLocation::Native if provider == ProviderId::Cursor => {
                credential_path(provider, &cursor_directory()?)?
            }
            CredentialLocation::Native => {
                let (variable, fallback) = match provider {
                    ProviderId::Codex => ("CODEX_HOME", ".codex"),
                    ProviderId::Claude => ("CLAUDE_CONFIG_DIR", ".claude"),
                    _ => return Err(AgentStatError::UnsupportedSource(provider)),
                };
                let home = std::env::var_os(variable)
                    .filter(|v| !v.is_empty())
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                            .map(|p| PathBuf::from(p).join(fallback))
                    })
                    .ok_or(AgentStatError::LocalRead)?;
                credential_path(provider, &home)?
            }
        };
        let file = tokio::fs::File::open(path)
            .await
            .map_err(|_| AgentStatError::LocalRead)?;
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        file.take(65537)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| AgentStatError::LocalRead)?;
        if bytes.len() > 65536 {
            return Err(AgentStatError::LocalRead);
        }
        let data: Value = serde_json::from_slice(&bytes).map_err(|_| AgentStatError::LocalRead)?;
        let (token, account) = match provider {
            ProviderId::Codex => (
                data["tokens"]["access_token"].as_str(),
                data["tokens"]["account_id"].as_str(),
            ),
            ProviderId::Claude => {
                let auth = &data["claudeAiOauth"];
                if auth["expiresAt"]
                    .as_i64()
                    .is_some_and(|n| n <= chrono::Utc::now().timestamp_millis())
                {
                    return Err(AgentStatError::Authentication);
                }
                (auth["accessToken"].as_str(), None)
            }
            ProviderId::Cursor => (data["accessToken"].as_str(), None),
            _ => return Err(AgentStatError::UnsupportedSource(provider)),
        };
        let token = token
            .filter(|t| !t.trim().is_empty())
            .ok_or(AgentStatError::InvalidCredentials(provider))?;
        let mut request = FetchRequest::new(provider, Credentials::bearer(token));
        request.scope.account_id = account.map(str::to_owned);
        request.credential_source = CredentialSource::LocalFile;
        Ok(request)
    }
}
fn credential_path(provider: ProviderId, home: &Path) -> Result<PathBuf> {
    match provider {
        ProviderId::Codex => Ok(home.join("auth.json")),
        ProviderId::Claude => Ok(home.join(".credentials.json")),
        ProviderId::Cursor => Ok(home.join("auth.json")),
        _ => Err(AgentStatError::UnsupportedSource(provider)),
    }
}

fn cursor_directory() -> Result<PathBuf> {
    let home = || {
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .ok_or(AgentStatError::LocalRead)
    };
    if cfg!(windows) {
        let root = std::env::var_os("APPDATA")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(|| home().map(|p| p.join("AppData/Roaming")))?;
        Ok(root.join("Cursor"))
    } else if cfg!(target_os = "macos") {
        Ok(home()?.join(".cursor"))
    } else {
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(|| home().map(|p| p.join(".config")))?;
        Ok(root.join("cursor"))
    }
}
pub(crate) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if !matches!(r.credentials, Credentials::None) {
        return Err(AgentStatError::Configuration(
            "local credential discovery requires Credentials::None",
        ));
    }
    let mut request = client
        .load_local_credentials(r.provider, CredentialLocation::Native)
        .await?;
    if let Some(expected) = &r.scope.account_id
        && request
            .scope
            .account_id
            .as_ref()
            .is_some_and(|actual| actual != expected)
    {
        return Err(AgentStatError::InvalidResponse(
            "local credential account scope mismatch",
        ));
    }
    if r.scope.account_id.is_some() {
        request.scope.account_id = r.scope.account_id.clone();
    }
    request.scope.organization_id = r.scope.organization_id.clone();
    request.scope.workspace_id = r.scope.workspace_id.clone();
    request.config = r.config.clone();
    request.include_optional = r.include_optional;
    request.deadline = r.deadline;
    let mut result = match r.provider {
        ProviderId::Codex => crate::providers::fetch_remote_codex(client, &request).await,
        ProviderId::Claude => crate::providers::fetch_remote_claude(client, &request).await,
        ProviderId::Cursor => crate::providers::fetch_remote_cursor(client, &request).await,
        _ => Err(AgentStatError::UnsupportedSource(r.provider)),
    }?;
    result.credential_source = CredentialSource::LocalFile;
    Ok(result)
}
