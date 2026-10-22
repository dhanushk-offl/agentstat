use agentstat::*;
use futures::future::BoxFuture;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
#[cfg(any(
    feature = "codex",
    feature = "claude",
    feature = "synthetic",
    feature = "openai",
    feature = "opencode",
    feature = "cursor",
    feature = "antigravity"
))]
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::{str::FromStr, time::Duration};
#[derive(Debug)]
struct MemoryHttp {
    replies: Mutex<VecDeque<Reply>>,
    requests: Mutex<Vec<String>>,
}
#[derive(Debug)]
struct Reply {
    status: u16,
    body: String,
    headers: Vec<(String, String)>,
}
impl Reply {
    fn json(body: Value) -> Self {
        Self {
            status: 200,
            body: body.to_string(),
            headers: vec![],
        }
    }
}
impl HttpExecutor for MemoryHttp {
    fn execute(&self, request: reqwest::Request) -> BoxFuture<'_, Result<HttpResponse>> {
        Box::pin(async move {
            let mut captured = format!(
                "{} {}{} HTTP/1.1\r\n",
                request.method(),
                request.url().path(),
                request
                    .url()
                    .query()
                    .map(|q| format!("?{q}"))
                    .unwrap_or_default()
            );
            for (k, v) in request.headers() {
                captured.push_str(&format!("{k}: {}\r\n", v.to_str().unwrap()));
            }
            captured.push_str("\r\n");
            if let Some(bytes) = request.body().and_then(reqwest::Body::as_bytes) {
                captured.push_str(std::str::from_utf8(bytes).unwrap());
            }
            self.requests.lock().unwrap().push(captured);
            let reply = self
                .replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected extra HTTP request");
            let mut headers = HeaderMap::new();
            for (k, v) in reply.headers {
                headers.insert(
                    HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    HeaderValue::from_str(&v).unwrap(),
                );
            }
            Ok(HttpResponse {
                status: reply.status,
                headers,
                body: reply.body.into_bytes(),
            })
        })
    }
}
struct Server(Arc<MemoryHttp>);
impl std::future::IntoFuture for Server {
    type Output = std::result::Result<Vec<String>, std::convert::Infallible>;
    type IntoFuture = std::future::Ready<Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        assert!(
            self.0.replies.lock().unwrap().is_empty(),
            "expected HTTP request was not made"
        );
        std::future::ready(Ok(self.0.requests.lock().unwrap().clone()))
    }
}
fn executors() -> &'static Mutex<HashMap<String, Arc<MemoryHttp>>> {
    static MAP: OnceLock<Mutex<HashMap<String, Arc<MemoryHttp>>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}
async fn server(replies: Vec<Reply>) -> (String, Server) {
    static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let url = format!(
        "http://127.0.0.1:{}",
        10000 + COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    );
    let executor = Arc::new(MemoryHttp {
        replies: Mutex::new(replies.into()),
        requests: Mutex::new(vec![]),
    });
    executors()
        .lock()
        .unwrap()
        .insert(url.clone(), executor.clone());
    (url, Server(executor))
}
fn builder(base: &str, provider: ProviderId) -> ClientBuilder {
    Client::builder()
        .endpoint_override(provider, base)
        .http_executor(executors().lock().unwrap().get(base).unwrap().clone())
}
fn config(v: &Value) -> ProviderConfig {
    if let Some(id) = v["cosmic"].as_str() {
        ProviderConfig::Cosmic {
            project_id: id.into(),
        }
    } else if let Some(id) = v["xai"].as_str() {
        ProviderConfig::Xai { team_id: id.into() }
    } else if let Some(id) = v["flora"].as_str() {
        ProviderConfig::Flora {
            workspace_id: id.into(),
        }
    } else if let Some(id) = v["exa"].as_str() {
        ProviderConfig::Exa {
            api_key_id: id.into(),
        }
    } else {
        ProviderConfig::Default
    }
}

#[cfg(feature = "cursor")]
#[tokio::test]
async fn cursor_cli_preserves_vendor_percentages_and_exact_money_separately() {
    let (base, server) = server(vec![Reply::json(json!({
        "billingCycleStart": "1790812800000", "billingCycleEnd": "1793491200000",
        "planUsage": {"totalPercentUsed": 10, "apiPercentUsed": 120,
            "totalSpend": "1500.125", "limit": "2000", "remaining": "499.875"},
        "spendLimitUsage": {"totalSpend": "12.345", "limitType": "user",
            "individualLimit": 0, "individualUsed": "12.345", "individualRemaining": 0,
            "pooledLimit": 100000}
    }))])
    .await;
    let snapshot = builder(&base, ProviderId::Cursor)
        .build()
        .unwrap()
        .fetch(FetchRequest::new(
            ProviderId::Cursor,
            Credentials::bearer("private"),
        ))
        .await
        .unwrap();
    assert_eq!(snapshot.source, Source::OAuth);
    let total = &snapshot.windows[0];
    assert_eq!(total.utilization_percent, Some(10.0));
    assert_eq!(total.limit, QuotaLimit::Unknown);
    assert_eq!(total.duration_seconds, None); // Billing periods are not assumed weekly/monthly.
    assert_eq!(total.resets_at.unwrap().timestamp_millis(), 1793491200000);
    assert_eq!(snapshot.windows[1].utilization_percent, Some(120.0));
    let allowance = snapshot
        .windows
        .iter()
        .find(|w| w.id == "plan-allowance")
        .unwrap();
    assert_eq!(allowance.used.as_ref().unwrap().as_str(), "15.00125");
    assert_eq!(allowance.remaining.as_ref().unwrap().as_str(), "4.99875");
    let spend = snapshot
        .windows
        .iter()
        .find(|w| w.id == "on-demand-individual")
        .unwrap();
    assert_eq!(spend.limit, QuotaLimit::Known(Decimal::from(0)));
    assert_eq!(spend.utilization_percent, None);
    assert_eq!(snapshot.spend[0].amount.as_str(), "0.12345");
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("connect-protocol-version: 1"));
    assert!(requests[0].ends_with("{}"));
}

#[cfg(feature = "cursor")]
#[tokio::test]
async fn cursor_cookie_uses_dashboard_and_keeps_missing_limits_unknown() {
    let (base, server) = server(vec![Reply::json(json!({
        "membershipType": "pro", "billingCycleEnd": "2026-11-01T00:00:00Z",
        "individualUsage": {"plan": {"enabled":true, "autoPercentUsed":0,
            "used": 100, "limit": null},
            "onDemand": {"enabled":true,"used":75,"limit":null,"remaining":null}},
        "teamUsage": {"onDemand": {"used":999999,"limit":999999}}
    }))])
    .await;
    let snapshot = builder(&base, ProviderId::Cursor)
        .build()
        .unwrap()
        .fetch(FetchRequest::new(
            ProviderId::Cursor,
            Credentials::cookie("WorkosCursorSessionToken=private"),
        ))
        .await
        .unwrap();
    assert_eq!(snapshot.source, Source::Web);
    assert_eq!(snapshot.plan.as_deref(), Some("pro"));
    assert_eq!(snapshot.windows[0].utilization_percent, Some(0.0));
    assert!(
        snapshot
            .windows
            .iter()
            .all(|w| w.limit == QuotaLimit::Unknown)
    );
    assert_eq!(snapshot.spend[0].amount.as_str(), "0.75");
    let requests = server.await.unwrap();
    assert!(requests[0].starts_with("GET /api/usage-summary "));
    assert!(requests[0].contains("cookie: WorkosCursorSessionToken=private"));
    assert!(!requests[0].contains("authorization:"));
}

#[cfg(feature = "cursor")]
#[tokio::test]
async fn cursor_rejects_empty_usage_and_warns_on_malformed_optional_amounts() {
    for body in [
        json!({}),
        json!({"individualUsage":{"plan":{"totalPercentUsed":null}}}),
    ] {
        let (base, server) = server(vec![Reply::json(body)]).await;
        assert!(matches!(
            builder(&base, ProviderId::Cursor)
                .build()
                .unwrap()
                .fetch(FetchRequest::new(
                    ProviderId::Cursor,
                    Credentials::cookie("session=private")
                ))
                .await,
            Err(AgentStatError::InvalidResponse(_))
        ));
        server.await.unwrap();
    }
    let (base, server) = server(vec![Reply::json(json!({
        "planUsage":{"totalPercentUsed":0,"totalSpend":-1,"limit":100},
        "billingCycleEnd":"invalid"
    }))])
    .await;
    let snapshot = builder(&base, ProviderId::Cursor)
        .build()
        .unwrap()
        .fetch(FetchRequest::new(
            ProviderId::Cursor,
            Credentials::bearer("private"),
        ))
        .await
        .unwrap();
    assert_eq!(snapshot.windows.len(), 1);
    assert_eq!(snapshot.windows[0].resets_at, None);
    assert!(snapshot.warnings.contains(&Warning::MalformedOptionalData));
    server.await.unwrap();
}

#[cfg(feature = "cursor")]
#[tokio::test]
async fn cursor_keeps_overages_and_negative_remaining_allowance() {
    let (base, server) = server(vec![Reply::json(json!({"planUsage": {
        "totalSpend":2500, "limit":2000, "remaining":-500
    }}))])
    .await;
    let snapshot = builder(&base, ProviderId::Cursor)
        .build()
        .unwrap()
        .fetch(FetchRequest::new(
            ProviderId::Cursor,
            Credentials::bearer("private"),
        ))
        .await
        .unwrap();
    assert_eq!(
        snapshot.windows[0].remaining.as_ref().unwrap().as_str(),
        "-5"
    );
    assert_eq!(snapshot.windows[0].utilization_percent, Some(125.0));
    server.await.unwrap();
}

#[cfg(all(feature = "cursor", feature = "local-files"))]
#[tokio::test]
async fn cursor_local_cli_token_is_used_by_the_public_fetch_api() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("auth.json"),
        r#"{"accessToken":"local-secret"}"#,
    )
    .unwrap();
    let (base, server) = server(vec![Reply::json(
        json!({"planUsage":{"totalPercentUsed":15}}),
    )])
    .await;
    let client = builder(&base, ProviderId::Cursor)
        .allow_local_files(true)
        .build()
        .unwrap();
    let request = client
        .load_local_credentials(
            ProviderId::Cursor,
            agentstat::local::CredentialLocation::Directory(dir.path().into()),
        )
        .await
        .unwrap();
    let snapshot = client.fetch(request).await.unwrap();
    assert_eq!(snapshot.credential_source, CredentialSource::LocalFile);
    assert_eq!(snapshot.windows[0].utilization_percent, Some(15.0));
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("local-secret")
    );
    assert!(server.await.unwrap()[0].contains("authorization: Bearer local-secret"));
}

#[cfg(feature = "antigravity")]
#[tokio::test]
async fn antigravity_reports_group_windows_without_inventing_token_allowances() {
    let (base, server) = server(vec![Reply::json(json!({"groups":[
        {"displayName":"Gemini Models", "buckets":[
            {"bucketId":"gemini-5h","remainingFraction":0.75,"resetTime":"2026-10-23T01:00:00Z"},
            {"bucketId":"gemini-weekly","remainingFraction":0,"remainingAmount":"0"},
            {"bucketId":"disabled-5h"},
            {"bucketId":"malformed-weekly","remainingFraction":1.5}
        ]}
    ]}))])
    .await;
    let snapshot = builder(&base, ProviderId::Antigravity)
        .build()
        .unwrap()
        .fetch(
            FetchRequest::new(ProviderId::Antigravity, Credentials::bearer("private")).config(
                ProviderConfig::Antigravity {
                    project_id: "project-a".into(),
                },
            ),
        )
        .await
        .unwrap();
    assert_eq!(snapshot.windows.len(), 2);
    assert_eq!(snapshot.windows[0].model.as_deref(), Some("Gemini Models"));
    assert_eq!(snapshot.windows[0].duration_seconds, Some(18000));
    assert_eq!(snapshot.windows[1].duration_seconds, Some(604800));
    assert_eq!(snapshot.windows[0].utilization_percent, Some(25.0));
    assert_eq!(snapshot.windows[1].utilization_percent, Some(100.0));
    assert!(
        snapshot
            .windows
            .iter()
            .all(|w| w.used.is_none() && w.limit == QuotaLimit::Unknown)
    );
    assert!(snapshot.warnings.contains(&Warning::IncompleteData));
    assert!(snapshot.warnings.contains(&Warning::MalformedOptionalData));
    let requests = server.await.unwrap();
    assert!(requests[0].contains("user-agent: antigravity"));
    assert!(requests[0].ends_with("{\"project\":\"project-a\"}"));
}

#[cfg(feature = "antigravity")]
#[tokio::test]
async fn antigravity_rejects_empty_duplicate_and_availability_only_data() {
    for body in [
        json!({}),
        json!({"groups": []}),
        json!({"models":{"gemini":{}}}),
        json!({"buckets":[{"bucketId":"duplicate","remainingFraction":1},
            {"bucketId":"duplicate","remainingFraction":0.5}]}),
    ] {
        let (base, server) = server(vec![Reply::json(body)]).await;
        assert!(matches!(
            builder(&base, ProviderId::Antigravity)
                .build()
                .unwrap()
                .fetch(FetchRequest::new(
                    ProviderId::Antigravity,
                    Credentials::bearer("private")
                ))
                .await,
            Err(AgentStatError::InvalidResponse(_))
        ));
        server.await.unwrap();
    }
}

#[cfg(all(feature = "cursor", feature = "antigravity"))]
#[tokio::test]
async fn cursor_and_antigravity_do_not_retry_or_refresh_on_http_errors() {
    for provider in [ProviderId::Cursor, ProviderId::Antigravity] {
        for status in [401, 403, 429, 302, 500] {
            let (base, server) = server(vec![Reply {
                status,
                body: "private".into(),
                headers: vec![],
            }])
            .await;
            let error = builder(&base, provider)
                .build()
                .unwrap()
                .fetch(FetchRequest::new(provider, Credentials::bearer("private")))
                .await
                .unwrap_err();
            assert!(!format!("{error:?}").contains("private"));
            assert!(matches!(
                (status, error),
                (401, AgentStatError::Authentication)
                    | (403, AgentStatError::PermissionDenied)
                    | (429, AgentStatError::RateLimited { .. })
                    | (302 | 500, AgentStatError::Http { .. })
            ));
            assert_eq!(server.await.unwrap().len(), 1);
        }
    }
}
#[tokio::test]
async fn all_implemented_remote_providers_have_request_and_normalization_fixtures() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("fixtures/providers.json")).unwrap();
    for descriptor in provider_catalog()
        .into_iter()
        .filter(|p| p.status == CoverageStatus::Implemented && p.id != ProviderId::CodeRabbit)
    {
        assert!(
            cases
                .iter()
                .any(|c| c["provider"] == descriptor.id.as_str()),
            "missing fixture for {}",
            descriptor.id
        );
    }
    for case in cases {
        let provider = ProviderId::from_str(case["provider"].as_str().unwrap()).unwrap();
        if !provider.descriptor().enabled {
            continue;
        }
        let (base, server) = server(vec![Reply::json(case["body"].clone())]).await;
        let client = builder(&base, provider)
            .timeout(Duration::from_secs(2))
            .http_client_builder(reqwest::Client::builder().no_proxy())
            .build()
            .unwrap();
        let credentials = match case["credential"].as_str() {
            Some("cookie") => {
                Credentials::cookie(case["cookie"].as_str().unwrap_or("sessionKey=test-secret"))
            }
            Some("api_key") => Credentials::api_key("test-secret"),
            _ => Credentials::bearer("test-secret"),
        };
        let snapshot = client
            .fetch(
                FetchRequest::new(provider, credentials)
                    .config(config(&case["config"]))
                    .scope(if case["scope"].is_null() {
                        AccountScope::default()
                    } else {
                        serde_json::from_value(case["scope"].clone()).unwrap()
                    }),
            )
            .await
            .unwrap_or_else(|e| panic!("{provider}: {e:?}"));
        assert_eq!(
            snapshot.windows.len(),
            case["windows"].as_u64().unwrap() as usize,
            "{provider}"
        );
        if let Some(percent) = case["percent"].as_f64() {
            assert!(
                (snapshot.windows[0].utilization_percent.unwrap() - percent).abs() < 1e-8,
                "{provider}"
            );
        }
        for (i, amount) in case["balances"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
        {
            assert_eq!(
                snapshot.balances[i].amount.to_string(),
                amount.as_str().unwrap(),
                "{provider}"
            )
        }
        for (i, amount) in case["spend"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
        {
            assert_eq!(
                snapshot.spend[i].amount.to_string(),
                amount.as_str().unwrap(),
                "{provider}"
            )
        }
        let output = serde_json::to_string(&snapshot).unwrap();
        assert!(!output.contains("test-secret"), "{provider}");
        let requests = tokio::time::timeout(
            Duration::from_secs(2),
            std::future::IntoFuture::into_future(server),
        )
        .await
        .unwrap()
        .unwrap();
        let request = &requests[0];
        let method = case["method"].as_str().unwrap_or("GET");
        let prefix = format!("{method} {}", case["path"].as_str().unwrap());
        assert!(request.starts_with(&prefix), "{provider}: {request}");
        let lower = request.to_lowercase();
        let expected_auth = match provider {
            ProviderId::Copilot => "authorization: token test-secret",
            ProviderId::OpenCode => "cookie: __host-console_session=test-secret",
            ProviderId::ElevenLabs => "xi-api-key: test-secret",
            ProviderId::Exa => "x-api-key: test-secret",
            ProviderId::Fal => "authorization: key test-secret",
            _ if case["credential"] == "cookie" => "cookie: sessionkey=test-secret",
            _ => "authorization: bearer test-secret",
        };
        assert!(
            lower.contains(expected_auth),
            "{provider}: wrong auth header"
        );
        if provider == ProviderId::Claude {
            assert!(lower.contains("anthropic-beta: oauth-2025-04-20"))
        }
        if matches!(
            provider,
            ProviderId::Tavily | ProviderId::GitKraken | ProviderId::Copilot | ProviderId::DevPass
        ) {
            assert!(
                snapshot
                    .windows
                    .iter()
                    .any(|w| w.limit == QuotaLimit::Unlimited)
            )
        }
    }
}

#[cfg(feature = "codex")]
#[tokio::test]
async fn http_failures_are_typed_redacted_and_never_retried() {
    for status in [401, 403, 429, 500, 302] {
        let (base, server) = server(vec![Reply {
            status,
            body: "test-secret".into(),
            headers: vec![
                ("Retry-After".into(), "120".into()),
                ("Location".into(), "http://127.0.0.1:1/secret".into()),
            ],
        }])
        .await;
        let client = builder(&base, ProviderId::Codex).build().unwrap();
        let err = client
            .fetch(FetchRequest::new(
                ProviderId::Codex,
                Credentials::bearer("test-secret"),
            ))
            .await
            .unwrap_err();
        assert!(!format!("{err} {err:?}").contains("test-secret"));
        match status {
            401 => assert!(matches!(err, AgentStatError::Authentication)),
            403 => assert!(matches!(err, AgentStatError::PermissionDenied)),
            429 => assert!(matches!(
                err,
                AgentStatError::RateLimited {
                    retry_after: Some(_)
                }
            )),
            _ => assert!(matches!(err,AgentStatError::Http{status:s} if s==status)),
        }
        assert_eq!(server.await.unwrap().len(), 1);
    }
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn codex_optional_failure_preserves_primary_and_scope_header() {
    let body = json!({"rate_limit":{"primary_window":{"used_percent":37,"limit_window_seconds":18000,"reset_at":1800000000}}});
    let (base, server) = server(vec![
        Reply::json(body),
        Reply {
            status: 500,
            body: "private".into(),
            headers: vec![],
        },
    ])
    .await;
    let client = builder(&base, ProviderId::Codex).build().unwrap();
    let request = FetchRequest::new(ProviderId::Codex, Credentials::bearer("private"))
        .scope(AccountScope {
            account_id: Some("workspace-a".into()),
            ..Default::default()
        })
        .include_optional(true);
    let result = client.fetch(request).await.unwrap();
    assert_eq!(result.windows[0].utilization_percent, Some(37.0));
    assert_eq!(result.warnings, vec![Warning::OptionalFetchFailed]);
    for request in server.await.unwrap() {
        assert!(
            request
                .to_lowercase()
                .contains("chatgpt-account-id: workspace-a")
        )
    }
}
#[cfg(feature = "claude")]
#[tokio::test]
async fn claude_cookie_source_uses_explicit_organization() {
    let (base, server) = server(vec![Reply::json(json!({"five_hour":{"utilization":7}}))]).await;
    let client = builder(&base, ProviderId::Claude).build().unwrap();
    let result = client
        .fetch(
            FetchRequest::new(
                ProviderId::Claude,
                Credentials::cookie("sessionKey=private"),
            )
            .source(Source::Web)
            .config(ProviderConfig::Claude {
                organization_id: "org-a".into(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(result.source, Source::Web);
    assert_eq!(result.scope.organization_id.as_deref(), Some("org-a"));
    assert!(server.await.unwrap()[0].starts_with("GET /api/organizations/org-a/usage "));
}
#[cfg(feature = "claude")]
#[tokio::test]
async fn claude_admin_costs_use_cents_and_api_key_headers() {
    let (base, server) = server(vec![Reply::json(
        json!({"data":[{"results":[{"amount":"12.34","currency":"USD"}]}],"has_more":false}),
    )])
    .await;
    let client = builder(&base, ProviderId::Claude).build().unwrap();
    let result = client
        .fetch(FetchRequest::new(
            ProviderId::Claude,
            Credentials::api_key("private"),
        ))
        .await
        .unwrap();
    assert_eq!(result.spend[0].amount.to_string(), "0.1234");
    let request = server.await.unwrap().remove(0).to_lowercase();
    assert!(request.contains("x-api-key: private"));
    assert!(request.contains("anthropic-version: 2023-06-01"));
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn malformed_responses_and_body_size_limit() {
    for (body, max, large) in [
        ("{}", 1024, false),
        ("{\"unexpected\":1234567890}", 4, true),
        ("null", 1024, false),
        ("test-secret", 1024, false),
    ] {
        let (base, server) = server(vec![Reply {
            status: 200,
            body: body.into(),
            headers: vec![],
        }])
        .await;
        let client = builder(&base, ProviderId::Codex)
            .max_response_bytes(max)
            .build()
            .unwrap();
        let err = client
            .fetch(FetchRequest::new(
                ProviderId::Codex,
                Credentials::bearer("test-secret"),
            ))
            .await
            .unwrap_err();
        if large {
            assert!(matches!(err, AgentStatError::ResponseTooLarge))
        } else {
            assert!(matches!(err, AgentStatError::InvalidResponse(_)))
        }
        server.await.unwrap();
    }
}
#[test]
fn catalog_covers_all_providers_and_manifest_matches() {
    let manifest: Value = serde_json::from_str(include_str!("../docs/coverage.json")).unwrap();
    assert_eq!(ProviderId::ALL.len(), 106);
    assert_eq!(manifest["providers"].as_array().unwrap().len(), 106);
    for descriptor in provider_catalog() {
        let m = manifest["providers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == descriptor.id.as_str())
            .unwrap();
        assert_eq!(m["name"], descriptor.name);
        assert_eq!(m["reason"], descriptor.reason);
        assert_eq!(
            serde_json::to_value(descriptor.status).unwrap(),
            m["status"]
        );
    }
}
#[test]
fn endpoint_validation_and_secret_debug() {
    for url in [
        "http://example.com",
        "https://user:private@example.com",
        "https://example.com/#private",
        "https://example.com/?private=1",
    ] {
        assert!(
            Client::builder()
                .endpoint_override(ProviderId::Codex, url)
                .build()
                .is_err()
        )
    }
    let request = FetchRequest::new(ProviderId::Codex, Credentials::bearer("private"));
    assert!(!format!("{request:?}").contains("private"));
    assert!(Client::builder().concurrency(0).build().is_err());
    assert!(Client::builder().timeout(Duration::ZERO).build().is_err());
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn explicit_source_is_not_silently_changed() {
    let client = Client::builder().build().unwrap();
    assert!(matches!(
        client
            .fetch(
                FetchRequest::new(ProviderId::Codex, Credentials::bearer("private"))
                    .source(Source::Web)
            )
            .await,
        Err(AgentStatError::UnsupportedSource(_))
    ));
}
#[tokio::test]
async fn disabled_and_unimplemented_providers_fail_explicitly() {
    let client = Client::builder().build().unwrap();
    for descriptor in provider_catalog() {
        if descriptor.status != CoverageStatus::Implemented || !descriptor.enabled {
            let error = client
                .fetch(FetchRequest::new(descriptor.id, Credentials::None))
                .await
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    AgentStatError::FeatureDisabled(_) | AgentStatError::Unsupported { .. }
                ),
                "{}: {error}",
                descriptor.id
            );
        }
    }
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn zero_missing_overages_and_malformed_optional_windows_are_preserved() {
    let body = json!({"plan_type":"future-plan","rate_limit":{"primary_window":{"used_percent":125,"limit_window_seconds":18000,"reset_at":1800000000},"secondary_window":"broken"},"additional_rate_limits":[null,{"rate_limit":{"primary_window":{"used_percent":0}}}],"credits":{"balance":"0"}});
    let (base, server) = server(vec![Reply::json(body)]).await;
    let s = builder(&base, ProviderId::Codex)
        .build()
        .unwrap()
        .fetch(FetchRequest::new(
            ProviderId::Codex,
            Credentials::bearer("private"),
        ))
        .await
        .unwrap();
    assert_eq!(s.windows[0].utilization_percent, Some(125.0));
    assert_eq!(s.windows[1].utilization_percent, Some(0.0));
    assert_eq!(s.windows[1].resets_at, None);
    assert_eq!(s.windows[1].limit, QuotaLimit::Unknown);
    assert_eq!(s.balances[0].amount.to_string(), "0");
    assert!(!s.warnings.is_empty());
    server.await.unwrap();
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn batch_preserves_order_and_account_failures() {
    let (base, server) = server(vec![
        Reply {
            status: 401,
            body: String::new(),
            headers: vec![],
        },
        Reply::json(json!({"rate_limit":{"primary_window":{"used_percent":20}},"account_id":"b"})),
    ])
    .await;
    let client = builder(&base, ProviderId::Codex)
        .concurrency(1)
        .build()
        .unwrap();
    let requests = ["a", "b"].into_iter().map(|id| {
        FetchRequest::new(ProviderId::Codex, Credentials::bearer("private")).scope(AccountScope {
            account_id: Some(id.into()),
            ..Default::default()
        })
    });
    let result = client.fetch_many(requests).await;
    assert!(matches!(result[0], Err(AgentStatError::Authentication)));
    assert_eq!(
        result[1].as_ref().unwrap().scope.account_id.as_deref(),
        Some("b")
    );
    server.await.unwrap();
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn account_mismatch_fails_instead_of_relabeling_data() {
    let (base, server) = server(vec![Reply::json(
        json!({"account_id":"b","rate_limit":{"primary_window":{"used_percent":20}}}),
    )])
    .await;
    let client = builder(&base, ProviderId::Codex).build().unwrap();
    let request =
        FetchRequest::new(ProviderId::Codex, Credentials::bearer("private")).scope(AccountScope {
            account_id: Some("a".into()),
            ..Default::default()
        });
    assert!(matches!(
        client.fetch(request).await,
        Err(AgentStatError::InvalidResponse(_))
    ));
    server.await.unwrap();
}
#[cfg(feature = "codex")]
#[derive(Debug)]
struct Hanging;
#[cfg(feature = "codex")]
impl HttpExecutor for Hanging {
    fn execute(&self, _: reqwest::Request) -> BoxFuture<'_, Result<HttpResponse>> {
        Box::pin(std::future::pending())
    }
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn request_deadline_cancels_transport() {
    let client = Client::builder()
        .http_executor(Arc::new(Hanging))
        .timeout(Duration::from_millis(5))
        .build()
        .unwrap();
    assert!(matches!(
        client
            .fetch(FetchRequest::new(
                ProviderId::Codex,
                Credentials::bearer("private")
            ))
            .await,
        Err(AgentStatError::Timeout)
    ));
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn batch_concurrency_is_bounded() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Debug)]
    struct Concurrent {
        active: AtomicUsize,
        peak: AtomicUsize,
    }
    impl HttpExecutor for Concurrent {
        fn execute(&self, _: reqwest::Request) -> BoxFuture<'_, Result<HttpResponse>> {
            Box::pin(async move {
                let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
                self.peak.fetch_max(active, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(5)).await;
                self.active.fetch_sub(1, Ordering::SeqCst);
                Ok(HttpResponse {
                    status: 200,
                    headers: HeaderMap::new(),
                    body: br#"{"rate_limit":{"primary_window":{"used_percent":10}}}"#.to_vec(),
                })
            })
        }
    }
    let executor = Arc::new(Concurrent {
        active: AtomicUsize::new(0),
        peak: AtomicUsize::new(0),
    });
    let client = Client::builder()
        .http_executor(executor.clone())
        .concurrency(2)
        .build()
        .unwrap();
    let results = client
        .fetch_many(
            (0..7).map(|_| FetchRequest::new(ProviderId::Codex, Credentials::bearer("private"))),
        )
        .await;
    assert!(results.iter().all(std::result::Result::is_ok));
    assert_eq!(executor.peak.load(Ordering::SeqCst), 2);
}
#[cfg(feature = "synthetic")]
#[tokio::test]
async fn synthetic_named_quota_slots_have_real_durations() {
    let (base,server)=server(vec![Reply::json(json!({"rollingFiveHourLimit":{"requests":25,"limit":100},"weeklyTokenLimit":{"used":40,"limit":100},"search":{"hourly":{"used":2,"limit":10}}}))]).await;
    let result = builder(&base, ProviderId::Synthetic)
        .build()
        .unwrap()
        .fetch(FetchRequest::new(
            ProviderId::Synthetic,
            Credentials::api_key("private"),
        ))
        .await
        .unwrap();
    assert_eq!(
        result
            .windows
            .iter()
            .map(|w| w.duration_seconds)
            .collect::<Vec<_>>(),
        vec![Some(18000), Some(604800), Some(3600)]
    );
    server.await.unwrap();
}
#[cfg(feature = "openai")]
#[tokio::test]
async fn admin_pagination_does_not_merge_currencies_or_replay_cursors() {
    let (base,server)=server(vec![Reply::json(json!({"data":[{"results":[{"amount":{"value":"0.1","currency":"usd"}}]}],"has_more":true,"next_page":"page-two"})),Reply::json(json!({"data":[{"results":[{"amount":{"value":"0.2","currency":"usd"}},{"amount":{"value":"1.25","currency":"eur"}}]}],"has_more":false}))]).await;
    let result = builder(&base, ProviderId::OpenAI)
        .build()
        .unwrap()
        .fetch(FetchRequest::new(
            ProviderId::OpenAI,
            Credentials::api_key("private"),
        ))
        .await
        .unwrap();
    assert_eq!(result.spend.len(), 2);
    assert!(
        result
            .spend
            .iter()
            .any(|s| s.currency == "USD" && s.amount.as_str() == "0.3")
    );
    assert!(
        result
            .spend
            .iter()
            .any(|s| s.currency == "EUR" && s.amount.as_str() == "1.25")
    );
    assert!(server.await.unwrap()[1].contains("page=page-two"));
}

#[cfg(feature = "opencode")]
mod opencode {
    use super::*;

    fn request(workspace: Option<&str>) -> FetchRequest {
        FetchRequest::new(
            ProviderId::OpenCode,
            Credentials::cookie("__Host-console_session=test-secret"),
        )
        .source(Source::Web)
        .scope(AccountScope {
            workspace_id: workspace.map(str::to_owned),
            ..Default::default()
        })
    }
    fn quota() -> Value {
        json!({"access":{"endsAt":"2026-11-01T00:00:00Z","meters":{
            "fiveHour":{"usedMicroCents":"300000000","limitMicroCents":"1200000000","resetsAt":"2026-10-23T03:00:00Z"},
            "week":{"usedMicroCents":"1200000000","limitMicroCents":"3000000000"},
            "month":{"usedMicroCents":"600000000","limitMicroCents":"6000000000"}
        }}})
    }
    fn payg_replies(balance: Value, total: Value) -> Vec<Reply> {
        vec![
            Reply::json(Value::Null),
            Reply::json(json!({"id":"wrk_A","hasGoSubscription":false})),
            Reply::json(
                json!({"mode":"pay-as-you-go","billingMode":"prepaid","balanceMicroCents":balance}),
            ),
            Reply::json(json!({"totalCostMicroCents":total})),
        ]
    }

    #[tokio::test]
    async fn discovers_unique_workspace_and_scopes_all_usage_requests() {
        let (base, server) = server(vec![
            Reply::json(json!([{"id":"wrk_A"}])),
            Reply::json(quota()),
        ])
        .await;
        let snapshot = builder(&base, ProviderId::OpenCode)
            .build()
            .unwrap()
            .fetch(request(None))
            .await
            .unwrap();
        assert_eq!(snapshot.scope.workspace_id.as_deref(), Some("wrk_A"));
        assert_eq!(snapshot.source, Source::Web);
        assert_eq!(
            snapshot
                .windows
                .iter()
                .map(|w| w.utilization_percent)
                .collect::<Vec<_>>(),
            vec![Some(25.0), Some(40.0), Some(10.0)]
        );
        assert_eq!(snapshot.windows[0].used.as_ref().unwrap().as_str(), "3");
        assert_eq!(
            snapshot.windows[0].limit,
            QuotaLimit::Known("12".parse().unwrap())
        );
        assert_eq!(
            snapshot.windows[0].remaining.as_ref().unwrap().as_str(),
            "9"
        );
        assert_eq!(snapshot.windows[0].duration_seconds, Some(18000));
        assert_eq!(snapshot.windows[1].duration_seconds, Some(604800));
        assert_eq!(snapshot.windows[1].resets_at, None);
        assert_eq!(snapshot.windows[2].kind, WindowKind::Calendar);
        assert_eq!(
            snapshot.windows[2].resets_at.unwrap().to_rfc3339(),
            "2026-11-01T00:00:00+00:00"
        );
        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("GET /console/api/orgs "));
        assert!(!requests[0].contains("x-org-id:"));
        assert!(requests[1].contains("x-org-id: wrk_A"));
        for request in requests {
            assert!(request.contains("cookie: __Host-console_session=test-secret"));
            assert!(!request.contains("authorization:"));
        }
    }

    #[tokio::test]
    async fn discovery_never_selects_arbitrary_workspace() {
        for body in [
            json!([]),
            json!([{"id":"not-a-workspace"}]),
            json!([{"id":"wrk_A"},{"id":"org_B"}]),
            json!({"error":"unexpected"}),
        ] {
            let (base, server) = server(vec![Reply::json(body)]).await;
            let result = builder(&base, ProviderId::OpenCode)
                .build()
                .unwrap()
                .fetch(request(None))
                .await;
            assert!(matches!(
                result,
                Err(AgentStatError::Configuration(_) | AgentStatError::InvalidResponse(_))
            ));
            assert_eq!(server.await.unwrap().len(), 1);
        }
    }

    #[tokio::test]
    async fn explicit_workspace_skips_discovery_and_invalid_selection_fails_locally() {
        let (base, server) = server(vec![Reply::json(quota())]).await;
        builder(&base, ProviderId::OpenCode)
            .build()
            .unwrap()
            .fetch(request(Some("org_SELECTED")))
            .await
            .unwrap();
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /console/api/go/status "));
        assert!(requests[0].contains("x-org-id: org_SELECTED"));
        let client = Client::builder().build().unwrap();
        for id in [
            "",
            "wrk_",
            "../wrk_A",
            "https://opencode.ai/console/wrk_A",
            "wrk_A\n",
        ] {
            assert!(matches!(
                client.fetch(request(Some(id))).await,
                Err(AgentStatError::Configuration(_))
            ));
        }
        assert!(matches!(
            client
                .fetch(FetchRequest::new(
                    ProviderId::OpenCode,
                    Credentials::api_key("private")
                ))
                .await,
            Err(AgentStatError::InvalidCredentials(_))
        ));
        assert!(matches!(
            client
                .fetch(request(Some("wrk_A")).source(Source::Api))
                .await,
            Err(AgentStatError::UnsupportedSource(_))
        ));
    }

    #[tokio::test]
    async fn missing_optional_meters_stay_missing_and_overages_survive() {
        let body = json!({"access":{"meters":{"fiveHour":{"usedMicroCents":"1500000000","limitMicroCents":"1200000000"},"week":{"usedMicroCents":"invalid","limitMicroCents":"100"}}}});
        let (base, server) = server(vec![Reply::json(body)]).await;
        let snapshot = builder(&base, ProviderId::OpenCode)
            .build()
            .unwrap()
            .fetch(request(Some("wrk_A")))
            .await
            .unwrap();
        assert_eq!(snapshot.windows.len(), 1);
        assert_eq!(snapshot.windows[0].utilization_percent, Some(125.0));
        assert_eq!(
            snapshot.windows[0].remaining.as_ref().unwrap().as_str(),
            "-3"
        );
        assert_eq!(snapshot.windows[0].resets_at, None);
        assert_eq!(snapshot.warnings, vec![Warning::MalformedOptionalData]);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn zero_limits_do_not_become_unlimited() {
        let (base, server) = server(vec![Reply::json(
            json!({"access":{"meters":{"fiveHour":{"usedMicroCents":0,"limitMicroCents":"0"}}}}),
        )])
        .await;
        let snapshot = builder(&base, ProviderId::OpenCode)
            .build()
            .unwrap()
            .fetch(request(Some("wrk_A")))
            .await
            .unwrap();
        assert_eq!(
            snapshot.windows[0].limit,
            QuotaLimit::Known(Decimal::from(0))
        );
        assert_eq!(snapshot.windows[0].utilization_percent, None);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn confirmed_payg_preserves_exact_spend_balance_and_workspace() {
        let (base, server) = server(payg_replies(json!("1250000000"), json!("325000000"))).await;
        let snapshot = builder(&base, ProviderId::OpenCode)
            .build()
            .unwrap()
            .fetch(request(Some("wrk_A")))
            .await
            .unwrap();
        assert!(snapshot.windows.is_empty());
        assert_eq!(snapshot.spend[0].id, "last-30-days");
        assert_eq!(snapshot.spend[0].amount.as_str(), "3.25");
        assert_eq!(snapshot.balances[0].amount.as_str(), "12.5");
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[3].starts_with("GET /console/api/usage/summary?range=30d "));
        for request in requests {
            assert!(request.starts_with("GET "));
            assert!(request.contains("x-org-id: wrk_A"));
        }
    }

    #[tokio::test]
    async fn optional_balance_failure_preserves_spend_and_negative_balance_is_valid() {
        for balance in [Value::Null, json!("invalid"), json!("-1250000000")] {
            let (base, server) = server(payg_replies(
                balance.clone(),
                json!("900719925474099312345678"),
            ))
            .await;
            let snapshot = builder(&base, ProviderId::OpenCode)
                .build()
                .unwrap()
                .fetch(request(Some("wrk_A")))
                .await
                .unwrap();
            assert_eq!(
                snapshot.spend[0].amount.as_str(),
                "9007199254740993.12345678"
            );
            if balance == "-1250000000" {
                assert_eq!(snapshot.balances[0].amount.as_str(), "-12.5")
            } else {
                assert!(snapshot.balances.is_empty());
                assert!(!snapshot.warnings.is_empty());
            }
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn broken_subscription_never_falls_back_to_spend() {
        for body in [
            json!({}),
            json!({"access":{"meters":{}}}),
            json!({"access":false}),
            json!({"access":{"meters":{"fiveHour":{"usedMicroCents":true,"limitMicroCents":100}}}}),
        ] {
            let (base, server) = server(vec![Reply::json(body)]).await;
            assert!(matches!(
                builder(&base, ProviderId::OpenCode)
                    .build()
                    .unwrap()
                    .fetch(request(Some("wrk_A")))
                    .await,
                Err(AgentStatError::InvalidResponse(_))
            ));
            assert_eq!(server.await.unwrap().len(), 1);
        }
        for organization in [
            json!({"hasGoSubscription":true}),
            json!({"hasGoSubscription":false,"id":"wrk_B"}),
            json!({}),
        ] {
            let (base, server) = server(vec![
                Reply::json(json!({"access":null})),
                Reply::json(organization),
            ])
            .await;
            assert!(matches!(
                builder(&base, ProviderId::OpenCode)
                    .build()
                    .unwrap()
                    .fetch(request(Some("wrk_A")))
                    .await,
                Err(AgentStatError::InvalidResponse(_))
            ));
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn invalid_microcent_totals_fail_instead_of_becoming_zero() {
        for total in [
            json!(true),
            json!(-1),
            json!(1.5),
            json!("NaN"),
            Value::Null,
            json!("-1"),
            json!("1.5"),
        ] {
            let (base, server) = server(payg_replies(json!("0"), total)).await;
            assert!(matches!(
                builder(&base, ProviderId::OpenCode)
                    .build()
                    .unwrap()
                    .fetch(request(Some("wrk_A")))
                    .await,
                Err(AgentStatError::InvalidResponse(_))
            ));
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn provider_errors_stop_without_retry_or_legacy_fallback() {
        for status in [401, 403, 429, 500] {
            let (base, server) = server(vec![Reply {
                status,
                body: "private-token".into(),
                headers: vec![],
            }])
            .await;
            let error = builder(&base, ProviderId::OpenCode)
                .build()
                .unwrap()
                .fetch(request(Some("wrk_A")))
                .await
                .unwrap_err();
            match status {
                401 => assert!(matches!(error, AgentStatError::Authentication)),
                403 => assert!(matches!(error, AgentStatError::PermissionDenied)),
                429 => assert!(matches!(error, AgentStatError::RateLimited { .. })),
                _ => assert!(matches!(error, AgentStatError::Http { status: 500 })),
            }
            assert!(!format!("{error:?}").contains("private-token"));
            assert_eq!(server.await.unwrap().len(), 1);
        }
    }
}
