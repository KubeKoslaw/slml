# SLML

**SLML — Strict Layout Markup Language.** A strict, layout-first alternative to HTML, with its own renderer written in Rust.

> 🇵🇱 [Polska wersja](README.pl.md)

```slml
slml 0.2
@card{p20 r12 bg#fff w:f}

page{bg#f6f6f9
  #"Hello, SLML!"
  r{g16
    c.card{"Layout is *built in*."}
    c.card{"[Why not HTML?](about.slml)"}
  }
  img{./logo.png w64 "SLML logo"}
}
```

Short to write: `c` is a column, `r` a row, `p20` padding, `bg#fff` a
background, `#"…"` a heading, and a bare string is a paragraph with
`*bold*`, `_italic_` and `[links](…)`. Files in the older long syntax
(`slml 0.1`) still work; `slml upgrade` converts them.

![The example site rendered by SLML](docs/screenshot.png)

## Why?

HTML has been extended for thirty years without breaking old pages, and it
shows. SLML starts over and keeps the good parts (a tree of elements, links,
text) without the historical accidents:

| HTML | SLML |
|------|------|
| Broken markup is silently "repaired" | Invalid documents are **rejected with a precise error and a hint** |
| `<div style="padding:20px;background:#fff"><p>Hi</p></div>` | `c{p20 bg#fff "Hi"}` |
| HTML + CSS + JS: three syntaxes, selectors, cascade | **One short grammar** for structure and style |
| `main`, `aside`, `nav`, `section` … and rules about where each may go | **No landmark elements**: boxes are layout, place them anywhere |
| Layout added later (floats → flexbox → grid) | `r`, `c`, `grid`, `g`, `w:f`, `a` from day one |
| Whitespace collapsing, `&amp;`, optional closing tags, quirks mode | Literal text, `{ }` blocks, a version line |
| `<img>` without `alt` is valid | `img{/a.png "description"}`: the description is **required** |
| Rendering depends on installed fonts and browser defaults | Fonts bundled, every default written in the spec |

Read more in [docs/principles.md](docs/principles.md) and the full
[language spec](SPEC.md).

## Install and use

`slml` opens documents in its own native browser. No Rust is needed to use a prebuilt release:

```bash
curl -fsSL https://raw.githubusercontent.com/KubeKoslaw/slml/main/install.sh | sh
```

Or from source: install Rust 1.88+ and run `make install`. The native browser
does not require a WebAssembly target, a web browser or an HTTP server. Then:

```bash
slml page.slml                 # open the original document in SLML's native browser
slml site/                     # open site/index.slml; site/ is the root for imports and links
slml handbook/                 # read the handbook in SLML
slml check site/               # only report errors
slml upgrade site/             # convert slml 0.1 files to the short 0.2 syntax
slml serve site/ --open        # serve original .slml files and open the native browser
```

Save a page, a shared component or an image and the native browser reloads.
Scroll with the mouse wheel or Page Up/Down, click links, use Backspace to go
back and F5 to reload manually. Close the window to exit.

**Read the [handbook](handbook/)** with `slml handbook/`.
For GitHub or Obsidian, use the [Markdown handbook](docs/handbook/README.md).
Open the complete example website with `slml examples/starter/`; the
[website tutorial](handbook/website-tutorial.slml) explains its shared layout
and responsive navigation. The native browser also fills in and sends
forms, selects and copies text, finds in page, zooms, keeps history and has
an element inspector (F12). Loading remote HTTP(S) documents is still on the
[roadmap](ROADMAP.md).

### Optional compatibility tools

These are explicit alternatives to the native browser:

```bash
slml build page.slml              # export page.html (+ _slml/ styles and fonts)
slml build site/ -o public/        # HTML compatibility export for existing web hosts
slml render page.slml -o page.png # native image export
slml serve site/ --web --open      # opt into a web-hosted WASM viewer
slml serve site/ --html --open     # opt into an HTML export preview
```

**Firefox:** the optional [extension](extensions/firefox/README.md) uses the
SLML engine inside a web browser. Build it with `make web` (requires Python 3
and Rust's `wasm32-unknown-unknown` target). It is an adapter for existing
browsers; opening an SLML document normally uses the native SLML browser.

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
  slml-html/     HTML+CSS backend (no JavaScript in output)
  slml-cli/      the `slml` compiler: compile, build sites, serve, check, view
examples/        sample documents (every one is tested in CI)
handbook/        the user guide, written in SLML
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
