# Roadmap

Items are roughly in priority order. Each one should become a GitHub issue
before work starts. Items marked **good first issue** are self-contained.

## 0.1 — foundation (done)

- [x] Lexer, parser, validator with helpful diagnostics
- [x] Elements: page, column, row, text, heading, button, image, divider, spacer, bold, italic, code, link, span
- [x] Named styles (`style card(...)`, `column.card`)
- [x] Layout engine: rows, columns, fill, gap, align, justify, max-width
- [x] Software renderer (tiny-skia + fontdue), bundled Noto fonts
- [x] `slml view` window with scrolling, links, history, live reload
- [x] `slml render` to PNG, `slml check`

## Also done

- [x] `slml 0.2`: compact syntax (short names, glued attributes, inline `*bold*` / `[link](…)` markup), no semantic-only elements
- [x] `slml upgrade`: verified conversion from 0.1, keeping comments
- [x] Native browser: visible cursor (also on Wayland), hover and pressed feedback, status bubble, scrollbar, history, zoom, keyboard focus, find in page, text selection and copy, element inspector (F12), form filling and sending

- [x] Native SLML browser as the default for files, site folders and `serve --open`; HTML and web-hosted previews require explicit commands

- [x] HTML backend, `slml serve` live preview, `slml site/` builds, broken-link errors
- [x] gcc-style compiler, release binaries, install scripts, man page
- [x] Handbook written in SLML; lists, codeblock, wrapping rows, imports (`use`)
- [x] Typing into form fields in the native viewer
- [ ] CJK fonts

## 0.2 — make it pleasant to write

- [x] **Font fallback** for glyphs missing from Noto Sans (arrows, emoji, CJK). **good first issue**
- [x] Canonical formatter: `slml upgrade` prints every file in the standard 0.2 layout
- [ ] Syntax highlighting: VS Code extension + tree-sitter grammar. **good first issue**
- [ ] Language server (LSP): diagnostics, completion of elements and attributes
- [x] Components: `define card(title) { column.card { heading $title  slot } }`
- [x] `grid(columns: 3)` container
- [ ] `stack` container (children on top of each other) instead of absolute positioning
- [ ] Percent and `fr` units, `min-width`, `min-height`
- [ ] Per-side padding (`padding-top` …) or a 2/4-value form (decide in spec)
- [x] Multi-line string literal for prose, e.g. `"""…"""` with defined whitespace rules

## 0.3 — a real browser

- [x] Text selection and copy in the WASM preview and Firefox extension
- [x] Text selection and copy in the native desktop viewer
- [x] Find in page
- [ ] HiDPI detection (instead of `--scale`)
- [x] JPEG/WebP/SVG images
- [ ] Load documents over HTTPS (`slml view https://…`) with a sandbox: a document may only read files under its own origin
- [x] Anchors: `#id` links within a page
- [ ] Accessibility tree export via AccessKit (screen readers)
- [ ] Complex text shaping (rustybuzz): Arabic, Devanagari, ligatures; bidi text

## 0.4 — interactivity without scripts

- [x] Forms: `input`, `checkbox`, `select`, `form(to: …)` with required labels
- [ ] Declarative state: `toggle`, `tabs`, `details` without JavaScript
- [ ] Themes: dark mode via named color tokens

## Tooling and community

- [x] WebAssembly renderer, browser source editor and Firefox extension (`make web`)
- [ ] Publish a signed Firefox add-on and host the online playground
- [ ] `html2slml` converter for simple HTML pages
- [ ] Conformance test suite: `.slml` file → expected PNG (golden images)
- [ ] Benchmarks: parse and layout time for large documents
