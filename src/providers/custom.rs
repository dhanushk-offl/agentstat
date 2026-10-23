//! Provider-specific adapters without declarative specs.

use super::common::{
    auth, check_source, credit_unit, date, decimal, finish, number, percent_window, push_balance,
    source, text, window,
};
use crate::{
    AgentStatError, Balance, Client, Credentials, FetchRequest, ProviderConfig, ProviderId,
    QuotaLimit, Result, Source, SpendSummary, Unit, UsageSnapshot, Warning, WindowKind, transport,
};
use reqwest::Method;
use serde_json::Value;

pub(super) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if r.provider == ProviderId::Gemini {
        return gemini(client, r).await;
    }
    check_source(r, source(r))?;
    if matches!(r.credentials, Credentials::Cookie(_) | Credentials::None) {
        return Err(AgentStatError::InvalidCredentials(r.provider));
    }
    let url = match r.provider {
        ProviderId::DeepSeek => "https://api.deepseek.com/user/balance",
        ProviderId::Copilot => "https://api.github.com/copilot_internal/user",
        ProviderId::ClinePass => "https://api.cline.bot/api/v1/users/me/plan/usage-limits",
        ProviderId::Synthetic => "https://api.synthetic.new/v2/quotas",
        ProviderId::SailResearch => "https://api.sailresearch.com/v2/usage/summary?range=30d",
        ProviderId::Zai => "https://api.z.ai/api/monitor/usage/quota/limit",
        ProviderId::Munito => "https://api.munito.ai/admin/v1/limits",
        _ => return Err(AgentStatError::UnsupportedSource(r.provider)),
    };
    let mut headers = auth(
        r,
        "authorization",
        if r.provider == ProviderId::Copilot {
            "token "
        } else {
            "Bearer "
        },
    )?;
    if r.provider == ProviderId::Copilot {
        transport::header(&mut headers, "editor-version", "vscode/1.96.2")?;
        transport::header(&mut headers, "editor-plugin-version", "copilot/1.250.0")?;
    }
    let data = client
        .transport
        .json(r.provider, Method::GET, url, headers, None)
        .await?;
    parse(r, &data)
}
pub(super) fn parse(r: &FetchRequest, v: &Value) -> Result<UsageSnapshot> {
    let mut s = UsageSnapshot::new(r, source(r));
    match r.provider {
        ProviderId::DeepSeek => {
            if v["is_available"].as_bool().is_none() {
                return Err(AgentStatError::InvalidResponse(
                    "missing balance availability",
                ));
            }
            for row in v["balance_infos"]
                .as_array()
                .ok_or(AgentStatError::InvalidResponse("missing balances"))?
            {
                let currency = text(&row["currency"])
                    .ok_or(AgentStatError::InvalidResponse("missing currency"))?;
                for key in ["total_balance", "granted_balance", "topped_up_balance"] {
                    push_balance(&mut s, key, &row[key], Unit::Currency(currency.clone()));
                }
            }
        }
        ProviderId::Copilot => {
            s.plan = text(&v["copilot_plan"]);
            for key in ["premium_interactions", "chat", "completions"] {
                let q = &v["quota_snapshots"][key];
                if !q.is_object() {
                    continue;
                }
                let mut w = window(
                    key,
                    Unit::Requests,
                    None,
                    decimal(&q["entitlement"]),
                    decimal(&q["remaining"]),
                );
                if let (QuotaLimit::Known(limit), Some(remaining)) = (&w.limit, &w.remaining) {
                    w.used = limit.checked_sub(remaining)
                }
                if q["unlimited"] == true {
                    w.limit = QuotaLimit::Unlimited;
                    w.used = None;
                    w.utilization_percent = None;
                } else {
                    w.utilization_percent = number(&q["percent_remaining"]).map(|p| 100.0 - p);
                }
                w.resets_at = date(&v["quota_reset_date"]).or_else(|| {
                    v["quota_reset_date"]
                        .as_str()
                        .and_then(|d| date(&Value::String(format!("{d}T00:00:00Z"))))
                });
                w.kind = WindowKind::Calendar;
                s.windows.push(w);
            }
        }
        ProviderId::ClinePass => {
            if v["success"] != true {
                return Err(AgentStatError::InvalidResponse(
                    "usage request did not succeed",
                ));
            }
            for row in v["data"]["limits"]
                .as_array()
                .ok_or(AgentStatError::InvalidResponse("missing limits"))?
            {
                let Some(id) = row["type"].as_str() else {
                    continue;
                };
                let duration = match id {
                    "five_hour" => 18000,
                    "weekly" => 604800,
                    "monthly" => 2592000,
                    _ => continue,
                };
                let percent = number(&row["percentUsed"])
                    .ok_or(AgentStatError::InvalidResponse("missing quota utilization"))?;
                let mut w =
                    percent_window(id, Some(percent), Some(duration), date(&row["resetsAt"]));
                w.kind = WindowKind::Rolling;
                s.windows.push(w);
            }
        }
        ProviderId::SailResearch => {
            if v["object"] != "usage.summary"
                || v["available"] != true
                || v["has_metronome_customer"] != true
            {
                return Err(AgentStatError::InvalidResponse(
                    "billing summary unavailable",
                ));
            }
            let amount = decimal(&v["period_spend"])
                .and_then(|v| v.divided_by_power_of_ten(2))
                .ok_or(AgentStatError::InvalidResponse("missing spend"))?;
            s.spend.push(SpendSummary {
                id: text(&v["effective_range"]).unwrap_or_else(|| "reported-period".into()),
                amount,
                currency: "USD".into(),
                starts_at: None,
                ends_at: None,
            });
            if v["balance_unavailable"] == false
                && let Some(amount) =
                    decimal(&v["balance"]).and_then(|v| v.divided_by_power_of_ten(2))
            {
                s.balances.push(Balance {
                    id: "available".into(),
                    amount,
                    unit: Unit::Currency("USD".into()),
                });
            }
        }
        ProviderId::Gemini => {
            for (i, bucket) in v["buckets"]
                .as_array()
                .ok_or(AgentStatError::InvalidResponse("missing quota buckets"))?
                .iter()
                .enumerate()
            {
                let Some(fraction) =
                    number(&bucket["remainingFraction"]).filter(|f| *f >= 0.0 && *f <= 1.0)
                else {
                    s.warnings.push(Warning::MalformedOptionalData);
                    continue;
                };
                let mut w = percent_window(
                    &format!("bucket-{i}"),
                    Some((1.0 - fraction) * 100.0),
                    None,
                    date(&bucket["resetTime"]),
                );
                w.model = text(&bucket["modelId"]);
                s.windows.push(w);
            }
        }
        ProviderId::Zai => {
            if v["success"] != true {
                return Err(AgentStatError::InvalidResponse("quota request failed"));
            }
            for (i, row) in v["data"]["limits"]
                .as_array()
                .ok_or(AgentStatError::InvalidResponse("missing quota limits"))?
                .iter()
                .enumerate()
            {
                let Some(t) = row["type"]
                    .as_str()
                    .filter(|t| ["TOKENS_LIMIT", "TIME_LIMIT", "CREDIT_LIMIT"].contains(t))
                else {
                    continue;
                };
                let unit = if t == "TOKENS_LIMIT" {
                    Unit::Tokens
                } else {
                    credit_unit(r.provider)
                };
                let mut w = window(
                    &format!("{t}-{i}"),
                    unit,
                    decimal(&row["currentValue"]),
                    decimal(&row["usage"]),
                    decimal(&row["remaining"]),
                );
                w.utilization_percent =
                    w.utilization_percent.or_else(|| number(&row["percentage"]));
                let multiplier = match row["unit"].as_u64() {
                    Some(1) => Some(86400u64),
                    Some(3) => Some(3600),
                    Some(5) => Some(60),
                    Some(6) => Some(604800),
                    _ => None,
                };
                w.duration_seconds =
                    multiplier.and_then(|m| row["number"].as_u64()?.checked_mul(m));
                w.resets_at = date(&row["nextResetTime"]);
                s.windows.push(w);
            }
        }
        ProviderId::Munito => {
            for row in v["periods"]
                .as_array()
                .ok_or(AgentStatError::InvalidResponse("missing spend periods"))?
            {
                let Some(id) = row["period"]
                    .as_str()
                    .filter(|p| ["day", "month"].contains(p))
                else {
                    continue;
                };
                let used = decimal(&row["spent_usd"])
                    .ok_or(AgentStatError::InvalidResponse("missing spend amount"))?;
                s.spend.push(SpendSummary {
                    id: id.into(),
                    amount: used.clone(),
                    currency: "USD".into(),
                    starts_at: None,
                    ends_at: None,
                });
                let mut w = window(
                    id,
                    Unit::Currency("USD".into()),
                    Some(used),
                    decimal(&row["limit_usd"]),
                    None,
                );
                w.kind = WindowKind::Calendar;
                if id == "day" {
                    w.duration_seconds = Some(86400)
                }
                s.windows.push(w);
            }
        }
        ProviderId::Synthetic => {
            let root = if v["data"].is_object() { &v["data"] } else { v };
            let slots = [
                ("five_hour", &root["rollingFiveHourLimit"]),
                ("weekly", &root["weeklyTokenLimit"]),
                ("search_hourly", &root["search"]["hourly"]),
            ];
            let candidates: Vec<(String, &Value)> = if slots.iter().any(|(_, v)| v.is_object()) {
                slots
                    .into_iter()
                    .filter(|(_, v)| v.is_object())
                    .map(|(id, v)| (id.to_owned(), v))
                    .collect()
            } else if let Some(a) = v.as_array().or_else(|| root["quotas"].as_array()) {
                a.iter()
                    .enumerate()
                    .map(|(i, v)| (format!("quota-{i}"), v))
                    .collect()
            } else {
                root.as_object()
                    .map(|o| {
                        o.iter()
                            .filter(|(_, v)| v.is_object())
                            .map(|(k, v)| (k.clone(), v))
                            .collect()
                    })
                    .unwrap_or_default()
            };
            for (fallback, row) in &candidates {
                let mut used = ["used", "usage", "requests", "usedMessages", "requestCount"]
                    .iter()
                    .find_map(|k| decimal(&row[k]));
                let mut limit = ["limit", "quota", "total", "messageLimit", "maxRequests"]
                    .iter()
                    .find_map(|k| decimal(&row[k]));
                let remaining = ["remaining", "available"]
                    .iter()
                    .find_map(|k| decimal(&row[k]));
                let id = ["name", "label", "type", "period"]
                    .iter()
                    .find_map(|k| text(&row[k]))
                    .unwrap_or_else(|| fallback.clone());
                if limit.is_none() {
                    limit = used
                        .as_ref()
                        .zip(remaining.as_ref())
                        .and_then(|(u, r)| u.checked_add(r));
                }
                if used.is_none() {
                    used = limit
                        .as_ref()
                        .zip(remaining.as_ref())
                        .and_then(|(l, r)| l.checked_sub(r));
                }
                let mut w = window(&id, credit_unit(r.provider), used, limit, remaining);
                if let Some(percent) = [
                    "usedPercent",
                    "percentUsed",
                    "usage_percent",
                    "used_percent",
                ]
                .iter()
                .find_map(|k| number(&row[k]))
                {
                    w.utilization_percent = Some(if percent <= 1.0 {
                        percent * 100.0
                    } else {
                        percent
                    })
                }
                w.resets_at = ["resetAt", "reset_at", "resetsAt", "resets_at", "renewsAt"]
                    .iter()
                    .find_map(|k| date(&row[k]));
                w.duration_seconds = row["windowMinutes"]
                    .as_u64()
                    .and_then(|n| n.checked_mul(60))
                    .or_else(|| match id.to_lowercase().as_str() {
                        "weekly" => Some(604800),
                        "subscription" | "five_hour" => Some(18000),
                        "search_hourly" | "search" => Some(3600),
                        _ => None,
                    });
                if w.used.is_some() || w.utilization_percent.is_some() || w.remaining.is_some() {
                    s.windows.push(w)
                }
            }
        }
        _ => return Err(AgentStatError::UnsupportedSource(r.provider)),
    }
    finish(s)
}

async fn gemini(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if !matches!(r.credentials, Credentials::Bearer(_)) {
        return Err(AgentStatError::InvalidCredentials(r.provider));
    }
    check_source(r, Source::OAuth)?;
    let mut body = serde_json::json!({});
    if let ProviderConfig::Gemini { project_id } = &r.config {
        body["project"] = Value::String(project_id.clone());
    }
    let data = client
        .transport
        .json(
            r.provider,
            Method::POST,
            "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota",
            auth(r, "authorization", "Bearer ")?,
            Some(body),
        )
        .await?;
    parse(r, &data)
}
