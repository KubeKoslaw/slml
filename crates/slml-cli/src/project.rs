//! Documents on disk: the site root, `use` imports and compiling files.
//!
//! The **site root** is the folder that paths starting with `/` refer to.
//! It is the current directory when the file is inside it, otherwise the
//! file's own folder; `--root` overrides it.

use std::path::{Path, PathBuf};

use slml_syntax::model::Document;
use slml_syntax::{Loader, SourceMap};

pub fn default_root(file: &Path) -> PathBuf {
    let cwd = std::env::current_dir().ok();
    let abs = std::path::absolute(file).ok();
    match (cwd, abs) {
        (Some(cwd), Some(abs)) if abs.starts_with(&cwd) => cwd,
        _ => base_dir(file),
    }
}

/// The folder a file is in ("." for a bare file name).
pub fn base_dir(file: &Path) -> PathBuf {
    file.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Resolves a site path: `/x` against the root, anything else against the
/// folder of the file that mentions it.
pub fn resolve(root: &Path, from_file: &Path, path: &str) -> PathBuf {
    match path.strip_prefix('/') {
        Some(rest) => root.join(rest),
        None => base_dir(from_file).join(path),
    }
}

struct FsLoader<'a> {
    root: &'a Path,
}

impl Loader for FsLoader<'_> {
    fn load(&mut self, from: &str, path: &str) -> Result<(String, String), String> {
        let full = resolve(self.root, Path::new(from), path);
        let src = std::fs::read_to_string(&full).map_err(|e| e.to_string())?;
        Ok((full.display().to_string(), src))
    }
}

/// Reads and compiles a file. On failure returns all diagnostics, formatted
/// with their source lines.
pub fn compile_file(path: &Path, root: &Path) -> Result<Document, String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("error: cannot read {}: {e}", path.display()))?;
    compile_source(path, &src, root).map_err(|(errors, sources)| errors_text(&errors, &sources))
}

pub fn compile_source(
    path: &Path,
    src: &str,
    root: &Path,
) -> Result<Document, (Vec<slml_syntax::Diagnostic>, SourceMap)> {
    let name = path.display().to_string();
    let out = slml_syntax::compile_with(&name, src, &mut FsLoader { root });
    out.document.map_err(|errors| (errors, out.sources))
}

pub fn errors_text(errors: &[slml_syntax::Diagnostic], sources: &SourceMap) -> String {
    sources.render_all(errors)
}

/// Opens a URL with the system's default program (browser, mail client).
pub fn open_external(url: &str) {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(e) = result {
        eprintln!("slml: cannot open {url}: {e}");
    }
}

/// Is this file a page (as opposed to a library of styles and components)?
pub fn is_page(path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|src| !slml_syntax::is_library(&src))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_root_and_relative_paths() {
        let root = Path::new("/site");
        let file = Path::new("/site/blog/post.slml");
        assert_eq!(
            resolve(root, file, "/img/a.png"),
            PathBuf::from("/site/img/a.png")
        );
        assert_eq!(
            resolve(root, file, "b.slml"),
            PathBuf::from("/site/blog/b.slml")
        );
    }
}
