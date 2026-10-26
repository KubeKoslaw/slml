# Instructions for AI coding agents

This file is for AI agents (Claude, Codex, Copilot, Gemini, Cursor, …)
working on this repository. Human contributors may find it useful too.

## What this project is

SLML is a new document language (an alternative to HTML) with its own
renderer. The language is defined by [SPEC.md](SPEC.md), and design
decisions are governed by [docs/principles.md](docs/principles.md). **Read
both before changing language behavior.**

## Golden rules

1. **The spec is the source of truth.** Any change to syntax, elements,
   attributes, defaults or layout rules must update `SPEC.md` in the same
   pull request. If the code and the spec disagree, do not "fix" the spec to
   match a bug.
2. **Never add error recovery or silent fallbacks** to the parser or
   validator. Invalid input must produce a `Diagnostic` with a precise span
   and, where possible, a `hint`.
3. **Respect crate boundaries:**
   - `slml-syntax` has **no dependencies**. Keep it that way.
   - `slml-render` is platform-independent: no windowing, no files except
     images through `ImageStore`.
   - `slml-cli` is the only crate that talks to the OS (windows, files, args).
4. **Every behavior change comes with a test.** Syntax and validation tests
   go next to the code in `slml-syntax`. Layout tests go in
   `slml-render/src/layout.rs` (assert on rectangles, not pixels). New
   elements also need an example in `examples/`.
5. **No new dependencies** unless the pull request explains why an existing
   one or ~100 lines of code would not do.
6. **Small pull requests.** Change one thing per pull request.

## Before you finish

Run all of these and make sure they pass. CI runs the same commands.

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
```

If you changed rendering, render an example and look at it:

```bash
cargo run --release -- render examples/site/index.slml -o /tmp/page.png
```

## Map of the code

| Task | Where |
|------|-------|
| New token or escape | `crates/slml-syntax/src/lexer.rs` |
| Grammar change | `crates/slml-syntax/src/parser.rs` + `ast.rs` |
| New element or attribute | `analyze.rs` (`ELEMENTS`, `attr_groups`, `apply_attr`) + `model.rs` + `SPEC.md` §4 |
| Layout behavior | `crates/slml-render/src/layout.rs` + `SPEC.md` §5 |
| Drawing | `crates/slml-render/src/paint.rs` |
| HTML output | `crates/slml-html/src/lib.rs` (must follow SPEC §5 like the native renderer) |
| Components / imports | `crates/slml-syntax/src/expand.rs`, `lib.rs` |
| Compiler CLI, site builds, preview server | `crates/slml-cli/src/{main,build,serve}.rs` |
| Native viewer | `crates/slml-cli/src/viewer.rs` |
| User documentation | `handbook/*.slml` (must pass `slml check handbook`) |

## Style

- Code, comments, commit messages and docs in English.
- Error messages: lowercase, no trailing period, name things in backticks:
  ``unknown element `colum` ``. Hints suggest a concrete fix.
- Prefer clear code over clever code. This is a learning-friendly project.

## Choosing work

Pick an unchecked item from [ROADMAP.md](ROADMAP.md) or an open GitHub
issue. If a feature needs a design decision that the spec doesn't settle
(for example, a new syntax), open an issue with a proposal first and don't
implement it straight away.
