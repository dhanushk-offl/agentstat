# Provider coverage

The catalog tracks implemented, planned, and unsupported integrations. Implemented entries have request and normalization fixtures for the data described below. Enabling a Cargo feature does not implement a planned integration.

| Provider ID | Status | Data surface / reason |
| --- | --- | --- |
| `codex` | implemented | OAuth session/weekly/additional windows, credits, monthly credit limits; optional reset-credit count. Native auth.json read is opt-in. No CLI/PAT/web-dashboard fallback. |
| `openai` | implemented | Admin API organization cost report for the last 30 days, with bounded pagination. No legacy credit-grant fallback or usage charts. |
| `azureopenai` | unsupported | Only an inference-based access probe has been identified; no compatible read-only quota endpoint is available. |
| `claude` | implemented | OAuth or explicit-organization manual cookie quotas, scoped weekly windows, extra usage; Admin API organization cost report. No PTY, keychain, browser extraction, or cloud grant inventory. |
| `clinepass` | implemented | API five-hour, weekly, and monthly percentage quotas. |
| `cursor` | planned | Adapter port pending. |
| `opencode` | implemented | Console session cookie: five-hour, weekly, optional monthly USD-equivalent quota meters and resets. Confirmed PAYG accounts: reported 30-day spend and optional prepaid balance. Explicit workspace selection; discovery requires exactly one workspace. No browser extraction or legacy server-function fallback. |
| `opencodego` | planned | Adapter port pending. |
| `alibaba` | planned | Adapter port pending. |
| `alibabatokenplan` | planned | Adapter port pending. |
| `qwencloud` | planned | Adapter port pending. |
| `factory` | planned | Adapter port pending. |
| `fireworks` | planned | Adapter port pending. |
| `gemini` | implemented | Existing bearer token → retrieveUserQuota model buckets; optional explicit project ID. No token refresh or project discovery. |
| `antigravity` | planned | Adapter port pending. |
| `copilot` | implemented | Existing GitHub OAuth token → monthly quota snapshots; preserves explicit unlimited allowances. No device login or enterprise host discovery. |
| `devin` | planned | Adapter port pending. |
| `zai` | implemented | Personal quota API: token/time/credit windows. No team routing or model-usage enrichment. |
| `minimax` | planned | Adapter port pending. |
| `manus` | planned | Adapter port pending. |
| `kimi` | planned | Adapter port pending. |
| `kilo` | planned | Adapter port pending. |
| `kiro` | planned | Adapter port pending. |
| `vertexai` | planned | Adapter port pending. |
| `augment` | planned | Adapter port pending. |
| `jetbrains` | planned | Adapter port pending. |
| `moonshot` | implemented | available_balance balance, cash_balance balance, voucher_balance balance. |
| `amp` | planned | Adapter port pending. |
| `t3chat` | planned | Adapter port pending. |
| `langdock` | implemented | session quota, weekly quota. Manual Cookie header only. |
| `ollama` | implemented | included quota, purchased balance. |
| `synthetic` | implemented | API quota arrays or named five-hour, weekly, and search-hourly slots. No estimated spend. |
| `openrouter` | implemented | key quota, lifetime spend, daily spend, weekly spend, monthly spend. |
| `elevenlabs` | implemented | characters quota, voice-slots quota, professional-voices quota. |
| `warp` | implemented | requests quota. Bonus grants are not ported. |
| `windsurf` | planned | Adapter port pending. |
| `zed` | planned | Adapter port pending. |
| `perplexity` | planned | Adapter port pending. |
| `mimo` | planned | Adapter port pending. |
| `doubao` | unsupported | The available validation path generates chat completions and can consume quota. |
| `sakana` | planned | Adapter port pending. |
| `abacus` | planned | Adapter port pending. |
| `mistral` | planned | Adapter port pending. |
| `deepseek` | implemented | API balance endpoint: total, granted, and topped-up balances by currency. |
| `deepinfra` | planned | Adapter port pending. |
| `codebuff` | implemented | credits quota, available balance. Subscription weekly enrichment is not ported. |
| `venice` | implemented | usd balance, diem balance. |
| `commandcode` | planned | Adapter port pending. |
| `qoder` | planned | Adapter port pending. |
| `stepfun` | planned | Adapter port pending. |
| `bedrock` | planned | Adapter port pending. |
| `grok` | planned | Adapter port pending. |
| `groq` | planned | Adapter port pending. |
| `llmproxy` | planned | Adapter port pending. |
| `litellm` | planned | Adapter port pending. |
| `bifrost` | planned | Adapter port pending. |
| `aixy` | planned | Adapter port pending. |
| `deepgram` | planned | Adapter port pending. |
| `poe` | implemented | points balance. |
| `chutes` | planned | Adapter port pending. |
| `neuralwatt` | planned | Adapter port pending. |
| `helmcode` | planned | Adapter port pending. |
| `clawrouter` | implemented | monthly-budget quota, monthly-actual spend. |
| `longcat` | planned | Adapter port pending. |
| `sub2api` | planned | Adapter port pending. |
| `wayfinder` | planned | Adapter port pending. |
| `zenmux` | implemented | five-hour quota, weekly quota. |
| `aiand` | planned | Adapter port pending. |
| `zoommate` | planned | Adapter port pending. |
| `xai` | implemented | prepaid balance. |
| `xapi` | planned | Adapter port pending. |
| `notion` | planned | Adapter port pending. |
| `ibmbob` | planned | Adapter port pending. |
| `nous` | implemented | subscription quota, purchased balance, total balance, subscription balance. |
| `muse` | unsupported | The available authentication path creates an API key; key creation is excluded. |
| `coderabbit` | implemented | Unix-only opt-in coderabbit usage probe: reported review count and plan; no inferred allowance. |
| `replicate` | planned | Adapter port pending. |
| `huggingface` | planned | Adapter port pending. |
| `raycast` | planned | Adapter port pending. |
| `pi` | unsupported | Local transcript scanning and estimated-cost history are outside this crate's scope. |
| `v0` | planned | Adapter port pending. |
| `typesafe` | planned | Adapter port pending. |
| `hyper` | planned | Adapter port pending. |
| `gitkraken` | implemented | personal quota, organization quota. |
| `futureagi` | planned | Adapter port pending. |
| `devpass` | implemented | key-all-time quota, plan quota, premium-weekly quota. |
| `atlascloud` | implemented | available balance. |
| `vercel` | implemented | available balance, lifetime spend. |
| `llmman` | planned | Adapter port pending. |
| `xkiro` | implemented | daily-free-tokens quota. |
| `museai` | planned | Adapter port pending. |
| `lithosai` | planned | Adapter port pending. |
| `workbuddy` | planned | Adapter port pending. |
| `tavily` | implemented | plan quota, key quota, paygo quota. |
| `linkup` | implemented | available balance. |
| `tinyapi` | planned | Adapter port pending. |
| `exa` | implemented | month-to-date spend. |
| `cosmic` | implemented | input quota, output quota. Compact textual allowances are not interpreted; numeric counters remain available. |
| `aerostack` | implemented | monthly-tokens quota. |
| `sailresearch` | implemented | Provider-reported 30-day spend and available balance, converted from cents. |
| `sofya` | implemented | credits balance, plan_credits balance, purchased_credits balance. |
| `fal` | implemented | available balance. |
| `flora` | implemented | available balance, monthly spend. |
| `parallel` | planned | Adapter port pending. |
| `claudeplatform` | planned | Adapter port pending. |
| `munito` | implemented | Admin API daily/monthly spend and limits. No organization-profile enrichment. |
