use reqwest::{Method, blocking::Client};
use rm_client_sync::exchange;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::Duration;

fn respond(status_line: &str, payload: &[u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let status_line = status_line.to_owned();
    let payload = payload.to_owned();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            if line == "\r\n" {
                break;
            }
        }
        let header = format!(
            "{status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            payload.len()
        );
        stream.write_all(header.as_bytes()).unwrap();
        stream.write_all(&payload).unwrap();
    });
    url
}

#[test]
fn missing_or_non_json_body_still_reports_the_http_status() {
    let cases = [
        (
            401,
            "HTTP/1.1 401 Unauthorized",
            &b""[..],
            json!({ "message": "" }),
        ),
        (
            400,
            "HTTP/1.1 400 Bad Request",
            &b"{"[..],
            json!({ "message": "{" }),
        ),
    ];
    for (status, status_line, payload, value) in cases {
        let url = respond(status_line, payload);
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let result = exchange(&client, &url, Method::GET, "/texts", "session-token", None).unwrap();
        assert_eq!(result, (status, value));
    }
}

#[test]
fn connection_failure_returns_an_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let result = exchange(
        &client,
        &format!("http://{address}"),
        Method::GET,
        "/ping",
        "",
        None,
    );
    assert!(result.is_err());
}

#[test]
fn exchange_times_out_when_the_peer_accepts_but_never_responds() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        std::thread::sleep(Duration::from_millis(800));
        drop(stream);
    });
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(200))
        .build()
        .unwrap();
    let result = exchange(
        &client,
        &format!("http://{address}"),
        Method::GET,
        "/ping",
        "",
        None,
    );
    let error = result.expect_err("a silent peer should time out");
    assert!(error.is_timeout(), "{error}");
    peer.join().unwrap();
}
