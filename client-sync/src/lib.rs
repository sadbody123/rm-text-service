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
/// A line containing only `..` is stored as a literal `.`. Lines are joined with
/// `\n`, and the result does not gain an extra trailing newline.
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
        if content == ".." {
            lines.push(".".to_owned());
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
    fn doubled_dot_is_a_literal_terminator_line() {
        assert_eq!(read("..\n.\n"), ".");
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
}
