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
    let invalid =
        || AgentStatError::InvalidResponse("missing or invalid OpenCode CLI text statistic");
    let required = |name: &str| -> Result<&str> { fields.get(name).copied().ok_or_else(invalid) };
    let mut snapshot = UsageSnapshot::new(request, Source::LocalProbe);
    snapshot.credential_source = CredentialSource::LocalProbe;
    snapshot.warnings.push(Warning::IncompleteData);
    push_amount(
        &mut snapshot,
        "sessions",
        integer_count(required("sessions")?).ok_or_else(invalid)?,
        Unit::Requests,
    );
    if let Some(raw) = fields.get("messages") {
        let messages = integer_count(raw).ok_or_else(invalid)?;
        push_amount(&mut snapshot, "messages", messages, Unit::Requests);
    }
    for (field, id) in [
        ("input", "input-tokens"),
        ("output", "output-tokens"),
        ("cache read", "cache-read-tokens"),
        ("cache write", "cache-write-tokens"),
    ] {
        let amount = token_count(required(field)?).ok_or_else(invalid)?;
        push_amount(&mut snapshot, id, amount, Unit::Tokens);
    }
    snapshot.spend.push(SpendSummary {
        id: "local-cli-cost".into(),
        amount: cost(required("total cost")?).ok_or_else(invalid)?,
        currency: "USD".into(),
        starts_at: None,
        ends_at: None,
    });
    Ok(snapshot)
}

fn normalized_number(raw: &str) -> Option<String> {
    let (whole, fraction) = match raw.split_once('.') {
        Some((whole, fraction)) if !fraction.is_empty() => (whole, Some(fraction)),
        Some(_) => return None,
        None => (raw, None),
    };
    let mut groups = whole.split(',');
    let first = groups.next()?;
    if first.is_empty() || !first.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if whole.contains(',') && first.len() > 3 {
        return None;
    }
    let mut number = first.to_owned();
    for group in groups {
        if group.len() != 3 || !group.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        number.push_str(group);
    }
    if let Some(fraction) = fraction {
        if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        number.push('.');
        number.push_str(fraction);
    }
    Some(number)
}

fn integer_count(raw: &str) -> Option<Decimal> {
    let number = normalized_number(raw)?;
    if number.contains('.') {
        return None;
    }
    number.parse().ok()
}

fn token_count(raw: &str) -> Option<Decimal> {
    let (number, exponent) = match raw.chars().last()? {
        'K' | 'k' => (&raw[..raw.len() - 1], 3),
        'M' | 'm' => (&raw[..raw.len() - 1], 6),
        'B' | 'b' => (&raw[..raw.len() - 1], 9),
        _ => (raw, 0),
    };
    let number = normalized_number(number)?;
    let value: Decimal = format!("{number}e{exponent}").parse().ok()?;
    (!value.as_str().contains('.')).then_some(value)
}

fn cost(raw: &str) -> Option<Decimal> {
    let number = normalized_number(raw.strip_prefix('$')?)?;
    number.parse().ok()
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
