//! Example for agentstat local credentials.

#[cfg(feature = "local-files")]
use std::error::Error;

#[cfg(feature = "local-files")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use agentstat::local::CredentialLocation;
    use agentstat::{Client, ProviderId};
    let client = Client::builder().allow_local_files(true).build()?;
    let request = client
        .load_local_credentials(ProviderId::Codex, CredentialLocation::Native)
        .await?;
    let snapshot = client.fetch(request).await?;
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    Ok(())
}
#[cfg(not(feature = "local-files"))]
fn main() {
    eprintln!("Enable local-files: cargo run --features local-files --example local_credentials");
}
