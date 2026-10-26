# Text

Text lives in **text blocks**: paragraphs, headings, buttons, list items and code blocks. Inside them, **inline elements** change part of the text.

## Paragraphs and headings

```slml
heading "Level 1 heading"
heading(level: 2) "Level 2 heading"
heading(level: 3) "Level 3 heading"
heading(level: 4) "Level 4 heading"
text "A paragraph wraps automatically when the line is full."
```

Strings may only appear inside text blocks. Writing a bare string inside `page` or `column` is an error: wrap it in `text`.

## Inline elements

```slml
text {
  "You can make text " bold "bold" ", " italic "italic" ", "
  code "monospaced" ", " span(color: #c2410c) "orange" " or a "
  link(to: "#inline") "link" "."
}
```

Strings and inline elements are joined exactly as written: `"Hel" bold "lo"` is one word. Put spaces inside the strings where you want them.

Inline elements can be nested: `bold { italic "both" }`.

## Text attributes

These attributes work on any element and are inherited by everything inside it:

| Name | Values | Description |
| --- | --- | --- |
| `color` | color | Text color. |
| `size` | number | Font size in pixels. The page default is 16. |
| `bold` | true / false | Bold text. |
| `italic` | true / false | Italic text. |
| `underline` | true / false | Underlined text. |
| `font` | sans / mono | The bundled Noto Sans or Noto Sans Mono. |

```slml
column(color: #2944a8, size: 18) {
  text "Blue and larger"
  text(bold: true) "Also blue, and bold"
}
```

```slml
column(font: mono, gap: 8) {
  text "This paragraph inherits the monospace font."
  text(font: sans) "This paragraph switches back to sans."
}
```

## Alignment and line breaks

```slml
text(align: center) "Centered"
text(align: end) "Right aligned"
text "Line one\nLine two"
```

**Whitespace is never collapsed**

Unlike HTML, two spaces are two spaces and a line break is written as \n. What you write is what you see.

## Selecting and copying

In the optional web preview (slml serve --web) and Firefox extension, drag across text to select it and press `Ctrl+C` (`Cmd+C` on macOS) to copy. `Ctrl+A` selects the document when you are not editing a field or the source.

Copying keeps literal spaces, code indentation and explicit newlines. A line wrapped only to fit the screen remains part of the same paragraph; separate text blocks are joined by a newline. Editable form fields keep their normal browser selection and copy behavior.

Text selection is not yet available in the native desktop viewer.

## Symbols and emoji

The bundled fonts cover Latin, Greek and Cyrillic, symbols, arrows, math and monochrome emoji: ★ ✓ → ∑ ♥ 😀. Pages look the same on every computer because the fonts travel with them.

[Syntax basics](syntax.md)

[Layout](layout-guide.md)

[SLML source](../../handbook/text.slml)
