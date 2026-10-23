//! Opt-in checks against real OpenCode accounts. These never run in ordinary CI.
//! Set the credentials described in docs/development.md before running them.

#[cfg(any(feature = "opencode", feature = "opencodego"))]
use agentstat::{Client, Credentials, FetchRequest, ProviderId, Source};

#[cfg(feature = "opencodego")]
#[tokio::test]
#[ignore = "requires OPENCODE_GO_API_KEY and a live OpenCode Go account"]
async fn opencode_go_live_quota_windows() {
    let key = std::env::var("OPENCODE_GO_API_KEY")
        .expect("set OPENCODE_GO_API_KEY before running this live test");
    assert!(!key.trim().is_empty(), "OPENCODE_GO_API_KEY is empty");

    let snapshot = Client::builder()
        .build()
        .unwrap()
        .fetch(FetchRequest::new(
            ProviderId::OpenCodeGo,
            Credentials::api_key(key),
        ))
        .await
        .expect("live OpenCode Go usage fetch failed");

    assert_eq!(snapshot.provider, ProviderId::OpenCodeGo);
    assert_eq!(snapshot.source, Source::Api);
    assert_eq!(snapshot.windows.len(), 3);
    for id in ["five-hour", "weekly", "monthly"] {
        let window = snapshot
            .windows
            .iter()
            .find(|window| window.id == id)
            .expect("missing OpenCode Go quota window");
        assert!(
            window
                .utilization_percent
                .is_some_and(|percent| (0.0..=100.0).contains(&percent)),
            "invalid {id} utilization"
        );
        assert!(window.resets_at.is_some(), "missing {id} reset time");
    }
}

#[cfg(feature = "opencode")]
#[tokio::test]
#[ignore = "requires OPENCODE_COOKIE and a live OpenCode Zen pay-as-you-go account"]
async fn opencode_zen_live_spend() {
    use agentstat::AccountScope;

    let cookie = std::env::var("OPENCODE_COOKIE")
        .expect("set OPENCODE_COOKIE before running this live test");
    assert!(!cookie.trim().is_empty(), "OPENCODE_COOKIE is empty");
    let workspace_id = std::env::var("OPENCODE_WORKSPACE_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());

    let snapshot = Client::builder()
        .build()
        .unwrap()
        .fetch(
            FetchRequest::new(ProviderId::OpenCode, Credentials::cookie(cookie)).scope(
                AccountScope {
                    workspace_id: workspace_id.clone(),
                    ..Default::default()
                },
            ),
        )
        .await
        .expect("live OpenCode Zen usage fetch failed");

    assert_eq!(snapshot.provider, ProviderId::OpenCode);
    assert_eq!(snapshot.source, Source::Web);
    assert!(snapshot.scope.workspace_id.is_some());
    if let Some(id) = workspace_id {
        assert_eq!(snapshot.scope.workspace_id.as_deref(), Some(id.as_str()));
    }
    assert!(
        snapshot.windows.is_empty(),
        "expected a Zen pay-as-you-go account"
    );
    assert!(
        snapshot
            .spend
            .iter()
            .any(|spend| spend.id == "last-30-days" && spend.currency == "USD"),
        "missing Zen 30-day spend"
    );
}
