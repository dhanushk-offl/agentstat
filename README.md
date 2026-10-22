# agentstat

A Rust library for fetching current AI account usage, quota limits, reset times,
credit balances, and provider-reported spend. No tray, UI, daemon, or polling loop.

**Status: initial 0.1 implementation.** There are 38 remote provider adapters and
one opt-in local usage adapter. The catalog tracks 106 provider IDs;
63 integrations remain planned and four are excluded by the read-only policy.
See [provider coverage](docs/providers.md) for the exact data surfaces and
[the machine-readable manifest](docs/coverage.json) for integration status.

## Use from another Rust project

Until published, add the local crate:

```toml
[dependencies]
agentstat = { path = "../agentstat" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use agentstat::{Client, Credentials, FetchRequest, ProviderId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder().build()?;
    let snapshot = client.fetch(FetchRequest::new(
        ProviderId::Codex,
        Credentials::bearer("existing-access-token"),
    )).await?;

    for window in snapshot.windows {
        println!("{}: {:?}% used; resets {:?}",
            window.id, window.utilization_percent, window.resets_at);
    }
    Ok(())
}
```

Default features enable Codex and Claude. Enable another provider by its stable ID,
for example `features = ["copilot", "gemini", "openrouter"]`, or use
`features = ["all-providers"]`. `default-features = false` allows a smaller chosen
provider set. Planned providers still return an explicit unsupported error when
their Cargo feature is enabled; feature presence is not a support claim.

The library requires a Tokio runtime. It makes one fetch only when called. Reuse
`Client` to reuse its connection pool. `fetch_many` returns one `Result` per input
in input order and defaults to four concurrent fetches. Calling applications own
scheduling, storage, caching, and rendering.

## Credentials and account scope

Supply a bearer token, API key, or full Cookie header. These are separate credential
variants. Codex/Claude subscription APIs need their existing OAuth access tokens;
ordinary inference API keys do not grant subscription quota access. Use
`Credentials::api_key` for OpenAI organization costs and Claude Admin API costs.
The crate never logs in, renews tokens, extracts browser credentials, or writes
credential files. Expired credentials return an error for the caller to handle.

To read existing Codex or Claude Code credential files, enable `local-files` and
explicitly opt in:

```rust
use agentstat::{Client, ProviderId, local::CredentialLocation};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder().allow_local_files(true).build()?;
    let request = client.load_local_credentials(
        ProviderId::Codex, CredentialLocation::Native,
    ).await?;
    let snapshot = client.fetch(request).await?;
    println!("{} quota windows", snapshot.windows.len());
    Ok(())
}
```

Native Codex resolution respects `CODEX_HOME`, otherwise uses `~/.codex/auth.json`.
Claude respects `CLAUDE_CONFIG_DIR`, otherwise uses `~/.claude/.credentials.json`.
On Windows, native home lookup uses `USERPROFILE`. Explicit directory selection
stays isolated. File reads are bounded to 64 KiB. macOS Keychain credentials are
not read; pass a token explicitly if the CLI stores it there.

Codex workspace routing uses `AccountScope.account_id`. Claude web usage needs
`ProviderConfig::Claude { organization_id }` plus `Credentials::cookie`.
OpenCode uses `AccountScope.workspace_id` (`wrk_…` or `org_…`) and a caller-supplied
`__Host-console_session` Cookie header. Without an explicit workspace, discovery
requires exactly one available workspace.
Other scoped APIs use typed configuration, such as `ProviderConfig::Cosmic` or
`ProviderConfig::Exa`. Unsupported account routing is rejected rather than ignored.
A fetch never falls back to another account, credential, or source.

CodeRabbit is available with `coderabbit` and `local-probes`, plus
`Client::builder().allow_local_probes(true)`. Pass `Credentials::None` and
`Source::LocalProbe`. The only command executed is `coderabbit usage`, without a
shell or login operation. This source currently supports Unix; Windows returns
`UnsupportedPlatform`. Output is bounded to 128 KiB and the dedicated process
group is killed on timeout/cancellation. The third-party executable owns its
internal behavior; agentstat does not invoke its credential-repair commands.

## OpenCode usage

Enable the `opencode` Cargo feature, then supply an existing Console session cookie:

```toml
agentstat = { path = "../agentstat", features = ["opencode"] }
```

```rust
use agentstat::{AccountScope, Client, Credentials, FetchRequest, ProviderId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cookie = std::env::var("OPENCODE_COOKIE")?;
    let request = FetchRequest::new(ProviderId::OpenCode, Credentials::cookie(cookie))
        .scope(AccountScope {
            workspace_id: Some("wrk_your_workspace".into()),
            ..Default::default()
        });
    let snapshot = Client::builder().build()?.fetch(request).await?;
    println!("{} quota windows", snapshot.windows.len());
    Ok(())
}
```

Subscription accounts return five-hour, weekly, and monthly meters when reported.
Amounts in those meters represent USD-equivalent quota consumption, not billed
spend. Confirmed pay-as-you-go accounts return provider-reported spend over the
last 30 days and an available prepaid balance when reported. A missing quota is
never treated as zero or silently replaced with spend for a subscribed account.
This adapter uses the Console JSON API; legacy workspace-page sessions are not
supported. The separate `opencodego` provider entry remains planned.

## Data semantics

- `windows` contains the reported usage windows, including model-specific windows.
  Each has optional used/remaining counts, a unit, utilization, duration, and reset.
- Limits distinguish `Known`, `Unknown`, and `Unlimited`. Missing data stays missing.
  Percent-only APIs do not become invented token allowances.
- Five-hour and weekly utilization are quota windows, not active-work hours.
  Local hourly activity and transcript-based cost estimates are outside this release.
- Money uses exact `Decimal` values serialized as strings. Currencies and provider
  credit units stay separate. Legitimate quota overages are preserved.
- `source` identifies API/OAuth/web/file/probe delivery; `credential_source` records
  whether credentials came from the caller or opt-in local access.
- `fetched_at` is UTC observation time. Results are never substituted from a cache.
- Optional Codex reset-credit inventory is enabled by `include_optional(true)`.
  Its failure becomes a warning and preserves the successful primary reading.

HTTP sources support Linux, macOS, and Windows. Platform-specific local restrictions
are documented in the coverage table. Internal/private provider APIs can change;
a 0.1 adapter has fixture verification, not a guarantee of live-account availability.

## Transport and errors

Default fetch deadline: 30 seconds. Default response limit: 2 MiB. No automatic
retries, redirects, token refresh, or background activity. Errors distinguish
authentication, permission, rate limits with `Retry-After`, HTTP status, malformed
data, transport failure, timeout, build features, and unavailable sources. Remote
response bodies and credential values are omitted from errors and debug output.

`http_client_builder` allows proxy/TLS configuration; agentstat always disables
redirects on the resulting reqwest client. `endpoint_override` explicitly trusts
an alternative origin for one provider. Overrides require HTTPS or loopback HTTP,
without embedded credentials, fragments, or query parameters. Provider gateway
configuration is also an explicit trust decision.

For custom networking, implement `HttpExecutor` and set `http_executor`. Executors
receive sensitive HTTP requests and must preserve the no-redirect/no-retry and
cancellation contracts. The library checks returned status and body size; a custom
executor owns its network buffering policy. The default executor bounds streaming
response reads.

## Examples and validation

```sh
cargo run --example fetch -- codex
cargo run --features local-files --example local_credentials
cargo test --all-features
cargo test --no-default-features
cargo clippy --all-features --all-targets -- -D warnings
python3 scripts/check_coverage.py
```

The explicit-token example reads `AGENTSTAT_TOKEN` in the example application.
Do not put a token on the command line. The regular tests use synthetic fixtures
and in-memory HTTP, requiring no accounts, sockets, or network. Socket-based
reqwest smoke tests are explicitly run in CI. See [development notes](docs/development.md)
for feature checks, source maintenance, packaging, and live validation boundaries.
