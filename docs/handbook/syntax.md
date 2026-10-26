# Syntax basics

SLML has one simple shape for everything: a name, optional attributes in parentheses, and either a text label or a block of content in braces.

## Elements

```slml
name                          // just the name
name(attribute: value)        // with attributes
name "label"                  // with a text label
name(attribute: value) {      // with a block of content
  child
  child
}
```

A label is a short form of a block with one string: `text "Hi"` means the same as `text { "Hi" }`. An element takes a label *or* a block, never both.

## Attributes

Attributes are written `name: value` inside parentheses. Commas between them are optional, so long lists can be written one per line:

```slml
column(
  padding: 24
  fill: #ffffff
  radius: 12
) { text "Card" }
```

A value is one of:

| Name | Values | Description |
| --- | --- | --- |
| `16` | number | Pixels, sizes, counts. No units are written. |
| `"Hello"` | string | Text in double quotes. |
| `#2457d6` | color | Hex: #rgb, #rgba, #rrggbb or #rrggbbaa. There are no color names. |
| `center` | name | One of the words an attribute accepts, such as center, fill or true. |

Every attribute is checked: an unknown name, a value of the wrong kind or an attribute the element doesn't accept is an error.

## Strings

Strings are **literal**: what you write is what you get. Spaces are kept, and there are no entities like `&amp;`.

- `\n` starts a new line, `\t` is a tab, `\"` is a quote and `\\` is a backslash.
- `\u{1F600}` is any Unicode character by its number.
- A normal string must end on the same line.

For longer text, especially code, use a **block string** between triple quotes. Nothing inside it is escaped, and the indentation of the closing `"""` is removed from every line:

```slml
codeblock """
    first line
      indented line
    """
```

## Comments and whitespace

```slml
// a comment until the end of the line
/* a comment
   over several lines */
```

Spaces, tabs and line breaks between elements have no meaning. Indent however you like: re-formatting a file never changes the page.

## File structure

```slml
slml 0.1                         // version line, always first

use "layout.slml"                // imports (optional)
style card(padding: 16)          // styles (optional)
define badge(label) { ... }      // components (optional)

meta(title: "Page title")        // page information (optional)
page { ... }                     // the page itself
```

[Your first page](first-page.md)

[Text](text.md)

[SLML source](../../handbook/syntax.slml)
