//! # slml-syntax
//!
//! Front end of the SLML document language: source text → tokens → syntax
//! tree → components expanded → validated, typed [`model::Document`].
//!
//! ```
//! let doc = slml_syntax::compile(r#"
//!     slml 0.1
//!     page { text "Hello" }
//! "#).unwrap();
//! assert_eq!(doc.page.children.len(), 1);
//! ```

pub mod analyze;
pub mod ast;
pub mod diagnostic;
pub mod expand;
pub mod lexer;
pub mod model;
pub mod parser;

use std::collections::HashSet;

pub use diagnostic::{Diagnostic, SourceMap, Span};

/// Parses source text into an untyped syntax tree.
pub fn parse(src: &str) -> Result<ast::Document, Diagnostic> {
    parser::parse(src, 0)
}

/// Resolves `use "path.slml"` imports. Front ends supply this; the syntax
/// crate itself never touches the file system.
pub trait Loader {
    /// `from` is the name of the importing file (as returned by an earlier
    /// call, or the main file's name), `path` is the string written after
    /// `use`. Returns the imported file's name and its source text.
    fn load(&mut self, from: &str, path: &str) -> Result<(String, String), String>;
}

/// A loader for single documents: every `use` is an error.
pub struct NoImports;

impl Loader for NoImports {
    fn load(&mut self, _from: &str, _path: &str) -> Result<(String, String), String> {
        Err("imports are not available when compiling a single string".into())
    }
}

/// The result of [`compile_with`]: the document or its errors, plus every
/// source file involved so errors can be printed.
pub struct Compiled {
    pub document: Result<model::Document, Vec<Diagnostic>>,
    pub sources: SourceMap,
}

/// Parses and validates a single self-contained document. Returns every
/// error found.
pub fn compile(src: &str) -> Result<model::Document, Vec<Diagnostic>> {
    compile_with("input.slml", src, &mut NoImports).document
}

/// Compiles a document whose `use` imports are resolved by `loader`.
pub fn compile_with(name: &str, src: &str, loader: &mut dyn Loader) -> Compiled {
    let mut sources = SourceMap::default();
    sources.add(name, src);
    let main = match parser::parse(src, 0) {
        Ok(doc) => doc,
        Err(d) => {
            return Compiled {
                document: Err(vec![d]),
                sources,
            };
        }
    };

    let mut errors = Vec::new();
    let mut libraries = Vec::new();
    let mut loaded = HashSet::new();
    let mut stack = vec![name.to_string()];
    load_imports(
        &main,
        name,
        loader,
        &mut sources,
        &mut libraries,
        &mut loaded,
        &mut stack,
        &mut errors,
    );

    let styles: Vec<&ast::StyleDef> = libraries
        .iter()
        .flat_map(|l: &ast::Document| l.styles.iter())
        .chain(main.styles.iter())
        .collect();
    let defines: Vec<&ast::Define> = libraries
        .iter()
        .flat_map(|l| l.defines.iter())
        .chain(main.defines.iter())
        .collect();

    let mut expander = expand::Expander::new(&defines, &analyze::element_names());
    let nodes = expander.expand(&main.nodes);
    errors.append(&mut expander.errors);

    let document = analyze::analyze(
        analyze::Input {
            version: &main.version,
            version_span: main.version_span,
            styles,
            nodes: &nodes,
        },
        errors,
    );
    Compiled { document, sources }
}

#[allow(clippy::too_many_arguments)]
fn load_imports(
    doc: &ast::Document,
    from: &str,
    loader: &mut dyn Loader,
    sources: &mut SourceMap,
    libraries: &mut Vec<ast::Document>,
    loaded: &mut HashSet<String>,
    stack: &mut Vec<String>,
    errors: &mut Vec<Diagnostic>,
) {
    for (path, span) in &doc.uses {
        if let Err(msg) = analyze::check_path(path) {
            errors.push(Diagnostic::new(
                *span,
                format!("invalid import path: {msg}"),
            ));
            continue;
        }
        if !path.ends_with(".slml") {
            errors.push(Diagnostic::new(*span, "only `.slml` files can be imported"));
            continue;
        }
        let (id, src) = match loader.load(from, path) {
            Ok(found) => found,
            Err(msg) => {
                errors.push(Diagnostic::new(
                    *span,
                    format!("cannot import `{path}`: {msg}"),
                ));
                continue;
            }
        };
        if stack.contains(&id) {
            errors.push(
                Diagnostic::new(*span, format!("`{path}` imports itself"))
                    .with_hint(format!("import chain: {} → {id}", stack.join(" → "))),
            );
            continue;
        }
        if !loaded.insert(id.clone()) {
            continue;
        }
        let file = sources.add(id.clone(), src.clone());
        match parser::parse(&src, file) {
            Err(d) => errors.push(d),
            Ok(lib) => {
                if let Some(node) = lib.nodes.first() {
                    errors.push(
                        Diagnostic::new(
                            *span,
                            format!("`{path}` contains `{}`; only `style`, `define` and `use` can be imported", node.name),
                        )
                        .with_hint("move shared parts into a file without `meta` and `page`"),
                    );
                }
                if lib.version != doc.version {
                    errors.push(Diagnostic::new(
                        lib.version_span,
                        format!(
                            "`{path}` is SLML {}, but the importing file is SLML {}",
                            lib.version, doc.version
                        ),
                    ));
                }
                stack.push(id.clone());
                load_imports(&lib, &id, loader, sources, libraries, loaded, stack, errors);
                stack.pop();
                libraries.push(lib);
            }
        }
    }
}

/// A file with no top-level elements (only `use`, `style` and `define`) is
/// a library meant to be imported, not a page.
pub fn is_library(src: &str) -> bool {
    parse(src).is_ok_and(|d| d.nodes.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Files(HashMap<&'static str, &'static str>);

    impl Loader for Files {
        fn load(&mut self, _from: &str, path: &str) -> Result<(String, String), String> {
            self.0
                .get(path.trim_start_matches('/'))
                .map(|s| (path.trim_start_matches('/').to_string(), s.to_string()))
                .ok_or_else(|| "no such file".to_string())
        }
    }

    #[test]
    fn imports_styles_and_components() {
        let mut files = Files(HashMap::from([
            (
                "parts.slml",
                "slml 0.1 use \"/colors.slml\" define hello(name) { text.big $name }",
            ),
            ("colors.slml", "slml 0.1 style big(size: 30)"),
        ]));
        let out = compile_with(
            "page.slml",
            r#"slml 0.1 use "parts.slml" page { hello(name: "Ann") }"#,
            &mut files,
        );
        let doc = out.document.unwrap();
        assert_eq!(doc.page.children.len(), 1);
        assert_eq!(out.sources.files.len(), 3);
    }

    #[test]
    fn import_errors_point_into_the_right_file() {
        let mut files = Files(HashMap::from([
            ("bad.slml", "slml 0.1\nstyle x(colr: #fff)"),
            ("page.slml", "slml 0.1 page { }"),
            ("loop.slml", "slml 0.1 use \"loop.slml\""),
        ]));
        let out = compile_with(
            "main.slml",
            r#"slml 0.1 use "bad.slml" use "page.slml" use "missing.slml" use "loop.slml" use "../x.slml" page { }"#,
            &mut files,
        );
        let errs = out.document.unwrap_err();
        let msgs: Vec<String> = errs.iter().map(|d| out.sources.render(d)).collect();
        assert_eq!(errs.len(), 5, "{msgs:#?}");
        assert!(msgs.iter().any(|m| m.contains("bad.slml:2")), "{msgs:#?}");
    }

    #[test]
    fn library_detection() {
        assert!(is_library("slml 0.1 style a(size: 3)"));
        assert!(!is_library("slml 0.1 page { }"));
    }
}
