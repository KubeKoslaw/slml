# SLML Language Specification — version 0.2

SLML stands for **Strict Layout Markup Language**.

Status: **draft**. This document is the single source of truth for the
language. If the implementation and this spec disagree, one of them is a
bug; fix both in the same change.

SLML has two spellings of one language:

- **`slml 0.2`, the compact syntax** — the current version, described in
  §2–§4. Short to write, not modelled on HTML.
- **`slml 0.1`, the long syntax** — still read by every tool and described
  in [Appendix A](#appendix-a-the-long-syntax-slml-01). `slml upgrade`
  converts it to 0.2 and checks that every page stays identical.

Both parse into the same elements and attributes (called **canonical
names** below, such as `column` and `padding`), so validation (§5),
layout (§6) and every backend are shared.

Contents:

1. [Goals](#1-goals)
2. [Lexical structure](#2-lexical-structure)
3. [Grammar](#3-grammar)
4. [Elements, attributes and text](#4-elements-attributes-and-text)
5. [Validation](#5-validation)
6. [Layout model](#6-layout-model)
7. [Text model](#7-text-model)
8. [Errors](#8-errors)
9. [Versioning](#9-versioning)
- [Appendix A: the long syntax](#appendix-a-the-long-syntax-slml-01)

---

## 1. Goals

SLML is a language for documents, websites and simple interfaces. The
design principles in [docs/principles.md](docs/principles.md) are binding;
in short:

- **Strict.** A document is valid or rejected. There is no error recovery.
- **Short to write.** Frequent things take one or two characters (`c`,
  `p20`, `#"Title"`, `*bold*`). Readability for newcomers is traded for
  speed for people who write SLML every day.
- **Layout, not semantics.** There are boxes (`c`, `r`, `grid`) and text.
  There are no landmark or sectioning elements (`main`, `aside`, `nav`,
  `section`, `article` …) and no rules about which may contain which.
  The only structure that carries meaning is what readers need: heading
  levels, lists, links, images with descriptions and labelled form fields.
- **Predictable.** Fonts are bundled and every default is written here.
- **Accessible by construction.** Image descriptions and field labels are
  required.
- **Efficient.** One pass, one token of lookahead, no backtracking.

The primary client is the native SLML browser, which reads `.slml` files
and renders them directly. HTML+CSS export and web-hosted WebAssembly are
optional compatibility backends, selected explicitly by the user. Every
backend must follow this spec.

## 2. Lexical structure

Source files are UTF-8 and use the `.slml` extension. Whitespace (spaces,
tabs, newlines) and comments (`// …` to the end of the line, `/* … */`)
separate tokens and have no other meaning: a whole document may be
written on one line.

Two tokens are **glued** when nothing separates them. Gluing decides what
a word means: `p20` is the attribute `p` with the value 20, while `p 20`
is an error.

| Token | Form | Example |
|-------|------|---------|
| name | letters, `_`, and `-` followed by a letter; **no digits** | `c`, `card`, `prev-label` |
| number | `[0-9]+(\.[0-9]+)?` | `20`, `1.5` |
| string | `"…"` on one line, with escapes | `"Hello"` |
| block string | `"""` … `"""`, may span lines, raw | see below |
| color | `#` + 3, 4, 6 or 8 hex digits, glued after a name | `bg#fff` |
| parameter | `$` + name | `$title` |
| path | starts with `/`, `./`, `http://` or `https://`; runs to whitespace, `{`, `}` or `"` | `/img/logo.png` |
| target | `>` glued to a path-like run, a string or a parameter | `>/about.slml`, `>#top`, `>"a b.pdf"` |
| heading | `#` to `####` (not glued after a name) | `##"Title"` |
| list marker | `-` (bullets), `+` (numbers), `=` (no markers) | `-"item"` |
| symbols | `~` (spacer), `\|` (divider), `@`, `{ } ( ) , . :` | |

- Names never contain digits, so in `cols3`, `w320` and `$size16` the
  digits are always the value.
- `./logo.png` is how a relative path is written unquoted; it means
  `logo.png`.
- **String escapes:** `\"` `\\` `\n` (line break) `\t` `\u{1F600}`, and in
  text `\*` `\_` `` \` `` `\[` `\]` `\$` for the markup characters
  (§4.4). Any other escape is an error, and so is a raw newline.
- **Block strings** start with `"""` and end at the next `"""`. They are
  raw: escapes and markup are not processed. A line break right after the
  opening `"""` is removed. If the closing `"""` stands on its own line,
  that line is removed and its indentation is stripped from every line.
- There are no character entities (`&amp;`): every character is written
  as itself.

## 3. Grammar

```ebnf
document  = "slml" "0.2" { top } ;
top       = use | style | define | node ;
use       = "use" STRING ;
style     = "@" NAME "{" { attr | flag } "}" ;
define    = "@" NAME "(" [ param { [ "," ] param } ] ")" block ;
param     = NAME [ value ] ;                       (* glued default *)

node      = element | heading | list | spacer | divider | "slot" ;
element   = NAME { "." NAME } ( label | block ) ;
heading   = ("#" | "##" | "###" | "####") { "." NAME } ( label | block ) ;
list      = ("-" | "+" | "=") { "." NAME } ( label | block ) ;
spacer    = "~" [ block ] ;
divider   = "|" [ block ] ;
label     = STRING | PARAM ;
block     = "{" { item } "}" ;
item      = attr | flag | arg | target | node | STRING | PARAM | PATH ;

attr      = KEY value ;                            (* glued: p20 bg#fff a:c *)
value     = NUMBER | COLOR | ":" WORD | STRING | PATH | PARAM ;
flag      = NAME ;                                 (* b i u wrap req on … *)
arg       = PARAM value ;                          (* call argument: $title"Fast" *)
target    = ">" ( PATHCHARS | STRING | PARAM ) ;
```

Disambiguation, all decided by the next token:

- A name followed by `{` or `.` is an element. A name glued to a string or
  parameter is an attribute if the name is an attribute key (§4.2), and an
  element with a label otherwise: `t"Hi"` is text, `ph"Hi"` a
  placeholder.
- A bare name (nothing glued, no block) is a flag, except `slot`.
- `@name{…}` defines a style, `@name(…){…}` a component.
- Adjacent label-form lists with the same marker are one list: `-"a" -"b"`
  is the same as `-{"a" "b"}`.

Top level: any number of `use`, styles and components, then the document
itself: exactly one `page` and at most one `meta`, after components are
expanded (§4.6).

## 4. Elements, attributes and text

### 4.1 Elements

| 0.2 | canonical | Content | Defaults |
|-----|-----------|---------|----------|
| `meta` | meta | a string (the title) | |
| `page` | page | blocks | padding 32, gap 16, fill `#fff`, color `#1b1b1f`, size 16, sans |
| `c` | column | blocks; children top to bottom | |
| `r` | row | blocks; children side by side | |
| `grid` | grid | blocks; equal columns (`cols` or `cw` required) | |
| `form` | form | blocks and fields; `>target` is where it is sent | gap 16, `get` |
| `t` | text | text (§4.4) | |
| `#` … `####` | heading, level 1–4 | text | bold; size 32 / 24 / 20 / 17 |
| `btn` | button | text; `>target` required | padding 10×18, fill `#2457d6`, white, radius 8, centered |
| `send` | submit | text; only inside `form` | like `btn` |
| `code` | codeblock | a string, shown as written (no markup) | mono, size 14, padding 14×16, fill `#f4f4f7`, radius 8 |
| `-` `+` `=` | list (bullet / number / none) | strings (items) and nested lists | gap 6 |
| `img` | image | a path (source) and a string (description, required) | |
| `\|` | divider | — | thickness 1, color `#d4d4d8` |
| `~` | spacer | — | takes free space when unsized |
| `in` | input | a string (label) | kind `text` |
| `area` | textarea | a string (label) | rows 4 |
| `check` | checkbox | a string (label) | |
| `sel` | select | a string (label) and `opt`s | |
| `opt` | option | a string (label) | |
| `slot` | slot | — | only inside a component |

**Bare strings** placed among blocks (in `page`, `c`, `r`, `grid`,
`form`, or a component call) are paragraphs: `c{"Hi"}` means
`c{t"Hi"}`. A `t` is only needed to give a paragraph attributes.

**Interactive elements never nest:** nothing inside a link, `btn`, `send`,
or a box or image with `>target` may be a link, button, field or another
linked element. Fields and `send` only work inside a `form`; forms are not
nested.

### 4.2 Attributes

Attributes are written as a key glued to its value. Keys are shared by all
elements that accept the attribute (§5).

**Box** (any block element):

| Key | Canonical | Value |
|-----|-----------|-------|
| `w`, `h` | width, height | number, `:f` (fill), `:a` (auto) |
| `nw`, `mw` | min-width, max-width | number |
| `p`, `px`, `py` | padding (all, horizontal, vertical) | number |
| `bg` | fill (background) | color |
| `r` | radius | number |
| `b` | border width | number (`b1`) |
| `bc` | border-color | color |
| `sh` | shadow | `:n` none, `:s` small, `:l` large |
| `id` | id (anchor name, unique) | `:name` or string |
| `>` | link target (`to`) | §4.3 |

**Flow** (`page`, `c`, `r`, `grid`, `form`; `g` also on lists):

| Key | Canonical | Value |
|-----|-----------|-------|
| `g` | gap | number |
| `a` | align (cross axis) | `:s` start, `:c` center, `:e` end, `:x` stretch |
| `j` | justify (main axis) | `:s`, `:c`, `:e`, `:b` between |
| `wrap` | wrap (rows) | flag |
| `cols`, `cw` | columns (1–12), cell-width (grids) | number |

In text blocks `a` aligns lines (`:s`, `:c`, `:e`).

**Text** (inherited by everything inside):

| Key | Canonical | Value |
|-----|-----------|-------|
| `c` | color | color (`c#333`) |
| `s` | size | number |
| `b`, `i`, `u` | bold, italic, underline | flags; `b:0` switches off |
| `mono`, `sans` | font | flags |

**Fields and other attributes:**

| Key | Canonical | Value |
|-----|-----------|-------|
| `n` | name (fields, required) | `:word` or string |
| `k` | kind (`in`) | `:text` `:email` `:password` `:number` `:tel` `:url` `:date` `:search` |
| `ph`, `v` | placeholder, value | string |
| `req` | required | flag |
| `on` | checked / selected | flag |
| `rows` | rows (`area`) | number 1–50 |
| `th` | thickness (`\|`) | number |
| `post`, `get` | method (`form`) | flags |
| `lang`, `desc`, `icon` | meta language, description, icon | `:word`/string, string, path |
| `src`, `alt` | image source, description | path/string; only needed for `$parameters` |

Enumerated values accept the full word as well (`a:center`). Numbers are
logical pixels. Colors are hex only.

### 4.3 Paths and links

A **path** is relative to the current document (`./img/logo.png`,
`"img/logo.png"`) or, when it starts with `/`, to the **site root**
(`/img/logo.png`). Paths never contain `..`, backslashes, drive letters
or a leading `//`. The site root is chosen by the tool; for a single file
it is the current directory or the file's folder.

Local paths contain literal file names, including spaces, Unicode and `%`.
When producing browser URLs, tools percent-encode the UTF-8 path bytes
except ASCII letters, digits, `/`, `-`, `.`, `_` and `~`. A literal `%` is
therefore encoded as `%25`; do not write an already percent-encoded path.
The `#id` suffix on a link is a fragment, not part of the file name.
External URLs are kept as URLs, including their existing escaping.

A link target (`>…` on `btn`, boxes and `img`, or `(…)` in text) is:

- a path, optionally followed by `#id` (`/docs/start.slml#install`),
- `#id` alone: an element with that `id` must exist in the document,
- a URL with the scheme `https:`, `http:`, `mailto:` or `tel:`. Other
  schemes (such as `javascript:`) are errors.

A form's `>target` is a path or an `https:`/`http:` URL. Images and icons
accept a path or an `https:`/`http:` URL; renderers must support PNG and
JPEG.

### 4.4 Text and inline markup

Text elements (`t`, headings, `btn`, `send`, list items and bare
paragraphs) take one or more strings, which are joined without spaces.
Inside them:

| Markup | Meaning |
|--------|---------|
| `*text*` | bold |
| `_text_` | italic |
| `` `text` `` | inline code (monospaced, taken literally) |
| `[text](target)` | link |
| `[text]{attrs}` | text with attributes, e.g. `[sale]{c#c2410c b}` |
| `[text](target){attrs}` | link with attributes; `{.name}` applies a style |
| `$param`, `${param}` | a component parameter |

An opening `*`, `_`, `` ` `` or `[` must be closed in the same string;
otherwise it is an error. Write `\*`, `\_`, `` \` ``, `\[`, `\]` or `\$`
for the characters themselves. Markup is not processed in block strings
or in `code`.

### 4.5 Styles and imports

```slml
@card{p20 bg#fff r12}
page{c.card{p24 "Hi"}}
```

`@name{…}` gives a name to a set of attributes and flags; `.name` applies
it. `c.a.b{…}` applies `a`, then `b`, then the element's own attributes;
later values win. Every attribute of a style must be valid for each
element it is used on. Styles have no selectors and never cascade.

`use"file.slml"` loads the styles and components of another file. An
imported file contains only `use`, styles and components (it is a
**library**), must have the same version line, and is loaded once.
Import cycles are errors.

### 4.6 Components

```slml
@tile(title, tone#ffffff){
  c{bg$tone p20 r12 ###$title slot}
}

page{tile{$title"Fast" "Content goes into the slot."}}
```

- `@name(param, other#default){…}` defines a component. A parameter
  glued to a value has that default; one without is required.
- A call writes its arguments as `$param` glued to a value
  (`$title"Fast"`, `$tone#eee`, `$size16`, `$a:c`). The rest of the
  call's block is content for `slot`. A call without arguments or content
  is written `name{}`.
- In the body, `$param` can be an attribute value (`bg$tone`), a label
  (`###$title`), text (`"Hello $name"`), or a link target (`[x]($to)`).
- Passing content to a component without `slot` is an error. Components
  may use other components (depth limit 32; recursion is an error).
- Component names may not be element names or flags (of either syntax).
  A component may produce `meta` and `page`, which makes whole-page
  layouts possible.
- Expansion happens before validation, so a component's output is
  validated like hand-written content.

## 5. Validation

Every attribute is checked against the element it is written on.

| Canonical element | Accepts |
|-------------------|---------|
| page | box, flow, text |
| column, row | box, flow, text, `>`; row also `wrap` |
| grid | box, flow, text, `cols`, `cw` |
| form | box, flow, text, `>` (required), `post`/`get` |
| list | box, text, `g` |
| text, codeblock, submit | box, text, `a` |
| heading | box, text, `a` (level comes from `#`) |
| button | box, text, `a`, `>` (required) |
| item | text, `a` (written via `[…]{…}` in 0.2) |
| image | box, source and description (required), `>` |
| divider | `w`, `c`, `th`, `id` |
| spacer | `w`, `h` |
| input | box, text, `n`, label, `req`, `k`, `ph`, `v` |
| textarea | box, text, `n`, label, `req`, `ph`, `v`, `rows` |
| checkbox | text, `n`, label, `req`, `on`, `id` |
| select | box, text, `n`, label, `req`; at least one `opt`, at most one `on` |
| option | `v`, `on` |
| meta | title, `lang`, `desc`, `icon` |

An unknown element, an unknown or misplaced attribute, a value of the
wrong kind, a missing required value, content in the wrong place, nested
interactive elements, a duplicate `id` and a `#link` to a missing `id` are
all errors.

## 6. Layout model

Every block element produces one rectangle. Containers place their
children along a **main axis**: vertical for `page`, `c` and `form`,
horizontal for `r`.

**Width resolution**, given the available width *A* offered by the parent:

| `w` | in a column with `a:x` (default) | in a column with another `a` | in a row |
|-----|------------------|-----------------|-------------------------------|
| number | that number | that number | that number |
| `:f` | *A* | *A* | `nw` (or 0) plus an equal share of free space |
| `:a` | *A* | content width, at most *A* | content width (shrinks if the row overflows) |

After that, `mw` limits and then `nw` raises the result. Sizes include
padding and border.

**Columns:** children are stacked top to bottom with `g` between them. If
the column has a definite height (a number, or stretched by its parent),
any free space goes first to `~` spacers and children with `h:f`.
Whatever is left is distributed by `j`.

**Rows:**

1. Each child has a base width: a fixed width, `nw` for `w:f` children,
   or the content width for `w:a` children.
2. With `wrap`, children are placed on a line until the next one does
   not fit (base widths plus gaps). It then starts a new line, `g` below.
   Without `wrap`, there is one line.
3. In each line: if the line is too wide, `w:a` children shrink
   proportionally, but never below their `nw`, and their text wraps.
   Otherwise free space is shared equally by `w:f` children and `~`
   spacers. If there are none, `j` distributes it.
4. A line is as tall as its tallest child. With `a:x`, children of `h:a`
   height are stretched to it. A `|` divider in a row is vertical and
   always spans the line.

**Grids:** the number of columns is `cols`, or, with `cw`, as many
columns of at least `cw` as fit (`floor((A + g) / (cw + g))`, at least 1,
at most `cols` if both are given). All columns are equally wide. Children
fill the cells left to right, top to bottom. Each row of cells is as tall
as its tallest child, and `a` works as in rows.

**Lists:** each item is indented by the widest marker plus 0.6 × the font
size. The marker is drawn on the item's first line: `•` for `-`, `1.`,
`2.` … for `+` (nested lists count separately), nothing for `=`. Entries
are `g` apart.

**Fields:** `in`, `area` and `sel` are a label (bold, 0.875 × font size,
` *` appended when `req`), 6 pixels of space, then a box. An auto width is
320 in content-sized contexts and the available width otherwise. A `check`
is a square of 1.1 × font size followed by its label.

**Page:** the page is at least as tall as the viewport, so `j` and
spacers work against the window height.

**Images:** without explicit sizes, an image uses its natural size and
never grows beyond the available width. With only one dimension given,
the aspect ratio is preserved. If an image cannot be loaded, its
description is shown in its place.

## 7. Text model

- Strings are literal apart from markup (§4.4). Spaces are not collapsed,
  and `\n` is a line break.
- Lines wrap only at spaces. Spaces at a wrap point are dropped.
- Adjacent strings and markup join without any implicit space:
  `"Hel*lo*"` is one word.
- Line height is 1.45 × the font size.
- Fonts: Noto Sans (regular, bold, italic, bold italic) and Noto Sans Mono
  (regular, bold) are bundled. Characters missing from them are taken, in
  this order, from Noto Sans Symbols 2, Noto Sans Symbols, Noto Sans Math
  and Noto Emoji (monochrome). Variation selectors and zero-width joiners
  take no space.

## 8. Errors

A conforming implementation must reject any document that violates this
spec, and it must not render a partial or guessed result. Diagnostics
carry a file, line and column, use the names as written in the file, and
should suggest likely fixes. Implementations should report all
independent errors instead of stopping at the first one; syntax errors
stop at the first.

## 9. Versioning

The header `slml X.Y` names the language version that the document is
written for. A renderer rejects versions it does not know. This build
reads `0.1` and `0.2`. An imported library must have the same version as
the file that imports it. Once a version is released, any change that
makes a valid document invalid, or changes how it renders, requires a new
version number.

---

## Appendix A: the long syntax (slml 0.1)

`slml 0.1` spells the same elements and attributes with their canonical
names. It remains supported; `slml upgrade` converts it to 0.2.

```ebnf
document  = "slml" "0.1" { use | style_def | define | node } EOF ;
use       = "use" STRING ;
style_def = "style" IDENT attrs ;
define    = "define" IDENT [ "(" { param [ "," ] } ")" ] block ;
param     = IDENT [ ":" value ] ;
node      = IDENT { "." IDENT } [ attrs ] [ STRING | PARAM | block ] ;
attrs     = "(" { IDENT ":" value [ "," ] } ")" ;
value     = NUMBER | STRING | COLOR | IDENT | PARAM ;
block     = "{" { node | STRING | PARAM } "}" ;
```

```slml
slml 0.1
style card(padding: 20, fill: #ffffff, radius: 12)
page {
  column.card(gap: 8) {
    heading(level: 2) "Title"
    text { "Some " bold "bold" " text and a " link(to: "/about.slml") "link" "." }
    image(src: "/logo.png", alt: "Logo", width: 64)
  }
}
```

Differences from 0.2:

- Elements use canonical names: `column`, `row`, `text`, `heading(level:
  N)`, `button`, `submit`, `codeblock`, `image`, `divider`, `spacer`,
  `list(marker: …) { item … }`, `input`, `textarea`, `checkbox`,
  `select`, `option`, and inline elements `bold`, `italic`, `code`,
  `link(to: …)`, `span` instead of markup in strings.
- Attributes are written `name: value` in parentheses with canonical
  names (`padding`, `fill`, `max-width`, `align: center`, `to:`,
  `src:`, `alt:`, `label:`, `name:`, `bold: true` …).
- Strings are fully literal (no markup); a string directly inside a
  container is an error.
- Styles are `style name(…)`, components `define name(…) { … }`, and a
  call passes arguments as attributes: `card(title: "Fast") { … }`.
- Names may contain digits.
