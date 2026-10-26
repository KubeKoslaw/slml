# SLML Language Specification — version 0.1

SLML stands for **Strict Layout Markup Language**.

Status: **draft**. This document is the single source of truth for the
language. If the implementation and this spec disagree, one of them is a bug;
fix both in the same change.

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

SLML is a language for documents and simple interfaces, displayed by its own
renderer. It keeps HTML's best idea (a tree of semantic elements) and drops
the accidents of its history. The design principles in
[docs/principles.md](docs/principles.md) are binding; in short:

- **Strict.** A document is valid or rejected. There is no error recovery.
- **One way to do things.** One grammar for structure and style.
- **Layout is part of the language.** It is not a styling afterthought.
- **Predictable.** The same document renders the same everywhere: fonts are
  bundled and every default is defined in this spec.
- **Accessible by construction.** Required metadata, such as an image's `alt`,
  is enforced.
- **Efficient.** The grammar is LL(1) with one token of lookahead, and it can
  be parsed in a single pass without backtracking.

## 2. Lexical structure

Source files are UTF-8 and use the `.slml` extension.

| Token      | Form                                              | Example            |
|------------|---------------------------------------------------|--------------------|
| identifier | `[A-Za-z_][A-Za-z0-9_]*`, plus `-` followed by a letter | `max-width`  |
| number     | `[0-9]+(\.[0-9]+)?`, no sign, no exponent         | `16`, `0.5`        |
| string     | `"` … `"` on one line                             | `"Hello"`          |
| color      | `#` + 3, 4, 6 or 8 hex digits (`rgb[a]`/`rrggbb[aa]`) | `#1e40af`      |
| punctuation| `{ } ( ) : , .`                                   |                    |

- **Whitespace** (spaces, tabs, newlines) separates tokens and has no other
  meaning. Indentation is never significant.
- **Comments:** `// to end of line` and `/* block */` (not nested).
- **String escapes:** `\"` `\\` `\n` (line break) `\t` `\u{1F600}`. Any other
  escape is an error. A raw newline inside a string is an error. For long
  text, write several strings one after another.
- There are no character entities (`&amp;`): every character is written as
  itself.

## 3. Grammar

```ebnf
document  = header { style_def | node } EOF ;
header    = "slml" NUMBER ;                       (* e.g. slml 0.1 *)
style_def = "style" IDENT attrs ;                 (* only at top level *)
node      = IDENT { "." IDENT } [ attrs ] [ STRING | block ] ;
attrs     = "(" { attr [ "," ] } ")" ;
attr      = IDENT ":" value ;
value     = NUMBER | STRING | COLOR | IDENT ;
block     = "{" { node | STRING } "}" ;
```

Notes:

- `node` is an element name, optional `.style` names, optional attributes,
  then **either** a single string label **or** a block. Never both.
  `text "Hi"` is shorthand for `text { "Hi" }`.
- Commas between attributes are optional; newlines work as separators.
- Top level: exactly one `page`, at most one `meta`, and any number of
  `style` definitions. Styles may be used before they are defined.

## 4. Elements and attributes

### 4.1 Attribute groups

**Frame**: the box around a block element. Sizing is always border-box: the
width includes padding and border.

| Attribute     | Value                         | Default |
|---------------|-------------------------------|---------|
| `width`       | number, `fill`, `auto`        | `auto`  |
| `height`      | number, `fill`, `auto`        | `auto`  |
| `max-width`   | number                        | none    |
| `padding`     | number (sets x and y)         | 0       |
| `padding-x`   | number (overrides `padding`)  | 0       |
| `padding-y`   | number (overrides `padding`)  | 0       |
| `fill`        | color (background)            | none    |
| `radius`      | number (corner radius)        | 0       |
| `border`      | number (width)                | 0       |
| `border-color`| color                         | `#d4d4d8` |

**Flow**: how a container arranges its children.

| Attribute | Value                                   | Default   |
|-----------|-----------------------------------------|-----------|
| `gap`     | number, space between children          | 0         |
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

Numbers are logical pixels. Colors are hex only: there are no named colors.

### 4.2 Elements

| Element   | Kind       | Attributes                         | Content                    | Defaults |
|-----------|------------|------------------------------------|----------------------------|----------|
| `meta`    | top level  | `title`, `lang`, `description` (strings) | none                 | |
| `page`    | container  | frame, flow, text                  | block elements             | padding 32, gap 16, fill `#ffffff`, color `#1b1b1f`, size 16, font sans |
| `column`  | container  | frame, flow, text                  | block elements             | |
| `row`     | container  | frame, flow, text                  | block elements             | |
| `text`    | text block | frame, text, `align`               | strings and inline elements | |
| `heading` | text block | frame, text, `align`, `level` (1–3) | strings and inline elements | bold; size 32 / 24 / 20 by level |
| `button`  | text block | frame, text, `align`, **`to`** (required) | strings and inline elements | padding 10×18, fill `#2457d6`, color `#ffffff`, radius 8, align center |
| `image`   | leaf       | frame, **`src`**, **`alt`** (both required) | none                 | |
| `divider` | leaf       | `width`, `color`, `thickness`      | none                       | thickness 1, color `#d4d4d8` |
| `spacer`  | leaf       | `width`, `height`                  | none                       | |
| `bold`    | inline     | text                               | strings and inline elements | bold |
| `italic`  | inline     | text                               | strings and inline elements | italic |
| `code`    | inline     | text                               | strings and inline elements | font mono |
| `link`    | inline     | text, **`to`** (required)          | strings and inline elements | color `#2457d6`, underline |
| `span`    | inline     | text                               | strings and inline elements | |

**Block elements** are containers, text blocks and leaves except `page`.
In text blocks, `align` aligns lines (`start`, `center`, `end`). In
containers, it is the cross-axis alignment of children.

Content rules:

- Strings may appear only inside text blocks and inline elements.
- Inline elements may appear only inside text blocks and other inline elements.
- `link` may not contain a `link`, and `button` may not contain a `link`.
  Interactive elements are never nested.
- `image(alt: "")` explicitly marks a decorative image.
- `image(src: …)` is a path relative to the document. It must not be
  absolute and must not contain `..`: a document can only read files from its
  own folder and subfolders. SLML 0.1 renderers must support PNG.

### 4.3 Styles

```slml
style card(padding: 16, fill: #ffffff, radius: 12)
page { column.card(padding: 24) { text "Hi" } }
```

A style is a named set of attributes. `element.a.b(...)` applies style `a`,
then `b`, then the element's own attributes. Later values win. Every
attribute in a style must be valid for each element it is applied to;
otherwise it is an error at the place where the style is used. Styles do not
cascade and have no selectors.

### 4.4 Links

`to` is a path relative to the current document (`"about.slml"`) or an
absolute URL (`"https://…"`). SLML 0.1 viewers only have to support relative
paths.

## 5. Layout model

Every block element produces one rectangle. Containers place their children
along a **main axis**: vertical for `page` and `column`, horizontal for `row`.

**Width resolution**, given the available width *A* offered by the parent:

| `width`  | in a column with `align: stretch` | in a column with another `align` | in a row |
|----------|-------------------------|-----------------|-------------------------------|
| number   | that number             | that number     | that number                   |
| `fill`   | *A*                     | *A*             | an equal share of free space  |
| `auto`   | *A*                     | content width, at most *A* | content width (shrinks if the row overflows) |

After that, `max-width` limits the result.

**Columns:** children are stacked top to bottom with `gap` between them.
If the column has a definite height (a number, or stretched by its parent),
any free space goes first to `spacer`s and children with `height: fill`.
Whatever is left is distributed by `justify`.

**Rows:**

1. Children with a fixed width keep it. `auto` children get their content
   width. If that overflows, `auto` children shrink proportionally and
   their text wraps.
2. Free space is shared equally by `fill` children and `spacer`s. If there
   are none, `justify` distributes it.
3. The row's height is the tallest child. With `align: stretch`, children of
   `auto` height are stretched to it. A `divider` in a row is vertical and
   always spans the full height.

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
  (regular, bold) are bundled.

## 7. Errors

A conforming implementation must reject any document that violates this
spec, and it must not render a partial or guessed result. Diagnostics carry a
line and column. Implementations should report all independent errors
instead of stopping at the first one, and they should suggest likely fixes.

## 8. Versioning

The header `slml X.Y` names the language version that the document is written
for. A renderer rejects versions it does not know. Any change to this spec
that makes a valid document invalid, or changes how it renders, requires a
new version number.
