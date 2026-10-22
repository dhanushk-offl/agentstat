use agentstat::{Client, Credentials, FetchRequest, ProviderId};
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider: ProviderId = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "codex".into())
        .parse()?;
    // Environment access belongs to this example application, not the library.
    let token = std::env::var("AGENTSTAT_TOKEN")?;
    let credentials = if provider == ProviderId::OpenAI {
        Credentials::api_key(token)
    } else {
        Credentials::bearer(token)
    };
    let snapshot = Client::builder()
        .build()?
        .fetch(FetchRequest::new(provider, credentials))
        .await?;
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    Ok(())
}
