//! Provider IDs, coverage status, and build-time availability.
use crate::{AgentStatError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[non_exhaustive]
/// Identifiers for supported and planned provider integrations.
pub enum ProviderId {
    #[serde(rename = "codex")]
    /// Selects the `codex` provider integration.
    Codex,
    #[serde(rename = "openai")]
    /// Selects the `openai` provider integration.
    OpenAI,
    #[serde(rename = "azureopenai")]
    /// Selects the `azureopenai` provider integration.
    AzureOpenAI,
    #[serde(rename = "claude")]
    /// Selects the `claude` provider integration.
    Claude,
    #[serde(rename = "clinepass")]
    /// Selects the `clinepass` provider integration.
    ClinePass,
    #[serde(rename = "cursor")]
    /// Selects the `cursor` provider integration.
    Cursor,
    #[serde(rename = "opencode")]
    /// Selects the `opencode` provider integration.
    OpenCode,
    #[serde(rename = "opencodego")]
    /// Selects the `opencodego` provider integration.
    OpenCodeGo,
    #[serde(rename = "alibaba")]
    /// Selects the `alibaba` provider integration.
    Alibaba,
    #[serde(rename = "alibabatokenplan")]
    /// Selects the `alibabatokenplan` provider integration.
    AlibabaTokenPlan,
    #[serde(rename = "qwencloud")]
    /// Selects the `qwencloud` provider integration.
    QwenCloud,
    #[serde(rename = "factory")]
    /// Selects the `factory` provider integration.
    Factory,
    #[serde(rename = "fireworks")]
    /// Selects the `fireworks` provider integration.
    Fireworks,
    #[serde(rename = "gemini")]
    /// Selects the `gemini` provider integration.
    Gemini,
    #[serde(rename = "antigravity")]
    /// Selects the `antigravity` provider integration.
    Antigravity,
    #[serde(rename = "copilot")]
    /// Selects the `copilot` provider integration.
    Copilot,
    #[serde(rename = "devin")]
    /// Selects the `devin` provider integration.
    Devin,
    #[serde(rename = "zai")]
    /// Selects the `zai` provider integration.
    Zai,
    #[serde(rename = "minimax")]
    /// Selects the `minimax` provider integration.
    Minimax,
    #[serde(rename = "manus")]
    /// Selects the `manus` provider integration.
    Manus,
    #[serde(rename = "kimi")]
    /// Selects the `kimi` provider integration.
    Kimi,
    #[serde(rename = "kilo")]
    /// Selects the `kilo` provider integration.
    Kilo,
    #[serde(rename = "kiro")]
    /// Selects the `kiro` provider integration.
    Kiro,
    #[serde(rename = "vertexai")]
    /// Selects the `vertexai` provider integration.
    VertexAI,
    #[serde(rename = "augment")]
    /// Selects the `augment` provider integration.
    Augment,
    #[serde(rename = "jetbrains")]
    /// Selects the `jetbrains` provider integration.
    JetBrains,
    #[serde(rename = "moonshot")]
    /// Selects the `moonshot` provider integration.
    Moonshot,
    #[serde(rename = "amp")]
    /// Selects the `amp` provider integration.
    Amp,
    #[serde(rename = "t3chat")]
    /// Selects the `t3chat` provider integration.
    T3Chat,
    #[serde(rename = "langdock")]
    /// Selects the `langdock` provider integration.
    Langdock,
    #[serde(rename = "ollama")]
    /// Selects the `ollama` provider integration.
    Ollama,
    #[serde(rename = "synthetic")]
    /// Selects the `synthetic` provider integration.
    Synthetic,
    #[serde(rename = "openrouter")]
    /// Selects the `openrouter` provider integration.
    OpenRouter,
    #[serde(rename = "elevenlabs")]
    /// Selects the `elevenlabs` provider integration.
    ElevenLabs,
    #[serde(rename = "warp")]
    /// Selects the `warp` provider integration.
    Warp,
    #[serde(rename = "windsurf")]
    /// Selects the `windsurf` provider integration.
    Windsurf,
    #[serde(rename = "zed")]
    /// Selects the `zed` provider integration.
    Zed,
    #[serde(rename = "perplexity")]
    /// Selects the `perplexity` provider integration.
    Perplexity,
    #[serde(rename = "mimo")]
    /// Selects the `mimo` provider integration.
    Mimo,
    #[serde(rename = "doubao")]
    /// Selects the `doubao` provider integration.
    Doubao,
    #[serde(rename = "sakana")]
    /// Selects the `sakana` provider integration.
    Sakana,
    #[serde(rename = "abacus")]
    /// Selects the `abacus` provider integration.
    Abacus,
    #[serde(rename = "mistral")]
    /// Selects the `mistral` provider integration.
    Mistral,
    #[serde(rename = "deepseek")]
    /// Selects the `deepseek` provider integration.
    DeepSeek,
    #[serde(rename = "deepinfra")]
    /// Selects the `deepinfra` provider integration.
    DeepInfra,
    #[serde(rename = "codebuff")]
    /// Selects the `codebuff` provider integration.
    Codebuff,
    #[serde(rename = "venice")]
    /// Selects the `venice` provider integration.
    Venice,
    #[serde(rename = "commandcode")]
    /// Selects the `commandcode` provider integration.
    CommandCode,
    #[serde(rename = "qoder")]
    /// Selects the `qoder` provider integration.
    Qoder,
    #[serde(rename = "stepfun")]
    /// Selects the `stepfun` provider integration.
    Stepfun,
    #[serde(rename = "bedrock")]
    /// Selects the `bedrock` provider integration.
    Bedrock,
    #[serde(rename = "grok")]
    /// Selects the `grok` provider integration.
    Grok,
    #[serde(rename = "groq")]
    /// Selects the `groq` provider integration.
    Groq,
    #[serde(rename = "llmproxy")]
    /// Selects the `llmproxy` provider integration.
    LLMProxy,
    #[serde(rename = "litellm")]
    /// Selects the `litellm` provider integration.
    LiteLLM,
    #[serde(rename = "bifrost")]
    /// Selects the `bifrost` provider integration.
    Bifrost,
    #[serde(rename = "aixy")]
    /// Selects the `aixy` provider integration.
    Aixy,
    #[serde(rename = "deepgram")]
    /// Selects the `deepgram` provider integration.
    Deepgram,
    #[serde(rename = "poe")]
    /// Selects the `poe` provider integration.
    Poe,
    #[serde(rename = "chutes")]
    /// Selects the `chutes` provider integration.
    Chutes,
    #[serde(rename = "neuralwatt")]
    /// Selects the `neuralwatt` provider integration.
    NeuralWatt,
    #[serde(rename = "helmcode")]
    /// Selects the `helmcode` provider integration.
    Helmcode,
    #[serde(rename = "clawrouter")]
    /// Selects the `clawrouter` provider integration.
    Clawrouter,
    #[serde(rename = "longcat")]
    /// Selects the `longcat` provider integration.
    LongCat,
    #[serde(rename = "sub2api")]
    /// Selects the `sub2api` provider integration.
    Sub2API,
    #[serde(rename = "wayfinder")]
    /// Selects the `wayfinder` provider integration.
    Wayfinder,
    #[serde(rename = "zenmux")]
    /// Selects the `zenmux` provider integration.
    Zenmux,
    #[serde(rename = "aiand")]
    /// Selects the `aiand` provider integration.
    Aiand,
    #[serde(rename = "zoommate")]
    /// Selects the `zoommate` provider integration.
    ZoomMate,
    #[serde(rename = "xai")]
    /// Selects the `xai` provider integration.
    Xai,
    #[serde(rename = "xapi")]
    /// Selects the `xapi` provider integration.
    XApi,
    #[serde(rename = "notion")]
    /// Selects the `notion` provider integration.
    Notion,
    #[serde(rename = "ibmbob")]
    /// Selects the `ibmbob` provider integration.
    IBMBob,
    #[serde(rename = "nous")]
    /// Selects the `nous` provider integration.
    Nous,
    #[serde(rename = "muse")]
    /// Selects the `muse` provider integration.
    Muse,
    #[serde(rename = "coderabbit")]
    /// Selects the `coderabbit` provider integration.
    CodeRabbit,
    #[serde(rename = "replicate")]
    /// Selects the `replicate` provider integration.
    Replicate,
    #[serde(rename = "huggingface")]
    /// Selects the `huggingface` provider integration.
    HuggingFace,
    #[serde(rename = "raycast")]
    /// Selects the `raycast` provider integration.
    Raycast,
    #[serde(rename = "pi")]
    /// Selects the `pi` provider integration.
    Pi,
    #[serde(rename = "v0")]
    /// Selects the `v0` provider integration.
    V0,
    #[serde(rename = "typesafe")]
    /// Selects the `typesafe` provider integration.
    TypeSafe,
    #[serde(rename = "hyper")]
    /// Selects the `hyper` provider integration.
    Hyper,
    #[serde(rename = "gitkraken")]
    /// Selects the `gitkraken` provider integration.
    GitKraken,
    #[serde(rename = "futureagi")]
    /// Selects the `futureagi` provider integration.
    FutureAGI,
    #[serde(rename = "devpass")]
    /// Selects the `devpass` provider integration.
    DevPass,
    #[serde(rename = "atlascloud")]
    /// Selects the `atlascloud` provider integration.
    AtlasCloud,
    #[serde(rename = "vercel")]
    /// Selects the `vercel` provider integration.
    Vercel,
    #[serde(rename = "llmman")]
    /// Selects the `llmman` provider integration.
    LLMMan,
    #[serde(rename = "xkiro")]
    /// Selects the `xkiro` provider integration.
    XKiro,
    #[serde(rename = "museai")]
    /// Selects the `museai` provider integration.
    MuseAI,
    #[serde(rename = "lithosai")]
    /// Selects the `lithosai` provider integration.
    LithosAI,
    #[serde(rename = "workbuddy")]
    /// Selects the `workbuddy` provider integration.
    WorkBuddy,
    #[serde(rename = "tavily")]
    /// Selects the `tavily` provider integration.
    Tavily,
    #[serde(rename = "linkup")]
    /// Selects the `linkup` provider integration.
    Linkup,
    #[serde(rename = "tinyapi")]
    /// Selects the `tinyapi` provider integration.
    TinyApi,
    #[serde(rename = "exa")]
    /// Selects the `exa` provider integration.
    Exa,
    #[serde(rename = "cosmic")]
    /// Selects the `cosmic` provider integration.
    Cosmic,
    #[serde(rename = "aerostack")]
    /// Selects the `aerostack` provider integration.
    Aerostack,
    #[serde(rename = "sailresearch")]
    /// Selects the `sailresearch` provider integration.
    SailResearch,
    #[serde(rename = "sofya")]
    /// Selects the `sofya` provider integration.
    Sofya,
    #[serde(rename = "fal")]
    /// Selects the `fal` provider integration.
    Fal,
    #[serde(rename = "flora")]
    /// Selects the `flora` provider integration.
    Flora,
    #[serde(rename = "parallel")]
    /// Selects the `parallel` provider integration.
    Parallel,
    #[serde(rename = "claudeplatform")]
    /// Selects the `claudeplatform` provider integration.
    ClaudePlatform,
    #[serde(rename = "munito")]
    /// Selects the `munito` provider integration.
    Munito,
}
impl ProviderId {
    /// Every known provider identifier, including disabled and planned providers.
    pub const ALL: &'static [Self] = &[
        Self::Codex,
        Self::OpenAI,
        Self::AzureOpenAI,
        Self::Claude,
        Self::ClinePass,
        Self::Cursor,
        Self::OpenCode,
        Self::OpenCodeGo,
        Self::Alibaba,
        Self::AlibabaTokenPlan,
        Self::QwenCloud,
        Self::Factory,
        Self::Fireworks,
        Self::Gemini,
        Self::Antigravity,
        Self::Copilot,
        Self::Devin,
        Self::Zai,
        Self::Minimax,
        Self::Manus,
        Self::Kimi,
        Self::Kilo,
        Self::Kiro,
        Self::VertexAI,
        Self::Augment,
        Self::JetBrains,
        Self::Moonshot,
        Self::Amp,
        Self::T3Chat,
        Self::Langdock,
        Self::Ollama,
        Self::Synthetic,
        Self::OpenRouter,
        Self::ElevenLabs,
        Self::Warp,
        Self::Windsurf,
        Self::Zed,
        Self::Perplexity,
        Self::Mimo,
        Self::Doubao,
        Self::Sakana,
        Self::Abacus,
        Self::Mistral,
        Self::DeepSeek,
        Self::DeepInfra,
        Self::Codebuff,
        Self::Venice,
        Self::CommandCode,
        Self::Qoder,
        Self::Stepfun,
        Self::Bedrock,
        Self::Grok,
        Self::Groq,
        Self::LLMProxy,
        Self::LiteLLM,
        Self::Bifrost,
        Self::Aixy,
        Self::Deepgram,
        Self::Poe,
        Self::Chutes,
        Self::NeuralWatt,
        Self::Helmcode,
        Self::Clawrouter,
        Self::LongCat,
        Self::Sub2API,
        Self::Wayfinder,
        Self::Zenmux,
        Self::Aiand,
        Self::ZoomMate,
        Self::Xai,
        Self::XApi,
        Self::Notion,
        Self::IBMBob,
        Self::Nous,
        Self::Muse,
        Self::CodeRabbit,
        Self::Replicate,
        Self::HuggingFace,
        Self::Raycast,
        Self::Pi,
        Self::V0,
        Self::TypeSafe,
        Self::Hyper,
        Self::GitKraken,
        Self::FutureAGI,
        Self::DevPass,
        Self::AtlasCloud,
        Self::Vercel,
        Self::LLMMan,
        Self::XKiro,
        Self::MuseAI,
        Self::LithosAI,
        Self::WorkBuddy,
        Self::Tavily,
        Self::Linkup,
        Self::TinyApi,
        Self::Exa,
        Self::Cosmic,
        Self::Aerostack,
        Self::SailResearch,
        Self::Sofya,
        Self::Fal,
        Self::Flora,
        Self::Parallel,
        Self::ClaudePlatform,
        Self::Munito,
    ];
    #[must_use]
    /// Returns the stable identifier used in configuration and serialization.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::OpenAI => "openai",
            Self::AzureOpenAI => "azureopenai",
            Self::Claude => "claude",
            Self::ClinePass => "clinepass",
            Self::Cursor => "cursor",
            Self::OpenCode => "opencode",
            Self::OpenCodeGo => "opencodego",
            Self::Alibaba => "alibaba",
            Self::AlibabaTokenPlan => "alibabatokenplan",
            Self::QwenCloud => "qwencloud",
            Self::Factory => "factory",
            Self::Fireworks => "fireworks",
            Self::Gemini => "gemini",
            Self::Antigravity => "antigravity",
            Self::Copilot => "copilot",
            Self::Devin => "devin",
            Self::Zai => "zai",
            Self::Minimax => "minimax",
            Self::Manus => "manus",
            Self::Kimi => "kimi",
            Self::Kilo => "kilo",
            Self::Kiro => "kiro",
            Self::VertexAI => "vertexai",
            Self::Augment => "augment",
            Self::JetBrains => "jetbrains",
            Self::Moonshot => "moonshot",
            Self::Amp => "amp",
            Self::T3Chat => "t3chat",
            Self::Langdock => "langdock",
            Self::Ollama => "ollama",
            Self::Synthetic => "synthetic",
            Self::OpenRouter => "openrouter",
            Self::ElevenLabs => "elevenlabs",
            Self::Warp => "warp",
            Self::Windsurf => "windsurf",
            Self::Zed => "zed",
            Self::Perplexity => "perplexity",
            Self::Mimo => "mimo",
            Self::Doubao => "doubao",
            Self::Sakana => "sakana",
            Self::Abacus => "abacus",
            Self::Mistral => "mistral",
            Self::DeepSeek => "deepseek",
            Self::DeepInfra => "deepinfra",
            Self::Codebuff => "codebuff",
            Self::Venice => "venice",
            Self::CommandCode => "commandcode",
            Self::Qoder => "qoder",
            Self::Stepfun => "stepfun",
            Self::Bedrock => "bedrock",
            Self::Grok => "grok",
            Self::Groq => "groq",
            Self::LLMProxy => "llmproxy",
            Self::LiteLLM => "litellm",
            Self::Bifrost => "bifrost",
            Self::Aixy => "aixy",
            Self::Deepgram => "deepgram",
            Self::Poe => "poe",
            Self::Chutes => "chutes",
            Self::NeuralWatt => "neuralwatt",
            Self::Helmcode => "helmcode",
            Self::Clawrouter => "clawrouter",
            Self::LongCat => "longcat",
            Self::Sub2API => "sub2api",
            Self::Wayfinder => "wayfinder",
            Self::Zenmux => "zenmux",
            Self::Aiand => "aiand",
            Self::ZoomMate => "zoommate",
            Self::Xai => "xai",
            Self::XApi => "xapi",
            Self::Notion => "notion",
            Self::IBMBob => "ibmbob",
            Self::Nous => "nous",
            Self::Muse => "muse",
            Self::CodeRabbit => "coderabbit",
            Self::Replicate => "replicate",
            Self::HuggingFace => "huggingface",
            Self::Raycast => "raycast",
            Self::Pi => "pi",
            Self::V0 => "v0",
            Self::TypeSafe => "typesafe",
            Self::Hyper => "hyper",
            Self::GitKraken => "gitkraken",
            Self::FutureAGI => "futureagi",
            Self::DevPass => "devpass",
            Self::AtlasCloud => "atlascloud",
            Self::Vercel => "vercel",
            Self::LLMMan => "llmman",
            Self::XKiro => "xkiro",
            Self::MuseAI => "museai",
            Self::LithosAI => "lithosai",
            Self::WorkBuddy => "workbuddy",
            Self::Tavily => "tavily",
            Self::Linkup => "linkup",
            Self::TinyApi => "tinyapi",
            Self::Exa => "exa",
            Self::Cosmic => "cosmic",
            Self::Aerostack => "aerostack",
            Self::SailResearch => "sailresearch",
            Self::Sofya => "sofya",
            Self::Fal => "fal",
            Self::Flora => "flora",
            Self::Parallel => "parallel",
            Self::ClaudePlatform => "claudeplatform",
            Self::Munito => "munito",
        }
    }
    #[must_use]
    /// Returns coverage and build availability for this provider.
    pub fn descriptor(self) -> ProviderDescriptor {
        match self {
            Self::Codex => ProviderDescriptor {
                id: self,
                name: "Codex",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "codex"),
                reason: "",
            },
            Self::OpenAI => ProviderDescriptor {
                id: self,
                name: "OpenAI",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "openai"),
                reason: "",
            },
            Self::AzureOpenAI => ProviderDescriptor {
                id: self,
                name: "Azure OpenAI",
                status: CoverageStatus::Unsupported,
                enabled: cfg!(feature = "azureopenai"),
                reason: "Only an inference-based access probe has been identified; no compatible read-only quota endpoint is available.",
            },
            Self::Claude => ProviderDescriptor {
                id: self,
                name: "Claude",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "claude"),
                reason: "",
            },
            Self::ClinePass => ProviderDescriptor {
                id: self,
                name: "ClinePass",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "clinepass"),
                reason: "",
            },
            Self::Cursor => ProviderDescriptor {
                id: self,
                name: "Cursor",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "cursor"),
                reason: "",
            },
            Self::OpenCode => ProviderDescriptor {
                id: self,
                name: "OpenCode",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "opencode"),
                reason: "",
            },
            Self::OpenCodeGo => ProviderDescriptor {
                id: self,
                name: "OpenCode Go",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "opencodego"),
                reason: "",
            },
            Self::Alibaba => ProviderDescriptor {
                id: self,
                name: "Alibaba Coding Plan",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "alibaba"),
                reason: "Adapter port pending.",
            },
            Self::AlibabaTokenPlan => ProviderDescriptor {
                id: self,
                name: "Alibaba Token Plan",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "alibabatokenplan"),
                reason: "Adapter port pending.",
            },
            Self::QwenCloud => ProviderDescriptor {
                id: self,
                name: "Qwen Cloud",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "qwencloud"),
                reason: "Adapter port pending.",
            },
            Self::Factory => ProviderDescriptor {
                id: self,
                name: "Droid",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "factory"),
                reason: "Adapter port pending.",
            },
            Self::Fireworks => ProviderDescriptor {
                id: self,
                name: "Fireworks",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "fireworks"),
                reason: "Adapter port pending.",
            },
            Self::Gemini => ProviderDescriptor {
                id: self,
                name: "Gemini app",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "gemini"),
                reason: "",
            },
            Self::Antigravity => ProviderDescriptor {
                id: self,
                name: "Antigravity",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "antigravity"),
                reason: "",
            },
            Self::Copilot => ProviderDescriptor {
                id: self,
                name: "Copilot",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "copilot"),
                reason: "",
            },
            Self::Devin => ProviderDescriptor {
                id: self,
                name: "Devin",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "devin"),
                reason: "Adapter port pending.",
            },
            Self::Zai => ProviderDescriptor {
                id: self,
                name: "z.ai / GLM",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "zai"),
                reason: "",
            },
            Self::Minimax => ProviderDescriptor {
                id: self,
                name: "MiniMax",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "minimax"),
                reason: "Adapter port pending.",
            },
            Self::Manus => ProviderDescriptor {
                id: self,
                name: "Manus",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "manus"),
                reason: "Adapter port pending.",
            },
            Self::Kimi => ProviderDescriptor {
                id: self,
                name: "Kimi",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "kimi"),
                reason: "Adapter port pending.",
            },
            Self::Kilo => ProviderDescriptor {
                id: self,
                name: "Kilo",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "kilo"),
                reason: "Adapter port pending.",
            },
            Self::Kiro => ProviderDescriptor {
                id: self,
                name: "Kiro",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "kiro"),
                reason: "Adapter port pending.",
            },
            Self::VertexAI => ProviderDescriptor {
                id: self,
                name: "Vertex AI",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "vertexai"),
                reason: "Adapter port pending.",
            },
            Self::Augment => ProviderDescriptor {
                id: self,
                name: "Augment",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "augment"),
                reason: "Adapter port pending.",
            },
            Self::JetBrains => ProviderDescriptor {
                id: self,
                name: "JetBrains AI",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "jetbrains"),
                reason: "Adapter port pending.",
            },
            Self::Moonshot => ProviderDescriptor {
                id: self,
                name: "Moonshot / Kimi Open Platform",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "moonshot"),
                reason: "",
            },
            Self::Amp => ProviderDescriptor {
                id: self,
                name: "Amp",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "amp"),
                reason: "Adapter port pending.",
            },
            Self::T3Chat => ProviderDescriptor {
                id: self,
                name: "T3 Chat",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "t3chat"),
                reason: "Adapter port pending.",
            },
            Self::Langdock => ProviderDescriptor {
                id: self,
                name: "Langdock",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "langdock"),
                reason: "",
            },
            Self::Ollama => ProviderDescriptor {
                id: self,
                name: "Ollama",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "ollama"),
                reason: "",
            },
            Self::Synthetic => ProviderDescriptor {
                id: self,
                name: "Synthetic",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "synthetic"),
                reason: "",
            },
            Self::OpenRouter => ProviderDescriptor {
                id: self,
                name: "OpenRouter",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "openrouter"),
                reason: "",
            },
            Self::ElevenLabs => ProviderDescriptor {
                id: self,
                name: "ElevenLabs",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "elevenlabs"),
                reason: "",
            },
            Self::Warp => ProviderDescriptor {
                id: self,
                name: "Warp",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "warp"),
                reason: "",
            },
            Self::Windsurf => ProviderDescriptor {
                id: self,
                name: "Windsurf",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "windsurf"),
                reason: "Adapter port pending.",
            },
            Self::Zed => ProviderDescriptor {
                id: self,
                name: "Zed",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "zed"),
                reason: "Adapter port pending.",
            },
            Self::Perplexity => ProviderDescriptor {
                id: self,
                name: "Perplexity",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "perplexity"),
                reason: "Adapter port pending.",
            },
            Self::Mimo => ProviderDescriptor {
                id: self,
                name: "Xiaomi MiMo",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "mimo"),
                reason: "Adapter port pending.",
            },
            Self::Doubao => ProviderDescriptor {
                id: self,
                name: "Doubao",
                status: CoverageStatus::Unsupported,
                enabled: cfg!(feature = "doubao"),
                reason: "The available validation path generates chat completions and can consume quota.",
            },
            Self::Sakana => ProviderDescriptor {
                id: self,
                name: "Sakana AI",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "sakana"),
                reason: "Adapter port pending.",
            },
            Self::Abacus => ProviderDescriptor {
                id: self,
                name: "Abacus AI",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "abacus"),
                reason: "Adapter port pending.",
            },
            Self::Mistral => ProviderDescriptor {
                id: self,
                name: "Mistral",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "mistral"),
                reason: "Adapter port pending.",
            },
            Self::DeepSeek => ProviderDescriptor {
                id: self,
                name: "DeepSeek",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "deepseek"),
                reason: "",
            },
            Self::DeepInfra => ProviderDescriptor {
                id: self,
                name: "DeepInfra",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "deepinfra"),
                reason: "Adapter port pending.",
            },
            Self::Codebuff => ProviderDescriptor {
                id: self,
                name: "Codebuff",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "codebuff"),
                reason: "",
            },
            Self::Venice => ProviderDescriptor {
                id: self,
                name: "Venice",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "venice"),
                reason: "",
            },
            Self::CommandCode => ProviderDescriptor {
                id: self,
                name: "Command Code",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "commandcode"),
                reason: "Adapter port pending.",
            },
            Self::Qoder => ProviderDescriptor {
                id: self,
                name: "Qoder",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "qoder"),
                reason: "Adapter port pending.",
            },
            Self::Stepfun => ProviderDescriptor {
                id: self,
                name: "StepFun",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "stepfun"),
                reason: "Adapter port pending.",
            },
            Self::Bedrock => ProviderDescriptor {
                id: self,
                name: "AWS Bedrock",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "bedrock"),
                reason: "Adapter port pending.",
            },
            Self::Grok => ProviderDescriptor {
                id: self,
                name: "Grok",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "grok"),
                reason: "Adapter port pending.",
            },
            Self::Groq => ProviderDescriptor {
                id: self,
                name: "Groq",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "groq"),
                reason: "Adapter port pending.",
            },
            Self::LLMProxy => ProviderDescriptor {
                id: self,
                name: "LLM Proxy",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "llmproxy"),
                reason: "Adapter port pending.",
            },
            Self::LiteLLM => ProviderDescriptor {
                id: self,
                name: "LiteLLM",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "litellm"),
                reason: "Adapter port pending.",
            },
            Self::Bifrost => ProviderDescriptor {
                id: self,
                name: "Bifrost",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "bifrost"),
                reason: "Adapter port pending.",
            },
            Self::Aixy => ProviderDescriptor {
                id: self,
                name: "Aixy",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "aixy"),
                reason: "Adapter port pending.",
            },
            Self::Deepgram => ProviderDescriptor {
                id: self,
                name: "Deepgram",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "deepgram"),
                reason: "Adapter port pending.",
            },
            Self::Poe => ProviderDescriptor {
                id: self,
                name: "Poe",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "poe"),
                reason: "",
            },
            Self::Chutes => ProviderDescriptor {
                id: self,
                name: "Chutes",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "chutes"),
                reason: "Adapter port pending.",
            },
            Self::NeuralWatt => ProviderDescriptor {
                id: self,
                name: "Neuralwatt",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "neuralwatt"),
                reason: "Adapter port pending.",
            },
            Self::Helmcode => ProviderDescriptor {
                id: self,
                name: "Helmcode",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "helmcode"),
                reason: "Adapter port pending.",
            },
            Self::Clawrouter => ProviderDescriptor {
                id: self,
                name: "ClawRouter",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "clawrouter"),
                reason: "",
            },
            Self::LongCat => ProviderDescriptor {
                id: self,
                name: "LongCat",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "longcat"),
                reason: "Adapter port pending.",
            },
            Self::Sub2API => ProviderDescriptor {
                id: self,
                name: "sub2api",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "sub2api"),
                reason: "Adapter port pending.",
            },
            Self::Wayfinder => ProviderDescriptor {
                id: self,
                name: "Wayfinder",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "wayfinder"),
                reason: "Adapter port pending.",
            },
            Self::Zenmux => ProviderDescriptor {
                id: self,
                name: "ZenMux",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "zenmux"),
                reason: "",
            },
            Self::Aiand => ProviderDescriptor {
                id: self,
                name: "ai&",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "aiand"),
                reason: "Adapter port pending.",
            },
            Self::ZoomMate => ProviderDescriptor {
                id: self,
                name: "ZoomMate",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "zoommate"),
                reason: "Adapter port pending.",
            },
            Self::Xai => ProviderDescriptor {
                id: self,
                name: "xAI",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "xai"),
                reason: "",
            },
            Self::XApi => ProviderDescriptor {
                id: self,
                name: "X API",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "xapi"),
                reason: "Adapter port pending.",
            },
            Self::Notion => ProviderDescriptor {
                id: self,
                name: "Notion AI",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "notion"),
                reason: "Adapter port pending.",
            },
            Self::IBMBob => ProviderDescriptor {
                id: self,
                name: "IBM Bob",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "ibmbob"),
                reason: "Adapter port pending.",
            },
            Self::Nous => ProviderDescriptor {
                id: self,
                name: "Nous Portal",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "nous"),
                reason: "",
            },
            Self::Muse => ProviderDescriptor {
                id: self,
                name: "Muse Code",
                status: CoverageStatus::Unsupported,
                enabled: cfg!(feature = "muse"),
                reason: "The available authentication path creates an API key; key creation is excluded.",
            },
            Self::CodeRabbit => ProviderDescriptor {
                id: self,
                name: "CodeRabbit",
                status: CoverageStatus::Implemented,
                enabled: cfg!(all(feature = "coderabbit", feature = "local-probes")),
                reason: "",
            },
            Self::Replicate => ProviderDescriptor {
                id: self,
                name: "Replicate",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "replicate"),
                reason: "Adapter port pending.",
            },
            Self::HuggingFace => ProviderDescriptor {
                id: self,
                name: "Hugging Face",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "huggingface"),
                reason: "Adapter port pending.",
            },
            Self::Raycast => ProviderDescriptor {
                id: self,
                name: "Raycast",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "raycast"),
                reason: "Adapter port pending.",
            },
            Self::Pi => ProviderDescriptor {
                id: self,
                name: "Pi",
                status: CoverageStatus::Unsupported,
                enabled: cfg!(feature = "pi"),
                reason: "Local transcript scanning and estimated-cost history are outside this crate's scope.",
            },
            Self::V0 => ProviderDescriptor {
                id: self,
                name: "v0",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "v0"),
                reason: "Adapter port pending.",
            },
            Self::TypeSafe => ProviderDescriptor {
                id: self,
                name: "TypeSafe",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "typesafe"),
                reason: "Adapter port pending.",
            },
            Self::Hyper => ProviderDescriptor {
                id: self,
                name: "Charm Hyper",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "hyper"),
                reason: "Adapter port pending.",
            },
            Self::GitKraken => ProviderDescriptor {
                id: self,
                name: "GitKraken AI",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "gitkraken"),
                reason: "",
            },
            Self::FutureAGI => ProviderDescriptor {
                id: self,
                name: "Future AGI",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "futureagi"),
                reason: "Adapter port pending.",
            },
            Self::DevPass => ProviderDescriptor {
                id: self,
                name: "DevPass",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "devpass"),
                reason: "",
            },
            Self::AtlasCloud => ProviderDescriptor {
                id: self,
                name: "Atlas Cloud",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "atlascloud"),
                reason: "",
            },
            Self::Vercel => ProviderDescriptor {
                id: self,
                name: "Vercel AI Gateway",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "vercel"),
                reason: "",
            },
            Self::LLMMan => ProviderDescriptor {
                id: self,
                name: "llmman",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "llmman"),
                reason: "Adapter port pending.",
            },
            Self::XKiro => ProviderDescriptor {
                id: self,
                name: "xKiro",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "xkiro"),
                reason: "",
            },
            Self::MuseAI => ProviderDescriptor {
                id: self,
                name: "Muse (muse.ai)",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "museai"),
                reason: "Adapter port pending.",
            },
            Self::LithosAI => ProviderDescriptor {
                id: self,
                name: "LithosAI",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "lithosai"),
                reason: "Adapter port pending.",
            },
            Self::WorkBuddy => ProviderDescriptor {
                id: self,
                name: "WorkBuddy",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "workbuddy"),
                reason: "Adapter port pending.",
            },
            Self::Tavily => ProviderDescriptor {
                id: self,
                name: "Tavily",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "tavily"),
                reason: "",
            },
            Self::Linkup => ProviderDescriptor {
                id: self,
                name: "Linkup",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "linkup"),
                reason: "",
            },
            Self::TinyApi => ProviderDescriptor {
                id: self,
                name: "TinyApi",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "tinyapi"),
                reason: "Adapter port pending.",
            },
            Self::Exa => ProviderDescriptor {
                id: self,
                name: "Exa",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "exa"),
                reason: "",
            },
            Self::Cosmic => ProviderDescriptor {
                id: self,
                name: "Cosmic AI",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "cosmic"),
                reason: "",
            },
            Self::Aerostack => ProviderDescriptor {
                id: self,
                name: "Aerostack",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "aerostack"),
                reason: "",
            },
            Self::SailResearch => ProviderDescriptor {
                id: self,
                name: "Sail Research",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "sailresearch"),
                reason: "",
            },
            Self::Sofya => ProviderDescriptor {
                id: self,
                name: "Sofya",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "sofya"),
                reason: "",
            },
            Self::Fal => ProviderDescriptor {
                id: self,
                name: "fal",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "fal"),
                reason: "",
            },
            Self::Flora => ProviderDescriptor {
                id: self,
                name: "FLORA",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "flora"),
                reason: "",
            },
            Self::Parallel => ProviderDescriptor {
                id: self,
                name: "Parallel",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "parallel"),
                reason: "Adapter port pending.",
            },
            Self::ClaudePlatform => ProviderDescriptor {
                id: self,
                name: "Claude Platform",
                status: CoverageStatus::Planned,
                enabled: cfg!(feature = "claudeplatform"),
                reason: "Adapter port pending.",
            },
            Self::Munito => ProviderDescriptor {
                id: self,
                name: "Munito",
                status: CoverageStatus::Implemented,
                enabled: cfg!(feature = "munito"),
                reason: "",
            },
        }
    }
}
impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl FromStr for ProviderId {
    type Err = AgentStatError;
    fn from_str(raw: &str) -> Result<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|id| id.as_str() == raw)
            .ok_or(AgentStatError::Configuration("unknown provider ID"))
    }
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Implementation status of a provider adapter.
pub enum CoverageStatus {
    /// An adapter is available.
    Implemented,
    /// An adapter is planned but unavailable.
    Planned,
    /// No adapter is planned.
    Unsupported,
}
#[derive(Debug, Clone, Serialize)]
/// Coverage and availability details for one provider.
#[expect(
    clippy::exhaustive_structs,
    reason = "public struct literals are part of the existing crate API"
)]
pub struct ProviderDescriptor {
    /// Stable identifier for this entry.
    pub id: ProviderId,
    /// Display name of the provider.
    pub name: &'static str,
    /// Implementation status of this provider.
    pub status: CoverageStatus,
    /// Whether this provider is enabled in the build.
    pub enabled: bool,
    /// Explanation for the status.
    pub reason: &'static str,
}
#[must_use]
/// Returns coverage details for all known providers.
pub fn provider_catalog() -> Vec<ProviderDescriptor> {
    ProviderId::ALL.iter().map(|id| id.descriptor()).collect()
}
