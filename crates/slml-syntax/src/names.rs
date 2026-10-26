//! Spellings of the compact `slml 0.2` dialect.
//!
//! Both dialects parse into the same syntax tree with the canonical (0.1)
//! names, so validation, components and every backend are shared. These
//! tables translate between the compact spelling written in files and the
//! canonical names. See SPEC.md §2–4.

/// Which surface syntax a document uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dialect {
    /// `slml 0.1`: long names, `name(attr: value) { … }`.
    #[default]
    Long,
    /// `slml 0.2`: compact, `c{p20 bg#fff t"…"}`.
    Compact,
}

impl Dialect {
    pub fn of_version(version: &str) -> Dialect {
        if version == "0.2" {
            Dialect::Compact
        } else {
            Dialect::Long
        }
    }
}

/// Elements spelled with a word in 0.2: (compact, canonical).
/// Headings (`#`…`####`), lists (`-` `+` `=`), `~` and `|` are symbols.
pub const ELEMENTS: &[(&str, &str)] = &[
    ("page", "page"),
    ("meta", "meta"),
    ("c", "column"),
    ("r", "row"),
    ("grid", "grid"),
    ("form", "form"),
    ("t", "text"),
    ("btn", "button"),
    ("send", "submit"),
    ("code", "codeblock"),
    ("img", "image"),
    ("in", "input"),
    ("area", "textarea"),
    ("check", "checkbox"),
    ("sel", "select"),
    ("opt", "option"),
    ("slot", "slot"),
];

/// Symbol elements: (compact, canonical).
pub const SYMBOLS: &[(&str, &str)] = &[
    ("#", "heading"),
    ("-", "list"),
    ("+", "list"),
    ("=", "list"),
    ("~", "spacer"),
    ("|", "divider"),
];

/// Attributes: (compact key, canonical name).
pub const ATTRS: &[(&str, &str)] = &[
    ("w", "width"),
    ("h", "height"),
    ("nw", "min-width"),
    ("mw", "max-width"),
    ("p", "padding"),
    ("px", "padding-x"),
    ("py", "padding-y"),
    ("bg", "fill"),
    ("r", "radius"),
    ("b", "border"),
    ("bc", "border-color"),
    ("sh", "shadow"),
    ("id", "id"),
    ("g", "gap"),
    ("a", "align"),
    ("j", "justify"),
    ("wrap", "wrap"),
    ("c", "color"),
    ("s", "size"),
    ("cols", "columns"),
    ("cw", "cell-width"),
    ("th", "thickness"),
    ("n", "name"),
    ("k", "kind"),
    ("ph", "placeholder"),
    ("v", "value"),
    ("req", "required"),
    ("rows", "rows"),
    ("lang", "lang"),
    ("desc", "description"),
    ("icon", "icon"),
    ("src", "src"),
    ("alt", "alt"),
    ("method", "method"),
];

/// Bare flags: (compact flag, canonical attribute, value).
/// `b`, `i` and `u` are text flags; `b1` (with a number) is a border.
pub const FLAGS: &[(&str, &str, &str)] = &[
    ("b", "bold", "true"),
    ("i", "italic", "true"),
    ("u", "underline", "true"),
    ("wrap", "wrap", "true"),
    ("req", "required", "true"),
    ("mono", "font", "mono"),
    ("sans", "font", "sans"),
    ("post", "method", "post"),
    ("get", "method", "get"),
];

/// Flags that accept `:0` / `:1` to switch off an inherited value.
pub const BOOL_KEYS: &[(&str, &str)] = &[
    ("b", "bold"),
    ("i", "italic"),
    ("u", "underline"),
    ("wrap", "wrap"),
    ("req", "required"),
    ("on", "checked"),
];

/// One-letter enum values: (canonical attribute, letter, canonical value).
pub const ENUMS: &[(&str, &str, &str)] = &[
    ("align", "s", "start"),
    ("align", "c", "center"),
    ("align", "e", "end"),
    ("align", "x", "stretch"),
    ("justify", "s", "start"),
    ("justify", "c", "center"),
    ("justify", "e", "end"),
    ("justify", "b", "between"),
    ("width", "f", "fill"),
    ("width", "a", "auto"),
    ("height", "f", "fill"),
    ("height", "a", "auto"),
    ("shadow", "n", "none"),
    ("shadow", "s", "small"),
    ("shadow", "l", "large"),
];

pub fn element(compact: &str) -> Option<&'static str> {
    ELEMENTS
        .iter()
        .find(|(c, _)| *c == compact)
        .map(|(_, l)| *l)
}

pub fn attr(compact: &str) -> Option<&'static str> {
    ATTRS.iter().find(|(c, _)| *c == compact).map(|(_, l)| *l)
}

pub fn flag(compact: &str) -> Option<(&'static str, &'static str)> {
    FLAGS
        .iter()
        .find(|(c, _, _)| *c == compact)
        .map(|(_, a, v)| (*a, *v))
}

/// Expands a one-letter enum value (`a:c` → `center`); full words pass.
pub fn enum_value(attr: &str, value: &str) -> Option<&'static str> {
    ENUMS
        .iter()
        .find(|(a, l, _)| *a == attr && *l == value)
        .map(|(_, _, v)| *v)
}

/// The compact spelling of a canonical element (for messages and printing).
pub fn element_compact(long: &str) -> &str {
    ELEMENTS
        .iter()
        .find(|(_, l)| *l == long)
        .map(|(c, _)| *c)
        .or_else(|| SYMBOLS.iter().find(|(_, l)| *l == long).map(|(c, _)| *c))
        .unwrap_or(long)
}

/// The compact spelling of a canonical attribute.
pub fn attr_compact(long: &str) -> &str {
    match long {
        "bold" => "b",
        "italic" => "i",
        "underline" => "u",
        "checked" | "selected" => "on",
        "font" => "mono/sans",
        "level" => "#…####",
        "marker" => "- + =",
        "to" => ">",
        other => ATTRS
            .iter()
            .find(|(_, l)| *l == other)
            .map(|(c, _)| *c)
            .unwrap_or(other),
    }
}

/// Every compact attribute key (for "did you mean" hints).
pub fn all_attr_keys() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = ATTRS.iter().map(|(c, _)| *c).collect();
    keys.extend(FLAGS.iter().map(|(c, _, _)| *c));
    keys.push("on");
    keys.sort_unstable();
    keys.dedup();
    keys
}

/// Every word element of the compact dialect.
pub fn all_elements() -> Vec<&'static str> {
    ELEMENTS.iter().map(|(c, _)| *c).collect()
}

/// Is `word` an attribute key (so `word"…"` is an attribute, not a label)?
pub fn is_attr_key(word: &str) -> bool {
    ATTRS.iter().any(|(c, _)| *c == word) || word == "on"
}
