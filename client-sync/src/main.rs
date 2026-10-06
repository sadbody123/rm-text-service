use clap::Parser;
use reqwest::Method;
use reqwest::blocking::Client;
use rm_client_sync::{
    TokenChange, exchange, prepare_delete_text, prepare_delete_user, prepare_echo, prepare_get,
    prepare_put, read_multiline_text, token_change,
};
use serde_json::json;
use std::io::{self, Write};
use std::time::Duration;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:7878")]
    url: String,
}
fn input(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    if io::stdin().read_line(&mut line)? == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}
fn read_text() -> io::Result<String> {
    println!("text: finish with a line containing only .");
    read_multiline_text(&mut io::stdin().lock())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let client = Client::builder()
        .timeout(Duration::from_secs(12))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let mut token = String::new();
    loop {
        let command = match input(
            "ping / register / login / logout / list / echo / delete-user / put / get / delete / q > ",
        ) {
            Ok(command) => command,
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        };
        let (method, path, body) = match command.as_str() {
            "q" => break,
            "ping" => (Method::GET, "/ping".to_owned(), None),
            "list" => (Method::GET, "/texts".to_owned(), None),
            "logout" => (Method::DELETE, "/sessions/current".to_owned(), None),
            "register" | "login" => {
                let body = json!({
                    "username": input("username: ")?,
                    "password": rpassword::prompt_password("password: ")?,
                });
                let path = if command == "register" {
                    "/users"
                } else {
                    "/sessions"
                };
                (Method::POST, path.to_owned(), Some(body))
            }
            "echo" => {
                let prepared = prepare_echo(&read_text()?);
                (prepared.method, prepared.path, prepared.body)
            }
            "put" => {
                let name = input("name: ")?;
                let prepared = prepare_put(&name, &read_text()?);
                (prepared.method, prepared.path, prepared.body)
            }
            "get" => {
                let prepared = prepare_get(&input("name: ")?);
                (prepared.method, prepared.path, prepared.body)
            }
            "delete" => {
                let prepared = prepare_delete_text(&input("name: ")?);
                (prepared.method, prepared.path, prepared.body)
            }
            "delete-user" => {
                let prepared = prepare_delete_user();
                (prepared.method, prepared.path, prepared.body)
            }
            _ => {
                println!("Unknown command.");
                continue;
            }
        };
        let result = exchange(&client, &args.url, method, &path, &token, body.as_ref());
        match result {
            Ok((status, value)) => {
                println!("{status} {value}");
                match token_change(&command, status, &value) {
                    TokenChange::Set(next) => token = next,
                    TokenChange::Clear { prompt_login } => {
                        if prompt_login {
                            println!("Please log in again.");
                        }
                        token.clear();
                    }
                    TokenChange::Keep => {}
                }
            }
            Err(error) => eprintln!("Request failed: {error}"),
        }
    }
    Ok(())
}
