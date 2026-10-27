//! Audited local usage probes with bounded output and deadlines.

#[cfg(unix)]
use crate::CredentialSource;
#[cfg(all(feature = "opencode-local", unix))]
use crate::opencode_local;
#[cfg(unix)]
use crate::providers;
use crate::{
    AgentStatError, Client, Credentials, FetchRequest, ProviderConfig, ProviderId, Result, Source,
    UsageSnapshot,
};
#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
#[cfg(unix)]
use std::collections::BTreeMap;
#[cfg(all(feature = "opencode-local", unix))]
use std::path::Path;
#[cfg(unix)]
use std::time::Duration;
#[cfg(unix)]
use tokio::io::AsyncReadExt;
#[cfg(unix)]
use tokio::process::Command;
#[cfg(unix)]
use tokio::time;

pub(crate) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if !client.local_access.probes() {
        return Err(AgentStatError::LocalAccessDisabled);
    }
    if !matches!(r.provider, ProviderId::CodeRabbit | ProviderId::Antigravity)
        && !(cfg!(feature = "opencode-local") && r.provider == ProviderId::OpenCode)
        || !matches!(r.source, Source::Auto | Source::LocalProbe)
        || !matches!(r.credentials, Credentials::None)
    {
        return Err(AgentStatError::UnsupportedSource(r.provider));
    }
    if !matches!(r.config, ProviderConfig::Default) {
        return Err(AgentStatError::Configuration(
            "local usage probes require default provider configuration",
        ));
    }
    if r.provider == ProviderId::OpenCode
        && (r.source != Source::LocalProbe || r.scope.workspace_id.is_some())
    {
        return Err(AgentStatError::Configuration(
            "OpenCode CLI statistics require Source::LocalProbe without a workspace scope",
        ));
    }
    #[cfg(not(unix))]
    {
        let _ = &client.probe_executables;
        Err(AgentStatError::UnsupportedPlatform)
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;
        let path = client
            .probe_executables
            .get(&r.provider)
            .cloned()
            .unwrap_or_else(|| {
                if r.provider == ProviderId::Antigravity {
                    "agy".into()
                } else if r.provider == ProviderId::OpenCode {
                    "opencode".into()
                } else {
                    "coderabbit".into()
                }
            });
        let mut command = Command::new(&path);
        if r.provider == ProviderId::Antigravity {
            command
                .args([
                    "--print",
                    "/usage",
                    "--output-format",
                    "json",
                    "--log-file",
                    "/dev/null",
                ])
                .env("AGY_CLI_DISABLE_AUTO_UPDATE", "true");
        } else if r.provider == ProviderId::OpenCode {
            command
                .args(["stats", "--days", "30", "--json"])
                .env("OPENCODE_DISABLE_AUTOUPDATE", "true")
                .env("OPENCODE_DISABLE_PRUNE", "true")
                .env("OPENCODE_DISABLE_MODELS_FETCH", "true");
        } else {
            command.arg("usage");
        }
        command
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        // OpenCode may start a managed local service that must outlive `stats`.
        // The other probes are bounded to their process group on timeout.
        if r.provider != ProviderId::OpenCode {
            command.as_std_mut().process_group(0);
        }
        let mut child = command.spawn().map_err(|_| AgentStatError::Probe)?;
        let guard = (r.provider != ProviderId::OpenCode)
            .then(|| ProcessGroup(child.id().map(|id| Pid::from_raw(id as i32))));
        let mut out = Vec::new();
        let max_bytes = if r.provider == ProviderId::OpenCode {
            1024 * 1024
        } else {
            128 * 1024
        };
        let output = child.stdout.take().ok_or(AgentStatError::Probe)?;
        let operation = async {
            output
                .take(max_bytes + 1)
                .read_to_end(&mut out)
                .await
                .map_err(|_| AgentStatError::Probe)?;
            if out.len() > max_bytes as usize {
                return Err(AgentStatError::ResponseTooLarge);
            }
            let status = child.wait().await.map_err(|_| AgentStatError::Probe)?;
            if !status.success() {
                #[cfg(feature = "opencode-local")]
                if r.provider == ProviderId::OpenCode {
                    return opencode_text(&path, r).await;
                }
                return Err(AgentStatError::Probe);
            }
            let text = String::from_utf8(out)
                .map_err(|_| AgentStatError::InvalidResponse("usage probe output is not UTF-8"))?;
            if r.provider == ProviderId::Antigravity {
                let body = serde_json::from_str(&text).map_err(|_| {
                    AgentStatError::InvalidResponse("expected Antigravity usage JSON")
                })?;
                providers::parse_antigravity_probe(r, &body)
            } else if r.provider == ProviderId::OpenCode {
                #[cfg(feature = "opencode-local")]
                {
                    opencode_local::parse(r, &text)
                }
                #[cfg(not(feature = "opencode-local"))]
                {
                    Err(AgentStatError::UnsupportedSource(r.provider))
                }
            } else {
                parse(r, &text)
            }
        };
        let timeout = if r.provider == ProviderId::OpenCode {
            25
        } else {
            15
        };
        let result = time::timeout(Duration::from_secs(timeout), operation)
            .await
            .map_err(|_| AgentStatError::Timeout)?;
        drop(guard);
        result
    }
}
#[cfg(all(feature = "opencode-local", unix))]
async fn opencode_text(path: &Path, r: &FetchRequest) -> Result<UsageSnapshot> {
    use std::process::Stdio;
    let mut command = Command::new(path);
    command
        .args(["stats", "--days", "30"])
        .env("OPENCODE_DISABLE_AUTOUPDATE", "true")
        .env("OPENCODE_DISABLE_PRUNE", "true")
        .env("OPENCODE_DISABLE_MODELS_FETCH", "true")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|_| AgentStatError::Probe)?;
    let mut out = Vec::new();
    let output = child.stdout.take().ok_or(AgentStatError::Probe)?;
    time::timeout(Duration::from_secs(25), async {
        output
            .take(128 * 1024 + 1)
            .read_to_end(&mut out)
            .await
            .map_err(|_| AgentStatError::Probe)?;
        if out.len() > 128 * 1024 {
            return Err(AgentStatError::ResponseTooLarge);
        }
        if !child
            .wait()
            .await
            .map_err(|_| AgentStatError::Probe)?
            .success()
        {
            return Err(AgentStatError::Probe);
        }
        let text = String::from_utf8(out)
            .map_err(|_| AgentStatError::InvalidResponse("OpenCode CLI output is not UTF-8"))?;
        opencode_local::parse_text(r, &text)
    })
    .await
    .map_err(|_| AgentStatError::Timeout)?
}
#[cfg(unix)]
struct ProcessGroup(Option<Pid>);
#[cfg(unix)]
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            let _ = signal::killpg(pid, Signal::SIGKILL);
        }
    }
}
#[cfg(unix)]
fn parse(r: &FetchRequest, text: &str) -> Result<UsageSnapshot> {
    let mut fields = BTreeMap::new();
    // Strip terminal CSI sequences without interpreting terminal control input.
    let mut clean = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.next() == Some('[') {
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
        } else {
            clean.push(c)
        }
    }
    for line in clean.lines() {
        if let Some((label, value)) = line.split_once(':') {
            fields
                .entry(label.trim().to_lowercase())
                .or_insert(value.trim());
        }
    }
    let used = fields
        .get("your reviews")
        .and_then(|n| n.parse::<u64>().ok())
        .ok_or(AgentStatError::InvalidResponse(
            "no review count in usage output",
        ))?;
    let mut s = UsageSnapshot::new(r, Source::LocalProbe);
    s.credential_source = CredentialSource::LocalProbe;
    s.plan = fields.get("plan").map(|s| s.to_string());
    s.windows.push(providers::probe_window(used));
    Ok(s)
}
