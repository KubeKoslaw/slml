# SLML

**SLML — Strict Layout Markup Language.** A strict, layout-first alternative to HTML, with its own renderer written in Rust.

> 🇵🇱 [Polska wersja](README.pl.md)

```slml
slml 0.1

style card(padding: 20, radius: 12, fill: #ffffff, width: fill)

page(fill: #f6f6f9) {
  heading "Hello, SLML!"
  row(gap: 16) {
    column.card { text { "Layout is " bold "built in" "." } }
    column.card { text { link(to: "about.slml") "Why not HTML?" } }
  }
  image(src: "logo.png", alt: "SLML logo", width: 64)
}
```

![The example site rendered by SLML](docs/screenshot.png)

## Why?

HTML has been extended for thirty years without breaking old pages, and it
shows. SLML starts over and keeps the good parts (a tree of elements, links,
text) without the historical accidents:

| HTML | SLML |
|------|------|
| Broken markup is silently "repaired" | Invalid documents are **rejected with a precise error and a hint** |
| HTML + CSS + JS: three syntaxes, selectors, cascade | **One grammar** for structure and style |
| Layout added later (floats → flexbox → grid) | `row`, `column`, `gap`, `fill`, `align` from day one |
| Whitespace collapsing, `&amp;`, optional closing tags, quirks mode | Quoted literal text, `{ }` blocks, a version line |
| `<img>` without `alt` is valid | `image` **requires** `alt` |
| Rendering depends on installed fonts and browser defaults | Fonts bundled, every default written in the spec |

Read more in [docs/principles.md](docs/principles.md) and the full
[language spec](SPEC.md).

## Try it

You need [Rust](https://rustup.rs) (1.85 or newer).

```bash
git clone https://github.com/KubeKoslaw/slml && cd slml
cargo run --release -- examples/site/index.slml
```

Other commands:

```bash
cargo run --release -- check examples/hello.slml
```

```bash
cargo run --release -- render examples/site/index.slml -o page.png --width 1000
```

The viewer reloads the page automatically when you save the file. Click
links to navigate, and press Backspace to go back.

When something is wrong, SLML tells you exactly what and where:

```text
error: unknown element `colum`
 --> page.slml:3:3
  |
3 |   colum {
  |   ^^^^^
  = hint: did you mean `column`?

error: `image` requires `alt`
 --> page.slml:5:5
  |
5 |     image(src: "cat.png")
  |     ^^^^^
  = hint: describe the image for people who cannot see it; use `alt: ""` only for decoration
```

## Project layout

```
crates/
  slml-syntax/   lexer → parser → validator → typed model (no dependencies)
  slml-render/   layout engine + CPU renderer (fontdue, tiny-skia)
  slml-cli/      the `slml` binary: check, render, view (minifb window)
examples/        sample documents (every one is tested in CI)
SPEC.md          the language specification: the source of truth
ROADMAP.md       what comes next
AGENTS.md        instructions for AI coding agents
```

## Contributing

Humans and AI agents are both welcome. Start with
[CONTRIBUTING.md](CONTRIBUTING.md). AI agents should read
[AGENTS.md](AGENTS.md). Good first tasks are marked in
[ROADMAP.md](ROADMAP.md).

## License

Code: MIT or Apache-2.0, at your option. Bundled Noto fonts: SIL Open Font
License 1.1 (see `assets/fonts/OFL.txt`).
