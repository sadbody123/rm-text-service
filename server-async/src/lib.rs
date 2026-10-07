pub mod http;

use pbkdf2::pbkdf2_hmac;
use rand::{RngCore, rngs::OsRng};
use serde_json::{Value, json};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

pub const ROUTES: &[(&str, &str)] = &[
    ("GET", "/ping"),
    ("POST", "/echo"),
    ("POST", "/users"),
    ("POST", "/sessions"),
    ("DELETE", "/sessions/current"),
    ("DELETE", "/users/me"),
    ("GET", "/texts"),
];

pub fn displayed_routes() -> impl Iterator<Item = (&'static str, &'static str)> {
    ROUTES.iter().copied().chain([
        ("PUT", "/texts/{name}"),
        ("GET", "/texts/{name}"),
        ("DELETE", "/texts/{name}"),
    ])
}

pub fn route_error(method: &str, path: &str) -> Option<u16> {
    if let Some((allowed, _)) = ROUTES.iter().find(|(_, route)| *route == path) {
        return (*allowed != method).then_some(405);
    }
    if text_name(path).is_some() {
        return if matches!(method, "GET" | "PUT" | "DELETE") {
            None
        } else {
            Some(405)
        };
    }
    Some(404)
}

fn text_name(path: &str) -> Option<&str> {
    let name = path.strip_prefix("/texts/")?;
    if name.is_empty() || name.contains('/') {
        None
    } else {
        Some(name)
    }
}

struct Credential {
    token: String,
    issued_at: Instant,
}

pub struct User {
    pub salt: [u8; 16],
    pub digest: [u8; 32],
    token: Option<Credential>,
    pub texts: BTreeMap<String, String>,
}

pub struct Service {
    pub users: Mutex<BTreeMap<String, User>>,
    token_ttl: Duration,
}

impl Default for Service {
    fn default() -> Self {
        Self::new(Duration::from_secs(300))
    }
}

impl Service {
    pub fn new(token_ttl: Duration) -> Self {
        Self {
            users: Mutex::new(BTreeMap::new()),
            token_ttl,
        }
    }
}

pub fn error(status: u16, message: &str) -> (u16, Value) {
    (status, json!({"message": message}))
}

pub fn valid_name(name: &str, max: usize) -> bool {
    !name.is_empty()
        && name.len() <= max
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

const MAX_TEXT_BYTES: usize = 65_536;

fn text_value(body: &Value) -> Result<String, (u16, Value)> {
    let Some(object) = body.as_object() else {
        return Err(error(400, "Expected object"));
    };
    if object.len() != 1 {
        return Err(error(400, "Invalid text fields"));
    }
    let Some(text) = object.get("text").and_then(Value::as_str) else {
        return Err(error(400, "Expected text"));
    };
    if text.len() > MAX_TEXT_BYTES {
        return Err(error(413, "Text too large"));
    }
    Ok(text.to_owned())
}

fn password_hash(password: &str, salt: &[u8; 16]) -> [u8; 32] {
    let mut output = [0; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut output);
    output
}

impl Service {
    fn login_snapshot(&self, name: &str) -> Result<([u8; 16], [u8; 32]), (u16, Value)> {
        let users = self.users.lock().unwrap();
        let Some(user) = users.get(name) else {
            return Err(error(401, "Invalid username or password"));
        };
        Ok((user.salt, user.digest))
    }

    fn complete_login(
        &self,
        name: &str,
        password: &str,
        salt: [u8; 16],
        expected: [u8; 32],
    ) -> (u16, Value) {
        let digest = password_hash(password, &salt);
        let mut users = self.users.lock().unwrap();
        let Some(user) = users.get_mut(name) else {
            return error(401, "Invalid username or password");
        };
        if user.salt != salt || !bool::from(digest.ct_eq(&expected)) {
            return error(401, "Invalid username or password");
        }
        let token = new_token();
        user.token = Some(Credential {
            token: token.clone(),
            issued_at: Instant::now(),
        });
        (
            200,
            json!({
                "data": {
                    "token": token,
                    "expires_in": self.token_ttl.as_secs(),
                }
            }),
        )
    }
}

fn new_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl Service {
    pub fn handle(
        &self,
        method: &str,
        path: &str,
        body: &Value,
        authorization: &str,
    ) -> (u16, Value) {
        if let Some(status) = route_error(method, path) {
            return error(
                status,
                if status == 404 {
                    "Not found"
                } else {
                    "Method not allowed"
                },
            );
        }
        if method == "GET" && path == "/ping" {
            return (200, json!({"data": "pong"}));
        }
        if method == "POST" && path == "/echo" {
            return match text_value(body) {
                Ok(text) => (200, json!({"data": text})),
                Err(response) => response,
            };
        }
        if method == "POST" && matches!(path, "/users" | "/sessions") {
            let Some(name) = body.get("username").and_then(Value::as_str) else {
                return error(400, "Expected username");
            };
            let Some(password) = body.get("password").and_then(Value::as_str) else {
                return error(400, "Expected password");
            };
            if body.as_object().map(|v| v.len()) != Some(2)
                || !valid_name(name, 32)
                || !(8..=128).contains(&password.chars().count())
            {
                return error(400, "Invalid account fields");
            }
            if path == "/users" {
                let mut salt = [0; 16];
                OsRng.fill_bytes(&mut salt);
                let digest = password_hash(password, &salt);
                let mut users = self.users.lock().unwrap();
                if users.contains_key(name) {
                    return error(409, "Username exists");
                }
                users.insert(
                    name.into(),
                    User {
                        salt,
                        digest,
                        token: None,
                        texts: BTreeMap::new(),
                    },
                );
                return (201, json!({"data": {"username": name}}));
            }
            let (salt, expected) = match self.login_snapshot(name) {
                Ok(snapshot) => snapshot,
                Err(response) => return response,
            };
            return self.complete_login(name, password, salt, expected);
        }
        let protected = matches!(path, "/texts" | "/sessions/current" | "/users/me")
            || text_name(path).is_some();
        if protected {
            if let Some(name) = text_name(path)
                && !valid_name(name, 64)
            {
                return error(400, "Invalid text name");
            }
            let token = authorization.strip_prefix("Bearer ").unwrap_or("");
            let now = Instant::now();
            let token_ttl = self.token_ttl;
            let mut users = self.users.lock().unwrap();
            let name = users
                .iter()
                .find(|(_, user)| {
                    !token.is_empty()
                        && user.token.as_ref().is_some_and(|credential| {
                            credential.token == token
                                && now.saturating_duration_since(credential.issued_at) < token_ttl
                        })
                })
                .map(|(name, _)| name.clone());
            let Some(name) = name else {
                return error(401, "Login required");
            };
            if method == "DELETE" && path == "/users/me" {
                users.remove(&name);
                return (200, json!({"data": null}));
            }
            let user = users.get_mut(&name).unwrap();
            if method == "DELETE" && path == "/sessions/current" {
                user.token = None;
                return (200, json!({"data": null}));
            }
            if method == "GET" && path == "/texts" {
                return (200, json!({"data": user.texts.keys().collect::<Vec<_>>()}));
            }
            if let Some(text_name) = text_name(path) {
                if method == "PUT" {
                    let text = match text_value(body) {
                        Ok(text) => text,
                        Err(response) => return response,
                    };
                    user.texts.insert(text_name.to_owned(), text);
                    return (200, json!({"data": null}));
                }
                if method == "GET" {
                    let Some(text) = user.texts.get(text_name) else {
                        return error(404, "Text not found");
                    };
                    return (200, json!({"data": text}));
                }
                if method == "DELETE" {
                    if user.texts.remove(text_name).is_none() {
                        return error(404, "Text not found");
                    }
                    return (200, json!({"data": null}));
                }
            }
        }
        error(404, "Not found")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn account_lifecycle() {
        let service = Service::default();
        let account = json!({"username":"alice", "password":"password1"});
        assert_eq!(service.handle("POST", "/users", &account, "").0, 201);
        assert_eq!(service.handle("POST", "/users", &account, "").0, 409);
        let login = service.handle("POST", "/sessions", &account, "").1;
        let old = format!("Bearer {}", login["data"]["token"].as_str().unwrap());
        let login = service.handle("POST", "/sessions", &account, "").1;
        let current = format!("Bearer {}", login["data"]["token"].as_str().unwrap());
        assert_eq!(login["data"]["expires_in"], 300);
        assert_ne!(old, current);
        assert_eq!(service.handle("GET", "/texts", &Value::Null, &old).0, 401);
        assert_eq!(
            service.handle("GET", "/texts", &Value::Null, &current),
            (200, json!({"data":[]}))
        );
        assert_eq!(
            service
                .handle("DELETE", "/sessions/current", &Value::Null, &current)
                .0,
            200
        );
        assert_eq!(
            service.handle("GET", "/texts", &Value::Null, &current).0,
            401
        );
    }

    #[test]
    fn stale_login_does_not_authenticate_a_recreated_account() {
        let service = Service::default();
        let original = json!({"username":"alice", "password":"password1"});
        assert_eq!(service.handle("POST", "/users", &original, "").0, 201);
        let (salt, digest) = service.login_snapshot("alice").unwrap();
        let login = service.handle("POST", "/sessions", &original, "");
        let authorization = format!("Bearer {}", login.1["data"]["token"].as_str().unwrap());
        assert_eq!(
            service
                .handle(
                    "PUT",
                    "/texts/note",
                    &json!({"text": "secret"}),
                    &authorization
                )
                .0,
            200
        );
        assert_eq!(
            service
                .handle("DELETE", "/users/me", &Value::Null, &authorization)
                .0,
            200
        );
        let replacement = json!({"username":"alice", "password":"password2"});
        assert_eq!(service.handle("POST", "/users", &replacement, "").0, 201);
        assert_eq!(
            service.complete_login("alice", "password1", salt, digest).0,
            401
        );
        {
            let users = service.users.lock().unwrap();
            let user = users.get("alice").unwrap();
            assert!(user.token.is_none());
            assert!(user.texts.is_empty());
        }
        let login = service.handle("POST", "/sessions", &replacement, "");
        assert_eq!(login.0, 200);
        let authorization = format!("Bearer {}", login.1["data"]["token"].as_str().unwrap());
        assert_eq!(
            service.handle("GET", "/texts", &Value::Null, &authorization),
            (200, json!({"data": []}))
        );
    }

    #[test]
    fn expired_token_is_rejected_without_renewal() {
        let service = Service::new(Duration::from_millis(200));
        let account = json!({"username":"alice", "password":"password1"});
        assert_eq!(service.handle("POST", "/users", &account, "").0, 201);
        let login = service.handle("POST", "/sessions", &account, "");
        let authorization = format!("Bearer {}", login.1["data"]["token"].as_str().unwrap());
        assert_eq!(
            service
                .handle("GET", "/texts", &Value::Null, &authorization)
                .0,
            200
        );
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(
            service
                .handle("GET", "/texts", &Value::Null, &authorization)
                .0,
            401
        );
        assert_eq!(
            service
                .handle("DELETE", "/sessions/current", &Value::Null, &authorization)
                .0,
            401
        );
    }

    #[test]
    fn very_large_ttl_does_not_panic() {
        let service = Service::new(Duration::from_secs(u64::MAX));
        let account = json!({"username":"alice", "password":"password1"});
        assert_eq!(service.handle("POST", "/users", &account, "").0, 201);
        let login = service.handle("POST", "/sessions", &account, "");
        assert_eq!(login.0, 200);
        let authorization = format!("Bearer {}", login.1["data"]["token"].as_str().unwrap());
        assert_eq!(
            service
                .handle("GET", "/texts", &Value::Null, &authorization)
                .0,
            200
        );
    }
}
