//! Integration tests for agentstat local.

#![cfg(feature = "local-files")]
#[cfg(any(feature = "codex", feature = "claude", feature = "cursor"))]
use agentstat::local::CredentialLocation;
#[cfg(any(
    feature = "codex",
    feature = "claude",
    feature = "cursor",
    all(
        feature = "local-probes",
        unix,
        any(
            feature = "coderabbit",
            feature = "antigravity",
            feature = "opencode-local"
        )
    )
))]
use agentstat::*;
#[cfg(any(
    feature = "codex",
    feature = "claude",
    feature = "cursor",
    all(feature = "local-probes", unix)
))]
use std::fs;
#[cfg(all(feature = "local-probes", unix))]
use std::path::{Path, PathBuf};
#[cfg(all(feature = "local-probes", unix))]
use std::time::Duration;
#[cfg(all(feature = "local-probes", unix))]
use tokio::time;

#[cfg(all(feature = "opencode-local", unix))]
mod opencode_cli_probe {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn executable(dir: &Path, output: &str) -> PathBuf {
        let path = dir.join("opencode");
        let script = format!(
            "#!/bin/sh\n[ \"$#\" = 4 ] && [ \"$1\" = stats ] && [ \"$2\" = --days ] && [ \"$3\" = 30 ] && [ \"$4\" = --json ] || exit 2\n[ \"$NO_COLOR\" = 1 ] || exit 3\n[ \"$OPENCODE_DISABLE_AUTOUPDATE\" = true ] && [ \"$OPENCODE_DISABLE_PRUNE\" = true ] && [ \"$OPENCODE_DISABLE_MODELS_FETCH\" = true ] || exit 4\ncat <<'OUTPUT'\n{output}\nOUTPUT\n"
        );
        fs::write(&path, script).expect("test fixture file should be writable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .expect("test fixture file should be writable");
        path
    }

    #[tokio::test]
    async fn local_stats_are_explicit_and_report_activity_without_quota() {
        let dir = tempfile::tempdir().unwrap();
        let path = executable(
            dir.path(),
            r#"{"range":{"from":1767205800000,"to":1792734285567},"sessions":2,"prompts":12,"steps":20,"tokens":{"input":100,"output":50,"reasoning":10,"cache":{"read":70,"write":0}},"cost":1.25,"models":[{"model":{"id":"glm-5","providerID":"opencode-go","variant":"medium"},"tokens":{"input":60,"output":20},"cost":0.75}]}"#,
        );
        let request =
            FetchRequest::new(ProviderId::OpenCode, Credentials::None).source(Source::LocalProbe);
        assert!(matches!(
            Client::builder()
                .build()
                .unwrap()
                .fetch(request.clone())
                .await,
            Err(AgentStatError::LocalAccessDisabled)
        ));
        let snapshot = Client::builder()
            .allow_local_probes(true)
            .probe_executable(ProviderId::OpenCode, path)
            .build()
            .unwrap()
            .fetch(request)
            .await
            .unwrap();
        assert_eq!(snapshot.source, Source::LocalProbe);
        assert_eq!(snapshot.credential_source, CredentialSource::LocalProbe);
        assert_eq!(snapshot.windows.len(), 10);
        assert_eq!(snapshot.windows[3].used.as_ref().unwrap().as_str(), "100");
        assert!(
            snapshot
                .windows
                .iter()
                .all(|w| matches!(w.limit, QuotaLimit::Unknown))
        );
        assert_eq!(
            snapshot.windows[8].model.as_deref(),
            Some("opencode-go/glm-5#medium")
        );
        assert_eq!(snapshot.spend[0].amount.as_str(), "1.25");
        assert_eq!(snapshot.spend[1].amount.as_str(), "0.75");
    }

    #[tokio::test]
    async fn local_stats_reject_invalid_data_and_scopes() {
        let dir = tempfile::tempdir().unwrap();
        for body in [
            "not JSON",
            r#"{"range":{"from":2,"to":1},"sessions":0}"#,
            r#"{"range":{"from":1767205800000,"to":1792734285567},"sessions":-1,"prompts":0,"steps":0,"tokens":{"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}},"cost":0}"#,
        ] {
            let path = executable(dir.path(), body);
            let error = Client::builder()
                .allow_local_probes(true)
                .probe_executable(ProviderId::OpenCode, path)
                .build()
                .unwrap()
                .fetch(
                    FetchRequest::new(ProviderId::OpenCode, Credentials::None)
                        .source(Source::LocalProbe),
                )
                .await
                .unwrap_err();
            assert!(matches!(error, AgentStatError::InvalidResponse(_)));
        }
        let error = Client::builder()
            .allow_local_probes(true)
            .build()
            .unwrap()
            .fetch(
                FetchRequest::new(ProviderId::OpenCode, Credentials::None)
                    .source(Source::LocalProbe)
                    .scope(AccountScope {
                        workspace_id: Some("wrk_example".into()),
                        ..Default::default()
                    }),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, AgentStatError::Configuration(_)));
    }

    #[tokio::test]
    async fn older_cli_text_report_is_used_when_json_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode");
        let script = r##"#!/bin/sh
[ "$1" = stats ] && [ "$2" = --days ] && [ "$3" = 30 ] || exit 2
if [ "$4" = --json ]; then
  printf 'opencode stats\nOptions: --days --models\n'
  exit 1
fi
[ "$#" = 3 ] || exit 3
cat <<'REPORT'
┌────────────────────────────────────────┐
│Sessions                            498 │
│Messages                         36,626 │
│Total Cost                       $19.10 │
│Input                            194.7M │
│Output                            11.1M │
│Cache Read                      3610.3M │
│Cache Write                           0 │
└────────────────────────────────────────┘
REPORT
"##;
        fs::write(&path, script).expect("test fixture file should be writable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .expect("test fixture file should be writable");
        let snapshot = Client::builder()
            .allow_local_probes(true)
            .probe_executable(ProviderId::OpenCode, path)
            .build()
            .unwrap()
            .fetch(
                FetchRequest::new(ProviderId::OpenCode, Credentials::None)
                    .source(Source::LocalProbe),
            )
            .await
            .unwrap();
        assert_eq!(snapshot.source, Source::LocalProbe);
        assert_eq!(snapshot.warnings, vec![Warning::IncompleteData]);
        assert_eq!(snapshot.windows[0].used.as_ref().unwrap().as_str(), "498");
        assert_eq!(snapshot.windows[1].used.as_ref().unwrap().as_str(), "36626");
        assert_eq!(
            snapshot.windows[2].used.as_ref().unwrap().as_str(),
            "194700000"
        );
        assert_eq!(
            snapshot.windows[4].used.as_ref().unwrap().as_str(),
            "3610300000"
        );
        assert_eq!(snapshot.spend[0].amount.as_str(), "19.1");
        assert!(snapshot.spend[0].starts_at.is_none());
    }
}

#[cfg(feature = "cursor")]
#[tokio::test]
async fn cursor_cli_credentials_are_opt_in_redacted_and_never_modified() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("auth.json");
    let bytes = br#"{"accessToken":"private","refreshToken":"refresh-private"}"#;
    fs::write(&path, bytes).expect("test fixture file should be writable");
    let location = CredentialLocation::Directory(directory.path().into());
    assert!(matches!(
        Client::builder()
            .build()
            .unwrap()
            .load_local_credentials(ProviderId::Cursor, location.clone())
            .await,
        Err(AgentStatError::LocalAccessDisabled)
    ));
    let client = Client::builder().allow_local_files(true).build().unwrap();
    let request = client
        .load_local_credentials(ProviderId::Cursor, location.clone())
        .await
        .unwrap();
    assert!(matches!(request.credentials, Credentials::Bearer(_)));
    assert!(!format!("{request:?}").contains("private"));
    assert_eq!(fs::read(&path).unwrap(), bytes);
    fs::write(&path, br#"{"apiKey":"private"}"#).expect("test fixture file should be writable");
    assert!(matches!(
        client
            .load_local_credentials(ProviderId::Cursor, location)
            .await,
        Err(AgentStatError::InvalidCredentials(ProviderId::Cursor))
    ));
}

#[cfg(all(feature = "local-probes", feature = "antigravity", unix))]
mod antigravity_probe {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn executable(dir: &Path, output: &str) -> PathBuf {
        let path = dir.join("agy");
        let script = format!(
            "#!/bin/sh\n[ \"$#\" = 6 ] || exit 2\n[ \"$1\" = --print ] && [ \"$2\" = /usage ] && [ \"$3\" = --output-format ] && [ \"$4\" = json ] && [ \"$5\" = --log-file ] && [ \"$6\" = /dev/null ] || exit 3\n[ \"$AGY_CLI_DISABLE_AUTO_UPDATE\" = true ] || exit 4\n[ \"$NO_COLOR\" = 1 ] || exit 5\ncat <<'OUTPUT'\n{output}\nOUTPUT\n"
        );
        fs::write(&path, script).expect("test fixture file should be writable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .expect("test fixture file should be writable");
        path
    }

    #[tokio::test]
    async fn structured_usage_probe_requires_opt_in_and_uses_fixed_read_only_arguments() {
        let dir = tempfile::tempdir().unwrap();
        let path = executable(
            dir.path(),
            r#"{"command":{"data":{"groups":[{"name":"Gemini Models","buckets":[{"id":"gemini-weekly","window":"weekly","remaining_fraction":0.6,"reset_time":"2026-10-29T00:00:00Z"},{"id":"gemini-5h","window":"5h","remaining_fraction":0.9}]}]}}}"#,
        );
        let request = FetchRequest::new(ProviderId::Antigravity, Credentials::None)
            .source(Source::LocalProbe);
        assert!(matches!(
            Client::builder()
                .build()
                .unwrap()
                .fetch(request.clone())
                .await,
            Err(AgentStatError::LocalAccessDisabled)
        ));
        let snapshot = Client::builder()
            .allow_local_probes(true)
            .probe_executable(ProviderId::Antigravity, path)
            .build()
            .unwrap()
            .fetch(request)
            .await
            .unwrap();
        assert_eq!(snapshot.source, Source::LocalProbe);
        assert_eq!(snapshot.credential_source, CredentialSource::LocalProbe);
        assert_eq!(snapshot.windows.len(), 2);
        assert_eq!(snapshot.windows[0].duration_seconds, Some(604800));
        assert_eq!(snapshot.windows[0].utilization_percent, Some(40.0));
        assert_eq!(snapshot.windows[0].model.as_deref(), Some("Gemini Models"));
        assert_eq!(
            snapshot.windows[0].resets_at.unwrap().to_rfc3339(),
            "2026-10-29T00:00:00+00:00"
        );
    }

    #[tokio::test]
    async fn agent_generated_text_and_unknown_probe_schemas_are_rejected() {
        for body in [
            r#"{"result":"You have 50% remaining"}"#,
            r#"{"command":{"data":{"groups":[]}}}"#,
            r#"{"command":{"data":{"groups":[{"buckets":[{"remaining_fraction":-1}]}]}}}"#,
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = executable(dir.path(), body);
            let err = Client::builder()
                .allow_local_probes(true)
                .probe_executable(ProviderId::Antigravity, path)
                .build()
                .unwrap()
                .fetch(
                    FetchRequest::new(ProviderId::Antigravity, Credentials::None)
                        .source(Source::LocalProbe),
                )
                .await
                .unwrap_err();
            assert!(matches!(err, AgentStatError::InvalidResponse(_)));
        }
    }
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn local_credential_reads_require_opt_in_and_preserve_files() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("auth.json");
    let bytes = br#"{"tokens":{"access_token":"private","account_id":"account-a"}}"#;
    fs::write(&path, bytes).expect("test fixture file should be writable");
    let location = CredentialLocation::Directory(directory.path().into());
    assert!(matches!(
        Client::builder()
            .build()
            .unwrap()
            .load_local_credentials(ProviderId::Codex, location.clone())
            .await,
        Err(AgentStatError::LocalAccessDisabled)
    ));
    let request = Client::builder()
        .allow_local_files(true)
        .build()
        .unwrap()
        .load_local_credentials(ProviderId::Codex, location)
        .await
        .unwrap();
    assert_eq!(request.scope.account_id.as_deref(), Some("account-a"));
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert!(!format!("{request:?}").contains("private"));
}
#[cfg(feature = "claude")]
#[tokio::test]
async fn expired_claude_credentials_are_not_refreshed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(".credentials.json");
    let bytes=br#"{"claudeAiOauth":{"accessToken":"private","refreshToken":"refresh-private","expiresAt":1}}"#;
    fs::write(&path, bytes).expect("test fixture file should be writable");
    let client = Client::builder().allow_local_files(true).build().unwrap();
    assert!(matches!(
        client
            .load_local_credentials(
                ProviderId::Claude,
                CredentialLocation::Directory(directory.path().into())
            )
            .await,
        Err(AgentStatError::Authentication)
    ));
    assert_eq!(fs::read(path).unwrap(), bytes);
}
#[cfg(feature = "codex")]
#[tokio::test]
async fn malformed_oversized_and_api_only_credentials_fail_without_leaking() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("auth.json");
    let client = Client::builder().allow_local_files(true).build().unwrap();
    for bytes in [
        br#"{"OPENAI_API_KEY":"private"}"#.to_vec(),
        vec![b'x'; 65537],
        b"private".to_vec(),
    ] {
        fs::write(&path, bytes).expect("test fixture file should be writable");
        let err = client
            .load_local_credentials(
                ProviderId::Codex,
                CredentialLocation::Directory(directory.path().into()),
            )
            .await
            .unwrap_err();
        assert!(!format!("{err} {err:?}").contains("private"));
    }
}
#[cfg(all(feature = "local-probes", feature = "coderabbit", unix))]
mod probes {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[cfg(not(target_os = "linux"))]
    use std::process::Command;
    fn executable(dir: &Path, script: &str) -> PathBuf {
        let path = dir.join("coderabbit");
        fs::write(&path, script).expect("test fixture file should be writable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .expect("test fixture file should be writable");
        path
    }
    #[tokio::test]
    async fn probe_is_opt_in_and_does_not_infer_quota() {
        let dir = tempfile::tempdir().unwrap();
        let path = executable(
            dir.path(),
            "#!/bin/sh\n[ \"$1\" = usage ] || exit 2\nprintf 'Plan: Pro\\nYour reviews: 42\\n'\n",
        );
        let request =
            FetchRequest::new(ProviderId::CodeRabbit, Credentials::None).source(Source::LocalProbe);
        assert!(matches!(
            Client::builder()
                .build()
                .unwrap()
                .fetch(request.clone())
                .await,
            Err(AgentStatError::LocalAccessDisabled)
        ));
        let client = Client::builder()
            .allow_local_probes(true)
            .probe_executable(ProviderId::CodeRabbit, path)
            .build()
            .unwrap();
        let snapshot = client.fetch(request).await.unwrap();
        assert_eq!(snapshot.windows[0].used.as_ref().unwrap().to_string(), "42");
        assert_eq!(snapshot.windows[0].limit, QuotaLimit::Unknown);
        assert_eq!(snapshot.windows[0].utilization_percent, None);
        assert_eq!(snapshot.plan.as_deref(), Some("Pro"));
    }
    #[tokio::test]
    async fn probe_output_is_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let path = executable(dir.path(), "#!/bin/sh\nhead -c 150000 /dev/zero\n");
        let client = Client::builder()
            .allow_local_probes(true)
            .probe_executable(ProviderId::CodeRabbit, path)
            .build()
            .unwrap();
        assert!(matches!(
            client
                .fetch(
                    FetchRequest::new(ProviderId::CodeRabbit, Credentials::None)
                        .source(Source::LocalProbe)
                )
                .await,
            Err(AgentStatError::ResponseTooLarge)
        ));
    }
    #[tokio::test]
    async fn timeout_terminates_probe_process_group() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("child-pid");
        let script = format!(
            "#!/bin/sh\nsleep 30 &\necho $! > '{}'\nwait\n",
            pidfile.display()
        );
        let path = executable(dir.path(), &script);
        let client = Client::builder()
            .allow_local_probes(true)
            .probe_executable(ProviderId::CodeRabbit, path)
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();
        assert!(matches!(
            client
                .fetch(FetchRequest::new(ProviderId::CodeRabbit, Credentials::None))
                .await,
            Err(AgentStatError::Timeout)
        ));
        if let Ok(pid) = fs::read_to_string(&pidfile) {
            let pid: i32 = pid.trim().parse().unwrap();
            // Linux may retain a killed grandchild as a zombie until its reaper runs.
            #[cfg(target_os = "linux")]
            for _ in 0..20 {
                match fs::read_to_string(format!("/proc/{pid}/stat")) {
                    Err(_) => return,
                    Ok(stat) if stat.split_whitespace().nth(2) == Some("Z") => return,
                    _ => time::sleep(Duration::from_millis(10)).await,
                }
            }
            // SIGKILL is delivered asynchronously. On macOS, a terminated child
            // can also remain visible to kill(pid, 0) while it is a zombie.
            #[cfg(not(target_os = "linux"))]
            for _ in 0..100 {
                let output = Command::new("ps")
                    .args(["-o", "stat=", "-p", &pid.to_string()])
                    .output()
                    .unwrap();
                let state = String::from_utf8_lossy(&output.stdout);
                if !output.status.success()
                    || state.trim().is_empty()
                    || state.trim().starts_with('Z')
                {
                    return;
                }
                time::sleep(Duration::from_millis(20)).await;
            }
            panic!("probe descendant is still running");
        }
    }
}
