//! Inspect provider-reported model and quota windows using a local CLI login.
//! Run: cargo run --features local-files --example model_usage -- codex
//! Cursor: cargo run --features cursor,local-files --example model_usage -- cursor

#[cfg(feature = "local-files")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use agentstat::{Client, ProviderId, QuotaLimit, local::CredentialLocation};

    let provider = match std::env::args().nth(1).as_deref().unwrap_or("codex") {
        "codex" => ProviderId::Codex,
        "claude" => ProviderId::Claude,
        "cursor" => ProviderId::Cursor,
        _ => return Err("Usage: model_usage [codex|claude|cursor]".into()),
    };
    if !provider.descriptor().enabled {
        return Err(format!("Enable the {} Cargo feature", provider.as_str()).into());
    }

    let client = Client::builder().allow_local_files(true).build()?;
    let request = client
        .load_local_credentials(provider, CredentialLocation::Native)
        .await?;
    let snapshot = client.fetch(request).await?;

    println!("Provider: {}", provider.as_str());
    println!("Observed: {}", snapshot.fetched_at.to_rfc3339());
    for window in &snapshot.windows {
        println!("\nWindow: {}", window.id);
        println!(
            "  Model/group: {}",
            window.model.as_deref().unwrap_or("not specified")
        );
        if let Some(seconds) = window.duration_seconds {
            println!("  Period: {seconds} seconds");
        }
        if let Some(percent) = window.utilization_percent {
            println!("  Used: {percent:.1}%");
        }
        if let Some(used) = &window.used {
            println!("  Used amount: {used} ({:?})", window.unit);
        }
        if let Some(remaining) = &window.remaining {
            println!("  Remaining: {remaining} ({:?})", window.unit);
        }
        match &window.limit {
            QuotaLimit::Known(limit) => println!("  Limit: {limit} ({:?})", window.unit),
            QuotaLimit::Unlimited => println!("  Limit: unlimited"),
            QuotaLimit::Unknown => println!("  Limit: not reported"),
            _ => println!("  Limit: not recognized by this version"),
        }
        if let Some(reset) = window.resets_at {
            println!("  Resets: {}", reset.to_rfc3339());
        }
    }

    if snapshot.windows.is_empty() {
        println!("No quota windows were reported by this provider.");
    }
    Ok(())
}

#[cfg(not(feature = "local-files"))]
fn main() {
    eprintln!(
        "Enable local-files: cargo run --features local-files --example model_usage -- codex"
    );
}
