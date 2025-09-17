use postgres::{Client, NoTls};
use postgres::Error as PostgresError;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::env;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json;

// model: User struct with id, name, email
#[derive(Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    name: String,
    email: String,
}

// database URL (runtime-initialized "global")
static DB_URL: Lazy<String> = Lazy::new(|| {
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
});

// constants
const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_ERROR: &str = "HTTP/1.1 500 INTERNAL ERROR\r\n\r\n";

fn main() {
    // set database
    if let Err(e) = set_database() {
        eprintln!("Error setting database: {e}");
        return;
    }

    // start server and print port
    let listener = TcpListener::bind("0.0.0.0:8080").expect("bind 0.0.0.0:8080");
    println!("Server listening on port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(e) => eprintln!("Unable to accept connection: {e}"),
        }
    }
}

// handle requests
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 4096];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /users") => handle_post_request(r),
                r if r.starts_with("GET /users/") => handle_get_request(r),
                r if r.starts_with("GET /users") => handle_get_all_request(r),
                r if r.starts_with("PUT /users/") => handle_put_request(r),
                r if r.starts_with("DELETE /users/") => handle_delete_request(r),
                _ => (NOT_FOUND.to_string(), "404 not found".to_string()),
            };

            if let Err(e) = stream.write_all(format!("{}{}", status_line, content).as_bytes()) {
                eprintln!("Unable to write response: {e}");
            }
        }
        Err(e) => eprintln!("Unable to read stream: {e}"),
    }
}

// handle POST /users
fn handle_post_request(request: &str) -> (String, String) {
    match (get_user_request_body(request), Client::connect(DB_URL.as_str(), NoTls)) {
        (Ok(user), Ok(mut client)) => {
            // RETURNING id so we can respond with created user
            match client.query_one(
                "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, name, email",
                &[&user.name, &user.email],
            ) {
                Ok(row) => {
                    let user = User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    };
                    (OK_RESPONSE.to_string(), serde_json::to_string(&user).unwrap_or_else(|_| "{}".to_string()))
                }
                Err(_) => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
            }
        }
        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

// handle GET /users/{id}
fn handle_get_request(request: &str) -> (String, String) {
    match (get_id(request).parse::<i32>(), Client::connect(DB_URL.as_str(), NoTls)) {
        (Ok(id), Ok(mut client)) => match client.query_opt(
            "SELECT id, name, email FROM users WHERE id = $1",
            &[&id],
        ) {
            Ok(Some(row)) => {
                let user = User {
                    id: row.get(0),
                    name: row.get(1),
                    email: row.get(2),
                };
                (OK_RESPONSE.to_string(), serde_json::to_string(&user).unwrap_or_else(|_| "{}".to_string()))
            }
            Ok(None) => (NOT_FOUND.to_string(), "User not found".to_string()),
            Err(_) => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
        },
        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

// handle GET /users
fn handle_get_all_request(_request: &str) -> (String, String) {
    match Client::connect(DB_URL.as_str(), NoTls) {
        Ok(mut client) => match client.query("SELECT id, name, email FROM users", &[]) {
            Ok(rows) => {
                let users: Vec<User> = rows
                    .into_iter()
                    .map(|row| User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    })
                    .collect();
                (OK_RESPONSE.to_string(), serde_json::to_string(&users).unwrap_or_else(|_| "[]".to_string()))
            }
            Err(_) => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
        },
        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

// handle PUT /users/{id}
fn handle_put_request(request: &str) -> (String, String) {
    match (
        get_id(request).parse::<i32>(),
        get_user_request_body(request),
        Client::connect(DB_URL.as_str(), NoTls),
    ) {
        (Ok(id), Ok(user), Ok(mut client)) => {
            match client.execute(
                "UPDATE users SET name = $1, email = $2 WHERE id = $3",
                &[&user.name, &user.email, &id],
            ) {
                Ok(n) if n > 0 => (OK_RESPONSE.to_string(), "User updated".to_string()),
                Ok(_) => (NOT_FOUND.to_string(), "User not found".to_string()),
                Err(_) => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
            }
        }
        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

// handle DELETE /users/{id}
fn handle_delete_request(request: &str) -> (String, String) {
    match (get_id(request).parse::<i32>(), Client::connect(DB_URL.as_str(), NoTls)) {
        (Ok(id), Ok(mut client)) => match client.execute("DELETE FROM users WHERE id = $1", &[&id]) {
            Ok(0) => (NOT_FOUND.to_string(), "User not found".to_string()),
            Ok(_) => (OK_RESPONSE.to_string(), "User deleted".to_string()),
            Err(_) => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
        },
        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

// db setup
fn set_database() -> Result<(), PostgresError> {
    let mut client = Client::connect(DB_URL.as_str(), NoTls)?;
    client.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE
        );
        ",
    )?;
    Ok(())
}

// Get id from request URL
fn get_id(request: &str) -> &str {
    request
        .split('/')
        .nth(2)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

// deserialize user from request body without id
fn get_user_request_body(request: &str) -> Result<User, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}
