# Design principles

These principles decide arguments. A change that breaks one needs a written
reason in its pull request and an update to this file.

## 1. Strict over forgiving

HTML's "be liberal in what you accept" produced decades of incompatible error
recovery. SLML rejects invalid documents with a precise, helpful error.

- ✅ Unknown element → error with a suggestion.
- ❌ Ignoring unknown attributes "for forward compatibility". Use the version
  header instead.

## 2. One obvious way

Each concept has one spelling in the current version. When we add a
feature, we check whether an existing construct already covers it.

- Styles are attribute sets, not a second language with selectors.
- There are no named colors. Hex is the only color syntax.
- The older long syntax (`slml 0.1`) is still read, but new documents and
  all of this repository use `slml 0.2`; `slml upgrade` converts.

## 3. Short to write, not modelled on HTML

SLML is written by people (and programs) who write a lot of it. Frequent
things cost one or two characters: `c` for a column, `p20` for padding,
`#"…"` for a heading, `*bold*` inside text. A newcomer needs a cheat sheet
for the first hour; after that, every page is faster to write and to
read at a glance.

- ✅ `c{p20 bg#fff "Hello"}`
- ❌ `<div style="padding: 20px; background: #fff"><p>Hello</p></div>`
- New keys stay short and consistent with existing ones. A key or element
  name never contains digits, because digits are values (`w320`).

## 4. Layout, not semantics

HTML spends a lot of effort on meaning: `main`, `aside`, `nav`, `header`,
`section`, `article`, with rules and exceptions about where each may
appear (an `aside` that is "not part of the main content" may still be
placed inside `main`, `header` means something different inside an
`article` …). Those rules rarely change what a reader sees and are easy
to get wrong.

SLML has boxes (`c`, `r`, `grid`) and text. A box means nothing beyond its
layout, so it can be placed anywhere. Meaning is kept only where it
helps readers and assistive technology: heading levels, lists, links,
image descriptions and form labels — and those are required, not
optional.

## 5. Layout is the language

`r`, `c`, `grid`, `g`, `w:f`, `a` and `j` form the layout model. We never
add positioning tricks (floats, negative margins, absolute positioning) to
work around missing layout features. If a layout is hard to express,
extend the model in the spec.

## 6. Predictable rendering

The same file looks the same on every machine and every implementation:
fonts are bundled, every default is written in SPEC.md, and text is
literal.

## 7. Accessible by construction

If information is needed for accessibility (image descriptions, link
targets, form labels), it is required by the grammar or the validator, not
recommended by a linter.

## 8. Efficient to parse and render

- LL(1) grammar, single pass, no backtracking.
- Layout is computed top-down with local measurement only. There are no
  global passes that can be invalidated by later content.
- No feature may require re-parsing a document.

## 9. Safe by default

Documents are data. No scripting in 0.x. When interactivity arrives, it must
be declarative and sandboxed. A document can never read local files outside
its own directory.

## 10. Small core, clear growth path

Prefer a few composable primitives over many special elements. New features
go through [ROADMAP.md](../ROADMAP.md) and the spec first, then the code.

## 11. Native SLML first

The normal way to read a document is SLML's own browser and renderer. Opening
an SLML file or site must not compile it to HTML or launch a system web
browser. HTML export and web-hosted runtimes are optional compatibility tools
and require an explicit command or option. The native client must build and
run without a WebAssembly toolchain.
