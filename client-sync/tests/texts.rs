use reqwest::{Method, blocking::Client};
use rm_client_sync::exchange;
use serde_json::json;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::time::Duration;

#[test]
fn put_sends_bearer_token_and_text() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            if line == "\r\n" {
                break;
            }
            headers.push_str(&line);
        }
        assert!(headers.starts_with("PUT /texts/note HTTP/1.1\r\n"));
        assert!(
            headers
                .to_lowercase()
                .contains("authorization: bearer session-token\r\n")
        );
        let length = headers
            .lines()
            .find_map(|line| {
                line.to_lowercase()
                    .strip_prefix("content-length:")
                    .map(|value| value.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        assert_eq!(std::str::from_utf8(&body).unwrap(), r#"{"text":"hello"}"#);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\n{\"data\":null}")
            .unwrap();
    });
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let result = exchange(
        &client,
        &url,
        Method::PUT,
        "/texts/note",
        "session-token",
        Some(&json!({ "text": "hello" })),
    )
    .unwrap();
    assert_eq!(result, (200, json!({ "data": null })));
    peer.join().unwrap();
}

#[test]
fn get_sends_bearer_token_without_a_body() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            if line == "\r\n" {
                break;
            }
            headers.push_str(&line);
        }
        assert!(headers.starts_with("GET /texts/note HTTP/1.1\r\n"));
        assert!(
            headers
                .to_lowercase()
                .contains("authorization: bearer session-token\r\n")
        );
        assert!(!headers.to_lowercase().contains("content-length:"));
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\n{\"data\":\"hello\"}",
            )
            .unwrap();
    });
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let result = exchange(
        &client,
        &url,
        Method::GET,
        "/texts/note",
        "session-token",
        None,
    )
    .unwrap();
    assert_eq!(result, (200, json!({ "data": "hello" })));
    peer.join().unwrap();
}
