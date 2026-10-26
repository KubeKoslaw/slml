//! `slml upgrade`: rewrites files in the compact `slml 0.2` syntax.
//!
//! `slml 0.1` files are converted, `slml 0.2` files are reformatted. Before
//! anything is written, every page is compiled twice, from the old sources
//! and from the new ones (imports included), and the two documents must be
//! identical. If any page would change, nothing is written.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use slml_syntax::model::Document;
use slml_syntax::{Loader, SourceMap};

use crate::project::{confined_path, resolve};

/// Serves files from disk, except those in `overrides`.
struct Sources<'a> {
    root: &'a Path,
    overrides: &'a HashMap<PathBuf, String>,
}

impl Loader for Sources<'_> {
    fn load(&mut self, from: &str, path: &str) -> Result<(String, String), String> {
        let full = confined_path(self.root, &resolve(self.root, Path::new(from), path))?;
        let src = match self.overrides.get(&full) {
            Some(s) => s.clone(),
            None => std::fs::read_to_string(&full).map_err(|e| e.to_string())?,
        };
        Ok((full.display().to_string(), src))
    }
}

fn compile(
    file: &Path,
    src: &str,
    root: &Path,
    overrides: &HashMap<PathBuf, String>,
) -> Result<Document, (Vec<slml_syntax::Diagnostic>, SourceMap)> {
    let out = slml_syntax::compile_with(
        &file.display().to_string(),
        src,
        &mut Sources { root, overrides },
    );
    out.document.map_err(|e| (e, out.sources))
}

pub struct Options {
    /// Only report files that would change.
    pub check: bool,
    pub quiet: bool,
}

/// Upgrades `files` (all under `root`). Returns the number of problems.
pub fn run(files: &[PathBuf], root: &Path, opts: &Options) -> usize {
    let mut failed = 0;
    let mut new_sources: HashMap<PathBuf, String> = HashMap::new();
    let mut old_sources: HashMap<PathBuf, String> = HashMap::new();
    let mut changed = Vec::new();

    for file in files {
        let src = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", file.display());
                failed += 1;
                continue;
            }
        };
        let name = file.display().to_string();
        let ast = match slml_syntax::parse(&src) {
            Ok(ast) => ast,
            Err(d) => {
                eprintln!("{}", d.render(&src, &name));
                failed += 1;
                continue;
            }
        };
        let printed = match slml_syntax::print::to_compact(&ast) {
            Ok(p) => p,
            Err(errors) => {
                for d in errors {
                    eprintln!("{}", d.render(&src, &name));
                }
                failed += 1;
                continue;
            }
        };
        let key = confined_path(root, file).unwrap_or_else(|_| file.clone());
        if printed != src {
            changed.push((file.clone(), ast.version.clone()));
        }
        old_sources.insert(key.clone(), src);
        new_sources.insert(key, printed);
    }
    if failed > 0 {
        return failed;
    }

    // Every page must mean exactly the same after the upgrade.
    let empty = HashMap::new();
    for (key, old) in &old_sources {
        if slml_syntax::is_library(old) {
            continue;
        }
        let before = compile(key, old, root, &empty);
        let after = compile(key, &new_sources[key], root, &new_sources);
        match (before, after) {
            (Ok(mut a), Ok(mut b)) => {
                a.version.clear();
                b.version.clear();
                if a != b {
                    eprintln!(
                        "error: upgrading {} would change the page; nothing was written\n(this is a bug in `slml upgrade`, please report it)",
                        key.display()
                    );
                    failed += 1;
                }
            }
            (Err((errors, sources)), _) => {
                eprintln!(
                    "{}\nerror: fix these errors before upgrading",
                    sources.render_all(&errors)
                );
                failed += 1;
            }
            (Ok(_), Err((errors, sources))) => {
                eprintln!(
                    "{}\nerror: the upgraded {} does not compile; nothing was written\n(this is a bug in `slml upgrade`, please report it)",
                    sources.render_all(&errors),
                    key.display()
                );
                failed += 1;
            }
        }
    }
    if failed > 0 {
        return failed;
    }

    for (file, version) in &changed {
        let key = confined_path(root, file).unwrap_or_else(|_| file.clone());
        let verb = if version == "0.2" {
            "formatted"
        } else {
            "upgraded"
        };
        if opts.check {
            println!("would be {verb}: {}", file.display());
            continue;
        }
        if let Err(e) = std::fs::write(file, &new_sources[&key]) {
            eprintln!("error: cannot write {}: {e}", file.display());
            failed += 1;
        } else if !opts.quiet {
            println!("{verb}: {}", file.display());
        }
    }
    if opts.check && !changed.is_empty() {
        failed += changed.len();
    }
    if !opts.quiet && changed.is_empty() {
        println!(
            "all {} file{} already in slml 0.2 format",
            files.len(),
            if files.len() == 1 { "" } else { "s" }
        );
    }
    failed
}
