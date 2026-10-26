//! Where an element was written: maps a layout path back to the source.
//!
//! The typed model carries no source positions, so this re-reads the files,
//! expands components like the compiler does and follows the same child
//! indices through the syntax tree, whose nodes keep their spans. Elements
//! that come from a component point into its `define`.

use std::path::{Path, PathBuf};

use slml_syntax::ast::{Child, Define, Document, Node};

use crate::project::{confined_path, resolve};

pub struct Sources {
    /// File names by span file id; 0 is the page.
    files: Vec<PathBuf>,
    page: Option<Node>,
}

/// A printable source position, e.g. `index.slml:12:5`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub file: PathBuf,
    pub line: u32,
    pub col: u32,
}

impl Sources {
    /// Reads `path` and its imports. Files that cannot be read or parsed
    /// simply have no locations.
    pub fn load(path: &Path, root: &Path) -> Sources {
        let mut sources = Sources {
            files: vec![path.to_path_buf()],
            page: None,
        };
        let Some(main) = std::fs::read_to_string(path)
            .ok()
            .and_then(|src| slml_syntax::parser::parse(&src, 0).ok())
        else {
            return sources;
        };
        let mut libraries = Vec::new();
        sources.imports(&main, path, root, &mut libraries);
        let defines: Vec<&Define> = libraries
            .iter()
            .flat_map(|l: &Document| &l.defines)
            .chain(&main.defines)
            .collect();
        let names = slml_syntax::analyze::element_names();
        let mut expander = slml_syntax::expand::Expander::new(&defines, &names);
        sources.page = expander
            .expand(&main.nodes)
            .into_iter()
            .find(|n| n.name == "page");
        sources
    }

    fn imports(&mut self, doc: &Document, from: &Path, root: &Path, out: &mut Vec<Document>) {
        for (path, _) in &doc.uses {
            let Ok(file) = confined_path(root, &resolve(root, from, path)) else {
                continue;
            };
            if self.files.contains(&file) {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&file) else {
                continue;
            };
            let id = self.files.len() as u32;
            self.files.push(file.clone());
            if let Ok(lib) = slml_syntax::parser::parse(&src, id) {
                self.imports(&lib, &file, root, out);
                out.push(lib);
            }
        }
    }

    /// Where the element at `path` (see `ElementBox::path`) starts.
    pub fn locate(&self, path: &[usize]) -> Option<Location> {
        let mut node = self.page.as_ref()?;
        for &index in path {
            node = node
                .children
                .iter()
                .filter_map(|c| match c {
                    Child::Node(n) => Some(n),
                    _ => None,
                })
                .nth(index)?;
        }
        Some(Location {
            file: self.files.get(node.span.file as usize)?.clone(),
            line: node.span.line,
            col: node.span.col,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_follow_layout_paths_into_components() {
        let root = std::env::temp_dir().join(format!("slml-source-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let page = root.join("index.slml");
        std::fs::write(
            root.join("parts.slml"),
            "slml 0.1\ndefine card(title) {\n  column { heading $title slot }\n}\n",
        )
        .unwrap();
        std::fs::write(
            &page,
            "slml 0.1\nuse \"parts.slml\"\npage {\n  text \"intro\"\n  card(title: \"T\") { text \"inside\" }\n  list { item \"a\" list { item \"b\" } }\n}\n",
        )
        .unwrap();
        let sources = Sources::load(&page, &root);
        let at = |path: &[usize]| {
            let l = sources.locate(path).unwrap();
            (
                l.file.file_name().unwrap().to_string_lossy().into_owned(),
                l.line,
                l.col,
            )
        };
        assert_eq!(at(&[]), ("index.slml".into(), 3, 1));
        assert_eq!(at(&[0]), ("index.slml".into(), 4, 3));
        // The component's column, its heading, then the slotted text.
        assert_eq!(at(&[1]), ("parts.slml".into(), 3, 3));
        assert_eq!(at(&[1, 0]), ("parts.slml".into(), 3, 12));
        assert_eq!(at(&[1, 1]), ("index.slml".into(), 5, 22));
        assert_eq!(at(&[2, 1, 0]), ("index.slml".into(), 6, 26));
        assert!(sources.locate(&[9]).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
