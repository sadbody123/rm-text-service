use rm_server_sync::{
    Service,
    http::{create_app, with_service},
};
use rocket::http::{ContentType, Header, Status};
use rocket::local::blocking::Client;
use serde_json::{Value, json};

#[test]
fn app_uses_supplied_service() {
    let service = Service::default();
    let account = json!({"username": "alice", "password": "password1"});
    assert_eq!(service.handle("POST", "/users", &account, "").0, 201);
    let configured = Client::tracked(with_service(service)).unwrap();
    let fresh = Client::tracked(create_app()).unwrap();
    for (client, expected) in [(&configured, Status::Ok), (&fresh, Status::Unauthorized)] {
        assert_eq!(
            client
                .post("/sessions")
                .header(ContentType::JSON)
                .body(account.to_string())
                .dispatch()
                .status(),
            expected
        );
    }
}

#[test]
fn http_account_lifecycle() {
    let client = Client::tracked(create_app()).unwrap();
    let ping = client.get("/ping").dispatch();
    assert_eq!(ping.status(), Status::Ok);
    assert_eq!(ping.into_json::<Value>().unwrap(), json!({"data": "pong"}));
    let account = json!({"username": "alice", "password": "password1"}).to_string();
    assert_eq!(
        client
            .post("/users")
            .header(ContentType::JSON)
            .body(&account)
            .dispatch()
            .status(),
        Status::Created
    );
    let login = client
        .post("/sessions")
        .header(ContentType::JSON)
        .body(&account)
        .dispatch()
        .into_json::<Value>()
        .unwrap();
    assert_eq!(login["data"]["expires_in"], 300);
    let authorization = format!("Bearer {}", login["data"]["token"].as_str().unwrap());
    let texts = client
        .get("/texts")
        .header(Header::new("Authorization", authorization.clone()))
        .dispatch();
    assert_eq!(texts.status(), Status::Ok);
    assert_eq!(texts.into_json::<Value>().unwrap(), json!({"data": []}));
    assert_eq!(
        client.get("/texts").dispatch().status(),
        Status::Unauthorized
    );
    assert_eq!(
        client
            .delete("/sessions/current")
            .header(Header::new("Authorization", authorization.clone()))
            .dispatch()
            .status(),
        Status::Ok
    );
    assert_eq!(
        client
            .get("/texts")
            .header(Header::new("Authorization", authorization.clone()))
            .dispatch()
            .status(),
        Status::Unauthorized
    );
    let login = client
        .post("/sessions")
        .header(ContentType::JSON)
        .body(&account)
        .dispatch()
        .into_json::<Value>()
        .unwrap();
    let authorization = format!("Bearer {}", login["data"]["token"].as_str().unwrap());
    assert_eq!(
        client
            .put("/texts/note")
            .header(ContentType::JSON)
            .header(Header::new("Authorization", authorization.clone()))
            .body(r#"{"text":"secret"}"#)
            .dispatch()
            .status(),
        Status::Ok
    );
    assert_eq!(
        client
            .delete("/users/me")
            .header(Header::new("Authorization", authorization.clone()))
            .dispatch()
            .status(),
        Status::Ok
    );
    assert_eq!(
        client
            .get("/texts")
            .header(Header::new("Authorization", authorization))
            .dispatch()
            .status(),
        Status::Unauthorized
    );
    assert_eq!(
        client
            .post("/users")
            .header(ContentType::JSON)
            .body(&account)
            .dispatch()
            .status(),
        Status::Created
    );
}

#[test]
fn http_input_and_routing() {
    let client = Client::tracked(create_app()).unwrap();
    for body in [b"not JSON".to_vec(), vec![0xff], b"NaN".to_vec()] {
        assert_eq!(
            client
                .post("/users")
                .header(ContentType::JSON)
                .body(body)
                .dispatch()
                .status(),
            Status::BadRequest
        );
    }
    let exact = format!("{{}}{}", " ".repeat(524_288 - 2));
    assert_eq!(
        client
            .post("/users")
            .header(ContentType::JSON)
            .body(&exact)
            .dispatch()
            .status(),
        Status::BadRequest
    );
    assert_eq!(
        client
            .post("/users")
            .header(ContentType::JSON)
            .body(format!("{exact} "))
            .dispatch()
            .status(),
        Status::PayloadTooLarge
    );
    assert_eq!(
        client
            .post("/users")
            .header(ContentType::JSON)
            .body(r#"{"username":true,"password":"password1"}"#)
            .dispatch()
            .status(),
        Status::BadRequest
    );
    assert_eq!(client.get("/missing").dispatch().status(), Status::NotFound);
    assert_eq!(
        client.get("/echo").dispatch().status(),
        Status::MethodNotAllowed
    );
    assert_eq!(
        client.patch("/ping").dispatch().status(),
        Status::MethodNotAllowed
    );
}

#[test]
fn http_echo_round_trip_and_limits() {
    let client = Client::tracked(create_app()).unwrap();
    let echo = client
        .post("/echo")
        .header(ContentType::JSON)
        .body(r#"{"text":"你好\nRM"}"#)
        .dispatch();
    assert_eq!(echo.status(), Status::Ok);
    assert_eq!(
        echo.into_json::<Value>().unwrap(),
        json!({"data": "你好\nRM"})
    );
    assert_eq!(
        client
            .post("/echo")
            .header(ContentType::JSON)
            .body(r#"{"text":""}"#)
            .dispatch()
            .status(),
        Status::Ok
    );
    assert_eq!(
        client
            .post("/echo")
            .header(ContentType::JSON)
            .body(r#"{"text":42}"#)
            .dispatch()
            .status(),
        Status::BadRequest
    );
    let too_large = json!({"text": "x".repeat(65_537)}).to_string();
    assert_eq!(
        client
            .post("/echo")
            .header(ContentType::JSON)
            .body(too_large)
            .dispatch()
            .status(),
        Status::PayloadTooLarge
    );
}

#[test]
fn http_put_and_get_text() {
    let client = Client::tracked(create_app()).unwrap();
    let account = r#"{"username":"alice","password":"password1"}"#;
    assert_eq!(
        client
            .post("/users")
            .header(ContentType::JSON)
            .body(account)
            .dispatch()
            .status(),
        Status::Created
    );
    let login = client
        .post("/sessions")
        .header(ContentType::JSON)
        .body(account)
        .dispatch()
        .into_json::<Value>()
        .unwrap();
    let authorization = format!("Bearer {}", login["data"]["token"].as_str().unwrap());
    let created = client
        .put("/texts/note")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", authorization.clone()))
        .body(r#"{"text":"你好\nRM"}"#)
        .dispatch();
    assert_eq!(created.status(), Status::Ok);
    assert_eq!(created.into_json::<Value>().unwrap(), json!({"data": null}));
    let fetched = client
        .get("/texts/note")
        .header(Header::new("Authorization", authorization.clone()))
        .dispatch();
    assert_eq!(fetched.status(), Status::Ok);
    assert_eq!(
        fetched.into_json::<Value>().unwrap(),
        json!({"data": "你好\nRM"})
    );
    assert_eq!(
        client.get("/texts/note").dispatch().status(),
        Status::Unauthorized
    );
    assert_eq!(
        client
            .put("/texts/Not A Name")
            .header(ContentType::JSON)
            .body(r#"{"text":"x"}"#)
            .dispatch()
            .status(),
        Status::BadRequest
    );
    assert_eq!(
        client
            .put("/texts/b")
            .header(ContentType::JSON)
            .header(Header::new("Authorization", authorization.clone()))
            .body(r#"{"text":"b"}"#)
            .dispatch()
            .status(),
        Status::Ok
    );
    let listed = client
        .get("/texts")
        .header(Header::new("Authorization", authorization.clone()))
        .dispatch();
    assert_eq!(listed.status(), Status::Ok);
    assert_eq!(
        listed.into_json::<Value>().unwrap(),
        json!({"data": ["b", "note"]})
    );
    assert_eq!(
        client
            .delete("/texts/missing")
            .header(Header::new("Authorization", authorization.clone()))
            .dispatch()
            .status(),
        Status::NotFound
    );
    assert_eq!(
        client
            .delete("/texts/note")
            .header(Header::new("Authorization", authorization.clone()))
            .dispatch()
            .status(),
        Status::Ok
    );
    let remaining = client
        .get("/texts")
        .header(Header::new("Authorization", authorization))
        .dispatch()
        .into_json::<Value>()
        .unwrap();
    assert_eq!(remaining, json!({"data": ["b"]}));
}

#[test]
fn protected_routes_and_unsupported_methods_return_expected_status() {
    let client = Client::tracked(create_app()).unwrap();
    assert_eq!(
        client.delete("/users/me").dispatch().status(),
        Status::Unauthorized
    );
    assert_eq!(
        client.delete("/texts/note").dispatch().status(),
        Status::Unauthorized
    );
    for path in [
        "/ping",
        "/users",
        "/sessions",
        "/sessions/current",
        "/texts",
    ] {
        assert_eq!(
            client.patch(path).dispatch().status(),
            Status::MethodNotAllowed
        );
    }
}
