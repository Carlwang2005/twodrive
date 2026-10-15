use super::*;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use tiny_http::{Header, Response, Server};

fn denied(body: &str, headers: &[(&str, &str)]) -> String {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}/private-item-id", server.server_addr());
    let response = headers.iter().fold(
        Response::from_string(body).with_status_code(403),
        |r, (k, v)| r.with_header(Header::from_bytes(*k, *v).unwrap()),
    );
    let worker = std::thread::spawn(move || {
        server
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
            .respond(response)
            .unwrap()
    });
    let calls = AtomicUsize::new(0);
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .unwrap();
    let error = http::retry_optional_request(|| {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(client.delete(&url))
    })
    .unwrap_err();
    worker.join().unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    error.to_string()
}

#[test]
fn ordinary_http_error_preserves_safe_code_and_request_id_without_remote_text() {
    let text = denied(
        r#"{"error":{"code":"accessDenied","message":"PRIVATE_BODY","innerError":{"code":"insufficient_claims","request-id":"11111111-1111-4111-8111-111111111111"}}}"#,
        &[
            ("Content-Type", "application/json"),
            ("WWW-Authenticate", "Bearer claims=PRIVATE_CLAIMS"),
        ],
    );
    assert!(text.contains("code=accessDenied"), "{text}");
    assert!(text.contains("inner_code=insufficient_claims"));
    assert!(text.contains("request_id=11111111-1111-4111-8111-111111111111"));
    assert!(text.contains("claims_challenge=true"));
    for private in [
        "PRIVATE_BODY",
        "PRIVATE_CLAIMS",
        "private-item-id",
        "http://",
    ] {
        assert!(!text.contains(private));
    }
}

#[test]
fn untrusted_codes_ids_oversized_and_non_json_responses_are_not_echoed() {
    let text = denied(
        r#"{"error":{"code":"PRIVATE_CODE","message":"PRIVATE_BODY","innerError":{"code":"PRIVATE_INNER","request-id":"PRIVATE_ID"}}}"#,
        &[("request-id", "PRIVATE_HEADER"), ("Date", "PRIVATE_DATE")],
    );
    assert!(text.contains("code=unknown"));
    assert!(!text.contains("PRIVATE"));
    let body = format!(
        "{{\"error\":{{\"code\":\"accessDenied\",\"message\":\"{}\"}}}}",
        "X".repeat(20000)
    );
    assert!(denied(&body, &[]).contains("code=unknown"));
    assert!(denied("<html>PRIVATE_BODY</html>", &[]).contains("code=unknown"));
}

#[test]
fn optional_http_404_and_retry_after_and_transient_retries_keep_their_contract() {
    for statuses in [vec![404], vec![429], vec![503, 503, 204]] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/item", server.server_addr());
        let expected = statuses.len();
        let first = statuses[0];
        let worker = std::thread::spawn(move || {
            for status in statuses {
                server
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap()
                    .unwrap()
                    .respond(
                        Response::empty(status).with_header(
                            Header::from_bytes(
                                "Retry-After",
                                if status == 429 { "120" } else { "0" },
                            )
                            .unwrap(),
                        ),
                    )
                    .unwrap();
            }
        });
        let calls = AtomicUsize::new(0);
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .unwrap();
        let result = http::retry_optional_request(|| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(client.delete(&url))
        });
        worker.join().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), expected);
        match first {
            404 => assert!(result.unwrap().is_none()),
            429 => assert_eq!(
                result
                    .unwrap_err()
                    .downcast_ref::<http::HttpFailure>()
                    .unwrap()
                    .retry_after,
                Duration::from_secs(120)
            ),
            _ => assert!(result.unwrap().is_some()),
        }
    }
}

#[test]
fn failed_refresh_is_labeled_and_not_repeated_by_delete_request_building() {
    let root =
        std::env::temp_dir().join(format!("td-refresh-diagnostic-{}", rand::random::<u64>()));
    std::fs::create_dir(&root).unwrap();
    let paths = AppPaths {
        config_dir: root.clone(),
        config_path: root.join("config.toml"),
        data_dir: root.clone(),
        cache_dir: root.join("cache"),
        db_path: root.join("db"),
        mount_dir: root.join("mount"),
        token_path: root.join("tokens.json"),
    };
    TokenStore::new(&paths.token_path)
        .save(&TokenData {
            access_token: "synthetic-expired".into(),
            refresh_token: Some("synthetic-refresh".into()),
            expires_at_unix: twodrive_core::now_unix() - 100,
        })
        .unwrap();
    let server = Server::http("127.0.0.1:0").unwrap();
    let mut graph = GraphBackend::from_paths(&paths).unwrap();
    graph.test_endpoint = Some(format!("http://{}", server.server_addr()));
    let worker = std::thread::spawn(move || {
        let request = server
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert_eq!(request.method().as_str(), "POST");
        assert_eq!(request.url(), "/token");
        request
            .respond(
                Response::from_string(r#"{"error":"invalid_grant","error_description":"PRIVATE"}"#)
                    .with_status_code(403),
            )
            .unwrap();
        assert!(
            server
                .recv_timeout(Duration::from_millis(250))
                .unwrap()
                .is_none()
        );
    });
    let error = graph.delete("never-sent-private-id").unwrap_err();
    worker.join().unwrap();
    let text = error.to_string();
    assert!(text.contains("operation=token.refresh"));
    assert!(text.contains("code=invalid_grant"));
    assert!(!text.contains("PRIVATE"));
    assert!(!text.contains("never-sent"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn transport_error_never_includes_request_url() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let url = format!("http://127.0.0.1:{port}/PRIVATE_ID?secret=PRIVATE_TOKEN");
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .unwrap();
    let error = http::retry_optional_request(|| Ok(client.delete(&url))).unwrap_err();
    assert!(!format!("{error:?}").contains("PRIVATE"));
    assert!(!error.to_string().contains("127.0.0.1"));
}
