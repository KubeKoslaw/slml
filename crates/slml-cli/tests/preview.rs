//! Exercise the preview as a real child process, using HTTP over loopback.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

struct Preview {
    child: Child,
    root: PathBuf,
    port: u16,
}

impl Preview {
    fn start() -> Self {
        let root = std::env::temp_dir().join(format!("slml-http-preview-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a folder")).unwrap();
        std::fs::write(
            root.join("parts.slml"),
            "slml 0.1 define greeting { text \"First version\" }",
        )
        .unwrap();
        std::fs::write(
            root.join("index.slml"),
            "slml 0.1 use \"parts.slml\" page { greeting }",
        )
        .unwrap();
        std::fs::write(
            root.join("a folder/index.slml"),
            "slml 0.1 page { text \"Nested page\" }",
        )
        .unwrap();
        std::fs::write(root.join("file.txt"), "A public asset").unwrap();
        std::fs::write(root.join("100%.txt"), "Literal percent").unwrap();
        std::fs::write(root.join("literal%2e%2e.txt"), "Literal escape sequence").unwrap();

        // Occupy the first port to also exercise the server's port fallback.
        let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let first_port = occupied.local_addr().unwrap().port();
        assert!(first_port < 65535);
        let mut child = Command::new(env!("CARGO_BIN_EXE_slml"))
            .arg("serve")
            .arg(&root)
            .arg("--port")
            .arg(first_port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut preview = Self {
            child,
            root,
            port: 0,
        };
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut first_line = String::new();
            let mut reader = BufReader::new(stdout);
            let result = reader.read_line(&mut first_line).map(|_| first_line);
            let _ = send.send(result);
            // Keep stdout open until shutdown, including the second startup line.
            let _ = std::io::copy(&mut reader, &mut std::io::sink());
        });
        let line = receive
            .recv_timeout(Duration::from_secs(10))
            .expect("preview startup timed out")
            .unwrap();
        preview.port = line
            .split("http://127.0.0.1:")
            .nth(1)
            .and_then(|s| s.trim().trim_end_matches('/').parse().ok())
            .unwrap_or_else(|| panic!("missing preview URL: {line}"));
        assert!(preview.port > first_port && preview.port <= first_port.saturating_add(19));
        preview
    }

    fn request(&self, method: &str, path: &str) -> (String, Vec<u8>) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            self.port
        )
        .unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        let split = bytes
            .windows(4)
            .position(|b| b == b"\r\n\r\n")
            .expect("HTTP headers");
        let headers = String::from_utf8(bytes[..split].to_vec()).unwrap();
        let body = bytes[split + 4..].to_vec();
        let length: usize = headers
            .lines()
            .find_map(|l| l.strip_prefix("Content-Length: "))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(body.len(), if method == "HEAD" { 0 } else { length });
        assert!(headers.contains("Cache-Control: no-store"));
        (headers, body)
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn browser_preview_serves_live_pages_assets_and_errors() {
    let preview = Preview::start();
    let (headers, body) = preview.request("GET", "/?search=hello");
    assert!(headers.starts_with("HTTP/1.1 200 OK"));
    assert!(headers.contains("text/html; charset=utf-8"));
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("First version"));
    assert!(html.contains("/_slml/version"), "live reload script");
    let (_, head) = preview.request("HEAD", "/");
    assert!(head.is_empty());
    let (_, asset) = preview.request("GET", "/file.txt");
    assert_eq!(asset, b"A public asset");
    assert_eq!(preview.request("GET", "/100%25.txt").1, b"Literal percent");
    assert_eq!(
        preview.request("GET", "/literal%252e%252e.txt").1,
        b"Literal escape sequence"
    );
    let (headers, css) = preview.request("GET", "/_slml/slml.css");
    assert!(headers.contains("text/css"));
    assert!(String::from_utf8(css).unwrap().contains("@font-face"));
    let (headers, font) = preview.request("GET", "/_slml/fonts/NotoSans-Regular.ttf");
    assert!(headers.starts_with("HTTP/1.1 200 OK"));
    assert!(font.len() > 1000);

    let (headers, _) = preview.request("GET", "/a%20folder?q=hello%20world");
    assert!(headers.starts_with("HTTP/1.1 301"));
    assert!(headers.contains("Location: /a%20folder/?q=hello%20world"));
    let (_, nested) = preview.request("GET", "/a%20folder/?q=hello%20world");
    let nested = String::from_utf8(nested).unwrap();
    assert!(nested.contains("Nested page"));
    assert!(nested.contains("../_slml/slml.css"));
    assert!(
        preview
            .request("GET", "/missing.html")
            .0
            .starts_with("HTTP/1.1 404")
    );
    assert!(preview.request("POST", "/").0.starts_with("HTTP/1.1 405"));
    assert!(
        preview
            .request("GET", "/bad%0d%0aurl")
            .0
            .starts_with("HTTP/1.1 400")
    );

    let (_, first_version) = preview.request("GET", "/_slml/version");
    std::fs::write(
        preview.root.join("parts.slml"),
        "slml 0.1 define greeting { colum }",
    )
    .unwrap();
    let (_, error_version) = preview.request("GET", "/_slml/version");
    assert_ne!(first_version, error_version);
    let (headers, errors) = preview.request("GET", "/");
    assert!(headers.starts_with("HTTP/1.1 500"));
    let errors = String::from_utf8(errors).unwrap();
    assert!(errors.contains("parts.slml"));
    assert!(errors.contains("unknown element `colum`"));
    assert!(errors.contains("did you mean `column`?"));
    assert!(errors.contains("/_slml/version"), "error pages also reload");
    assert!(!errors.contains("First version"), "no partial rendering");

    std::fs::write(
        preview.root.join("parts.slml"),
        "slml 0.1 define greeting { text \"Fixed and reloaded\" }",
    )
    .unwrap();
    assert_ne!(error_version, preview.request("GET", "/_slml/version").1);
    let (headers, fixed) = preview.request("GET", "/");
    assert!(headers.starts_with("HTTP/1.1 200 OK"));
    assert!(
        String::from_utf8(fixed)
            .unwrap()
            .contains("Fixed and reloaded")
    );
}

#[test]
fn preview_rejects_port_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_slml"))
        .args(["serve", "--port", "0"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--port must be a number from 1 to 65535")
    );
}
