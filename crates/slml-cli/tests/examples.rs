//! Every file in `examples/` must be valid and must render.

use std::path::{Path, PathBuf};
use std::process::Command;

fn slml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            slml_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "slml") {
            out.push(path);
        }
    }
}

#[test]
fn all_examples_compile_and_render() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut files = Vec::new();
    slml_files(&root, &mut files);
    assert!(!files.is_empty(), "no examples found in {}", root.display());
    let out = std::env::temp_dir().join(format!("slml-examples-{}.png", std::process::id()));
    for file in files {
        let src = std::fs::read_to_string(&file).unwrap();
        if slml_syntax::is_library(&src) {
            continue;
        }
        let result = Command::new(env!("CARGO_BIN_EXE_slml"))
            .arg("render")
            .arg(&file)
            .arg("--root")
            .arg(&root)
            .args(["--width", "800", "-q", "-o"])
            .arg(&out)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}: {}",
            file.display(),
            String::from_utf8_lossy(&result.stderr)
        );
        let bytes = std::fs::read(&out).unwrap();
        assert!(
            bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "{}",
            file.display()
        );
        std::fs::remove_file(&out).unwrap();
    }
}
