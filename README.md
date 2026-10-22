<p align="center">
  <img src="docs/media/logo.png" width="240" alt="agentstat logo">
</p>

<h1 align="center">agentstat</h1>

<p align="center">AI account usage, quotas, balances, and reset times for Rust applications.</p>

<p align="center">
  <a href="https://github.com/dhanushk-offl/agentstat/actions/workflows/ci.yml"><img src="https://github.com/dhanushk-offl/agentstat/actions/workflows/ci.yml/badge.svg" alt="Rust CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="docs/development.md"><img src="https://img.shields.io/badge/Rust-stable-orange?logo=rust" alt="Rust stable"></a>
  <a href="#currently-supported"><img src="https://img.shields.io/badge/providers-41_supported-brightgreen" alt="41 supported providers"></a>
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#currently-supported">Supported providers</a> ·
  <a href="#coming-soon">Coming soon</a> ·
  <a href="docs/usage.md">Usage guide</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

Fetch provider-reported data through one async API and use it in your own CLI,
dashboard, or application. agentstat is a library: your application decides when
to fetch, where to store results, and how to display them.

- Quota windows, utilization, limits, and reset times when the provider reports them.
- Exact decimal balances and spend, with currencies and credit units kept separate.
- Typed errors and batch fetching with bounded concurrency.
- Caller-supplied credentials, with explicit opt-in for local files and CLI probes.

**Early release: `0.1.0`.** The catalog tracks **41 implemented**, **61 planned**,
and **4 out-of-scope** providers. Implemented adapters have synthetic request and
normalization fixtures; live account compatibility still needs verification.
HTTP adapters target Linux, macOS, and Windows. Local CLI probes support Unix.

## Quick start

Until a crates.io release is available, use a local checkout:

```toml
[dependencies]
agentstat = { path = "../agentstat" }
tokio = { version = "1", features = ["macros", "rt"] }
```

Fetch Codex usage with an existing OAuth access token supplied by your application:

```rust
use agentstat::{Client, Credentials, FetchRequest, ProviderId};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("AGENTSTAT_TOKEN")?;
    let client = Client::builder().build()?;
    let request = FetchRequest::new(ProviderId::Codex, Credentials::bearer(token));
    let usage = client.fetch(request).await?;

    for window in usage.windows {
        if let Some(percent) = window.utilization_percent {
            println!("{}: {percent:.1}% used", window.id);
        }
        if let Some(reset) = window.resets_at {
            println!("  Resets: {reset}");
        }
    }
    Ok(())
}
```

Codex and Claude are enabled by default. Select other integrations by their
**feature ID** in the table below:

```toml
agentstat = { path = "../agentstat", default-features = false, features = ["cursor", "local-files"] }
```

`all-providers` enables every catalog feature; planned integrations still return
an unsupported error. Reuse `Client` for connection pooling. `fetch_many` returns
one result per request in input order, with four concurrent fetches by default.

## Try your real account

From this repository, after signing into the corresponding CLI:

| Account | Live command | Credentials used |
| --- | --- | --- |
| Codex | `cargo run --features local-files --example real_usage -- codex` | Existing local OAuth login |
| Claude Code | `cargo run --features local-files --example real_usage -- claude` | Existing local OAuth credential file |
| Cursor / `agent` | `cargo run --features cursor,local-files --example real_usage -- cursor` | Existing CLI `auth.json` |
| Antigravity / `agy` | `cargo run --features antigravity,local-probes --example real_usage -- agy` | Installed CLI login; Unix only |
| OpenCode | `cargo run --features opencode --example real_usage -- opencode` | Console cookie supplied in `OPENCODE_COOKIE` |

The examples fetch live data and return an error if the fetch fails. They never
substitute demonstration values. OpenCode needs a Console cookie rather than its
CLI provider API key. Some CLI logins use OS credential stores that agentstat
does not read. See the [usage guide](docs/usage.md) for cookie setup, native file
paths, account selection, direct token access, and platform restrictions.

## Currently supported

**41 providers**, with 40 remote adapters and two optional local probes
(Antigravity supports both paths). Each row describes the data implemented today;
not every provider exposes hourly or weekly limits.

| Provider | Feature ID | Available data |
| --- | --- | --- |
| Codex | `codex` | Five-hour, weekly, and additional quotas; credits and resets |
| Claude | `claude` | Subscription quotas, model windows, extra usage; Admin costs |
| Cursor | `cursor` | CLI or dashboard usage, plan allowances, on-demand spend and resets |
| Antigravity | `antigravity` | Grouped five-hour/weekly quotas and resets; optional `agy` probe |
| OpenCode | `opencode` | Console subscription quotas; confirmed PAYG spend and prepaid balance |
| OpenAI | `openai` | Organization costs through the Admin API |
| ClinePass | `clinepass` | Five-hour, weekly, and monthly quotas |
| Gemini app | `gemini` | Model quota buckets using an existing bearer token |
| Copilot | `copilot` | Monthly quota snapshots and reported unlimited allowances |
| z.ai / GLM | `zai` | Token, time, and credit quotas |
| Moonshot / Kimi Open Platform | `moonshot` | Available, cash, and voucher balances |
| Langdock | `langdock` | Session and weekly quotas using a supplied cookie |
| Ollama | `ollama` | Included quota and purchased balance |
| Synthetic | `synthetic` | Quota arrays; five-hour, weekly, and hourly search windows |
| OpenRouter | `openrouter` | Key quota; daily, weekly, monthly, and lifetime spend |
| ElevenLabs | `elevenlabs` | Character and voice-slot quotas |
| Warp | `warp` | Request quota |
| DeepSeek | `deepseek` | Balances by currency |
| Codebuff | `codebuff` | Credit quota and available balance |
| Venice | `venice` | USD and DIEM balances |
| Poe | `poe` | Points balance |
| ClawRouter | `clawrouter` | Monthly budget quota and reported spend |
| ZenMux | `zenmux` | Five-hour and weekly quotas |
| xAI | `xai` | Prepaid balance |
| Nous Portal | `nous` | Subscription quota and balances |
| CodeRabbit | `coderabbit` | Review count and plan through an opt-in Unix CLI probe |
| GitKraken AI | `gitkraken` | Personal and organization quotas |
| DevPass | `devpass` | Key, plan, and weekly premium quotas |
| Atlas Cloud | `atlascloud` | Available balance |
| Vercel AI Gateway | `vercel` | Available balance and lifetime spend |
| xKiro | `xkiro` | Daily free-token quota |
| Tavily | `tavily` | Plan, key, and PAYG quotas |
| Linkup | `linkup` | Available balance |
| Exa | `exa` | Month-to-date spend |
| Cosmic AI | `cosmic` | Numeric input and output quotas |
| Aerostack | `aerostack` | Monthly token quota |
| Sail Research | `sailresearch` | Reported 30-day spend and available balance |
| Sofya | `sofya` | Credit, plan-credit, and purchased-credit balances |
| fal | `fal` | Available balance |
| FLORA | `flora` | Available balance and monthly spend |
| Munito | `munito` | Admin daily/monthly spend and limits |

For endpoint details, authentication restrictions, and omitted enrichments, see
[full provider coverage](docs/providers.md). The
[coverage manifest](docs/coverage.json) records the status of every catalog entry.

## Coming soon

**61 integrations are planned.** They have catalog IDs and Cargo features, but no
working fetch adapter yet. This is a backlog, with no promised release order or
dates. Contributions are welcome.

<details>
<summary>View all planned providers</summary>

| Provider | Planned feature ID |
| --- | --- |
| OpenCode Go | `opencodego` |
| Alibaba Coding Plan | `alibaba` |
| Alibaba Token Plan | `alibabatokenplan` |
| Qwen Cloud | `qwencloud` |
| Droid | `factory` |
| Fireworks | `fireworks` |
| Devin | `devin` |
| MiniMax | `minimax` |
| Manus | `manus` |
| Kimi | `kimi` |
| Kilo | `kilo` |
| Kiro | `kiro` |
| Vertex AI | `vertexai` |
| Augment | `augment` |
| JetBrains AI | `jetbrains` |
| Amp | `amp` |
| T3 Chat | `t3chat` |
| Windsurf | `windsurf` |
| Zed | `zed` |
| Perplexity | `perplexity` |
| Xiaomi MiMo | `mimo` |
| Sakana AI | `sakana` |
| Abacus AI | `abacus` |
| Mistral | `mistral` |
| DeepInfra | `deepinfra` |
| Command Code | `commandcode` |
| Qoder | `qoder` |
| StepFun | `stepfun` |
| AWS Bedrock | `bedrock` |
| Grok | `grok` |
| Groq | `groq` |
| LLM Proxy | `llmproxy` |
| LiteLLM | `litellm` |
| Bifrost | `bifrost` |
| Aixy | `aixy` |
| Deepgram | `deepgram` |
| Chutes | `chutes` |
| Neuralwatt | `neuralwatt` |
| Helmcode | `helmcode` |
| LongCat | `longcat` |
| sub2api | `sub2api` |
| Wayfinder | `wayfinder` |
| ai& | `aiand` |
| ZoomMate | `zoommate` |
| X API | `xapi` |
| Notion AI | `notion` |
| IBM Bob | `ibmbob` |
| Replicate | `replicate` |
| Hugging Face | `huggingface` |
| Raycast | `raycast` |
| v0 | `v0` |
| TypeSafe | `typesafe` |
| Charm Hyper | `hyper` |
| Future AGI | `futureagi` |
| llmman | `llmman` |
| Muse (muse.ai) | `museai` |
| LithosAI | `lithosai` |
| WorkBuddy | `workbuddy` |
| TinyApi | `tinyapi` |
| Parallel | `parallel` |
| Claude Platform | `claudeplatform` |

</details>

### Outside the current scope

These entries are excluded from the current implementation rather than scheduled
for release:

| Provider | Feature ID | Reason |
| --- | --- | --- |
| Azure OpenAI | `azureopenai` | The identified access probe requires inference. |
| Doubao | `doubao` | The identified validation path generates a chat completion. |
| Muse Code | `muse` | The identified authentication path creates an API key. |
| Pi | `pi` | Local transcript scanning and estimated cost history are outside this crate. |

## Data and credentials

Quota windows describe allowance consumption, not hours actively spent coding.
Missing values stay missing; limits distinguish `Known`, `Unknown`, and
`Unlimited`. Monetary values use exact decimals serialized as strings. Quota
allowances and billed spend are separate concepts.

Pass a bearer token, API key, or full Cookie header as the provider requires.
The direct adapters do not log in, renew tokens, extract browser credentials, or
write credential files. Local file reads and probes require both a Cargo feature
and a runtime opt-in. An installed CLI owns its authentication and runtime behavior.

Fetches have a 30-second deadline and a 2 MiB HTTP response limit by default.
There are no automatic retries, redirects, or background polling. Errors omit
remote response bodies and credential values. Private provider APIs can change;
check the returned data against your provider's usage page.

## Documentation and community

| Resource | What you will find |
| --- | --- |
| [Usage guide](docs/usage.md) | Credentials, scopes, live examples, and transport options |
| [Provider coverage](docs/providers.md) | Data surfaces and restrictions for every provider |
| [Development guide](docs/development.md) | Architecture, feature checks, and packaging |
| [Contributing](CONTRIBUTING.md) | Bug reports, provider adapters, and pull requests |
| [Code of conduct](CODE_OF_CONDUCT.md) | Community expectations and reporting concerns |
| [Security policy](SECURITY.md) | Reporting vulnerabilities and credential exposure |

Run the local checks:

```sh
cargo fmt --check
cargo test --all-features
cargo test --no-default-features
cargo clippy --all-features --all-targets -- -D warnings
python3 scripts/check_coverage.py
```

The regular tests use synthetic fixtures and in-memory HTTP, with no live accounts
required. CI also runs socket smoke tests on Linux, macOS, and Windows.

## License

[MIT](LICENSE) © 2026 agentstat contributors.
