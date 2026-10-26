# Syntax basics

SLML has one shape for everything: a short name, then either a string or a block in braces. Inside a block come attributes (glued to their values) and content. Spaces and line breaks don't matter: a whole page can be one line.

## Elements

```slml
t"Hello"                 // an element with a text label
c{p20 bg#fff "Hello"}    // an element with a block: attributes, then content
~                        // a few symbols need nothing at all
```

The most used elements:

| Name | Values | Description |
| --- | --- | --- |
| `c{…}  r{…}` | column, row | Boxes that stack their content vertically or side by side. |
| `grid{…}` | grid | Equal columns: grid{cols3 …} or grid{cw200 …}. |
| `"…"  t"…"` | paragraph | A bare string in a box is a paragraph; t is only needed for attributes: t{s14 "…"}. |
| `#"…" … ####"…"` | heading 1–4 | The number of # is the level. |
| `-{…}  +{…}` | lists | Each string is an item: - bullets, + numbers, = no marker. |
| `img{/a.png "…"}` | image | A path and a required description. |
| `btn{>/x "…"}` | button | A link that looks like a button. |
| `code"…"` | code block | Monospaced text, shown exactly as written. |
| `\|  ~` | divider, spacer | A line, and flexible empty space. |

The full list is in the [Reference](reference.md).

## Attributes

An attribute is a short key **glued** to its value, with no space in between. The kind of value decides how it is written:

| Name | Values | Description |
| --- | --- | --- |
| `p20 w320 s18` | number | Pixels, sizes, counts. No units. |
| `bg#fff c#2457d6` | color | Hex: #rgb, #rgba, #rrggbb or #rrggbbaa. There are no color names. |
| `a:c w:f j:b` | choice | A colon and a short word: a:c is align center, w:f is width fill. |
| `ph"Your name"` | string | Text in double quotes, glued to the key. |
| `>/about.slml` | target | > starts a link target: a path, #anchor or URL. |
| `b i u wrap req` | flag | A bare word switches something on: b is bold. b:0 switches an inherited value off. |

Names never contain digits: in `cols3`, `w320` and `p20` the digits are always the value. Every attribute is checked: an unknown key, a value of the wrong kind or an attribute the element doesn't accept is an error.

## Strings and text

Strings are **literal**: spaces are kept, and there are no entities like `&amp;`. Inside text, a few characters format the text:

```slml
"*bold*, _italic_, `code`, a [link](#strings) and [orange]{c#c2410c}"
```

- `\n` starts a new line, `\t` is a tab, `\"` is a quote and `\\` a backslash. `\u{1F600}` is any Unicode character.
- `\*`, `\_`, `` \` ``, `\[`, `\]` and `\$` write the formatting characters themselves. An opening star, underscore, backtick or bracket that is never closed is an error, so a stray star can't silently turn half a page bold.
- A normal string ends on the same line.

For longer text, especially code, use a **block string** between triple quotes. Nothing inside it is escaped or formatted, and the indentation of the closing `"""` is removed from every line:

```slml
code"""
    first line
      indented line
    """
```

## Styles and components

```slml
@card{p20 bg#fff r12}               // a style: a named set of attributes
@badge(label){t{bg#2457d6 c#fff $label}}   // a component
page{
  c.card{"Styled with .card"}
  badge{$label"New"}
}
```

See [Boxes and styles](styling.md) and [Components and imports](components.md).

## Comments and whitespace

```slml
// a comment until the end of the line
/* a comment
   over several lines */
```

Spaces, tabs and line breaks between elements have no meaning. `slml upgrade` re-formats a file without changing the page.

## File structure

```slml
slml 0.2                        // version line, always first
use"layout.slml"                // imports (optional)
@card{p16}                      // styles (optional)
@badge(label){…}                // components (optional)
meta{"Page title" lang:en}      // page information (optional)
page{…}                         // the page itself
```

## Files in the older syntax

SLML 0.1 used long names: `column(padding: 20) { text "Hi" }`. Those files still work. `slml upgrade` rewrites them in the short syntax and checks that every page stays exactly the same:

```sh
slml upgrade my-site/
```

[Your first page](first-page.md)

[Text](text.md)

[SLML source](../../handbook/syntax.slml)
