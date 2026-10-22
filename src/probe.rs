use crate::*;
#[cfg(unix)]
use tokio::io::AsyncReadExt;

pub(crate) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if !client.local_probes {
        return Err(AgentStatError::LocalAccessDisabled);
    }
    if !matches!(r.provider, ProviderId::CodeRabbit | ProviderId::Antigravity)
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
    #[cfg(not(unix))]
    {
        let _ = &client.probe_executables;
        Err(AgentStatError::UnsupportedPlatform)
    }
    #[cfg(unix)]
    {
        use std::{os::unix::process::CommandExt, process::Stdio};
        let path = client
            .probe_executables
            .get(&r.provider)
            .cloned()
            .unwrap_or_else(|| {
                if r.provider == ProviderId::Antigravity {
                    "agy".into()
                } else {
                    "coderabbit".into()
                }
            });
        let mut command = tokio::process::Command::new(path);
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
        } else {
            command.arg("usage");
        }
        command
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        command.as_std_mut().process_group(0);
        let mut child = command.spawn().map_err(|_| AgentStatError::Probe)?;
        let guard = ProcessGroup(child.id().map(|id| nix::unistd::Pid::from_raw(id as i32)));
        let mut out = Vec::new();
        let output = child.stdout.take().ok_or(AgentStatError::Probe)?;
        let operation = async {
            output
                .take(128 * 1024 + 1)
                .read_to_end(&mut out)
                .await
                .map_err(|_| AgentStatError::Probe)?;
            if out.len() > 128 * 1024 {
                return Err(AgentStatError::ResponseTooLarge);
            }
            let status = child.wait().await.map_err(|_| AgentStatError::Probe)?;
            if !status.success() {
                return Err(AgentStatError::Probe);
            }
            let text = String::from_utf8(out)
                .map_err(|_| AgentStatError::InvalidResponse("usage probe output is not UTF-8"))?;
            if r.provider == ProviderId::Antigravity {
                let body = serde_json::from_str(&text).map_err(|_| {
                    AgentStatError::InvalidResponse("expected Antigravity usage JSON")
                })?;
                crate::providers::parse_antigravity_probe(r, &body)
            } else {
                parse(r, &text)
            }
        };
        let result = tokio::time::timeout(std::time::Duration::from_secs(15), operation)
            .await
            .map_err(|_| AgentStatError::Timeout)?;
        drop(guard);
        result
    }
}
#[cfg(unix)]
struct ProcessGroup(Option<nix::unistd::Pid>);
#[cfg(unix)]
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            let _ = nix::sys::signal::killpg(pid, nix::sys::signal::Signal::SIGKILL);
        }
    }
}
#[cfg(unix)]
fn parse(r: &FetchRequest, text: &str) -> Result<UsageSnapshot> {
    let mut fields = std::collections::BTreeMap::new();
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
    s.windows.push(crate::providers::probe_window(used));
    Ok(s)
}
