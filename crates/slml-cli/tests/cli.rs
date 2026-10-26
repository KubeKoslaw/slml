//! End-to-end tests of the `slml` command.

use std::path::{Path, PathBuf};
use std::process::Command;

fn slml() -> Command {
    Command::new(env!("CARGO_BIN_EXE_slml"))
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("slml-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

#[test]
fn builds_a_site() {
    let out = temp_dir("site");
    let status = slml()
        .arg(examples())
        .args(["-o", out.to_str().unwrap(), "-q"])
        .status()
        .unwrap();
    assert!(status.success());
    for file in [
        "hello.html",
        "features.html",
        "site/index.html",
        "site/logo.png",
        "starter/index.html",
        "starter/about.html",
        "starter/contact.html",
        "starter/preview.html",
        "_slml/slml.css",
        "_slml/fonts/NotoSans-Regular.ttf",
    ] {
        assert!(out.join(file).is_file(), "missing {file}");
    }
    let index = std::fs::read_to_string(out.join("site/index.html")).unwrap();
    assert!(index.contains("href=\"../_slml/slml.css\""));
    assert!(index.contains("href=\"about.html\""));
    assert!(!out.join("starter/layout.html").exists());
    let contact = std::fs::read_to_string(out.join("starter/contact.html")).unwrap();
    assert!(contact.contains("action=\"preview.html\" method=\"get\""));
    assert!(contact.contains("name=\"email\""));
    assert!(contact.contains("required"));
}

#[test]
fn html_export_requires_an_explicit_request() {
    let dir = temp_dir("single");
    let page = dir.join("page.slml");
    std::fs::write(&page, "slml 0.1\npage { text \"hi\" }\n").unwrap();
    assert!(
        slml()
            .arg("build")
            .arg(&page)
            .arg("-q")
            .status()
            .unwrap()
            .success()
    );
    assert!(dir.join("page.html").is_file());
    assert!(dir.join("_slml/slml.css").is_file());

    let png = dir.join("page.png");
    assert!(
        slml()
            .arg(&page)
            .arg("-o")
            .arg(&png)
            .arg("-q")
            .status()
            .unwrap()
            .success()
    );
    assert!(png.is_file());
}

#[test]
fn errors_fail_with_exit_code_1() {
    let dir = temp_dir("errors");
    let page = dir.join("bad.slml");
    std::fs::write(
        &page,
        "slml 0.1\npage { colum { } text { link(to: \"missing.slml\") \"x\" } }\n",
    )
    .unwrap();
    let out = slml().arg("check").arg(&page).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("did you mean `column`?"), "{stderr}");

    std::fs::write(
        &page,
        "slml 0.1\npage { text { link(to: \"missing.slml\") \"x\" } }\n",
    )
    .unwrap();
    let out = slml().arg("build").arg(&page).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("broken link"));
    assert!(
        !dir.join("bad.html").exists(),
        "no output for a failed page"
    );
}

#[test]
fn imports_and_root_paths() {
    let dir = temp_dir("imports");
    std::fs::create_dir_all(dir.join("blog")).unwrap();
    std::fs::write(
        dir.join("parts.slml"),
        "slml 0.1\ndefine nav { row(gap: 8) { text { link(to: \"/index.slml\") \"Home\" } } }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("index.slml"),
        "slml 0.1\nuse \"parts.slml\"\npage { nav }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("blog/post.slml"),
        "slml 0.1\nuse \"/parts.slml\"\npage { nav text \"post\" }\n",
    )
    .unwrap();
    let out = dir.join("_site");
    assert!(
        slml()
            .arg("build")
            .arg(&dir)
            .arg("-q")
            .status()
            .unwrap()
            .success()
    );
    let post = std::fs::read_to_string(out.join("blog/post.html")).unwrap();
    assert!(post.contains("href=\"../index.html\""), "{post}");
    assert!(!out.join("parts.html").exists(), "libraries are not pages");
}

#[test]
fn handbook_builds_without_errors() {
    let out = temp_dir("handbook");
    let handbook = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../handbook");
    let result = slml()
        .arg(&handbook)
        .args(["-o", out.to_str().unwrap(), "-q"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(out.join("index.html").is_file());
    assert!(
        !out.join("layout.html").exists(),
        "layout.slml is a library"
    );
}

#[test]
fn links_cannot_target_private_site_files() {
    let dir = temp_dir("private-links");
    std::fs::create_dir_all(dir.join("_drafts")).unwrap();
    std::fs::write(dir.join("_drafts/secret.txt"), "private").unwrap();
    let page = dir.join("index.slml");
    std::fs::write(
        &page,
        "slml 0.1 page { button(to: \"_drafts/secret.txt\") \"Draft\" }",
    )
    .unwrap();
    let output = slml().arg("check").arg(&dir).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("broken link"));
}

#[test]
fn site_links_preserve_literal_file_names() {
    let dir = temp_dir("literal-names");
    std::fs::write(dir.join("report 100%.txt"), "Download").unwrap();
    std::fs::write(
        dir.join("żółć.slml"),
        "slml 0.1 page { text \"Polish page\" }",
    )
    .unwrap();
    std::fs::write(dir.join("index.slml"), "slml 0.1 page { button(to: \"report 100%.txt\") \"Report\" button(to: \"żółć.slml\") \"Read\" }").unwrap();
    let out = dir.join("_site");
    let result = slml().arg("build").arg(&dir).arg("-q").output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(out.join("report 100%.txt").is_file());
    assert!(out.join("żółć.html").is_file());
    let html = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(html.contains("href=\"report%20100%25.txt\""));
    assert!(html.contains("href=\"%C5%BC%C3%B3%C5%82%C4%87.html\""));
}

#[cfg(target_os = "linux")]
#[test]
fn bare_documents_do_not_export_html_or_launch_a_web_browser() {
    let dir = temp_dir("native-default");
    let page = dir.join("index.slml");
    std::fs::write(&page, "slml 0.1 page { text \"Native SLML\" }").unwrap();
    for input in [&page, &dir] {
        let output = slml()
            .arg(input)
            // Force a predictable window-creation error instead of opening a
            // real desktop window in the test runner. An HTML route would
            // succeed and write output, so this exercises the actual dispatch.
            .env_remove("WAYLAND_SOCKET")
            .env("WAYLAND_DISPLAY", "")
            .env("DISPLAY", "")
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("cannot open a window")
                || error.contains("built without the native browser"),
            "{error}"
        );
        assert!(!dir.join("index.html").exists());
        assert!(!dir.join("_site").exists());
        assert!(!dir.join("_slml").exists());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn serve_open_uses_the_native_browser() {
    let dir = temp_dir("serve-native-open");
    std::fs::write(
        dir.join("index.slml"),
        "slml 0.1 page { text \"Native preview\" }",
    )
    .unwrap();
    let output = slml()
        .args(["serve", "--open"])
        .arg(&dir)
        .env_remove("WAYLAND_SOCKET")
        .env("WAYLAND_DISPLAY", "")
        .env("DISPLAY", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("cannot open a window")
            || error.contains("built without the native browser"),
        "{error}"
    );
    assert!(!dir.join("index.html").exists());
    assert!(!dir.join("_site").exists());
}
