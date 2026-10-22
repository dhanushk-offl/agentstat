# Usage guide

Detailed credential setup, account routing, live examples, and data behavior for agentstat.

[Back to the README](../README.md) · [Provider coverage](providers.md)

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

To read existing Codex, Claude Code, or Cursor CLI credential files, enable `local-files` and
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
Cursor CLI uses `$XDG_CONFIG_HOME/cursor/auth.json` (default `~/.config/cursor/auth.json`)
on Linux, `~/.cursor/auth.json` on macOS, and `%APPDATA%/Cursor/auth.json` on Windows
(default `%USERPROFILE%/AppData/Roaming/Cursor/auth.json`). Only its `accessToken`
is used; the crate does not read the Cursor IDE database or OS credential store.
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
`Source::LocalProbe`. Its command is `coderabbit usage`, without a
shell or login operation. This source currently supports Unix; Windows returns
`UnsupportedPlatform`. Output is bounded to 128 KiB and the dedicated process
group is killed on timeout/cancellation. The third-party executable owns its
internal behavior; agentstat does not invoke its credential-repair commands.

## Antigravity CLI and Cursor CLI usage

For an existing Antigravity (`agy`) CLI login on Linux or macOS:

```sh
cargo run --features antigravity,local-probes --example real_usage -- agy
```

This executes the CLI's read-only `agy --print /usage --output-format json` command,
with auto-update disabled and the main log directed to `/dev/null`. It reads
`command.data.groups[].buckets[]`, converts remaining fractions into usage
percentages, and preserves five-hour/weekly buckets and reset times. Disabled
buckets remain unknown. The installed CLI owns its login and runtime behavior;
agentstat supplies no prompts to a model. An older CLI without structured `/usage`
support returns an error. The process limit is 15 seconds and 128 KiB of stdout;
the process group is terminated on completion, timeout, or cancellation. The CLI
probe returns `UnsupportedPlatform` on Windows.

For direct HTTPS fetching on any supported platform, pass an existing OAuth token
as `Credentials::bearer` with `ProviderId::Antigravity`. The only endpoint queried
is `POST https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary`.
Use `ProviderConfig::Antigravity { project_id }` if your account requires a project.
The example uses `ANTIGRAVITY_ACCESS_TOKEN` and optional `ANTIGRAVITY_PROJECT_ID`
when supplied, bypassing the CLI. No onboarding, account switching, or token
refresh is performed by the direct adapter.

For an existing Cursor `agent` / `cursor-agent` CLI file login:

```sh
cargo run --features cursor,local-files --example real_usage -- cursor
```

This sends the CLI access token to the read-only `GetCurrentPeriodUsage` RPC at
`api2.cursor.sh`. For a dashboard session instead, supply the complete Cookie
header in `CURSOR_COOKIE` and run with `--features cursor`. The cookie path reads
`https://cursor.com/api/usage-summary`; it does not reuse or synthesize browser
cookies from the CLI token. Ordinary Cursor API keys are not supported by these
account-usage adapters.

Cursor returns provider-computed total, Cursor-model, and other-model percentages
where available, plus a separate monetary plan allowance and on-demand spend/limits.
Cents are converted exactly to USD. Plan allowance amounts describe quota
consumption, not an invoice. Percentages are preserved even if they differ from
the monetary allowance ratio. Team-wide dashboard usage is not substituted for
individual usage; RPC pooled limits have a distinct `on-demand-pooled` ID.
Reset times come from the reported billing cycle; no five-hour or weekly period
is invented for Cursor.

These integrations have request, normalization, and local-probe fixtures.
Fixture coverage does not establish live account compatibility. Compare a live
fetch with the provider’s own usage page before relying on its output.

## OpenCode usage

Enable the `opencode` Cargo feature, then supply an existing Console session cookie:

To run the live sample, sign into the OpenCode Console in your browser. In browser
developer tools, open Network, select an authenticated Console request, and copy
the complete `Cookie` request header value. Enter it at the prompt below:

```bash
read -r -s -p "OpenCode Console cookie: " OPENCODE_COOKIE
echo
export OPENCODE_COOKIE
cargo run --features opencode --example real_usage -- opencode
unset OPENCODE_COOKIE
```

If you have multiple workspaces, set `OPENCODE_WORKSPACE_ID` to your `wrk_...` or
`org_...` workspace ID before running. A single workspace is discovered
automatically. This sample uses a Console session cookie, not an OpenCode API key
or the CLI's saved provider credentials. It prints live usage; this adapter has
fixture coverage but has not yet been verified against a live OpenCode account.

To use it from another Rust project:

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
cargo run --features local-files --example real_usage -- codex
cargo test --all-features
cargo test --no-default-features
cargo clippy --all-features --all-targets -- -D warnings
python3 scripts/check_coverage.py
```

The `real_usage` example fetches live data using your existing local Codex login
and prints quota usage, limits, and reset times. Use `-- claude` for a local Claude
OAuth login, or enable `opencode` and use `-- opencode` with `OPENCODE_COOKIE`.
It requires network access and valid credentials; it does not log in
or refresh expired credentials. No sample values are substituted if fetching fails.

The explicit-token example reads `AGENTSTAT_TOKEN` in the example application.
Do not put a token on the command line. The regular tests use synthetic fixtures
and in-memory HTTP, requiring no accounts, sockets, or network. Socket-based
reqwest smoke tests are explicitly run in CI. See [development notes](development.md)
for feature checks, source maintenance, packaging, and live validation boundaries.
