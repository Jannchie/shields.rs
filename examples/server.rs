//! Minimal badge HTTP server built only on the standard library.
//!
//! Run with:
//!
//! ```bash
//! cargo run --release --example server
//! ```
//!
//! Then request badges like:
//!
//! ```text
//! http://127.0.0.1:8080/badge?label=build&message=passing&message_color=brightgreen
//! http://127.0.0.1:8080/badge?label=version&message=1.2.0&style=for-the-badge&logo=rust
//! ```
//!
//! Rendering is lock-free, so a thread per connection scales with cores.

use shields::{BadgeParamsOwned, BadgeStyle};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    eprintln!("badge server listening on http://127.0.0.1:8080/badge?label=build&message=passing");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    let _ = handle(stream);
                });
            }
            Err(e) => eprintln!("accept failed: {e}"),
        }
    }
    Ok(())
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
    let Some(query) = path.strip_prefix("/badge?").or(path.strip_prefix("/badge")) else {
        return respond(&mut stream, "404 Not Found", "text/plain", "not found");
    };

    let mut params = BadgeParamsOwned::default();
    for pair in query.trim_start_matches('?').split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key {
            "style" => {
                params.style = match value.as_str() {
                    "flat-square" => BadgeStyle::FlatSquare,
                    "plastic" => BadgeStyle::Plastic,
                    "social" => BadgeStyle::Social,
                    "for-the-badge" => BadgeStyle::ForTheBadge,
                    _ => BadgeStyle::Flat,
                }
            }
            "label" => params.label = Some(value),
            "message" => params.message = Some(value),
            "label_color" | "labelColor" => params.label_color = Some(value),
            "message_color" | "color" => params.message_color = Some(value),
            "logo" => params.logo = Some(value),
            "logo_color" | "logoColor" => params.logo_color = Some(value),
            "link" => params.link = Some(value),
            "extra_link" => params.extra_link = Some(value),
            _ => {}
        }
    }

    respond(&mut stream, "200 OK", "image/svg+xml", &params.render())
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
