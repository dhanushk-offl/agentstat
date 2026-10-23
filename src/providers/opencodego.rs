//! `OpenCode` Go's key-authenticated, read-only subscription usage endpoint.
use super::common::{auth, check_source, date, finish, number, percent_window};
use crate::{
    AgentStatError, Client, Credentials, FetchRequest, Result, Source, UsageSnapshot, WindowKind,
};
use reqwest::Method;
use serde_json::Value;

const USAGE_URL: &str = "https://opencode.ai/zen/go/v1/usage";

pub(super) async fn fetch(client: &Client, request: &FetchRequest) -> Result<UsageSnapshot> {
    if !matches!(
        request.credentials,
        Credentials::ApiKey(_) | Credentials::Bearer(_)
    ) {
        return Err(AgentStatError::InvalidCredentials(request.provider));
    }
    check_source(request, Source::Api)?;
    let headers = auth(request, "authorization", "Bearer ")?;
    let body = client
        .transport
        .json(request.provider, Method::GET, USAGE_URL, headers, None)
        .await?;
    parse(request, &body)
}

fn parse(request: &FetchRequest, body: &Value) -> Result<UsageSnapshot> {
    let usage = body["usage"]
        .as_object()
        .ok_or(AgentStatError::InvalidResponse("missing OpenCode Go usage"))?;
    let mut snapshot = UsageSnapshot::new(request, Source::Api);
    for (key, id, duration, kind) in [
        ("rolling", "five-hour", Some(18_000), WindowKind::Rolling),
        ("weekly", "weekly", Some(604_800), WindowKind::Calendar),
        ("monthly", "monthly", None, WindowKind::Calendar),
    ] {
        let row =
            usage
                .get(key)
                .filter(|row| row.is_object())
                .ok_or(AgentStatError::InvalidResponse(
                    "missing OpenCode Go quota window",
                ))?;
        if !matches!(row["status"].as_str(), Some("ok" | "rate-limited")) {
            return Err(AgentStatError::InvalidResponse(
                "invalid OpenCode Go quota status",
            ));
        }
        let percent = number(&row["percent"])
            .filter(|percent| percent.is_finite() && (0.0..=100.0).contains(percent))
            .ok_or(AgentStatError::InvalidResponse(
                "invalid OpenCode Go quota percentage",
            ))?;
        let reset = date(&row["resetsAt"]).ok_or(AgentStatError::InvalidResponse(
            "invalid OpenCode Go reset time",
        ))?;
        let mut window = percent_window(id, Some(percent), duration, Some(reset));
        window.kind = kind;
        snapshot.windows.push(window);
    }
    finish(snapshot)
}
