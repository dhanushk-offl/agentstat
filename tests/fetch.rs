use agentstat::*;
use futures::future::BoxFuture;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
#[cfg(any(
    feature = "codex",
    feature = "claude",
    feature = "synthetic",
    feature = "openai"
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
            Some("cookie") => Credentials::cookie("sessionKey=test-secret"),
            Some("api_key") => Credentials::api_key("test-secret"),
            _ => Credentials::bearer("test-secret"),
        };
        let snapshot = client
            .fetch(FetchRequest::new(provider, credentials).config(config(&case["config"])))
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
fn catalog_covers_all_pinned_providers_and_manifest_matches() {
    let manifest: Value = serde_json::from_str(include_str!("../docs/coverage.json")).unwrap();
    assert_eq!(manifest["upstream_revision"], UPSTREAM_REVISION);
    assert_eq!(ProviderId::ALL.len(), 106);
    assert_eq!(manifest["providers"].as_array().unwrap().len(), 106);
    for descriptor in provider_catalog() {
        let m = manifest["providers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == descriptor.id.as_str())
            .unwrap();
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
