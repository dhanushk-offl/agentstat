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
        any(feature = "coderabbit", feature = "antigravity")
    )
))]
use agentstat::*;

#[cfg(feature = "cursor")]
#[tokio::test]
async fn cursor_cli_credentials_are_opt_in_redacted_and_never_modified() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("auth.json");
    let bytes = br#"{"accessToken":"private","refreshToken":"refresh-private"}"#;
    std::fs::write(&path, bytes).unwrap();
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
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::write(&path, br#"{"apiKey":"private"}"#).unwrap();
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

    fn executable(dir: &std::path::Path, output: &str) -> std::path::PathBuf {
        let path = dir.join("agy");
        let script = format!(
            "#!/bin/sh\n[ \"$#\" = 6 ] || exit 2\n[ \"$1\" = --print ] && [ \"$2\" = /usage ] && [ \"$3\" = --output-format ] && [ \"$4\" = json ] && [ \"$5\" = --log-file ] && [ \"$6\" = /dev/null ] || exit 3\n[ \"$AGY_CLI_DISABLE_AUTO_UPDATE\" = true ] || exit 4\n[ \"$NO_COLOR\" = 1 ] || exit 5\ncat <<'OUTPUT'\n{output}\nOUTPUT\n"
        );
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
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
    std::fs::write(&path, bytes).unwrap();
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
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    assert!(!format!("{request:?}").contains("private"));
}
#[cfg(feature = "claude")]
#[tokio::test]
async fn expired_claude_credentials_are_not_refreshed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(".credentials.json");
    let bytes=br#"{"claudeAiOauth":{"accessToken":"private","refreshToken":"refresh-private","expiresAt":1}}"#;
    std::fs::write(&path, bytes).unwrap();
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
    assert_eq!(std::fs::read(path).unwrap(), bytes);
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
        std::fs::write(&path, bytes).unwrap();
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
    fn executable(dir: &std::path::Path, script: &str) -> std::path::PathBuf {
        let path = dir.join("coderabbit");
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
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
            .timeout(std::time::Duration::from_millis(100))
            .build()
            .unwrap();
        assert!(matches!(
            client
                .fetch(FetchRequest::new(ProviderId::CodeRabbit, Credentials::None))
                .await,
            Err(AgentStatError::Timeout)
        ));
        if let Ok(pid) = std::fs::read_to_string(&pidfile) {
            let pid: i32 = pid.trim().parse().unwrap();
            // Linux may retain a killed grandchild as a zombie until its reaper runs.
            #[cfg(target_os = "linux")]
            for _ in 0..20 {
                match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                    Err(_) => return,
                    Ok(stat) if stat.split_whitespace().nth(2) == Some("Z") => return,
                    _ => tokio::time::sleep(std::time::Duration::from_millis(10)).await,
                }
            }
            #[cfg(not(target_os = "linux"))]
            assert!(nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_err());
            #[cfg(target_os = "linux")]
            panic!("probe descendant is still running");
        }
    }
}
