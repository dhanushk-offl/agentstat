//! Fetch real account usage using existing credentials or a read-only CLI probe.
//! Run: cargo run --features local-files --example real_usage -- codex
//! OpenCode: cargo run --features opencode --example real_usage -- opencode
//! OpenCode CLI: cargo run --features opencode-local --example real_usage -- opencode-cli

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use agentstat::{AccountScope, Client, Credentials, FetchRequest, ProviderId};

    let selection = std::env::args().nth(1).unwrap_or_else(|| "codex".into());
    let opencode_cli = matches!(selection.as_str(), "opencode-cli" | "opencode-local");
    let provider = match selection.as_str() {
        "codex" => ProviderId::Codex,
        "claude" => ProviderId::Claude,
        "opencode" | "zen" => ProviderId::OpenCode,
        "opencode-cli" | "opencode-local" => ProviderId::OpenCode,
        "opencode-go" | "go" => ProviderId::OpenCodeGo,
        "ollama" => ProviderId::Ollama,
        "cursor" | "agent" => ProviderId::Cursor,
        "antigravity" | "agy" => ProviderId::Antigravity,
        _ => {
            return Err(
                "Usage: real_usage [codex|claude|opencode|opencode-cli|opencode-go|ollama|cursor|agy]".into(),
            );
        }
    };
    if !provider.descriptor().enabled {
        return Err(format!(
            "Enable the {} Cargo feature to use this provider",
            provider.as_str()
        )
        .into());
    }
    let builder = Client::builder();
    #[cfg(feature = "local-files")]
    let builder = builder.allow_local_files(true);
    #[cfg(feature = "local-probes")]
    let builder = builder.allow_local_probes(true);
    let client = builder.build()?;
    let request = if opencode_cli {
        if !cfg!(feature = "opencode-local") {
            return Err("Enable OpenCode CLI statistics with --features opencode-local".into());
        }
        FetchRequest::new(provider, Credentials::None).source(agentstat::Source::LocalProbe)
    } else if provider == ProviderId::OpenCode {
        let cookie = std::env::var("OPENCODE_COOKIE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or(
                "Set OPENCODE_COOKIE to your Console Cookie header before running this sample",
            )?;
        let workspace_id = match std::env::var("OPENCODE_WORKSPACE_ID") {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(_) => return Err("OPENCODE_WORKSPACE_ID must contain valid text".into()),
        };
        FetchRequest::new(provider, Credentials::cookie(cookie))
            .scope(AccountScope {
                workspace_id,
                ..Default::default()
            })
            .include_optional(true)
    } else if provider == ProviderId::OpenCodeGo {
        FetchRequest::new(
            provider,
            Credentials::api_key(std::env::var("OPENCODE_GO_API_KEY")?),
        )
    } else if provider == ProviderId::Ollama {
        FetchRequest::new(
            provider,
            Credentials::api_key(std::env::var("OLLAMA_API_KEY")?),
        )
    } else if provider == ProviderId::Antigravity {
        if let Ok(token) = std::env::var("ANTIGRAVITY_ACCESS_TOKEN") {
            let mut request = FetchRequest::new(provider, Credentials::bearer(token));
            if let Ok(project_id) = std::env::var("ANTIGRAVITY_PROJECT_ID") {
                request = request.config(agentstat::ProviderConfig::Antigravity { project_id });
            }
            request
        } else {
            if !cfg!(feature = "local-probes") {
                return Err("Use --features antigravity,local-probes for the agy login, or supply ANTIGRAVITY_ACCESS_TOKEN for direct HTTPS fetching".into());
            }
            FetchRequest::new(provider, Credentials::None).source(agentstat::Source::LocalProbe)
        }
    } else if provider == ProviderId::Cursor && std::env::var_os("CURSOR_COOKIE").is_some() {
        FetchRequest::new(
            provider,
            Credentials::cookie(std::env::var("CURSOR_COOKIE")?),
        )
    } else {
        #[cfg(feature = "local-files")]
        {
            client
                .load_local_credentials(provider, agentstat::local::CredentialLocation::Native)
                .await?
        }
        #[cfg(not(feature = "local-files"))]
        return Err("Enable local login loading with --features local-files".into());
    };
    let usage = match client.fetch(request).await {
        Ok(usage) => usage,
        Err(agentstat::AgentStatError::Probe) if opencode_cli => {
            return Err("OpenCode CLI probe failed. Check that `opencode` is on PATH, then run `opencode stats --days 30` directly to see its error".into());
        }
        Err(error) => return Err(error.into()),
    };

    println!("Provider: {provider:?}");
    println!("Source: {:?}", usage.source);
    println!("Plan: {}", usage.plan.as_deref().unwrap_or("not reported"));
    println!("Fetched: {}", usage.fetched_at.to_rfc3339());
    for window in &usage.windows {
        let period = match window.duration_seconds {
            Some(18_000) => "5 hours".to_owned(),
            Some(604_800) => "weekly".to_owned(),
            Some(seconds) => format!("{seconds} seconds"),
            None => "period not reported".to_owned(),
        };
        println!("\n{} ({period})", window.id);
        if let Some(model) = &window.model {
            println!("  Model/group: {model}");
        }
        if let Some(percent) = window.utilization_percent {
            println!("  Used: {percent:.1}%");
        }
        if let Some(used) = &window.used {
            println!("  Used amount: {used} ({:?})", window.unit);
        }
        if let Some(remaining) = &window.remaining {
            println!("  Remaining amount: {remaining} ({:?})", window.unit);
        }
        println!("  Limit: {:?}", window.limit);
        println!(
            "  Resets: {}",
            window
                .resets_at
                .map(|time| time.to_rfc3339())
                .unwrap_or_else(|| "not reported".to_owned())
        );
    }
    if usage.windows.is_empty() {
        println!("No quota windows reported by the provider.");
    }
    for balance in &usage.balances {
        println!(
            "Balance {}: {} ({:?})",
            balance.id, balance.amount, balance.unit
        );
    }
    for spend in &usage.spend {
        println!("Spend {}: {} {}", spend.id, spend.amount, spend.currency);
    }
    if !usage.warnings.is_empty() {
        println!("Warnings: {:?}", usage.warnings);
    }
    Ok(())
}
