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
        "_slml/slml.css",
        "_slml/fonts/NotoSans-Regular.ttf",
    ] {
        assert!(out.join(file).is_file(), "missing {file}");
    }
    let index = std::fs::read_to_string(out.join("site/index.html")).unwrap();
    assert!(index.contains("href=\"../_slml/slml.css\""));
    assert!(index.contains("href=\"about.html\""));
}

#[test]
fn compiles_like_a_c_compiler() {
    let dir = temp_dir("single");
    let page = dir.join("page.slml");
    std::fs::write(&page, "slml 0.1\npage { text \"hi\" }\n").unwrap();
    assert!(slml().arg(&page).arg("-q").status().unwrap().success());
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
    let out = slml().arg(&page).output().unwrap();
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
    assert!(slml().arg(&dir).arg("-q").status().unwrap().success());
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
