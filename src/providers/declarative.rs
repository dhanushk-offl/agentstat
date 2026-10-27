//! Audited single-response adapters. Specs contain only response paths and public endpoints.
use super::common::{auth, check_source, date, decimal, finish, number, source, text, window};
use crate::{
    AgentStatError, Balance, Client, Credentials, Decimal, FetchRequest, ProviderConfig,
    ProviderId, QuotaLimit, Result, SpendSummary, Unit, UsageSnapshot, WindowKind, transport,
};
use reqwest::Method;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;
#[derive(Debug, Deserialize)]
pub(super) struct Spec {
    pub url: String,
    pub method: Option<String>,
    pub body: Option<Value>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub currency_path: Option<String>,
    #[serde(default = "default_auth")]
    pub auth: String,
    #[serde(default)]
    pub plan: Option<String>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub guards: BTreeMap<String, Value>,
    #[serde(default)]
    pub windows: Vec<WindowSpec>,
    #[serde(default)]
    pub balances: Vec<BalanceSpec>,
    #[serde(default)]
    pub spend: Vec<SpendSpec>,
}
fn default_auth() -> String {
    "bearer".into()
}
#[derive(Debug, Deserialize)]
pub(super) struct Field {
    pub path: String,
    #[serde(default)]
    pub subtract: Option<String>,
    #[serde(default)]
    pub scale: u16,
    #[serde(default)]
    pub negate: bool,
}
#[derive(Debug, Default, Deserialize)]
#[serde(from = "bool")]
pub(super) enum PercentScale {
    #[default]
    Percent,
    Fraction,
}
impl From<bool> for PercentScale {
    fn from(fraction: bool) -> Self {
        if fraction {
            Self::Fraction
        } else {
            Self::Percent
        }
    }
}
#[derive(Debug, Default, Deserialize)]
#[serde(from = "bool")]
pub(super) enum NullLimit {
    #[default]
    Unknown,
    Unlimited,
}
impl From<bool> for NullLimit {
    fn from(unlimited: bool) -> Self {
        if unlimited {
            Self::Unlimited
        } else {
            Self::Unknown
        }
    }
}
#[derive(Debug, Deserialize)]
pub(super) struct WindowSpec {
    pub id: String,
    pub unit: Unit,
    pub used: Option<Field>,
    pub limit: Option<Field>,
    pub remaining: Option<Field>,
    pub percent: Option<String>,
    pub reset: Option<String>,
    pub duration: Option<u64>,
    pub kind: Option<WindowKind>,
    #[serde(default)]
    #[serde(rename = "percent_fraction")]
    pub percent_scale: PercentScale,
    #[serde(default)]
    #[serde(rename = "null_unlimited")]
    pub null_limit: NullLimit,
    pub unlimited_value: Option<String>,
}
#[derive(Debug, Deserialize)]
pub(super) struct BalanceSpec {
    pub id: String,
    pub amount: Field,
    pub unit: Unit,
}
#[derive(Debug, Deserialize)]
pub(super) struct SpendSpec {
    pub id: String,
    pub amount: Field,
    pub currency: String,
    pub start: Option<String>,
    pub end: Option<String>,
}
fn specs() -> &'static BTreeMap<String, Spec> {
    static SPECS: OnceLock<BTreeMap<String, Spec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        serde_json::from_str(include_str!("specs.json")).expect("checked built-in adapter specs")
    })
}
pub(super) fn spec(id: ProviderId) -> Option<&'static Spec> {
    specs().get(id.as_str())
}
pub(super) fn pointer<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('|')
        .find_map(|p| v.pointer(p).filter(|v| !v.is_null()))
}
fn amount(v: &Value, field: Option<&Field>) -> Option<Decimal> {
    field.and_then(|f| field_amount(v, f))
}
fn field_amount(v: &Value, f: &Field) -> Option<Decimal> {
    let mut n = decimal(pointer(v, &f.path)?)?;
    if let Some(sub) = &f.subtract {
        n = n.checked_sub(&decimal(pointer(v, sub)?)?)?;
    }
    if f.negate {
        n = Decimal::from(0).checked_sub(&n)?;
    }
    if f.scale > 0 {
        n = n.divided_by_power_of_ten(f.scale)?;
    }
    Some(n)
}
pub(super) fn parse(r: &FetchRequest, spec: &Spec, v: &Value) -> Result<UsageSnapshot> {
    if !v.is_object() && !v.is_array() {
        return Err(AgentStatError::InvalidResponse("expected usage object"));
    }
    for path in &spec.required {
        if pointer(v, path).is_none() {
            return Err(AgentStatError::InvalidResponse(
                "missing required provider field",
            ));
        }
    }
    for (path, expected) in &spec.guards {
        if v.pointer(path) != Some(expected) {
            return Err(AgentStatError::InvalidResponse(
                "provider response reports failure or incompatible scope",
            ));
        }
    }
    let mut s = UsageSnapshot::new(r, source(r));
    s.plan = spec.plan.as_ref().and_then(|p| text(pointer(v, p)?));
    for spec in &spec.windows {
        let (used, mut limit, remaining) = (
            amount(v, spec.used.as_ref()),
            amount(v, spec.limit.as_ref()),
            amount(v, spec.remaining.as_ref()),
        );
        let sentinel = limit.as_ref().is_some_and(|l| {
            spec.unlimited_value
                .as_ref()
                .is_some_and(|v| l.as_str() == v)
        });
        if sentinel {
            limit = None;
        }
        let percent = spec.percent.as_ref().and_then(|p| number(pointer(v, p)?));
        let unlimited = matches!(spec.null_limit, NullLimit::Unlimited)
            && spec
                .limit
                .as_ref()
                .is_some_and(|f| v.pointer(&f.path).is_some_and(Value::is_null));
        if used.is_none()
            && limit.is_none()
            && remaining.is_none()
            && percent.is_none()
            && !unlimited
        {
            continue;
        }
        if [&used, &limit, &remaining]
            .into_iter()
            .flatten()
            .any(Decimal::is_negative)
        {
            return Err(AgentStatError::InvalidResponse("negative quota counter"));
        }
        let mut w = window(&spec.id, spec.unit.clone(), used, limit, remaining);
        if unlimited || sentinel {
            w.limit = QuotaLimit::Unlimited;
        }
        w.utilization_percent = percent
            .map(|p| {
                if matches!(spec.percent_scale, PercentScale::Fraction) {
                    p * 100.0
                } else {
                    p
                }
            })
            .or(w.utilization_percent);
        if w.utilization_percent
            .is_some_and(|n| !n.is_finite() || n < 0.0)
        {
            return Err(AgentStatError::InvalidResponse("invalid utilization"));
        }
        w.duration_seconds = spec.duration;
        w.kind = spec.kind.unwrap_or(WindowKind::Unknown);
        w.resets_at = spec.reset.as_ref().and_then(|p| date(pointer(v, p)?));
        s.windows.push(w);
    }
    for spec in &spec.balances {
        if let Some(amount) = field_amount(v, &spec.amount) {
            s.balances.push(Balance {
                id: spec.id.clone(),
                amount,
                unit: spec.unit.clone(),
            })
        }
    }
    if r.provider == ProviderId::Warp
        && v["data"]["user"]["user"]["requestLimitInfo"]["isUnlimited"] == true
    {
        for w in &mut s.windows {
            w.limit = QuotaLimit::Unlimited;
            w.utilization_percent = None;
        }
    }
    if r.provider == ProviderId::Langdock
        && v[0]["result"]["data"]["json"]["planUsage"]["sessionUsageLimitsEnabled"] == false
    {
        s.windows.retain(|w| w.id != "session");
    }
    for spec in &spec.spend {
        if let Some(amount) = field_amount(v, &spec.amount) {
            if amount.is_negative() {
                return Err(AgentStatError::InvalidResponse("negative spend"));
            }
            s.spend.push(SpendSummary {
                id: spec.id.clone(),
                amount,
                currency: spec.currency.clone(),
                starts_at: spec.start.as_ref().and_then(|p| date(pointer(v, p)?)),
                ends_at: spec.end.as_ref().and_then(|p| date(pointer(v, p)?)),
            })
        }
    }
    if let Some(path) = &spec.currency_path
        && let Some(currency) = pointer(v, path).and_then(Value::as_str)
    {
        if currency.len() != 3 || !currency.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(AgentStatError::InvalidResponse("invalid currency"));
        }
        let currency = currency.to_uppercase();
        for b in &mut s.balances {
            if matches!(b.unit, Unit::Currency(_)) {
                b.unit = Unit::Currency(currency.clone())
            }
        }
        for w in &mut s.windows {
            if matches!(w.unit, Unit::Currency(_)) {
                w.unit = Unit::Currency(currency.clone())
            }
        }
        for item in &mut s.spend {
            item.currency = currency.clone();
        }
    }
    finish(s)
}
pub(super) async fn fetch(client: &Client, r: &FetchRequest, spec: &Spec) -> Result<UsageSnapshot> {
    let (name, scheme) = match spec.auth.as_str() {
        "bearer" => ("authorization", "Bearer "),
        "cookie" => ("cookie", ""),
        "xi-api-key" => ("xi-api-key", ""),
        "x-api-key" => ("x-api-key", ""),
        "fal" => ("authorization", "Key "),
        _ => {
            return Err(AgentStatError::Configuration(
                "invalid adapter authentication",
            ));
        }
    };
    if (spec.auth == "cookie") != matches!(r.credentials, Credentials::Cookie(_))
        || matches!(r.credentials, Credentials::None)
    {
        return Err(AgentStatError::InvalidCredentials(r.provider));
    }
    check_source(r, source(r))?;
    // Codebuff's CLI session token belongs in the JSON request body. A paid API
    // key retains the existing Bearer-header request shape.
    let cli_session =
        r.provider == ProviderId::Codebuff && matches!(r.credentials, Credentials::Bearer(_));
    let mut headers = if cli_session {
        HeaderMap::new()
    } else {
        auth(r, name, scheme)?
    };
    for (name, value) in &spec.headers {
        headers.insert(
            HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| AgentStatError::Configuration("invalid adapter header"))?,
            HeaderValue::from_str(value)
                .map_err(|_| AgentStatError::Configuration("invalid adapter header"))?,
        );
    }
    let url = resolve_url(r, &spec.url)?;
    let method = if spec.method.as_deref() == Some("POST") {
        Method::POST
    } else {
        Method::GET
    };
    let body = if cli_session {
        let token = r
            .credentials
            .value()
            .ok_or(AgentStatError::InvalidCredentials(r.provider))?;
        Some(serde_json::json!({"fingerprintId": "agentstat-usage", "authToken": token}))
    } else {
        spec.body.clone()
    };
    let data = client
        .transport
        .json(r.provider, method, &url, headers, body)
        .await?;
    parse(r, spec, &data)
}

fn encode(raw: &str) -> String {
    raw.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn resolve_url(r: &FetchRequest, template: &str) -> Result<String> {
    let now = chrono::Utc::now();
    let mut url = template
        .replace(
            "{month_start}",
            &encode(&now.format("%Y-%m-01T00:00:00Z").to_string()),
        )
        .replace(
            "{now}",
            &encode(&now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        )
        .replace("{month_date}", &now.format("%Y-%m-01").to_string())
        .replace("{today}", &now.format("%Y-%m-%d").to_string());
    let pair = match &r.config {
        ProviderConfig::Fireworks { account_id } => Some(("{account_id}", account_id)),
        ProviderConfig::Exa { api_key_id } => Some(("{api_key_id}", api_key_id)),
        ProviderConfig::Cosmic { project_id } => Some(("{project_id}", project_id)),
        ProviderConfig::Flora { workspace_id } => Some(("{workspace_id}", workspace_id)),
        ProviderConfig::Xai { team_id } => Some(("{team_id}", team_id)),
        _ => None,
    };
    if let Some((key, value)) = pair {
        if value.is_empty() || value == "." || value == ".." {
            return Err(AgentStatError::Configuration(
                "provider scope identifier is empty",
            ));
        }
        url = url.replace(key, &encode(value));
    }
    if url.contains('{') {
        return Err(AgentStatError::Configuration(
            "provider-specific configuration is required",
        ));
    }
    if let ProviderConfig::Gateway { base_url } = &r.config {
        let mut base = reqwest::Url::parse(base_url)
            .map_err(|_| AgentStatError::Configuration("invalid gateway URL"))?;
        transport::validate_url(&base)?;
        if base.query().is_some() {
            return Err(AgentStatError::Configuration(
                "gateway URL cannot contain query parameters",
            ));
        }
        let original = reqwest::Url::parse(&url)
            .map_err(|_| AgentStatError::Configuration("invalid adapter URL"))?;
        base.set_path(&format!(
            "{}{}",
            base.path().trim_end_matches('/'),
            original.path()
        ));
        base.set_query(original.query());
        url = base.to_string();
    }
    Ok(url)
}
