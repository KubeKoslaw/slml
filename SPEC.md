# SLML Language Specification — version 0.1

SLML stands for **Strict Layout Markup Language**.

Status: **draft** (0.1 is not frozen until the first release). This document
is the single source of truth for the language. If the implementation and
this spec disagree, one of them is a bug; fix both in the same change.

Contents:

1. [Goals](#1-goals)
2. [Lexical structure](#2-lexical-structure)
3. [Grammar](#3-grammar)
4. [Elements and attributes](#4-elements-and-attributes)
5. [Layout model](#5-layout-model)
6. [Text model](#6-text-model)
7. [Errors](#7-errors)
8. [Versioning](#8-versioning)

---

## 1. Goals

SLML is a language for documents, websites and simple interfaces. It keeps
HTML's best idea (a tree of semantic elements) and drops the accidents of
its history. The design principles in [docs/principles.md](docs/principles.md)
are binding; in short:

- **Strict.** A document is valid or rejected. There is no error recovery.
- **One way to do things.** One grammar for structure, style and reuse.
- **Layout is part of the language.** It is not a styling afterthought.
- **Predictable.** The same document renders the same everywhere: fonts are
  bundled and every default is defined in this spec.
- **Accessible by construction.** Required metadata, such as an image's
  `alt` or a form field's `label`, is enforced.
- **Efficient.** The grammar is LL(1), single pass, no backtracking.

The primary client is the native SLML browser, which reads `.slml` files
and renders them directly. HTML+CSS export and web-hosted WebAssembly are
optional compatibility backends, selected explicitly by the user. Every
backend must follow this spec.

## 2. Lexical structure

Source files are UTF-8 and use the `.slml` extension.

| Token      | Form                                              | Example            |
|------------|---------------------------------------------------|--------------------|
| identifier | `[A-Za-z_][A-Za-z0-9_]*`, plus `-` followed by a letter | `max-width`  |
| parameter  | `$` + identifier                                  | `$title`           |
| number     | `[0-9]+(\.[0-9]+)?`, no sign, no exponent         | `16`, `0.5`        |
| string     | `"` … `"` on one line                             | `"Hello"`          |
| block string | `"""` … `"""`, may span lines, raw              | see below          |
| color      | `#` + 3, 4, 6 or 8 hex digits (`rgb[a]`/`rrggbb[aa]`) | `#1e40af`      |
| punctuation| `{ } ( ) : , .`                                   |                    |

- **Whitespace** (spaces, tabs, newlines) separates tokens and has no other
  meaning. Indentation is never significant (except inside block strings).
- **Comments:** `// to end of line` and `/* block */` (not nested).
- **String escapes:** `\"` `\\` `\n` (line break) `\t` `\u{1F600}`. Any other
  escape is an error. A raw newline inside a `"…"` string is an error.
- **Block strings** start with `"""` and end at the next `"""`. Their content
  is raw: escapes are not processed. A line break right after the opening
  `"""` is removed. If the closing `"""` stands on its own line, that line is
  removed and its indentation is stripped from every line. This is meant
  for code samples and longer text:

  ```slml
  codeblock """
      page {
        text "Hello"
      }
      """
  ```

- There are no character entities (`&amp;`): every character is written as
  itself.

## 3. Grammar

```ebnf
document  = header { use | style_def | define | node } EOF ;
header    = "slml" NUMBER ;                       (* e.g. slml 0.1 *)
use       = "use" STRING ;
style_def = "style" IDENT attrs ;
define    = "define" IDENT [ "(" { param [ "," ] } ")" ] block ;
param     = IDENT [ ":" value ] ;
node      = IDENT { "." IDENT } [ attrs ] [ STRING | PARAM | block ] ;
attrs     = "(" { attr [ "," ] } ")" ;
attr      = IDENT ":" value ;
value     = NUMBER | STRING | COLOR | IDENT | PARAM ;
block     = "{" { node | STRING | PARAM } "}" ;
```

Notes:

- `use`, `style` and `define` are only allowed at the top level.
- `node` is an element (or component) name, optional `.style` names,
  optional attributes, then **either** a single label **or** a block.
  `text "Hi"` is shorthand for `text { "Hi" }`.
- Commas between attributes are optional; newlines work as separators.
- After components are expanded (§4.6), the top level must contain exactly
  one `page` and at most one `meta`.

## 4. Elements and attributes

### 4.1 Attribute groups

**Frame**: the box around a block element. Sizing is always border-box: the
width includes padding and border.

| Attribute     | Value                         | Default |
|---------------|-------------------------------|---------|
| `width`       | number, `fill`, `auto`        | `auto`  |
| `height`      | number, `fill`, `auto`        | `auto`  |
| `min-width`   | number                        | none    |
| `max-width`   | number                        | none    |
| `padding`     | number (sets x and y)         | 0       |
| `padding-x`   | number (overrides `padding`)  | 0       |
| `padding-y`   | number (overrides `padding`)  | 0       |
| `fill`        | color (background)            | none    |
| `radius`      | number (corner radius)        | 0       |
| `border`      | number (width)                | 0       |
| `border-color`| color                         | `#d4d4d8` |
| `shadow`      | `none` `small` `large`        | `none`  |
| `id`          | string: a letter, then letters, digits, `-`, `_`; unique in the document | none |

**Flow**: how a container arranges its children.

| Attribute | Value                                   | Default   |
|-----------|-----------------------------------------|-----------|
| `gap`     | number, space between children (and between lines/rows) | 0 |
| `align`   | `start` `center` `end` `stretch` (cross axis) | `stretch` |
| `justify` | `start` `center` `end` `between` (main axis)  | `start`   |

**Text**: inherited by descendants unless they set their own value.

| Attribute   | Value              |
|-------------|--------------------|
| `color`     | color              |
| `size`      | number > 0 (font size) |
| `bold`      | `true` / `false`   |
| `italic`    | `true` / `false`   |
| `underline` | `true` / `false`   |
| `font`      | `sans` / `mono`    |

**Field**: `name` (string without spaces, required), `label` (non-empty
string, required), `required` (`true`/`false`, default `false`).

Numbers are logical pixels. Colors are hex only: there are no named colors.

### 4.2 Elements

| Element     | Kind       | Attributes | Content | Defaults |
|-------------|------------|------------|---------|----------|
| `meta`      | top level  | `title`, `lang`, `description` (strings), `icon` (path) | none | |
| `page`      | container  | frame, flow, text | block elements | padding 32, gap 16, fill `#ffffff`, color `#1b1b1f`, size 16, font sans |
| `column`    | container  | frame, flow, text, `to` | block elements | |
| `row`       | container  | frame, flow, text, `to`, `wrap` (bool) | block elements | |
| `grid`      | container  | frame, flow, text, `columns` (1–12), `cell-width` (number); at least one required | block elements | |
| `form`      | container  | frame, flow, text, **`to`**, `method` (`get`/`post`) | block elements and fields | gap 16, method get |
| `list`      | list       | frame, text, `gap`, `marker` (`bullet`/`number`/`none`) | `item`s and nested `list`s | gap 6, marker bullet |
| `item`      | text block | text, `align`, `id` | strings and inline elements | only inside `list` |
| `text`      | text block | frame, text, `align` | strings and inline elements | |
| `heading`   | text block | frame, text, `align`, `level` (1–4) | strings and inline elements | bold; size 32 / 24 / 20 / 17 by level |
| `codeblock` | text block | frame, text, `align` | strings and inline elements | font mono, size 14, padding 14×16, fill `#f4f4f7`, radius 8 |
| `button`    | text block | frame, text, `align`, **`to`** | strings and inline elements | padding 10×18, fill `#2457d6`, color `#ffffff`, radius 8, align center |
| `submit`    | text block | frame, text, `align` | strings and inline elements | like `button`; only inside `form` |
| `image`     | leaf       | frame, **`src`**, **`alt`**, `to` | none | |
| `divider`   | leaf       | `width`, `color`, `thickness`, `id` | none | thickness 1, color `#d4d4d8` |
| `spacer`    | leaf       | `width`, `height` | none | |
| `input`     | field      | frame, text, field, `kind`, `placeholder`, `value` | none | kind `text` |
| `textarea`  | field      | frame, text, field, `rows` (1–50), `placeholder`, `value` | none | rows 4 |
| `checkbox`  | field      | text, field, `checked`, `id` | none | |
| `select`    | field      | frame, text, field | `option`s (at least one) | |
| `option`    | —          | `value` (string), `selected` (bool) | one string | only inside `select`; at most one selected |
| `bold`      | inline     | text | strings and inline elements | bold |
| `italic`    | inline     | text | strings and inline elements | italic |
| `code`      | inline     | text | strings and inline elements | font mono |
| `link`      | inline     | text, **`to`** | strings and inline elements | color `#2457d6`, underline |
| `span`      | inline     | text | strings and inline elements | |

`kind` of an `input` is one of `text`, `email`, `password`, `number`,
`tel`, `url`, `date`, `search`.

**Block elements** are containers, lists, text blocks except `item`, leaves
and fields. In text blocks, `align` aligns lines (`start`, `center`, `end`);
in containers it is the cross-axis alignment of children.

Content rules:

- Strings may appear only inside text blocks, inline elements and `option`.
- Inline elements may appear only inside text blocks and inline elements.
- `item` only inside `list`. A nested `list` must follow an `item`.
- Fields and `submit` only inside a `form`. Forms are not nested.
- **Interactive elements never nest:** nothing inside a `link`, `button`,
  `submit`, or a container or image with `to` may be a link, button, field
  or another linked element.
- `image(alt: "")` marks a decorative image. A linked image needs a
  non-empty `alt`, because it is the link's text.

### 4.3 Paths and links

A **path** is relative to the current document (`"img/logo.png"`) or, when
it starts with `/`, to the **site root** (`"/img/logo.png"`). Paths never
contain `..`, backslashes, drive letters or a leading `//`. The site root is
chosen by the tool (see the handbook); for a single file it is the current
directory or the file's folder.

Local paths contain literal file names, including spaces, Unicode and `%`.
When producing browser URLs, tools percent-encode the UTF-8 path bytes
except ASCII letters, digits, `/`, `-`, `.`, `_` and `~`. A literal `%` is
therefore encoded as `%25`; do not write an already percent-encoded path.
The `#id` suffix on a link is a fragment, not part of the file name.
External URLs are kept as URLs, including their existing escaping.

`src` and `icon` accept a path or an `https://`/`http://` URL. Renderers must
support PNG and JPEG.

`to` (on `link`, `button`, `column`, `row`, `image`) accepts:

- a path, optionally followed by `#id` (`"docs/start.slml#install"`),
- `#id` alone: an element with that `id` must exist in the document,
- a URL with the scheme `https:`, `http:`, `mailto:` or `tel:`. Other
  schemes (such as `javascript:`) are errors.

`form(to: …)` takes a path or an `https:`/`http:` URL where the form data is
sent.

### 4.4 Styles

```slml
style card(padding: 16, fill: #ffffff, radius: 12)
page { column.card(padding: 24) { text "Hi" } }
```

A style is a named set of attributes. `element.a.b(...)` applies style `a`,
then `b`, then the element's own attributes. Later values win. Every
attribute in a style must be valid for each element it is applied to;
otherwise it is an error at the place where the style is used. Styles do
not cascade and have no selectors. Style names share one namespace across
the document and its imports; defining a name twice is an error.

### 4.5 Imports

```slml
use "/components.slml"
```

`use` loads another file's `style`s and `define`s. An imported file may
contain only `use`, `style` and `define` (no `meta`, no `page`), and must
have the same version header. Import cycles are errors. A file imported
twice is loaded once.

### 4.6 Components

```slml
define card(title, tone: #ffffff) {
  column(fill: $tone, padding: 20, radius: 12) {
    heading(level: 3) $title
    slot
  }
}

page {
  card(title: "Fast") { text "Content goes into the slot." }
}
```

- `define name(params) { … }` declares a component. Parameters may have a
  default value; parameters without one are required.
- A call `name(param: value) { content }` is replaced by the component's
  body. `$param` may be used as an attribute value or as text; as text it
  must hold a string.
- `slot` marks where the call's content goes. Passing content to a
  component without a `slot` is an error.
- Components may call other components (depth limit 32; recursion is an
  error). Styles (`.name`) cannot be applied to a component call.
- Component names may not be built-in element names. A component may
  produce several nodes, including `meta` and `page`, which allows layout
  components for whole pages.
- Expansion happens before validation, so a component's output is
  validated like hand-written content.

## 5. Layout model

Every block element produces one rectangle. Containers place their
children along a **main axis**: vertical for `page`, `column` and `form`,
horizontal for `row`.

**Width resolution**, given the available width *A* offered by the parent:

| `width`  | in a column with `align: stretch` | in a column with another `align` | in a row |
|----------|-------------------------|-----------------|-------------------------------|
| number   | that number             | that number     | that number                   |
| `fill`   | *A*                     | *A*             | `min-width` (or 0) plus an equal share of free space |
| `auto`   | *A*                     | content width, at most *A* | content width (shrinks if the row overflows) |

After that, `max-width` limits and then `min-width` raises the result.

**Columns:** children are stacked top to bottom with `gap` between them.
If the column has a definite height (a number, or stretched by its parent),
any free space goes first to `spacer`s and children with `height: fill`.
Whatever is left is distributed by `justify`.

**Rows:**

1. Each child has a base width: a fixed width, `min-width` for `fill`
   children, or the content width for `auto` children.
2. With `wrap: true`, children are placed on a line until the next one does
   not fit (base widths plus gaps). It then starts a new line, `gap` below.
   Without `wrap`, there is one line.
3. In each line: if the line is too wide, `auto` children shrink
   proportionally, but never below their `min-width`, and their text wraps.
   Otherwise free space is shared equally by `fill` children and `spacer`s.
   If there are none, `justify` distributes it.
4. A line is as tall as its tallest child. With `align: stretch`, children
   of `auto` height are stretched to it. A `divider` in a row is vertical
   and always spans the line.

**Grids:** the number of columns is `columns`, or, with `cell-width`, as
many columns of at least `cell-width` as fit (`floor((A + gap) / (cell-width
+ gap))`, at least 1, at most `columns` if both are given). All columns are
equally wide. Children fill the cells left to right, top to bottom. Each row
of cells is as tall as its tallest child, and `align` works as in rows.

**Lists:** each item is indented by the widest marker plus 0.6 × the font
size. The marker is drawn on the item's first line: `•` for bullets, `1.`,
`2.` … for numbers (nested lists count separately). Entries are `gap`
apart.

**Fields:** `input`, `textarea` and `select` are a label (bold, 0.875 ×
font size, `*` appended when required), 6 pixels of space, then a box. An
`auto` width is 320 in content-sized contexts and the available width
otherwise. A checkbox is a square of 1.1 × font size followed by its label.

**Page:** the page is at least as tall as the viewport, so `justify` and
spacers work against the window height.

**Images:** without explicit sizes, an image uses its natural size and never
grows beyond the available width. With only one dimension given, the aspect
ratio is preserved. If an image cannot be loaded, its `alt` text is shown in
its place.

## 6. Text model

- Strings are literal. Spaces are not collapsed, and `\n` is a line break.
- Lines wrap only at spaces. Spaces at a wrap point are dropped.
- Adjacent strings and inline elements join without any implicit space:
  `"Hel" bold "lo"` is one word.
- Line height is 1.45 × the font size.
- Fonts: Noto Sans (regular, bold, italic, bold italic) and Noto Sans Mono
  (regular, bold) are bundled. Characters missing from them are taken, in
  this order, from Noto Sans Symbols 2, Noto Sans Symbols, Noto Sans Math
  and Noto Emoji (monochrome). Variation selectors and zero-width joiners
  take no space.

## 7. Errors

A conforming implementation must reject any document that violates this
spec, and it must not render a partial or guessed result. Diagnostics carry
a file, line and column. Implementations should report all independent
errors instead of stopping at the first one, and they should suggest
likely fixes.

## 8. Versioning

The header `slml X.Y` names the language version that the document is
written for. A renderer rejects versions it does not know. Once a version
is released, any change that makes a valid document invalid, or changes how
it renders, requires a new version number.
