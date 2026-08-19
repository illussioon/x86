use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

fn response(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut output = header.into_bytes();
    output.extend_from_slice(body);
    output
}

fn handle(mut stream: TcpStream, root: Option<PathBuf>) {
    let mut request = [0u8; 4096];
    let size = stream.read(&mut request).unwrap_or(0);
    let text = String::from_utf8_lossy(&request[..size]);
    let path = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let clean_path = path.split('?').next().unwrap_or("/");
    let response = if clean_path == "/health" {
        response("200 OK", "text/plain; charset=utf-8", b"ok\n")
    } else if clean_path == "/" {
        response(
            "200 OK",
            "text/plain; charset=utf-8",
            b"x86-test-server\nGET /health\n",
        )
    } else if let Some(root) = root {
        let relative = clean_path.trim_start_matches('/');
        let candidate = root.join(relative);
        match std::fs::read(&candidate) {
            Ok(body) => response("200 OK", "application/octet-stream", &body),
            Err(_) => response("404 Not Found", "text/plain; charset=utf-8", b"not found\n"),
        }
    } else {
        response("404 Not Found", "text/plain; charset=utf-8", b"not found\n")
    };
    let _ = stream.write_all(&response);
}

fn main() -> std::io::Result<()> {
    let mut port = 8080u16;
    let mut root = None;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--port" => {
                index += 1;
                port = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(port);
            }
            "--root" => {
                index += 1;
                root = args.get(index).map(PathBuf::from);
            }
            "--help" | "-h" => {
                println!("x86-test-server [--port <port>] [--root <directory>]");
                return Ok(());
            }
            _ => {}
        }
        index += 1;
    }
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!("x86-test-server listening on http://127.0.0.1:{port}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let root = root.clone();
                thread::spawn(move || handle(stream, root));
            }
            Err(error) => eprintln!("connection error: {error}"),
        }
    }
    Ok(())
}
