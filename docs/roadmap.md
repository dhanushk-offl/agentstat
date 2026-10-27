# Roadmap and provider scope

[Back to the README](../README.md) · [Provider coverage](providers.md)

## Planned integrations

**60 integrations are planned.** They have catalog IDs and Cargo features, but no
working fetch adapter yet. This is a backlog, with no promised release order or
dates. Contributions are welcome.

| Provider | Planned feature ID |
| --- | --- |
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

## Outside the current scope

These entries are excluded from the current implementation rather than scheduled
for release:

| Provider | Feature ID | Reason |
| --- | --- | --- |
| Azure OpenAI | `azureopenai` | The identified access probe requires inference. |
| Doubao | `doubao` | The identified validation path generates a chat completion. |
| Muse Code | `muse` | The identified authentication path creates an API key. |
| Pi | `pi` | Local transcript scanning and estimated cost history are outside this crate. |
