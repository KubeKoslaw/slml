//! Components: replaces calls of `define`d components with their bodies,
//! substituting `$parameters` and `slot`. Runs before validation, so a
//! component's body is checked exactly like hand-written content.
//! See SPEC.md §4.4.

use std::collections::HashMap;

use crate::ast::{Attr, Child, Define, Node, Value};
use crate::diagnostic::{Diagnostic, Span, suggest};

/// How deep components may call other components. Protects against
/// accidental recursion (`define a { a }`).
const MAX_DEPTH: usize = 32;

type Env = HashMap<String, (Value, Span)>;

pub struct Expander<'a> {
    defines: HashMap<&'a str, &'a Define>,
    pub errors: Vec<Diagnostic>,
}

#[derive(Clone, Copy)]
struct Ctx<'c> {
    env: Option<&'c Env>,
    slot: Option<&'c [Child]>,
    depth: usize,
}

impl<'a> Expander<'a> {
    /// `reserved` are names that cannot be used for components (the
    /// built-in elements).
    pub fn new(defines: &[&'a Define], reserved: &[&str]) -> Self {
        let mut ex = Expander {
            defines: HashMap::new(),
            errors: Vec::new(),
        };
        for def in defines {
            if reserved.contains(&def.name.as_str()) || def.name == "slot" {
                ex.errors.push(Diagnostic::new(
                    def.span,
                    format!(
                        "`{}` is a built-in element and cannot be redefined",
                        def.name
                    ),
                ));
                continue;
            }
            if let Some(prev) = ex.defines.insert(&def.name, def) {
                ex.errors.push(
                    Diagnostic::new(
                        def.span,
                        format!("component `{}` is defined twice", def.name),
                    )
                    .with_hint(format!(
                        "the first definition is on line {}",
                        prev.span.line
                    )),
                );
            }
            let mut seen = Vec::new();
            for p in &def.params {
                if seen.contains(&p.name.as_str()) {
                    ex.errors.push(Diagnostic::new(
                        p.span,
                        format!("parameter `{}` is declared twice", p.name),
                    ));
                }
                seen.push(&p.name);
                if let Some((Value::Param(_), span)) = &p.default {
                    ex.errors.push(Diagnostic::new(
                        *span,
                        "a default value cannot refer to another parameter",
                    ));
                }
            }
        }
        ex
    }

    /// Expands top-level nodes (they may themselves be component calls,
    /// e.g. a `layout` component that produces `meta` and `page`).
    pub fn expand(&mut self, nodes: &[Node]) -> Vec<Node> {
        let children: Vec<Child> = nodes.iter().cloned().map(Child::Node).collect();
        let ctx = Ctx {
            env: None,
            slot: None,
            depth: 0,
        };
        self.children(&children, ctx)
            .into_iter()
            .filter_map(|c| match c {
                Child::Node(n) => Some(n),
                Child::Text(_, span) | Child::Param(_, span) => {
                    self.errors
                        .push(Diagnostic::new(span, "text outside of `page`"));
                    None
                }
            })
            .collect()
    }

    fn children(&mut self, list: &[Child], ctx: Ctx) -> Vec<Child> {
        let mut out = Vec::new();
        for child in list {
            match child {
                Child::Text(..) => out.push(child.clone()),
                Child::Param(name, span) => {
                    if let Some(value) = self.lookup(name, *span, ctx) {
                        match value {
                            Value::Str(s) => out.push(Child::Text(s, *span)),
                            other => self.errors.push(Diagnostic::new(
                                *span,
                                format!(
                                    "`${name}` holds {}; only strings can be shown as text",
                                    other.describe()
                                ),
                            )),
                        }
                    }
                }
                Child::Node(node) => self.node(node, ctx, &mut out),
            }
        }
        out
    }

    fn lookup(&mut self, name: &str, span: Span, ctx: Ctx) -> Option<Value> {
        let Some(env) = ctx.env else {
            self.errors.push(
                Diagnostic::new(span, format!("`${name}` can only be used inside `define`"))
                    .with_hint(
                        "parameters belong to components: `define card(title) { heading $title }`",
                    ),
            );
            return None;
        };
        match env.get(name) {
            Some((v, _)) => Some(v.clone()),
            None => {
                let names: Vec<&str> = env.keys().map(String::as_str).collect();
                let mut d = Diagnostic::new(span, format!("unknown parameter `${name}`"));
                if let Some(s) = suggest(name, &names) {
                    d = d.with_hint(format!("did you mean `${s}`?"));
                }
                self.errors.push(d);
                None
            }
        }
    }

    fn substitute(&mut self, attrs: &[Attr], ctx: Ctx) -> Vec<Attr> {
        attrs
            .iter()
            .filter_map(|a| match &a.value {
                Value::Param(p) => self
                    .lookup(p, a.value_span, ctx)
                    .map(|value| Attr { value, ..a.clone() }),
                _ => Some(a.clone()),
            })
            .collect()
    }

    fn node(&mut self, node: &Node, ctx: Ctx, out: &mut Vec<Child>) {
        let attrs = self.substitute(&node.attrs, ctx);

        if node.name == "slot" {
            match ctx.slot {
                None => self.errors.push(
                    Diagnostic::new(node.span, "`slot` can only be used inside `define`")
                        .with_hint("`slot` marks where a component's content goes"),
                ),
                Some(_)
                    if !attrs.is_empty()
                        || !node.children.is_empty()
                        || !node.classes.is_empty() =>
                {
                    self.errors.push(Diagnostic::new(
                        node.span,
                        "`slot` takes no attributes or content",
                    ))
                }
                Some(slot) => out.extend(slot.iter().cloned()),
            }
            return;
        }

        let Some(def) = self.defines.get(node.name.as_str()).copied() else {
            out.push(Child::Node(Node {
                attrs,
                children: self.children(&node.children, ctx),
                ..node.clone()
            }));
            return;
        };

        if ctx.depth >= MAX_DEPTH {
            self.errors.push(
                Diagnostic::new(
                    node.span,
                    format!("component `{}` is nested too deeply", def.name),
                )
                .with_hint("a component probably uses itself"),
            );
            return;
        }
        if let Some((_, span)) = node.classes.first() {
            self.errors.push(
                Diagnostic::new(
                    *span,
                    format!("styles cannot be applied to component `{}`", def.name),
                )
                .with_hint("apply the style inside the `define`, or pass a parameter"),
            );
        }

        let mut env = Env::new();
        for a in attrs {
            if !def.params.iter().any(|p| p.name == a.name) {
                let names: Vec<&str> = def.params.iter().map(|p| p.name.as_str()).collect();
                let mut d = Diagnostic::new(
                    a.span,
                    format!("component `{}` has no parameter `{}`", def.name, a.name),
                );
                d = match suggest(&a.name, &names) {
                    Some(s) => d.with_hint(format!("did you mean `{s}`?")),
                    None if names.is_empty() => {
                        d.with_hint(format!("`{}` takes no parameters", def.name))
                    }
                    None => d.with_hint(format!("parameters: {}", names.join(", "))),
                };
                self.errors.push(d);
            } else if env.contains_key(&a.name) {
                self.errors.push(Diagnostic::new(
                    a.span,
                    format!("`{}` is set twice", a.name),
                ));
            } else {
                env.insert(a.name.clone(), (a.value, a.value_span));
            }
        }
        for p in &def.params {
            if env.contains_key(&p.name) {
                continue;
            }
            match &p.default {
                Some((v, span)) => {
                    env.insert(p.name.clone(), (v.clone(), *span));
                }
                None => self.errors.push(
                    Diagnostic::new(
                        node.span,
                        format!("component `{}` requires parameter `{}`", def.name, p.name),
                    )
                    .with_hint(format!("e.g. `{}({}: \"...\")`", def.name, p.name)),
                ),
            }
        }

        let content = self.children(&node.children, ctx);
        if !content.is_empty() && !has_slot(&def.body) {
            self.errors.push(
                Diagnostic::new(
                    node.span,
                    format!("component `{}` takes no content", def.name),
                )
                .with_hint("add `slot` to its `define` to show content passed to it"),
            );
        }
        let inner = Ctx {
            env: Some(&env),
            slot: Some(&content),
            depth: ctx.depth + 1,
        };
        let body = self.children(&def.body, inner);
        out.extend(body);
    }
}

fn has_slot(children: &[Child]) -> bool {
    children.iter().any(|c| match c {
        Child::Node(n) => n.name == "slot" || has_slot(&n.children),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use crate::compile;
    use crate::model::{Inline, Node};

    fn errors(src: &str) -> Vec<String> {
        compile(src)
            .unwrap_err()
            .into_iter()
            .map(|d| d.message)
            .collect()
    }

    #[test]
    fn expands_parameters_and_slot() {
        let doc = compile(
            r#"slml 0.1
            define card(title, color: #ff0000) {
              column(fill: $color) { heading $title  slot }
            }
            page { card(title: "Hi") { text "body" } }"#,
        )
        .unwrap();
        let Node::Container(col) = &doc.page.children[0] else {
            panic!()
        };
        assert_eq!(col.frame.fill.map(|c| c.0), Some(0xff0000ff));
        let Node::Text(h) = &col.children[0] else {
            panic!()
        };
        assert_eq!(h.content, vec![Inline::Text("Hi".into())]);
        assert_eq!(col.children.len(), 2);
    }

    #[test]
    fn layout_component_can_produce_the_page() {
        let doc = compile(
            r#"slml 0.1
            define layout(title) { meta(title: $title) page { heading $title slot } }
            layout(title: "Home") { text "hello" }"#,
        )
        .unwrap();
        assert_eq!(doc.meta.title.as_deref(), Some("Home"));
        assert_eq!(doc.page.children.len(), 2);
    }

    #[test]
    fn component_errors() {
        let e = errors(
            r#"slml 0.1
            define a(x) { text $x }
            define b { text "no slot" }
            page { a(y: "1") b { text "c" } text $z slot }"#,
        );
        assert!(e.iter().any(|m| m.contains("no parameter `y`")), "{e:?}");
        assert!(
            e.iter().any(|m| m.contains("requires parameter `x`")),
            "{e:?}"
        );
        assert!(e.iter().any(|m| m.contains("takes no content")), "{e:?}");
        assert!(
            e.iter().any(|m| m.contains("only be used inside `define`")),
            "{e:?}"
        );
        assert!(e.iter().any(|m| m.contains("`slot` can only")), "{e:?}");
    }

    #[test]
    fn recursion_is_caught() {
        let e = errors("slml 0.1 define a { a } page { a }");
        assert!(e[0].contains("nested too deeply"), "{e:?}");
    }

    #[test]
    fn cannot_redefine_builtins() {
        let e = errors("slml 0.1 define text { row { } } page { }");
        assert!(e[0].contains("built-in"), "{e:?}");
    }
}
