use super::common::*;
use crate::*;
use reqwest::Method;
use serde_json::Value;

pub(super) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if !matches!(r.credentials, Credentials::Bearer(_)) {
        return Err(AgentStatError::InvalidCredentials(r.provider));
    }
    check_source(r, Source::OAuth)?;
    let mut headers = auth(r, "authorization", "Bearer ")?;
    if let Some(account) = &r.scope.account_id {
        crate::transport::header(&mut headers, "chatgpt-account-id", account)?;
    }
    let body = client
        .transport
        .json(
            r.provider,
            Method::GET,
            "https://chatgpt.com/backend-api/wham/usage",
            headers.clone(),
            None,
        )
        .await?;
    let mut snapshot = parse(r, &body)?;
    if r.include_optional {
        let remaining = r
            .deadline
            .map(|d| d.saturating_duration_since(tokio::time::Instant::now()))
            .unwrap_or(std::time::Duration::from_secs(2));
        let budget = remaining
            .saturating_sub(std::time::Duration::from_millis(10))
            .min(std::time::Duration::from_secs(2));
        if budget.is_zero() {
            snapshot.warnings.push(Warning::OptionalFetchFailed);
            return finish(snapshot);
        }
        crate::transport::header(&mut headers, "openai-beta", "codex-1")?;
        crate::transport::header(&mut headers, "originator", "Codex Desktop")?;
        let extra = tokio::time::timeout(
            budget,
            client.transport.json(
                r.provider,
                Method::GET,
                "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits",
                headers,
                None,
            ),
        )
        .await;
        match extra {
            Ok(Ok(data)) => {
                if let Some(amount) = decimal(&data["available_count"]) {
                    snapshot.balances.push(Balance {
                        id: "rate-limit-reset-credits".into(),
                        amount,
                        unit: credit_unit(r.provider),
                    })
                } else {
                    snapshot.warnings.push(Warning::MalformedOptionalData)
                }
            }
            _ => snapshot.warnings.push(Warning::OptionalFetchFailed),
        }
    }
    finish(snapshot)
}
fn add_windows(s: &mut UsageSnapshot, v: &Value, prefix: &str, model: Option<String>) {
    for (key, id) in [
        ("primary_window", "primary"),
        ("secondary_window", "secondary"),
    ] {
        let w = &v[key];
        if w.is_null() {
            continue;
        }
        if let Some(p) = number(&w["used_percent"]).filter(|p| *p >= 0.0) {
            let mut out = percent_window(
                &format!("{prefix}{id}"),
                Some(p),
                w["limit_window_seconds"].as_u64(),
                date(&w["reset_at"]),
            );
            out.model = model.clone();
            out.kind = WindowKind::Rolling;
            s.windows.push(out);
        } else {
            s.warnings.push(Warning::MalformedOptionalData)
        }
    }
}
pub(super) fn parse(r: &FetchRequest, v: &Value) -> Result<UsageSnapshot> {
    if !v.is_object() {
        return Err(AgentStatError::InvalidResponse(
            "expected Codex usage object",
        ));
    }
    let mut s = UsageSnapshot::new(r, Source::OAuth);
    s.plan = text(&v["plan_type"]);
    if let Some(account) = text(&v["account_id"]).or_else(|| text(&v["accountId"])) {
        if r.scope
            .account_id
            .as_ref()
            .is_some_and(|expected| expected != &account)
        {
            return Err(AgentStatError::InvalidResponse("account scope mismatch"));
        }
        s.scope.account_id = Some(account);
    }
    add_windows(&mut s, &v["rate_limit"], "", None);
    if v["credits"]["unlimited"] == true {
        let mut w = window("credits", credit_unit(r.provider), None, None, None);
        w.limit = QuotaLimit::Unlimited;
        s.windows.push(w);
    }
    if let Some(extras) = v["additional_rate_limits"].as_array() {
        for (i, extra) in extras.iter().enumerate() {
            let model = text(&extra["metered_feature"]).or_else(|| text(&extra["limit_name"]));
            let prefix = model.as_ref().map_or_else(
                || format!("additional-{i}-"),
                |model| format!("model-{model}-"),
            );
            add_windows(&mut s, &extra["rate_limit"], &prefix, model);
        }
    } else if !v["additional_rate_limits"].is_null() {
        s.warnings.push(Warning::MalformedOptionalData)
    }
    push_balance(
        &mut s,
        "credits",
        &v["credits"]["balance"],
        credit_unit(r.provider),
    );
    // Root, rate_limit, then workspace spend_control, matching upstream precedence.
    let limit = [
        &v["individual_limit"],
        &v["individualLimit"],
        &v["rate_limit"]["individual_limit"],
        &v["rate_limit"]["individualLimit"],
        &v["spend_control"]["individual_limit"],
        &v["spendControl"]["individualLimit"],
    ]
    .into_iter()
    .find(|v| v.is_object());
    if let Some(l) = limit {
        let mut w = window(
            "monthly-credits",
            credit_unit(r.provider),
            decimal(&l["used"]),
            decimal(&l["limit"]),
            None,
        );
        w.kind = WindowKind::Calendar;
        w.resets_at = date(&l["resets_at"])
            .or_else(|| date(&l["resetsAt"]))
            .or_else(|| date(&l["reset_at"]));
        if w.utilization_percent.is_none() {
            w.utilization_percent = number(&l["remaining_percent"])
                .or_else(|| number(&l["remainingPercent"]))
                .map(|n| 100.0 - n)
        }
        s.windows.push(w);
    }
    finish(s)
}
