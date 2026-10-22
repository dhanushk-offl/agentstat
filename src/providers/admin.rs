//! Provider-reported organization costs. No inference or credential exchanges.
use super::common::*;
use crate::*;
use chrono::{Duration, Utc};
use reqwest::{Method, Url};
use std::collections::{BTreeMap, HashSet};

pub(super) async fn fetch(client: &Client, r: &FetchRequest) -> Result<UsageSnapshot> {
    if !matches!(r.credentials, Credentials::ApiKey(_)) {
        return Err(AgentStatError::InvalidCredentials(r.provider));
    }
    check_source(r, Source::Api)?;
    if r.scope != AccountScope::default() {
        return Err(AgentStatError::Configuration(
            "admin cost reports use the credential account scope",
        ));
    }
    let anthropic = r.provider == ProviderId::Claude;
    let mut headers = if anthropic {
        auth(r, "x-api-key", "")?
    } else {
        auth(r, "authorization", "Bearer ")?
    };
    if anthropic {
        crate::transport::header(&mut headers, "anthropic-version", "2023-06-01")?;
    }
    let end = Utc::now();
    let start = end - Duration::days(30);
    let base = if anthropic {
        "https://api.anthropic.com/v1/organizations/cost_report"
    } else {
        "https://api.openai.com/v1/organization/costs"
    };
    let mut totals = BTreeMap::<String, Decimal>::new();
    let mut page: Option<String> = None;
    let mut seen = HashSet::new();
    let mut complete = false;
    for _ in 0..20 {
        let mut url = Url::parse(base).expect("static URL");
        {
            let mut query = url.query_pairs_mut();
            if anthropic {
                query
                    .append_pair("starting_at", &start.to_rfc3339())
                    .append_pair("ending_at", &end.to_rfc3339());
            } else {
                query
                    .append_pair("start_time", &start.timestamp().to_string())
                    .append_pair("end_time", &end.timestamp().to_string());
            }
            query
                .append_pair("bucket_width", "1d")
                .append_pair("limit", "31");
            if let Some(p) = &page {
                query.append_pair("page", p);
            }
        }
        let data = client
            .transport
            .json(r.provider, Method::GET, url.as_str(), headers.clone(), None)
            .await?;
        let buckets = data["data"]
            .as_array()
            .ok_or(AgentStatError::InvalidResponse("missing cost buckets"))?;
        for bucket in buckets {
            for row in bucket["results"]
                .as_array()
                .ok_or(AgentStatError::InvalidResponse("missing cost results"))?
            {
                let raw = if anthropic {
                    &row["amount"]
                } else {
                    &row["amount"]["value"]
                };
                let mut amount =
                    decimal(raw).ok_or(AgentStatError::InvalidResponse("invalid cost amount"))?;
                if anthropic {
                    amount = amount
                        .divided_by_power_of_ten(2)
                        .ok_or(AgentStatError::InvalidResponse("cost amount overflow"))?;
                }
                let currency = if anthropic {
                    text(&row["currency"]).unwrap_or_else(|| "USD".into())
                } else {
                    text(&row["amount"]["currency"])
                        .ok_or(AgentStatError::InvalidResponse("missing cost currency"))?
                }
                .to_uppercase();
                let total = totals.entry(currency).or_insert_with(|| Decimal::from(0));
                *total = total
                    .checked_add(&amount)
                    .ok_or(AgentStatError::InvalidResponse("cost amount overflow"))?;
            }
        }
        if data["has_more"] != true {
            complete = true;
            break;
        }
        let next = data["next_page"]
            .as_str()
            .filter(|p| !p.is_empty())
            .ok_or(AgentStatError::InvalidResponse("missing next cost page"))?;
        if !seen.insert(next.to_owned()) {
            return Err(AgentStatError::InvalidResponse(
                "repeated cost pagination cursor",
            ));
        }
        page = Some(next.to_owned());
    }
    let mut s = UsageSnapshot::new(r, Source::Api);
    if !complete {
        s.warnings.push(Warning::IncompleteData)
    }
    // An explicitly empty, complete report establishes zero USD spend for this period.
    if totals.is_empty() && complete {
        totals.insert("USD".into(), Decimal::from(0));
    }
    for (currency, amount) in totals {
        s.spend.push(SpendSummary {
            id: "last-30-days".into(),
            amount,
            currency,
            starts_at: Some(start),
            ends_at: Some(end),
        });
    }
    finish(s)
}
