//! `slml serve`: a local web server for previewing a site in any browser.
//!
//! Pages are compiled on every request, so the browser always shows the
//! current files. A tiny script (added only by this preview server, never
//! to built sites) polls `/_slml/version` and reloads the page when any file
//! changes. Errors are shown in the browser with the same messages as the
//! command line. The server only listens on 127.0.0.1.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use slml_render::fonts::FONT_FILES;

use crate::build::{broken_links, page_html, public_path, site_files, stylesheet};
use crate::project::{compile_file, confined_path};

const RELOAD_SCRIPT: &str = r#"<script>/* SLML preview: reload on change */(function(){var v=null;setInterval(function(){fetch("/_slml/version",{cache:"no-store"}).then(function(r){return r.text()}).then(function(t){if(v!==null&&t!==v){location.reload()}v=t}).catch(function(){})},500)})();</script>
"#;

#[cfg(slml_web_built)]
const WASM: &[u8] = include_bytes!(env!("SLML_WASM_PATH"));
#[cfg(not(slml_web_built))]
const WASM: &[u8] = &[];

pub fn run(root: PathBuf, port: u16, open: bool, html: bool) -> Result<(), String> {
    if !root.is_dir() {
        return Err(format!("error: {} is not a folder", root.display()));
    }
    if !html && WASM.is_empty() {
        return Err("error: the SLML browser renderer is not built; run `make web` and rebuild slml, or use `--html` to explicitly preview HTML output".into());
    }
    let (listener, port) = (port..=port.saturating_add(19))
        .find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok().map(|l| (l, p)))
        .ok_or_else(|| format!("error: no free port from {port}"))?;
    let url = format!("http://127.0.0.1:{port}/");
    println!("SLML preview of {} at {url}", root.display());
    println!("Pages reload automatically when you save. Press Ctrl+C to stop.");
    if open {
        crate::project::open_external(&url);
    }
    for stream in listener.incoming().flatten() {
        let root = root.clone();
        std::thread::spawn(move || {
            if let Err(e) = handle(stream, &root, html) {
                eprintln!("slml serve: {e}");
            }
        });
    }
    Ok(())
}

struct Response {
    status: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
    location: Option<String>,
}

impl Response {
    fn ok(content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status: "200 OK",
            content_type,
            body,
            location: None,
        }
    }

    fn html(status: &'static str, body: String) -> Self {
        Self {
            status,
            content_type: "text/html; charset=utf-8",
            body: body.into_bytes(),
            location: None,
        }
    }

    fn redirect(to: String) -> Self {
        Self {
            location: Some(to),
            ..Self::html("301 Moved Permanently", String::new())
        }
    }
}

fn handle(stream: TcpStream, root: &Path, html: bool) -> std::io::Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    // Drain the headers; the server needs nothing from them.
    let mut line = String::new();
    while reader.read_line(&mut line)? > 2 {
        line.clear();
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    let response = if method == "GET" || method == "HEAD" {
        if html {
            respond(target, root)
        } else {
            respond_native(target, root)
        }
    } else {
        Response::html("405 Method Not Allowed", "only GET is supported".into())
    };
    let mut out = &stream;
    let location = response
        .location
        .map(|l| format!("Location: {l}\r\n"))
        .unwrap_or_default();
    write!(
        out,
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{location}Cache-Control: no-store\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    )?;
    if method != "HEAD" {
        out.write_all(&response.body)?;
    }
    out.flush()
}

/// Browser chrome is HTML; the document stays SLML and is rendered in WASM.
fn respond_native(target: &str, root: &Path) -> Response {
    let raw_path = target.split(['?', '#']).next().unwrap_or("/");
    let Some(path) = decode(raw_path) else {
        return not_found(target);
    };
    if path.split('/').any(|p| p == "..")
        || path.contains('\\')
        || path.chars().any(char::is_control)
    {
        return Response::html("403 Forbidden", "forbidden".into());
    }
    if path == "/_slml/version" {
        return respond(target, root);
    }
    let asset = match path.strip_prefix("/_slml/web/") {
        Some("viewer.css") => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../web/viewer.css").as_slice(),
        )),
        Some("viewer.mjs") => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../../../web/viewer.mjs").as_slice(),
        )),
        Some("text.mjs") => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../../../web/text.mjs").as_slice(),
        )),
        Some("engine.mjs") => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../../../web/engine.mjs").as_slice(),
        )),
        Some("slml.wasm") => Some(("application/wasm", WASM)),
        _ => None,
    };
    if let Some((kind, bytes)) = asset {
        return Response::ok(kind, bytes.to_vec());
    }
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() || rel.ends_with('/') {
        format!("{rel}index.slml")
    } else {
        rel.to_string()
    };
    if !public_path(Path::new(&rel)) {
        return not_found(&path);
    }
    let Ok(file) = confined_path(root, &root.join(&rel)) else {
        return Response::html("403 Forbidden", "forbidden".into());
    };
    if path.ends_with(".slml") && file.is_file() {
        return match std::fs::read(file) {
            Ok(bytes) => Response::ok("text/slml; charset=utf-8", bytes),
            Err(_) => not_found(&path),
        };
    }
    let source = file.with_extension("slml");
    let Ok(source) = confined_path(root, &source) else {
        return Response::html("403 Forbidden", "forbidden".into());
    };
    if source.is_file() && (rel.ends_with(".html") || file.extension().is_none())
        || path.ends_with('/')
    {
        let source = Path::new(&rel).with_extension("slml");
        let url = slml_html::asset(
            &format!("/{}", source.to_string_lossy()),
            &slml_html::Options {
                root_prefix: "/".into(),
                ..slml_html::Options::default()
            },
        );
        let html = include_str!("../../../web/viewer.html")
            .replace("href=\"viewer.css\"", "href=\"/_slml/web/viewer.css\"")
            .replace("src=\"viewer.mjs\"", "src=\"/_slml/web/viewer.mjs\"")
            .replace(
                "<body>",
                &format!(
                    "<body data-source=\"{}\" data-watch=\"true\">",
                    slml_html::escape(&url)
                ),
            );
        return Response::html("200 OK", html);
    }
    respond(target, root)
}

fn respond(target: &str, root: &Path) -> Response {
    let path = target.split(['?', '#']).next().unwrap_or("/");
    let Some(path) = decode(path) else {
        return not_found(target);
    };
    if path.split('/').any(|c| c == "..") || path.contains('\\') {
        return Response::html("403 Forbidden", "forbidden".into());
    }
    if path.chars().any(char::is_control) {
        return Response::html("400 Bad Request", "invalid URL path".into());
    }

    match path.as_str() {
        "/_slml/version" => return Response::ok("text/plain", version(root).into_bytes()),
        "/_slml/slml.css" => {
            return Response::ok("text/css; charset=utf-8", stylesheet().into_bytes());
        }
        _ => {}
    }
    if let Some(font) = path.strip_prefix("/_slml/fonts/") {
        return match FONT_FILES.iter().find(|f| f.0 == font) {
            Some(f) => Response::ok("font/ttf", f.4.to_vec()),
            None => not_found(&path),
        };
    }

    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() || rel.ends_with('/') {
        format!("{rel}index.html")
    } else {
        rel.to_string()
    };
    if !public_path(Path::new(&rel)) {
        return not_found(&path);
    }
    let Ok(file) = confined_path(root, &root.join(&rel)) else {
        return Response::html("403 Forbidden", "forbidden".into());
    };

    // A page: `x.html` (or `x.slml`) is compiled from `x.slml`.
    let source = match file.extension().and_then(|e| e.to_str()) {
        Some("html") | Some("slml") => Some(file.with_extension("slml")),
        None if file.with_extension("slml").is_file() => Some(file.with_extension("slml")),
        _ => None,
    };
    if let Some(source) = source {
        let Ok(source) = confined_path(root, &source) else {
            return Response::html("403 Forbidden", "forbidden".into());
        };
        if source.is_file() {
            let depth = Path::new(&rel).components().count().saturating_sub(1);
            return page(&source, root, &"../".repeat(depth));
        }
    }
    if file.is_dir() {
        // Keep the original escaping and GET form data in the URL.
        let (raw_path, query) = target.split_once('?').unwrap_or((target, ""));
        let suffix = if query.is_empty() {
            String::new()
        } else {
            format!("?{query}")
        };
        return Response::redirect(format!("{raw_path}/{suffix}"));
    }
    match std::fs::read(&file) {
        Ok(body) => Response::ok(content_type(&file), body),
        Err(_) => not_found(&path),
    }
}

fn page(source: &Path, root: &Path, prefix: &str) -> Response {
    if !source.is_file() {
        return not_found(&source.display().to_string());
    }
    let result = compile_file(source, root).and_then(|doc| {
        let broken = broken_links(&doc, source, root);
        if broken.is_empty() {
            Ok(doc)
        } else {
            Err(broken.join("\n"))
        }
    });
    match result {
        Ok(doc) => Response::html("200 OK", page_html(&doc, prefix, RELOAD_SCRIPT)),
        Err(errors) => {
            eprintln!("{errors}\n");
            Response::html("500 Internal Server Error", error_page(source, &errors))
        }
    }
}

fn error_page(source: &Path, errors: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Error — SLML</title><link rel="stylesheet" href="/_slml/slml.css"></head>
<body style="background:#fff8f7"><main class="s-page s-col s-v" style="padding:32px;gap:14px">
<h2 class="s-text" style="color:#b42318;font-size:24px;font-weight:700">This page has errors</h2>
<p class="s-text" style="color:#55555f">{}</p>
<pre class="s-text s-mono" style="font-size:13px;background:#fff;padding:16px;border-radius:8px;border:1px solid #f1c4c0;overflow:auto">{}</pre>
<p class="s-text" style="color:#55555f">Fix the file and save it; this page reloads automatically.</p>
</main>{RELOAD_SCRIPT}</body></html>"#,
        slml_html::escape(&source.display().to_string()),
        slml_html::escape(errors)
    )
}

fn not_found(path: &str) -> Response {
    Response::html(
        "404 Not Found",
        format!(
            "<!doctype html><meta charset=\"utf-8\"><title>Not found</title><p style=\"font-family:sans-serif;padding:32px\">Not found: <code>{}</code></p>{RELOAD_SCRIPT}",
            slml_html::escape(path)
        ),
    )
}

/// Changes whenever any file in the site changes.
fn version(root: &Path) -> String {
    let mut h = DefaultHasher::new();
    for file in site_files(root, &root.join("_site")) {
        file.hash(&mut h);
        if let Ok(meta) = std::fs::metadata(&file) {
            meta.len().hash(&mut h);
            if let Ok(modified) = meta.modified() {
                modified
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
                    .hash(&mut h);
            }
        }
    }
    format!("{:x}", h.finish())
}

/// Percent-decoding of a URL path.
fn decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("ttf") => "font/ttf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("pdf") => "application/pdf",
        Some("txt" | "slml" | "md") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Site(PathBuf);

    impl Site {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("slml-preview-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, name: &str, body: &str) {
            let file = self.0.join(name);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, body).unwrap();
        }
    }

    impl Drop for Site {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn decodes_urls() {
        assert_eq!(decode("/a%20b/%C5%BC.slml").as_deref(), Some("/a b/ż.slml"));
        assert_eq!(decode("/bad%2"), None);
    }

    #[test]
    fn native_preview_serves_source_and_canvas_without_html_translation() {
        let site = Site::new("native");
        let source = "slml 0.1 page { heading \"Drawn by SLML\" }";
        site.write("index.slml", source);
        site.write("parts.slml", "slml 0.1 style card(padding: 12)");
        let shell = respond_native("/", &site.0);
        let shell = String::from_utf8(shell.body).unwrap();
        assert!(shell.contains("<canvas"));
        assert!(shell.contains("data-source=\"/index.slml\""));
        assert!(
            !shell.contains("Drawn by SLML"),
            "no document is compiled into HTML"
        );
        let raw = respond_native("/index.slml", &site.0);
        assert_eq!(raw.content_type, "text/slml; charset=utf-8");
        assert_eq!(raw.body, source.as_bytes());
        assert!(
            respond_native("/parts.slml", &site.0)
                .body
                .starts_with(b"slml 0.1")
        );
        assert_eq!(respond_native("/_slml/version", &site.0).status, "200 OK");
        assert_eq!(
            respond_native("/.git/config", &site.0).status,
            "404 Not Found"
        );
        assert_eq!(
            respond_native("/_slml/web/text.mjs", &site.0).content_type,
            "text/javascript; charset=utf-8"
        );
        #[cfg(slml_web_built)]
        assert!(
            respond_native("/_slml/web/slml.wasm", &site.0)
                .body
                .starts_with(b"\0asm")
        );
    }

    #[test]
    fn refuses_to_leave_the_root() {
        let r = respond("/../etc/passwd", Path::new("."));
        assert_eq!(r.status, "403 Forbidden");
        let r = respond("/%2e%2e/etc/passwd", Path::new("."));
        assert_eq!(r.status, "403 Forbidden");
    }

    #[test]
    fn private_files_are_neither_served_nor_discovered() {
        let site = Site::new("private");
        site.write("index.slml", "slml 0.1 page { text \"public\" }");
        site.write("docs/readme.txt", "public asset");
        for path in [
            ".env",
            ".git/config",
            "_drafts/a.txt",
            "target/a.txt",
            "CLAUDE.md",
            "notes/plan.md",
        ] {
            site.write(path, "private data");
            let response = respond(&format!("/{path}"), &site.0);
            assert_eq!(response.status, "404 Not Found", "{path}");
            assert!(!String::from_utf8_lossy(&response.body).contains("private data"));
        }
        assert_eq!(respond("/%2egit/config", &site.0).status, "404 Not Found");
        let discovered = site_files(&site.0, &site.0.join("_site"));
        assert_eq!(
            discovered,
            vec![site.0.join("docs/readme.txt"), site.0.join("index.slml")]
        );
        assert_eq!(respond("/", &site.0).status, "200 OK");
        assert_eq!(respond("/docs/readme.txt", &site.0).body, b"public asset");
        assert_eq!(respond("/_slml/slml.css", &site.0).status, "200 OK");
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_publish_or_import_files() {
        use std::os::unix::fs::symlink;
        let site = Site::new("symlinks");
        let outside = Site::new("outside");
        outside.write("secret.txt", "private data");
        outside.write("page.slml", "slml 0.1 page { text \"private page\" }");
        outside.write(
            "parts.slml",
            "slml 0.1 define secret { text \"private component\" }",
        );
        symlink(outside.0.join("secret.txt"), site.0.join("secret.txt")).unwrap();
        symlink(outside.0.join("page.slml"), site.0.join("page.slml")).unwrap();
        symlink(outside.0.join("parts.slml"), site.0.join("parts.slml")).unwrap();
        symlink(&outside.0, site.0.join("outside")).unwrap();
        symlink(&site.0, site.0.join("loop")).unwrap();
        for path in [
            "/secret.txt",
            "/page.html",
            "/page.slml",
            "/outside/secret.txt",
            "/loop/secret.txt",
        ] {
            assert_eq!(respond(path, &site.0).status, "403 Forbidden", "{path}");
        }
        assert!(site_files(&site.0, &site.0.join("_site")).is_empty());
        site.write("index.slml", "slml 0.1 use \"parts.slml\" page { secret }");
        let response = respond("/", &site.0);
        assert_eq!(response.status, "500 Internal Server Error");
        let error = String::from_utf8(response.body).unwrap();
        assert!(
            error.contains("symlinks inside the site are not supported"),
            "{error}"
        );
        assert!(!error.contains("private component"));
    }
}
