use reqwest::{Method, blocking::Client};
use serde_json::{Value, json};
use std::io::{self, BufRead};

/// Preserve HTTP status even when the error body is not JSON.
pub fn exchange(
    client: &Client,
    url: &str,
    method: Method,
    path: &str,
    token: &str,
    body: Option<&Value>,
) -> Result<(u16, Value), reqwest::Error> {
    let mut request = client.request(method, format!("{}{path}", url.trim_end_matches('/')));
    if !token.is_empty() {
        request = request.bearer_auth(token);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.send()?;
    let status = response.status().as_u16();
    let text = response.text()?;
    let value =
        serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({"message": text}));
    Ok((status, value))
}

/// Read text until a line containing only `.`.
///
/// A line that starts with `..` drops one leading dot, so `.` and `..` in the
/// body can both be expressed. Lines are joined with `\n`, and the result does
/// not gain an extra trailing newline.
pub fn read_multiline_text(reader: &mut impl BufRead) -> io::Result<String> {
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        let content = line.trim_end_matches(['\r', '\n']);
        if content == "." {
            break;
        }
        if let Some(rest) = content.strip_prefix("..") {
            lines.push(format!(".{rest}"));
        } else {
            lines.push(content.to_owned());
        }
    }
    Ok(lines.join("\n"))
}

#[derive(Debug, PartialEq, Eq)]
pub struct PreparedRequest {
    pub method: Method,
    pub path: String,
    pub body: Option<Value>,
}

pub fn prepare_echo(text: &str) -> PreparedRequest {
    PreparedRequest {
        method: Method::POST,
        path: "/echo".to_owned(),
        body: Some(json!({ "text": text })),
    }
}

pub fn prepare_put(name: &str, text: &str) -> PreparedRequest {
    PreparedRequest {
        method: Method::PUT,
        path: format!("/texts/{name}"),
        body: Some(json!({ "text": text })),
    }
}

pub fn prepare_get(name: &str) -> PreparedRequest {
    PreparedRequest {
        method: Method::GET,
        path: format!("/texts/{name}"),
        body: None,
    }
}

pub fn prepare_delete_text(name: &str) -> PreparedRequest {
    PreparedRequest {
        method: Method::DELETE,
        path: format!("/texts/{name}"),
        body: None,
    }
}

pub fn prepare_delete_user() -> PreparedRequest {
    PreparedRequest {
        method: Method::DELETE,
        path: "/users/me".to_owned(),
        body: None,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TokenChange {
    Keep,
    Set(String),
    Clear { prompt_login: bool },
}

pub fn token_change(command: &str, status: u16, value: &Value) -> TokenChange {
    if command == "login"
        && status == 200
        && let Some(token) = value["data"]["token"].as_str()
    {
        return TokenChange::Set(token.to_owned());
    }
    if status == 401 {
        return TokenChange::Clear { prompt_login: true };
    }
    if status == 200 && (command == "logout" || command == "delete-user") {
        return TokenChange::Clear {
            prompt_login: false,
        };
    }
    TokenChange::Keep
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn read(input: &str) -> String {
        read_multiline_text(&mut Cursor::new(input)).unwrap()
    }

    #[test]
    fn empty_text_is_an_immediate_terminator() {
        assert_eq!(read(".\n"), "");
    }

    #[test]
    fn text_without_trailing_newline() {
        assert_eq!(read("hello\n.\n"), "hello");
    }

    #[test]
    fn empty_line_before_terminator_keeps_trailing_newline() {
        assert_eq!(read("hello\n\n.\n"), "hello\n");
    }

    #[test]
    fn unicode_lines_keep_internal_newlines() {
        assert_eq!(read("你好\nRM\n.\n"), "你好\nRM");
    }

    #[test]
    fn dot_stuffing_is_reversible() {
        assert_eq!(read("..\n.\n"), ".");
        assert_eq!(read("...\n.\n"), "..");
        assert_eq!(read("....\n.\n"), "...");
    }

    #[test]
    fn echo_request_uses_the_text_field() {
        assert_eq!(
            prepare_echo("你好\nRM"),
            PreparedRequest {
                method: Method::POST,
                path: "/echo".to_owned(),
                body: Some(json!({ "text": "你好\nRM" })),
            }
        );
    }

    #[test]
    fn put_and_get_use_the_name_and_only_put_sends_text() {
        assert_eq!(
            prepare_put("note", "你好\nRM"),
            PreparedRequest {
                method: Method::PUT,
                path: "/texts/note".to_owned(),
                body: Some(json!({ "text": "你好\nRM" })),
            }
        );
        assert_eq!(
            prepare_put("note", "replaced"),
            PreparedRequest {
                method: Method::PUT,
                path: "/texts/note".to_owned(),
                body: Some(json!({ "text": "replaced" })),
            }
        );
        assert_eq!(
            prepare_get("note"),
            PreparedRequest {
                method: Method::GET,
                path: "/texts/note".to_owned(),
                body: None,
            }
        );
    }

    #[test]
    fn delete_text_targets_the_named_text() {
        assert_eq!(
            prepare_delete_text("note"),
            PreparedRequest {
                method: Method::DELETE,
                path: "/texts/note".to_owned(),
                body: None,
            }
        );
    }

    #[test]
    fn delete_user_targets_the_current_account() {
        assert_eq!(
            prepare_delete_user(),
            PreparedRequest {
                method: Method::DELETE,
                path: "/users/me".to_owned(),
                body: None,
            }
        );
    }

    #[test]
    fn login_saves_token_and_logout_or_deletion_clears_it() {
        assert_eq!(
            token_change("login", 200, &json!({ "data": { "token": "abc" } })),
            TokenChange::Set("abc".to_owned())
        );
        assert_eq!(
            token_change("logout", 200, &json!({ "data": null })),
            TokenChange::Clear {
                prompt_login: false
            }
        );
        assert_eq!(
            token_change("delete-user", 200, &json!({ "data": null })),
            TokenChange::Clear {
                prompt_login: false
            }
        );
    }

    #[test]
    fn unauthorized_response_clears_the_token_and_asks_for_login() {
        assert_eq!(
            token_change("list", 401, &json!({ "message": "expired" })),
            TokenChange::Clear { prompt_login: true }
        );
    }

    #[test]
    fn login_without_a_token_keeps_the_current_session() {
        assert_eq!(
            token_change("login", 200, &json!({ "data": {} })),
            TokenChange::Keep
        );
    }
}
