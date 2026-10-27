//! `OpenCode` Console read-only JSON endpoints. Legacy server-function scraping is excluded.
use super::common::{auth, check_source, date, finish, window};
use crate::{
    AgentStatError, Balance, Client, Credentials, Decimal, FetchRequest, ProviderConfig, Result,
    Source, SpendSummary, Unit, UsageSnapshot, Warning, WindowKind, transport,
};
use chrono::{Duration, Utc};
use reqwest::Method;
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::collections::BTreeSet;

const BASE: &str = "https://opencode.ai/console/api";

pub(super) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if !matches!(r.credentials, Credentials::Cookie(_)) {
        return Err(AgentStatError::InvalidCredentials(r.provider));
    }
    check_source(r, Source::Web)?;
    if !matches!(r.config, ProviderConfig::Default) {
        return Err(AgentStatError::Configuration(
            "OpenCode uses AccountScope.workspace_id and default provider configuration",
        ));
    }
    let mut headers = auth(r, "cookie", "")?;
    transport::header(&mut headers, "origin", "https://opencode.ai")?;
    transport::header(&mut headers, "referer", "https://opencode.ai/console")?;
    let workspace = match &r.scope.workspace_id {
        Some(id) if valid_workspace(id) => id.clone(),
        Some(_) => {
            return Err(AgentStatError::Configuration(
                "invalid OpenCode workspace ID",
            ));
        }
        None => {
            let rows = get(client, r, &headers, "/orgs").await?;
            let rows = rows.as_array().ok_or(AgentStatError::InvalidResponse(
                "expected OpenCode workspace list",
            ))?;
            let ids: BTreeSet<&str> = rows
                .iter()
                .filter_map(|row| row["id"].as_str().filter(|id| valid_workspace(id)))
                .collect();
            match ids.len() {
                0 => {
                    return Err(AgentStatError::InvalidResponse(
                        "no OpenCode workspace available",
                    ));
                }
                1 => ids.first().expect("one workspace").to_string(),
                _ => {
                    return Err(AgentStatError::Configuration(
                        "multiple OpenCode workspaces; select AccountScope.workspace_id explicitly",
                    ));
                }
            }
        }
    };
    transport::header(&mut headers, "x-org-id", &workspace)?;
    let mut snapshot = UsageSnapshot::new(r, Source::Web);
    snapshot.scope.workspace_id = Some(workspace.clone());
    let status = get(client, r, &headers, "/go/status").await?;
    if let Some(access) = status.get("access").filter(|v| v.is_object()) {
        parse_meters(&mut snapshot, access)?;
        if r.include_optional {
            match get(client, r, &headers, "/billing/status").await {
                Ok(billing) => add_prepaid_balance(&mut snapshot, &billing),
                Err(_) => snapshot.warnings.push(Warning::OptionalFetchFailed),
            }
        }
        return finish(snapshot);
    }
    // Only an explicit null establishes absent quota. Never relabel broken subscription data.
    if !(status.is_null() || status.get("access").is_some_and(Value::is_null)) {
        return Err(AgentStatError::InvalidResponse(
            "invalid OpenCode subscription status",
        ));
    }
    let organization = get(client, r, &headers, "/orgs/current").await?;
    if organization["hasGoSubscription"] != false {
        return Err(AgentStatError::InvalidResponse(
            "OpenCode subscription quota unavailable",
        ));
    }
    if let Some(id) = organization["id"].as_str()
        && id != workspace
    {
        return Err(AgentStatError::InvalidResponse(
            "OpenCode workspace scope mismatch",
        ));
    }
    let billing = get(client, r, &headers, "/billing/status").await?;
    if billing["mode"] != "pay-as-you-go" {
        return Err(AgentStatError::InvalidResponse(
            "unsupported OpenCode billing mode",
        ));
    }
    let summary = get(client, r, &headers, "/usage/summary?range=30d").await?;
    let amount = microcents(&summary["totalCostMicroCents"], false).ok_or(
        AgentStatError::InvalidResponse("invalid OpenCode spend total"),
    )?;
    let now = Utc::now();
    snapshot.spend.push(SpendSummary {
        id: "last-30-days".into(),
        amount,
        currency: "USD".into(),
        starts_at: Some(now - Duration::days(30)),
        ends_at: Some(now),
    });
    if billing["billingMode"] == "prepaid" {
        add_prepaid_balance(&mut snapshot, &billing);
    }
    finish(snapshot)
}

fn add_prepaid_balance(snapshot: &mut UsageSnapshot, billing: &Value) {
    if billing["billingMode"] != "prepaid" {
        return;
    }
    if let Some(amount) = microcents(&billing["balanceMicroCents"], true) {
        snapshot.balances.push(Balance {
            id: "prepaid".into(),
            amount,
            unit: Unit::Currency("USD".into()),
        });
    } else {
        snapshot
            .warnings
            .push(if billing["balanceMicroCents"].is_null() {
                Warning::IncompleteData
            } else {
                Warning::MalformedOptionalData
            });
    }
}

async fn get(client: &Client, r: &FetchRequest, headers: &HeaderMap, path: &str) -> Result<Value> {
    client
        .transport
        .json(
            r.provider,
            Method::GET,
            &format!("{BASE}{path}"),
            headers.clone(),
            None,
        )
        .await
}

fn valid_workspace(id: &str) -> bool {
    id.strip_prefix("wrk_")
        .or_else(|| id.strip_prefix("org_"))
        .is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
}

/// One USD = 100 million microcents. Counts must be whole numbers, never floats/booleans.
fn microcents(value: &Value, signed: bool) -> Option<Decimal> {
    let raw = match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return None,
    };
    let digits = if signed {
        raw.strip_prefix('-').unwrap_or(&raw)
    } else {
        &raw
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<Decimal>().ok()?.divided_by_power_of_ten(8)
}

fn parse_meters(s: &mut UsageSnapshot, access: &Value) -> Result<()> {
    let meters = access["meters"]
        .as_object()
        .ok_or(AgentStatError::InvalidResponse("missing OpenCode meters"))?;
    for (key, id, duration) in [
        ("fiveHour", "five-hour", Some(18000)),
        ("week", "weekly", Some(604800)),
        ("month", "monthly", None),
    ] {
        let Some(meter) = meters.get(key).filter(|m| !m.is_null()) else {
            if key == "fiveHour" {
                return Err(AgentStatError::InvalidResponse(
                    "missing OpenCode five-hour meter",
                ));
            }
            continue;
        };
        let values = microcents(&meter["usedMicroCents"], false)
            .zip(microcents(&meter["limitMicroCents"], false));
        let Some((used, limit)) = values else {
            if key == "fiveHour" {
                return Err(AgentStatError::InvalidResponse(
                    "invalid OpenCode five-hour meter",
                ));
            }
            s.warnings.push(Warning::MalformedOptionalData);
            continue;
        };
        let mut w = window(
            id,
            Unit::Currency("USD".into()),
            Some(used),
            Some(limit),
            None,
        );
        w.duration_seconds = duration;
        w.kind = if key == "month" {
            WindowKind::Calendar
        } else {
            WindowKind::Rolling
        };
        let reset = if key == "month" && meter["resetsAt"].is_null() {
            &access["endsAt"]
        } else {
            &meter["resetsAt"]
        };
        w.resets_at = date(reset);
        if !reset.is_null() && w.resets_at.is_none() {
            s.warnings.push(Warning::MalformedOptionalData);
        }
        s.windows.push(w);
    }
    Ok(())
}
