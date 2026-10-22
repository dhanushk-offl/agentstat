use super::common::*;
use crate::*;
use reqwest::Method;
use serde_json::{Value, json};

pub(super) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if !matches!(r.config, ProviderConfig::Default) {
        return Err(AgentStatError::Configuration(
            "Cursor requires default provider configuration",
        ));
    }
    let (source, method, url, header, scheme, body) = match r.credentials {
        Credentials::Bearer(_) => (
            Source::OAuth,
            Method::POST,
            "https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage",
            "authorization",
            "Bearer ",
            Some(json!({})),
        ),
        Credentials::Cookie(_) => (
            Source::Web,
            Method::GET,
            "https://cursor.com/api/usage-summary",
            "cookie",
            "",
            None,
        ),
        _ => return Err(AgentStatError::InvalidCredentials(r.provider)),
    };
    check_source(r, source)?;
    let mut headers = auth(r, header, scheme)?;
    if source == Source::OAuth {
        crate::transport::header(&mut headers, "connect-protocol-version", "1")?;
    }
    let body = client
        .transport
        .json(r.provider, method, url, headers, body)
        .await?;
    parse(r, &body, source)
}

fn cents(value: &Value) -> Option<Decimal> {
    decimal(value)
        .filter(|v| !v.is_negative())?
        .divided_by_power_of_ten(2)
}

fn parse(r: &FetchRequest, body: &Value, source: Source) -> Result<UsageSnapshot> {
    if !body.is_object() {
        return Err(AgentStatError::InvalidResponse(
            "expected Cursor usage object",
        ));
    }
    let mut snapshot = UsageSnapshot::new(r, source);
    snapshot.plan = text(&body["membershipType"]);
    let reset = date(&body["billingCycleEnd"]);
    let start = date(&body["billingCycleStart"]);
    if (!body["billingCycleEnd"].is_null() && reset.is_none())
        || (!body["billingCycleStart"].is_null() && start.is_none())
    {
        snapshot.warnings.push(Warning::MalformedOptionalData);
    }
    if let (Some(start), Some(end)) = (start, reset)
        && end <= start
    {
        return Err(AgentStatError::InvalidResponse(
            "invalid Cursor billing period",
        ));
    }
    let plan = if source == Source::OAuth {
        &body["planUsage"]
    } else {
        &body["individualUsage"]["plan"]
    };
    if plan["enabled"] != false {
        for (key, id) in [
            ("totalPercentUsed", "included-total"),
            ("autoPercentUsed", "cursor-models"),
            ("apiPercentUsed", "other-models"),
        ] {
            if plan[key].is_null() {
                continue;
            }
            if let Some(percent) = number(&plan[key]).filter(|n| *n >= 0.0) {
                let mut w = percent_window(id, Some(percent), None, reset);
                w.kind = WindowKind::Calendar;
                snapshot.windows.push(w);
            } else {
                snapshot.warnings.push(Warning::MalformedOptionalData);
            }
        }
        // Preserve the monetary allowance separately from vendor-computed percentages:
        // they can use different denominators, including promotional credits.
        let used = if source == Source::OAuth {
            &plan["totalSpend"]
        } else {
            &plan["used"]
        };
        add_money_window(
            &mut snapshot,
            "plan-allowance",
            used,
            &plan["limit"],
            &plan["remaining"],
            reset,
        );
    }
    if source == Source::Web {
        let on_demand = &body["individualUsage"]["onDemand"];
        if on_demand["enabled"] == true {
            add_money_window(
                &mut snapshot,
                "on-demand",
                &on_demand["used"],
                &on_demand["limit"],
                &on_demand["remaining"],
                reset,
            );
            add_spend(&mut snapshot, &on_demand["used"], start, reset);
        }
    } else if body["spendLimitUsage"].is_object() {
        let spend = &body["spendLimitUsage"];
        add_spend(&mut snapshot, &spend["totalSpend"], start, reset);
        // Select the explicitly reported limit scope. Never substitute a team's
        // shared allowance for an individual's limit.
        let prefix = match spend["limitType"].as_str() {
            Some("user") => Some("individual"),
            Some("team") => Some("pooled"),
            _ => None,
        };
        if let Some(prefix) = prefix {
            add_money_window(
                &mut snapshot,
                &format!("on-demand-{prefix}"),
                &spend[format!("{prefix}Used")],
                &spend[format!("{prefix}Limit")],
                &spend[format!("{prefix}Remaining")],
                reset,
            );
        }
    }
    finish(snapshot)
}

fn add_money_window(
    s: &mut UsageSnapshot,
    id: &str,
    used: &Value,
    limit: &Value,
    remaining: &Value,
    reset: Option<chrono::DateTime<chrono::Utc>>,
) {
    let values = [used, limit, remaining];
    if values.iter().all(|v| v.is_null()) {
        return;
    }
    let remaining_amount = decimal(remaining).and_then(|v| v.divided_by_power_of_ten(2));
    if [used, limit]
        .iter()
        .any(|v| !v.is_null() && cents(v).is_none())
        || (!remaining.is_null() && remaining_amount.is_none())
    {
        s.warnings.push(Warning::MalformedOptionalData);
        return;
    }
    let mut w = window(
        id,
        Unit::Currency("USD".into()),
        cents(used),
        cents(limit),
        remaining_amount,
    );
    w.resets_at = reset;
    w.kind = WindowKind::Calendar;
    s.windows.push(w);
}

fn add_spend(
    s: &mut UsageSnapshot,
    value: &Value,
    start: Option<chrono::DateTime<chrono::Utc>>,
    end: Option<chrono::DateTime<chrono::Utc>>,
) {
    if let Some(amount) = cents(value) {
        s.spend.push(SpendSummary {
            id: "on-demand".into(),
            amount,
            currency: "USD".into(),
            starts_at: start,
            ends_at: end,
        });
    } else if !value.is_null() {
        s.warnings.push(Warning::MalformedOptionalData);
    }
}
