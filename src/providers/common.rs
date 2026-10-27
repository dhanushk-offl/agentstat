//! Shared parsing helpers for provider adapters.

use crate::{
    AgentStatError, Balance, Credentials, Decimal, FetchRequest, ProviderId, QuotaLimit,
    QuotaWindow, Result, Source, Unit, UsageSnapshot, WindowKind, transport,
};
use chrono::{DateTime, Utc};
use reqwest::header::HeaderMap;
use serde_json::Value;
pub(super) fn decimal(value: &Value) -> Option<Decimal> {
    match value {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.to_string().parse().ok(),
        _ => None,
    }
}
pub(super) fn number(value: &Value) -> Option<f64> {
    decimal(value)?.to_f64()
}
pub(super) fn text(value: &Value) -> Option<String> {
    value.as_str().filter(|s| !s.is_empty()).map(str::to_owned)
}
pub(super) fn date(value: &Value) -> Option<DateTime<Utc>> {
    if let Some(raw) = value.as_str()
        && let Ok(v) = DateTime::parse_from_rfc3339(raw)
    {
        return Some(v.with_timezone(&Utc));
    }
    let raw = value
        .as_i64()
        .or_else(|| value.as_str()?.parse::<i64>().ok())?;
    if raw.unsigned_abs() > 100_000_000_000 {
        DateTime::from_timestamp_millis(raw)
    } else {
        DateTime::from_timestamp(raw, 0)
    }
}
pub(super) fn auth(
    request: &FetchRequest,
    header_name: &'static str,
    scheme: &str,
) -> Result<HeaderMap> {
    let value = request
        .credentials
        .value()
        .filter(|s| !s.trim().is_empty())
        .ok_or(AgentStatError::InvalidCredentials(request.provider))?;
    let mut headers = HeaderMap::new();
    transport::header(&mut headers, header_name, &format!("{scheme}{value}"))?;
    Ok(headers)
}
pub(super) fn source(request: &FetchRequest) -> Source {
    match request.credentials {
        Credentials::Cookie(_) => Source::Web,
        Credentials::Bearer(_)
            if matches!(
                request.provider,
                ProviderId::Codex | ProviderId::Claude | ProviderId::Gemini
            ) =>
        {
            Source::OAuth
        }
        _ => Source::Api,
    }
}
pub(super) fn check_source(request: &FetchRequest, actual: Source) -> Result<()> {
    if request.source != Source::Auto && request.source != actual {
        Err(AgentStatError::UnsupportedSource(request.provider))
    } else {
        Ok(())
    }
}
pub(super) fn window(
    id: &str,
    unit: Unit,
    used: Option<Decimal>,
    limit: Option<Decimal>,
    remaining: Option<Decimal>,
) -> QuotaWindow {
    let remaining = remaining.or_else(|| limit.as_ref()?.checked_sub(used.as_ref()?));
    let utilization_percent = used
        .as_ref()
        .and_then(Decimal::to_f64)
        .zip(limit.as_ref().and_then(Decimal::to_f64))
        .filter(|(_, l)| *l > 0.0)
        .map(|(u, l)| u / l * 100.0)
        .filter(|n| n.is_finite());
    QuotaWindow {
        id: id.into(),
        model: None,
        used,
        remaining,
        limit: limit.map_or(QuotaLimit::Unknown, QuotaLimit::Known),
        unit,
        utilization_percent,
        duration_seconds: None,
        resets_at: None,
        kind: WindowKind::Unknown,
    }
}
pub(super) fn percent_window(
    id: &str,
    percent: Option<f64>,
    duration: Option<u64>,
    reset: Option<DateTime<Utc>>,
) -> QuotaWindow {
    let mut w = window(id, Unit::Percent, None, None, None);
    w.utilization_percent = percent.filter(|p| p.is_finite() && *p >= 0.0);
    w.duration_seconds = duration;
    w.resets_at = reset;
    w
}
pub(super) fn finish(mut snapshot: UsageSnapshot) -> Result<UsageSnapshot> {
    if snapshot.windows.is_empty() && snapshot.balances.is_empty() && snapshot.spend.is_empty() {
        return Err(AgentStatError::InvalidResponse("no recognized usage data"));
    }
    for window in &snapshot.windows {
        if window.used.as_ref().is_some_and(Decimal::is_negative)
            || matches!(&window.limit,QuotaLimit::Known(n) if n.is_negative())
            || window
                .utilization_percent
                .is_some_and(|n| !n.is_finite() || n < 0.0)
        {
            return Err(AgentStatError::InvalidResponse("invalid quota counter"));
        }
    }
    snapshot.fetched_at = Utc::now();
    Ok(snapshot)
}
pub(super) fn push_balance(s: &mut UsageSnapshot, id: &str, v: &Value, unit: Unit) {
    if let Some(amount) = decimal(v) {
        s.balances.push(Balance {
            id: id.into(),
            amount,
            unit,
        })
    }
}
pub(super) fn credit_unit(provider: ProviderId) -> Unit {
    Unit::Credits(provider.to_string())
}
