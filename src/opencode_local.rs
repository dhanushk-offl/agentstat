//! OpenCode's `stats --json` describes local session activity, not account quota.
use crate::*;
use chrono::{DateTime, Utc};
use serde_json::Value;

pub(crate) fn parse(request: &FetchRequest, text: &str) -> Result<UsageSnapshot> {
    let body: Value = serde_json::from_str(text)
        .map_err(|_| AgentStatError::InvalidResponse("expected OpenCode CLI stats JSON"))?;
    let range = &body["range"];
    let starts_at = timestamp(&range["from"])?;
    let ends_at = timestamp(&range["to"])?;
    if starts_at >= ends_at {
        return Err(AgentStatError::InvalidResponse(
            "invalid OpenCode CLI range",
        ));
    }
    let mut snapshot = UsageSnapshot::new(request, Source::LocalProbe);
    snapshot.credential_source = CredentialSource::LocalProbe;
    for (key, id) in [
        ("sessions", "sessions"),
        ("prompts", "prompts"),
        ("steps", "steps"),
    ] {
        push_count(&mut snapshot, id, &body[key], Unit::Requests, None)?;
    }
    let tokens = &body["tokens"];
    for (key, id) in [
        ("input", "input-tokens"),
        ("output", "output-tokens"),
        ("reasoning", "reasoning-tokens"),
    ] {
        push_count(&mut snapshot, id, &tokens[key], Unit::Tokens, None)?;
    }
    for (key, id) in [
        ("read", "cache-read-tokens"),
        ("write", "cache-write-tokens"),
    ] {
        push_count(&mut snapshot, id, &tokens["cache"][key], Unit::Tokens, None)?;
    }
    snapshot.spend.push(SpendSummary {
        id: "local-cli-cost".into(),
        amount: nonnegative_decimal(&body["cost"])
            .ok_or(AgentStatError::InvalidResponse("invalid OpenCode CLI cost"))?,
        currency: "USD".into(),
        starts_at: Some(starts_at),
        ends_at: Some(ends_at),
    });
    if let Some(models) = body.get("models") {
        let models = models.as_array().ok_or(AgentStatError::InvalidResponse(
            "invalid OpenCode CLI models",
        ))?;
        for row in models {
            let mut name = row["model"]["id"]
                .as_str()
                .zip(row["model"]["providerID"].as_str())
                .filter(|(id, provider)| !id.is_empty() && !provider.is_empty())
                .map(|(id, provider)| format!("{provider}/{id}"))
                .ok_or(AgentStatError::InvalidResponse(
                    "invalid OpenCode CLI model identifier",
                ))?;
            if let Some(variant) = row["model"]["variant"].as_str()
                && !variant.is_empty()
                && variant != "default"
            {
                name.push('#');
                name.push_str(variant);
            }
            for (key, id) in [("input", "input-tokens"), ("output", "output-tokens")] {
                push_count(
                    &mut snapshot,
                    id,
                    &row["tokens"][key],
                    Unit::Tokens,
                    Some(name.clone()),
                )?;
            }
            snapshot.spend.push(SpendSummary {
                id: format!("local-cli-cost:{name}"),
                amount: nonnegative_decimal(&row["cost"]).ok_or(
                    AgentStatError::InvalidResponse("invalid OpenCode CLI model cost"),
                )?,
                currency: "USD".into(),
                starts_at: Some(starts_at),
                ends_at: Some(ends_at),
            });
        }
    }
    Ok(snapshot)
}

fn timestamp(value: &Value) -> Result<DateTime<Utc>> {
    value
        .as_i64()
        .and_then(DateTime::from_timestamp_millis)
        .ok_or(AgentStatError::InvalidResponse(
            "invalid OpenCode CLI timestamp",
        ))
}

fn nonnegative_decimal(value: &Value) -> Option<Decimal> {
    let raw = value.as_number()?.to_string();
    let decimal = raw.parse::<Decimal>().ok()?;
    (!decimal.is_negative()).then_some(decimal)
}

fn push_count(
    snapshot: &mut UsageSnapshot,
    id: &str,
    value: &Value,
    unit: Unit,
    model: Option<String>,
) -> Result<()> {
    let count = value.as_u64().ok_or(AgentStatError::InvalidResponse(
        "invalid OpenCode CLI count",
    ))?;
    snapshot.windows.push(QuotaWindow {
        id: id.into(),
        model,
        used: Some(Decimal::from(count)),
        remaining: None,
        limit: QuotaLimit::Unknown,
        unit,
        utilization_percent: None,
        duration_seconds: None,
        resets_at: None,
        kind: WindowKind::Unknown,
    });
    Ok(())
}
