use super::common::*;
use crate::*;
use reqwest::Method;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if !matches!(r.credentials, Credentials::Bearer(_)) {
        return Err(AgentStatError::InvalidCredentials(r.provider));
    }
    check_source(r, Source::OAuth)?;
    let body = match &r.config {
        ProviderConfig::Default => json!({}),
        ProviderConfig::Antigravity { project_id } if !project_id.trim().is_empty() => {
            json!({"project": project_id})
        }
        _ => {
            return Err(AgentStatError::Configuration(
                "invalid Antigravity project configuration",
            ));
        }
    };
    let mut headers = auth(r, "authorization", "Bearer ")?;
    crate::transport::header(&mut headers, "user-agent", "antigravity")?;
    let body = client
        .transport
        .json(
            r.provider,
            Method::POST,
            "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
            headers,
            Some(body),
        )
        .await?;
    parse(r, &body, Source::OAuth)
}

fn field<'a>(value: &'a Value, camel: &str, snake: &str) -> &'a Value {
    value
        .get(camel)
        .or_else(|| value.get(snake))
        .unwrap_or(&Value::Null)
}

pub(super) fn parse(r: &FetchRequest, body: &Value, source: Source) -> Result<UsageSnapshot> {
    let data = if source == Source::LocalProbe {
        // Only accept the structured slash-command response, never an agent's generated text.
        body.pointer("/command/data")
            .filter(|v| v.is_object())
            .ok_or(AgentStatError::InvalidResponse(
                "expected Antigravity usage command data",
            ))?
    } else {
        body
    };
    let mut snapshot = UsageSnapshot::new(r, source);
    if source == Source::LocalProbe {
        snapshot.credential_source = CredentialSource::LocalProbe;
    }
    snapshot.plan = text(field(data, "planTier", "plan_tier"));
    let mut ids = BTreeSet::new();
    if let Some(groups) = data.get("groups") {
        let groups = groups.as_array().ok_or(AgentStatError::InvalidResponse(
            "invalid Antigravity quota groups",
        ))?;
        for (index, group) in groups.iter().enumerate() {
            let label =
                text(field(group, "displayName", "display_name")).or_else(|| text(&group["name"]));
            let buckets = group["buckets"]
                .as_array()
                .ok_or(AgentStatError::InvalidResponse(
                    "invalid Antigravity quota buckets",
                ))?;
            for (bucket_index, bucket) in buckets.iter().enumerate() {
                add_bucket(
                    &mut snapshot,
                    bucket,
                    label.clone(),
                    &format!("group-{index}-{bucket_index}"),
                    &mut ids,
                )?;
            }
        }
    } else if let Some(buckets) = data["buckets"].as_array() {
        for (index, bucket) in buckets.iter().enumerate() {
            add_bucket(
                &mut snapshot,
                bucket,
                None,
                &format!("bucket-{index}"),
                &mut ids,
            )?;
        }
    } else {
        return Err(AgentStatError::InvalidResponse(
            "expected Antigravity quota summary",
        ));
    }
    finish(snapshot)
}

fn add_bucket(
    s: &mut UsageSnapshot,
    bucket: &Value,
    group: Option<String>,
    fallback: &str,
    ids: &mut BTreeSet<String>,
) -> Result<()> {
    let fraction = field(bucket, "remainingFraction", "remaining_fraction");
    // Disabled buckets have no fraction. They do not establish zero consumption.
    if fraction.is_null() {
        s.warnings.push(Warning::IncompleteData);
        return Ok(());
    }
    let Some(fraction) = number(fraction).filter(|n| (0.0..=1.0).contains(n)) else {
        s.warnings.push(Warning::MalformedOptionalData);
        return Ok(());
    };
    let id = text(field(bucket, "bucketId", "bucket_id"))
        .or_else(|| text(&bucket["id"]))
        .unwrap_or_else(|| fallback.to_owned());
    if !ids.insert(id.clone()) {
        return Err(AgentStatError::InvalidResponse(
            "duplicate Antigravity quota bucket ID",
        ));
    }
    let duration = text(&bucket["window"])
        .or_else(|| id.rsplit_once('-').map(|(_, period)| period.to_owned()));
    let duration =
        duration
            .as_deref()
            .and_then(|period| match period.to_ascii_lowercase().as_str() {
                "5h" | "five_hour" | "five-hour" | "18000s" => Some(18_000),
                "weekly" | "week" | "7d" | "168h" | "604800s" => Some(604_800),
                _ => None,
            });
    let raw_reset = field(bucket, "resetTime", "reset_time");
    let reset = date(raw_reset);
    if !raw_reset.is_null() && reset.is_none() {
        s.warnings.push(Warning::MalformedOptionalData);
    }
    let mut window = percent_window(&id, Some((1.0 - fraction) * 100.0), duration, reset);
    window.model = group;
    window.kind = WindowKind::Rolling;
    s.windows.push(window);
    Ok(())
}
