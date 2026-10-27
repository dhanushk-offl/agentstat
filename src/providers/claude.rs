//! Claude usage adapter.

use super::common::{
    auth, check_source, date, decimal, finish, number, percent_window, text, window,
};
use crate::{
    AgentStatError, Client, Credentials, FetchRequest, ProviderConfig, Result, Source,
    SpendSummary, Unit, UsageSnapshot, Warning, WindowKind, transport,
};
use reqwest::Method;
use serde_json::Value;
use std::collections::HashSet;
pub(super) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    let (src, headers, url) = match &r.credentials {
        Credentials::Bearer(_) => {
            if r.scope.organization_id.is_some() || !matches!(r.config, ProviderConfig::Default) {
                return Err(AgentStatError::Configuration(
                    "Claude OAuth does not accept organization routing; select Web with an organization or use an organization-specific token",
                ));
            }
            let mut h = auth(r, "authorization", "Bearer ")?;
            transport::header(&mut h, "anthropic-beta", "oauth-2025-04-20")?;
            transport::header(&mut h, "user-agent", "claude-code/2.1.0")?;
            (
                Source::OAuth,
                h,
                "https://api.anthropic.com/api/oauth/usage".to_owned(),
            )
        }
        Credentials::Cookie(_) => {
            if let ProviderConfig::Claude { organization_id } = &r.config
                && r.scope
                    .organization_id
                    .as_ref()
                    .is_some_and(|id| id != organization_id)
            {
                return Err(AgentStatError::Configuration(
                    "conflicting organization scope",
                ));
            }
            let org = match &r.config {
                ProviderConfig::Claude { organization_id } => Some(organization_id),
                _ => r.scope.organization_id.as_ref(),
            }
            .filter(|s| !s.is_empty())
            .ok_or(AgentStatError::Configuration(
                "Claude web requires organization_id",
            ))?;
            let mut url =
                reqwest::Url::parse("https://claude.ai/api/organizations/").expect("static URL");
            url.path_segments_mut()
                .expect("base URL")
                .pop_if_empty()
                .push(org)
                .push("usage");
            (Source::Web, auth(r, "cookie", "")?, url.to_string())
        }
        _ => return Err(AgentStatError::InvalidCredentials(r.provider)),
    };
    check_source(r, src)?;
    let body = client
        .transport
        .json(r.provider, Method::GET, &url, headers, None)
        .await?;
    parse(r, src, &body)
}
pub(super) fn parse(r: &FetchRequest, src: Source, v: &Value) -> Result<UsageSnapshot> {
    if !v.is_object() {
        return Err(AgentStatError::InvalidResponse(
            "expected Claude usage object",
        ));
    }
    let mut s = UsageSnapshot::new(r, src);
    if let ProviderConfig::Claude { organization_id } = &r.config {
        if r.scope
            .organization_id
            .as_ref()
            .is_some_and(|o| o != organization_id)
        {
            return Err(AgentStatError::Configuration(
                "conflicting organization scope",
            ));
        }
        s.scope.organization_id = Some(organization_id.clone());
    }
    for (key, val) in v.as_object().expect("checked") {
        let duration = if key == "five_hour" {
            Some(18000)
        } else if key.starts_with("seven_day") {
            Some(604800)
        } else {
            None
        };
        if duration.is_none() || val.is_null() {
            continue;
        }
        if let Some(p) = number(&val["utilization"]).filter(|p| *p >= 0.0) {
            let mut w = percent_window(key, Some(p), duration, date(&val["resets_at"]));
            w.kind = WindowKind::Rolling;
            s.windows.push(w)
        } else {
            s.warnings.push(Warning::MalformedOptionalData)
        }
    }
    if let Some(limits) = v["limits"].as_array() {
        let mut seen = HashSet::new();
        for limit in limits {
            if limit["group"] != "weekly" || limit["kind"] != "weekly_scoped" {
                continue;
            }
            let Some(model) = text(&limit["scope"]["model"]["id"])
                .or_else(|| text(&limit["scope"]["model"]["display_name"]))
            else {
                continue;
            };
            if !seen.insert(model.clone()) {
                continue;
            }
            if let Some(percent) = number(&limit["percent"]) {
                let mut w = percent_window(
                    &format!("weekly-model-{model}"),
                    Some(percent),
                    Some(604800),
                    date(&limit["resets_at"]),
                );
                w.model = Some(model);
                w.kind = WindowKind::Rolling;
                s.windows.push(w);
            } else {
                s.warnings.push(Warning::MalformedOptionalData)
            }
        }
    }
    let extra = &v["extra_usage"];
    if extra["is_enabled"] == true {
        let currency = text(&extra["currency"]).unwrap_or_else(|| "USD".into());
        let used = decimal(&extra["used_credits"]).and_then(|n| n.divided_by_power_of_ten(2));
        let limit = decimal(&extra["monthly_limit"]).and_then(|n| n.divided_by_power_of_ten(2));
        if used.is_some() || limit.is_some() {
            let mut w = window(
                "extra-usage",
                Unit::Currency(currency.clone()),
                used.clone(),
                limit,
                None,
            );
            w.utilization_percent = number(&extra["utilization"]).or(w.utilization_percent);
            w.kind = WindowKind::Calendar;
            s.windows.push(w);
        }
        if let Some(amount) = used {
            s.spend.push(SpendSummary {
                id: "extra-usage".into(),
                amount,
                currency,
                starts_at: None,
                ends_at: None,
            })
        }
    }
    finish(s)
}
