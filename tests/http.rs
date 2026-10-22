//! Real reqwest wiring, run explicitly in CI because agent sandboxes may deny sockets.
#![cfg(feature = "codex")]
use agentstat::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
#[tokio::test]
#[ignore = "requires loopback sockets; CI runs this explicitly"]
async fn reqwest_fetch_and_redirect_policy() {
    for status in [200, 302] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut data = Vec::new();
            loop {
                let mut chunk = [0u8; 2048];
                let n = stream.read(&mut chunk).await.unwrap();
                assert_ne!(n, 0);
                data.extend_from_slice(&chunk[..n]);
                if data.windows(4).any(|b| b == b"\r\n\r\n") {
                    break;
                }
            }
            let body = r#"{"rate_limit":{"primary_window":{"used_percent":25}}}"#;
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nContent-Type: application/json\r\nLocation: http://127.0.0.1:1/never-follow\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8(data).unwrap()
        });
        let client = Client::builder()
            .endpoint_override(ProviderId::Codex, &base)
            .http_client_builder(reqwest::Client::builder().no_proxy())
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        let result = client
            .fetch(FetchRequest::new(
                ProviderId::Codex,
                Credentials::bearer("test-only"),
            ))
            .await;
        if status == 200 {
            assert_eq!(result.unwrap().windows[0].utilization_percent, Some(25.0))
        } else {
            assert!(matches!(result, Err(AgentStatError::Http { status: 302 })))
        }
        let request = task.await.unwrap();
        assert!(request.starts_with("GET /backend-api/wham/usage "));
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer test-only")
        );
    }
}
