//! Ollama Cloud balance, including current credit plans and legacy quota plans.
use super::common::*;
use crate::*;
use reqwest::Method;
use serde_json::Value;

const BALANCE_URL: &str = "https://ollama.com/api/balance";

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
        .json(request.provider, Method::GET, BALANCE_URL, headers, None)
        .await?;
    parse(request, &body)
}

fn parse(request: &FetchRequest, body: &Value) -> Result<UsageSnapshot> {
    let included = body["included"]
        .as_object()
        .ok_or(AgentStatError::InvalidResponse(
            "missing Ollama included quota",
        ))?;
    let mut snapshot = UsageSnapshot::new(request, Source::Api);
    if included.contains_key("allowance_usd") || included.contains_key("balance_usd") {
        let allowance = decimal(&body["included"]["allowance_usd"])
            .filter(|amount| !amount.is_negative())
            .ok_or(AgentStatError::InvalidResponse(
                "invalid Ollama included allowance",
            ))?;
        let remaining = decimal(&body["included"]["balance_usd"]).ok_or(
            AgentStatError::InvalidResponse("invalid Ollama included balance"),
        )?;
        let used = allowance
            .checked_sub(&remaining)
            .ok_or(AgentStatError::InvalidResponse(
                "invalid Ollama included usage",
            ))?;
        let reset = date(&body["included"]["period"]["until"]).ok_or(
            AgentStatError::InvalidResponse("invalid Ollama included reset time"),
        )?;
        let mut window = window(
            "included",
            Unit::Currency("USD".into()),
            Some(used),
            Some(allowance),
            Some(remaining),
        );
        window.kind = WindowKind::Calendar;
        window.resets_at = Some(reset);
        snapshot.windows.push(window);
    } else {
        for (key, id, duration) in [
            ("session", "session", 18_000),
            ("weekly", "weekly", 604_800),
        ] {
            let row = &body["included"][key];
            if !row.is_object() {
                return Err(AgentStatError::InvalidResponse(
                    "missing Ollama legacy quota window",
                ));
            }
            let remaining = number(&row["remaining_percent"])
                .filter(|percent| percent.is_finite() && (0.0..=100.0).contains(percent))
                .ok_or(AgentStatError::InvalidResponse(
                    "invalid Ollama remaining percentage",
                ))?;
            let reset = date(&row["resets_at"]).ok_or(AgentStatError::InvalidResponse(
                "invalid Ollama legacy reset time",
            ))?;
            let mut window =
                percent_window(id, Some(100.0 - remaining), Some(duration), Some(reset));
            window.kind = WindowKind::Rolling;
            snapshot.windows.push(window);
        }
    }

    if let Some(purchased) = body.get("purchased") {
        let amount = decimal(&purchased["balance_usd"])
            .filter(|amount| !amount.is_negative())
            .ok_or(AgentStatError::InvalidResponse(
                "invalid Ollama purchased balance",
            ))?;
        snapshot.balances.push(Balance {
            id: "purchased".into(),
            amount,
            unit: Unit::Currency("USD".into()),
        });
    }
    finish(snapshot)
}
