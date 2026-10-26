//! Compiling pages and whole sites to HTML (`slml page.slml`, `slml site/`).

use std::path::{Component, Path, PathBuf};

use slml_render::fonts::FONT_FILES;
use slml_syntax::analyze::is_external;
use slml_syntax::model::{Container, ContainerKind, Document, Inline, ListEntry, Node};

use crate::project::{compile_file, resolve};

/// The stylesheet shared by all pages, with the bundled fonts.
pub fn stylesheet() -> String {
    let faces: Vec<slml_html::FontFace> = FONT_FILES
        .iter()
        .map(|(file, family, weight, italic, _)| (*file, *family, *weight, *italic))
        .collect();
    slml_html::stylesheet(&faces)
}

/// Writes `_slml/slml.css` and `_slml/fonts/*` into an output root.
pub fn write_assets(out_root: &Path) -> Result<(), String> {
    let dir = out_root.join(slml_html::ASSET_DIR);
    let fonts = dir.join("fonts");
    std::fs::create_dir_all(&fonts)
        .map_err(|e| format!("error: cannot create {}: {e}", fonts.display()))?;
    write(&dir.join(slml_html::STYLESHEET), stylesheet().as_bytes())?;
    for (file, _, _, _, bytes) in FONT_FILES {
        let path = fonts.join(file);
        let same = std::fs::metadata(&path).is_ok_and(|m| m.len() == bytes.len() as u64);
        if !same {
            write(&path, bytes)?;
        }
    }
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("error: cannot create {}: {e}", parent.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("error: cannot write {}: {e}", path.display()))
}

/// Lexically normalized absolute path (no file system access).
pub fn normalize(path: &Path) -> PathBuf {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// `"../../"` style prefix leading from `from_dir` up to `root`, or `None`
/// when `from_dir` is not inside `root`.
pub fn prefix_to(root: &Path, from_dir: &Path) -> Option<String> {
    let rel = normalize(from_dir)
        .strip_prefix(normalize(root))
        .ok()?
        .to_path_buf();
    Some("../".repeat(rel.components().count()))
}

/// HTML for one page. `body_end` is extra markup before `</body>`.
pub fn page_html(doc: &Document, prefix: &str, body_end: &str) -> String {
    slml_html::page(
        doc,
        &slml_html::Options {
            root_prefix: prefix.to_string(),
            body_end: body_end.to_string(),
        },
    )
}

/// All link targets in a document (`to` on links, buttons and boxes).
pub fn links(doc: &Document) -> Vec<String> {
    fn inlines(content: &[Inline], out: &mut Vec<String>) {
        for i in content {
            if let Inline::Span(s) = i {
                out.extend(s.link.clone());
                inlines(&s.content, out);
            }
        }
    }
    fn list(l: &slml_syntax::model::List, out: &mut Vec<String>) {
        for e in &l.items {
            match e {
                ListEntry::Item(t) => inlines(&t.content, out),
                ListEntry::List(sub) => list(sub, out),
            }
        }
    }
    fn container(c: &Container, out: &mut Vec<String>) {
        out.extend(c.frame.link.clone());
        for n in &c.children {
            match n {
                Node::Container(c) => container(c, out),
                Node::Text(t) => {
                    out.extend(t.frame.link.clone());
                    inlines(&t.content, out);
                }
                Node::Image(i) => out.extend(i.frame.link.clone()),
                Node::List(l) => list(l, out),
                Node::Field(_) | Node::Divider(_) | Node::Spacer(_) => {}
            }
        }
        if let ContainerKind::Form(f) = &c.kind
            && !is_external(&f.to)
        {
            out.push(f.to.clone());
        }
    }
    let mut out = Vec::new();
    container(&doc.page, &mut out);
    out
}

/// Links to local files that do not exist. In SLML a broken link is an
/// error, like any other mistake.
pub fn broken_links(doc: &Document, page: &Path, root: &Path) -> Vec<String> {
    links(doc)
        .into_iter()
        .filter(|to| !is_external(to) && !to.starts_with('#'))
        .filter(|to| {
            let path = to.split('#').next().unwrap_or("");
            let target = resolve(root, page, path);
            let public = normalize(&target)
                .strip_prefix(normalize(root))
                .is_ok_and(public_path);
            !public || crate::project::confined_path(root, &target).is_err() || !target.is_file()
        })
        .map(|to| {
            format!(
                "error: broken link in {}: `{to}` does not exist",
                page.display()
            )
        })
        .collect()
}

/// `slml a.slml b.slml [-o out.html]`: compiles single pages next to
/// themselves (or to `output`). Returns the number of failed files.
pub fn compile_pages(
    files: &[PathBuf],
    output: Option<&Path>,
    root: Option<&Path>,
    quiet: bool,
) -> usize {
    let mut failed = 0;
    for file in files {
        let root = root
            .map(Path::to_path_buf)
            .unwrap_or_else(|| crate::project::default_root(file));
        let out = output
            .map(Path::to_path_buf)
            .unwrap_or_else(|| file.with_extension("html"));
        match compile_one(file, &root, &out) {
            Ok(()) => {
                if !quiet {
                    println!("{} → {}", file.display(), out.display());
                }
            }
            Err(e) => {
                eprintln!("{e}\n");
                failed += 1;
            }
        }
    }
    failed
}

fn compile_one(file: &Path, root: &Path, out: &Path) -> Result<(), String> {
    let doc = compile_file(file, root)?;
    let broken = broken_links(&doc, file, root);
    if !broken.is_empty() {
        return Err(broken.join("\n"));
    }
    let out_dir = crate::project::base_dir(out);
    // Root paths stay valid when the output lies inside the site root;
    // otherwise the output folder acts as the root.
    let (prefix, out_root) = match prefix_to(root, &out_dir) {
        Some(p) => (p, root.to_path_buf()),
        None => (String::new(), out_dir.clone()),
    };
    write_assets(&out_root)?;
    write(out, page_html(&doc, &prefix, "").as_bytes())
}

/// Folders never scanned as part of a site.
fn skipped(name: &str) -> bool {
    name.starts_with('.') || name.starts_with('_') || name == "target" || name == "node_modules"
}

/// Only public site files are built or served. Working notes are private.
pub fn public_path(path: &Path) -> bool {
    let parts: Vec<_> = path.components().collect();
    parts.iter().enumerate().all(|(i, part)| {
        let Component::Normal(name) = part else {
            return false;
        };
        let name = name.to_string_lossy();
        if i + 1 < parts.len() {
            !skipped(&name)
        } else {
            !name.starts_with('.')
                && !matches!(
                    name.to_ascii_lowercase().as_str(),
                    "claude.md" | "plan.md" | "progress.md"
                )
        }
    })
}

/// All files in a site, in a stable order.
pub fn site_files(dir: &Path, exclude: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let exclude = normalize(exclude);
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if !skipped(&name) && normalize(&path) != exclude {
                    stack.push(path);
                }
            } else if kind.is_file() && public_path(path.strip_prefix(dir).unwrap_or(&path)) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// `slml site/ -o dist/`: compiles every page, copies every other file and
/// writes the shared assets. Returns the number of failed pages.
pub fn build_site(dir: &Path, out: &Path, quiet: bool) -> usize {
    let mut failed = 0;
    let mut pages = 0;
    for file in site_files(dir, out) {
        let rel = file.strip_prefix(dir).unwrap_or(&file).to_path_buf();
        if file.extension().is_some_and(|e| e == "slml") {
            if !crate::project::is_page(&file) {
                continue; // a library of styles and components
            }
            let target = out.join(&rel).with_extension("html");
            let depth = rel.components().count().saturating_sub(1);
            let result = compile_file(&file, dir).and_then(|doc| {
                let broken = broken_links(&doc, &file, dir);
                if broken.is_empty() {
                    write(
                        &target,
                        page_html(&doc, &"../".repeat(depth), "").as_bytes(),
                    )
                } else {
                    Err(broken.join("\n"))
                }
            });
            match result {
                Ok(()) => pages += 1,
                Err(e) => {
                    eprintln!("{e}\n");
                    failed += 1;
                }
            }
        } else {
            let target = out.join(&rel);
            let copied = std::fs::create_dir_all(crate::project::base_dir(&target))
                .and_then(|_| std::fs::copy(&file, &target));
            if let Err(e) = copied {
                eprintln!("error: cannot copy {}: {e}", file.display());
                failed += 1;
            }
        }
    }
    if let Err(e) = write_assets(out) {
        eprintln!("{e}");
        failed += 1;
    }
    if !quiet && failed == 0 {
        println!(
            "built {pages} page{} into {}",
            if pages == 1 { "" } else { "s" },
            out.display()
        );
    }
    failed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes() {
        assert_eq!(
            prefix_to(Path::new("/a"), Path::new("/a")),
            Some(String::new())
        );
        assert_eq!(
            prefix_to(Path::new("/a"), Path::new("/a/b/c")),
            Some("../../".into())
        );
        assert_eq!(prefix_to(Path::new("/a"), Path::new("/x")), None);
    }

    #[test]
    fn working_files_are_not_public() {
        for path in [
            ".env",
            "a/.git/config",
            "_drafts/a.slml",
            "target/a.txt",
            "node_modules/pkg/a.js",
            "CLAUDE.md",
            "notes/Plan.md",
            "progress.md",
            "../outside.txt",
        ] {
            assert!(!public_path(Path::new(path)), "{path}");
        }
        for path in ["index.slml", "docs/guide.md", "img/logo.png", "_redirects"] {
            assert!(public_path(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn finds_links() {
        let doc = slml_syntax::compile(
            r##"slml 0.1 page { heading(id: "x") "h" column(to: "a.slml") { text "a" } text { link(to: "b.slml#x") "b" link(to: "#x") "c" } list { item { link(to: "https://e.com") "e" } } }"##,
        )
        .unwrap();
        assert_eq!(
            links(&doc),
            vec!["a.slml", "b.slml#x", "#x", "https://e.com"]
        );
    }
}
