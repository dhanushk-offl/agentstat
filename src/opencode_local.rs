//! `OpenCode`'s `stats --json` describes local session activity, not account quota.
use crate::{
    AgentStatError, CredentialSource, Decimal, FetchRequest, QuotaLimit, QuotaWindow, Result,
    Source, SpendSummary, Unit, UsageSnapshot, Warning, WindowKind,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;

/// Older `OpenCode` CLIs do not expose `stats --json`. Their text report rounds
/// token and cost values, so preserve that limitation in the snapshot.
pub(crate) fn parse_text(request: &FetchRequest, text: &str) -> Result<UsageSnapshot> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let inner = line.trim().trim_matches('│').trim();
        let parts: Vec<_> = inner.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let label = parts[..parts.len() - 1].join(" ").to_ascii_lowercase();
        fields.insert(label, parts[parts.len() - 1]);
    }
    let required = |name: &str| -> Result<Decimal> {
        fields.get(name).and_then(|raw| compact_decimal(raw)).ok_or(
            AgentStatError::InvalidResponse("missing or invalid OpenCode CLI text statistic"),
        )
    };
    let mut snapshot = UsageSnapshot::new(request, Source::LocalProbe);
    snapshot.credential_source = CredentialSource::LocalProbe;
    snapshot.warnings.push(Warning::IncompleteData);
    push_amount(
        &mut snapshot,
        "sessions",
        required("sessions")?,
        Unit::Requests,
    );
    if let Some(messages) = fields.get("messages").and_then(|raw| compact_decimal(raw)) {
        push_amount(&mut snapshot, "messages", messages, Unit::Requests);
    }
    for (field, id) in [
        ("input", "input-tokens"),
        ("output", "output-tokens"),
        ("cache read", "cache-read-tokens"),
        ("cache write", "cache-write-tokens"),
    ] {
        push_amount(&mut snapshot, id, required(field)?, Unit::Tokens);
    }
    snapshot.spend.push(SpendSummary {
        id: "local-cli-cost".into(),
        amount: required("total cost")?,
        currency: "USD".into(),
        starts_at: None,
        ends_at: None,
    });
    Ok(snapshot)
}

fn compact_decimal(raw: &str) -> Option<Decimal> {
    let number = raw.trim_start_matches('$').replace(',', "");
    let (number, exponent) = match number.chars().last()? {
        'K' | 'k' => (&number[..number.len() - 1], 3),
        'M' | 'm' => (&number[..number.len() - 1], 6),
        'B' | 'b' => (&number[..number.len() - 1], 9),
        _ => (number.as_str(), 0),
    };
    let value = format!("{number}e{exponent}").parse::<Decimal>().ok()?;
    (!value.is_negative()).then_some(value)
}

fn push_amount(snapshot: &mut UsageSnapshot, id: &str, amount: Decimal, unit: Unit) {
    snapshot.windows.push(QuotaWindow {
        id: id.into(),
        model: None,
        used: Some(amount),
        remaining: None,
        limit: QuotaLimit::Unknown,
        unit,
        utilization_percent: None,
        duration_seconds: None,
        resets_at: None,
        kind: WindowKind::Unknown,
    });
}

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
