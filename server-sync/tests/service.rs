use rm_server_sync::Service;
use serde_json::{Value, json};

#[test]
fn input_validation_and_baseline() {
    let service = Service::default();
    assert_eq!(
        service.handle("GET", "/ping", &Value::Null, ""),
        (200, json!({"data":"pong"}))
    );
    for body in [
        Value::Null,
        json!([]),
        json!({"username":true,"password":"password1"}),
        json!({"username":"a/b","password":"password1"}),
    ] {
        assert_eq!(service.handle("POST", "/users", &body, "").0, 400);
    }
    assert_eq!(service.handle("GET", "/texts", &Value::Null, "").0, 401);
    assert_eq!(service.handle("GET", "/missing", &Value::Null, "").0, 404);
}

#[test]
fn echo_returns_text_and_rejects_bad_input() {
    let service = Service::default();
    for text in ["", "你好\nRM", "line"] {
        assert_eq!(
            service.handle("POST", "/echo", &json!({"text": text}), ""),
            (200, json!({"data": text}))
        );
    }
    assert_eq!(service.handle("GET", "/echo", &Value::Null, "").0, 405);
    for body in [
        Value::Null,
        json!({}),
        json!({"text": 42}),
        json!({"text": "ok", "extra": 1}),
        json!({"message": "ok"}),
    ] {
        assert_eq!(service.handle("POST", "/echo", &body, "").0, 400);
    }
    let too_large = "x".repeat(65_537);
    assert_eq!(
        service
            .handle("POST", "/echo", &json!({"text": too_large}), "")
            .0,
        413
    );
    let limit = "y".repeat(65_536);
    assert_eq!(
        service
            .handle("POST", "/echo", &json!({"text": limit}), "")
            .0,
        200
    );
}

#[test]
fn concurrent_registration_has_one_winner() {
    let service = std::sync::Arc::new(Service::default());
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let service = service.clone();
            std::thread::spawn(move || {
                service
                    .handle(
                        "POST",
                        "/users",
                        &json!({"username":"alice","password":"password1"}),
                        "",
                    )
                    .0
            })
        })
        .collect();
    let statuses: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(statuses.iter().filter(|&&s| s == 201).count(), 1);
    assert_eq!(statuses.iter().filter(|&&s| s == 409).count(), 3);
}

fn account(service: &Service, name: &str) -> String {
    let body = json!({"username": name, "password": "password1"});
    assert_eq!(service.handle("POST", "/users", &body, "").0, 201);
    let login = service.handle("POST", "/sessions", &body, "");
    format!("Bearer {}", login.1["data"]["token"].as_str().unwrap())
}

#[test]
fn put_and_get_are_scoped_to_the_authenticated_user() {
    let service = Service::default();
    let alice = account(&service, "alice");
    let bob = account(&service, "bob");
    assert_eq!(
        service.handle("PUT", "/texts/note", &json!({"text": "你好\nRM"}), &alice),
        (200, json!({"data": null}))
    );
    assert_eq!(
        service.handle("GET", "/texts/note", &Value::Null, &alice),
        (200, json!({"data": "你好\nRM"}))
    );
    assert_eq!(
        service
            .handle("PUT", "/texts/note", &json!({"text": "replaced"}), &alice)
            .0,
        200
    );
    assert_eq!(
        service.handle("GET", "/texts/note", &Value::Null, &alice).1,
        json!({"data": "replaced"})
    );
    assert_eq!(
        service.handle("GET", "/texts/note", &Value::Null, &bob).0,
        404
    );
    assert_eq!(
        service
            .handle("GET", "/texts/missing", &Value::Null, &alice)
            .0,
        404
    );
    assert_eq!(
        service
            .handle("PUT", "/texts/note", &json!({"text": "x"}), "")
            .0,
        401
    );
    assert_eq!(
        service
            .handle("PUT", "/texts/bad name", &json!({"text": "x"}), &alice)
            .0,
        400
    );
    assert_eq!(
        service
            .handle("POST", "/texts/note", &json!({"text": "x"}), &alice)
            .0,
        405
    );
    let too_large = "x".repeat(65_537);
    assert_eq!(
        service
            .handle("PUT", "/texts/note", &json!({"text": too_large}), &alice)
            .0,
        413
    );
}
