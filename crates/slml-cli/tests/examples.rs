//! Every file in `examples/` must be valid and must render.

use std::path::{Path, PathBuf};

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
    for file in files {
        let src = std::fs::read_to_string(&file).unwrap();
        let doc = slml_syntax::compile(&src).unwrap_or_else(|errors| {
            let name = file.display().to_string();
            panic!(
                "{}",
                errors
                    .iter()
                    .map(|d| d.render(&src, &name))
                    .collect::<String>()
            )
        });
        let mut renderer = slml_render::Renderer::new(file.parent().unwrap());
        let pixmap = renderer.render_page(&doc, 800, 1.0);
        assert!(pixmap.height() > 0, "{}", file.display());
    }
}
