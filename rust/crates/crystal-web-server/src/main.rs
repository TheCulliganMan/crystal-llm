use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::thread;

use anyhow::{Context, Result, bail};

const DEFAULT_PORT: u16 = 8080;

fn main() -> Result<()> {
    let config = Config::parse(env::args().skip(1))?;
    let root = fs::canonicalize(&config.root)
        .with_context(|| format!("resolve web root {}", config.root.display()))?;
    if !root.is_dir() {
        bail!("web root {} is not a directory", root.display());
    }

    let address = SocketAddr::new(config.host, config.port);
    let listener = TcpListener::bind(address)
        .with_context(|| format!("bind web server to http://{address}"))?;
    println!("Serving {} at http://{address}", root.display());

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                let root = root.clone();
                thread::spawn(move || {
                    if let Err(error) = serve_connection(stream, &root) {
                        eprintln!("request failed: {error:#}");
                    }
                });
            }
            Err(error) => eprintln!("accept failed: {error}"),
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Config {
    root: PathBuf,
    host: IpAddr,
    port: u16,
}

impl Config {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut root = PathBuf::from("web-dist");
        let mut host = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let mut port = DEFAULT_PORT;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--dir" => root = PathBuf::from(args.next().context("--dir requires a path")?),
                "--host" => {
                    host = args
                        .next()
                        .context("--host requires an IP address")?
                        .parse()
                        .context("parse --host IP address")?;
                }
                "--port" => {
                    port = args
                        .next()
                        .context("--port requires a number")?
                        .parse()
                        .context("parse --port number")?;
                }
                "-h" | "--help" => {
                    println!(
                        "crystal-web-server [--dir web-dist] [--host 127.0.0.1] [--port 8080]"
                    );
                    std::process::exit(0);
                }
                other => bail!("unknown argument '{other}'"),
            }
        }
        Ok(Self { root, host, port })
    }
}

fn serve_connection(mut stream: TcpStream, root: &Path) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone().context("clone request stream")?);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .context("read request line")?;
    let mut fields = request_line.split_whitespace();
    let method = fields.next().unwrap_or("");
    let target = fields.next().unwrap_or("");
    if !matches!(method, "GET" | "HEAD") {
        return send(
            &mut stream,
            405,
            "text/plain; charset=utf-8",
            b"method not allowed\n",
            method == "HEAD",
        );
    }
    if target == "/healthz" {
        return send(
            &mut stream,
            200,
            "text/plain; charset=utf-8",
            b"ok\n",
            method == "HEAD",
        );
    }

    let Some(relative) = safe_request_path(target) else {
        return send(
            &mut stream,
            400,
            "text/plain; charset=utf-8",
            b"bad request\n",
            method == "HEAD",
        );
    };
    let mut path = root.join(relative);
    if path.is_dir() {
        path = path.join("index.html");
    }
    if !path.is_file() {
        return send(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"not found\n",
            method == "HEAD",
        );
    }
    let body = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    send(
        &mut stream,
        200,
        content_type(&path),
        &body,
        method == "HEAD",
    )
}

fn safe_request_path(target: &str) -> Option<PathBuf> {
    let path = target.split(['?', '#']).next()?.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let mut safe = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(value) => safe.push(value),
            _ => return None,
        }
    }
    Some(safe)
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "crystalpack" => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

fn send(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head: bool,
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nCross-Origin-Opener-Policy: same-origin\r\nCross-Origin-Embedder-Policy: require-corp\r\nCross-Origin-Resource-Policy: same-origin\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head {
        stream.write_all(body)?;
    }
    stream.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_paths_reject_traversal_and_strip_queries() {
        assert_eq!(
            safe_request_path("/pkg/game.wasm?v=1"),
            Some(PathBuf::from("pkg/game.wasm"))
        );
        assert_eq!(safe_request_path("/"), Some(PathBuf::from("index.html")));
        assert_eq!(safe_request_path("/../secret"), None);
        assert_eq!(
            safe_request_path("/pkg/./game.wasm"),
            Some(PathBuf::from("pkg/game.wasm"))
        );
    }

    #[test]
    fn wasm_has_the_required_mime_type() {
        assert_eq!(content_type(Path::new("game.wasm")), "application/wasm");
        assert_eq!(
            content_type(Path::new("game.crystalpack")),
            "application/octet-stream"
        );
    }
}
