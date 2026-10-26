//! Source positions and error reporting.
//!
//! SLML never guesses what a broken document meant (unlike HTML's error
//! recovery). Every problem becomes a [`Diagnostic`] pointing at the exact
//! place in the source.

use std::fmt;

/// A position range in the source text. Lines and columns are 1-based,
/// columns count Unicode scalar values. `file` indexes a [`SourceMap`]
/// (0 is the document being compiled, higher numbers are imported files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub file: u32,
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
    /// Optional suggestion, e.g. "did you mean `column`?".
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span,
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Renders the diagnostic with the offending source line and a caret,
    /// in the style of rustc:
    ///
    /// ```text
    /// error: unknown element `colum`
    ///  --> page.slml:4:3
    ///   |
    /// 4 |   colum {
    ///   |   ^^^^^
    ///   = hint: did you mean `column`?
    /// ```
    pub fn render(&self, source: &str, file_name: &str) -> String {
        let line_no = self.span.line.max(1);
        let line_text = source.lines().nth(line_no as usize - 1).unwrap_or("");
        let gutter = line_no.to_string().len();
        let pad = " ".repeat(gutter);
        let col = self.span.col.max(1) as usize;
        let width = source
            .get(self.span.start..self.span.end)
            .map(|s| s.lines().next().unwrap_or("").chars().count())
            .unwrap_or(1)
            .max(1);
        let mut out = format!(
            "error: {}\n{pad}--> {}:{}:{}\n{pad} |\n{} | {}\n{pad} | {}{}\n",
            self.message,
            file_name,
            line_no,
            col,
            line_no,
            line_text,
            " ".repeat(col - 1),
            "^".repeat(width),
        );
        if let Some(hint) = &self.hint {
            out.push_str(&format!("{pad} = hint: {hint}\n"));
        }
        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.span.line, self.span.col, self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, " (hint: {hint})")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Display name, usually the path.
    pub name: String,
    pub src: String,
}

/// All files that took part in a compilation, so diagnostics can be shown
/// with the right source line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMap {
    pub files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn add(&mut self, name: impl Into<String>, src: impl Into<String>) -> u32 {
        self.files.push(SourceFile {
            name: name.into(),
            src: src.into(),
        });
        (self.files.len() - 1) as u32
    }

    pub fn render(&self, d: &Diagnostic) -> String {
        match self.files.get(d.span.file as usize) {
            Some(f) => d.render(&f.src, &f.name),
            None => format!("error: {d}\n"),
        }
    }

    /// All diagnostics plus a summary line.
    pub fn render_all(&self, diagnostics: &[Diagnostic]) -> String {
        let mut out: Vec<String> = diagnostics.iter().map(|d| self.render(d)).collect();
        let main = self
            .files
            .first()
            .map(|f| f.name.as_str())
            .unwrap_or("input");
        let n = diagnostics.len();
        out.push(format!(
            "{n} error{} in {main}",
            if n == 1 { "" } else { "s" }
        ));
        out.join("\n")
    }
}

/// Returns the candidate closest to `word` if it is a plausible typo.
pub(crate) fn suggest<'a>(word: &str, candidates: &[&'a str]) -> Option<&'a str> {
    candidates
        .iter()
        .map(|c| (levenshtein(word, c), *c))
        .filter(|(d, c)| *d <= 2.max(c.len() / 3))
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_close_words_only() {
        let names = ["column", "row", "text"];
        assert_eq!(suggest("colum", &names), Some("column"));
        assert_eq!(suggest("txet", &names), Some("text"));
        assert_eq!(suggest("banana", &names), None);
    }
}
