# Text

Text is written in strings. A bare string in a box is a paragraph; headings, buttons and list items take strings too. Inside a string, a few characters format the text.

## Paragraphs and headings

```slml
#"Level 1 heading"
##"Level 2 heading"
###"Level 3 heading"
####"Level 4 heading"
"A paragraph wraps automatically when the line is full."
t{s14 c#5b5b70 "A paragraph with attributes needs t."}
```

Several strings in one text are joined without spaces: `t{"Hello, " "world"}` is `Hello, world`. Use it to split long text over several lines of source.

## Formatting inside text

```slml
"You can make text *bold*, _italic_, `monospaced`,
[orange]{c#c2410c}, [big]{s22 b} or a [link](#inline)."
```

| Name | Values | Description |
| --- | --- | --- |
| `*text*` | bold | Bold. |
| `_text_` | italic | Italic. |
| `` `text` `` | code | Monospaced and literal: no formatting inside. |
| `[text](target)` | link | A link; see Links and navigation. |
| `[text]{attrs}` | styled text | Any text attributes for part of a sentence: [sale]{c#c2410c b}. |
| `[text](target){.style}` | styled link | A link with attributes or a style. |

Formatting can be nested: `*bold and _italic_*`. Write `\*`, `\_`, `` \` ``, `\[` or `\]` for the characters themselves.

## Text attributes

These attributes work on any element and are inherited by everything inside it:

| Name | Values | Description |
| --- | --- | --- |
| `c#333` | color | Text color. |
| `s18` | number | Font size in pixels. The page default is 16. |
| `b  i  u` | flags | Bold, italic, underlined. b:0, i:0, u:0 switch an inherited value off. |
| `mono  sans` | flags | The bundled Noto Sans Mono or Noto Sans. |

```slml
c{c#2944a8 s18
  "Blue and larger"
  t{b "Also blue, and bold"}
}
```

```slml
c{mono g8
  "This paragraph inherits the monospace font."
  t{sans "This paragraph switches back to sans."}
}
```

## Alignment and line breaks

```slml
t{a:c "Centered"}
t{a:e "Right aligned"}
"Line one\nLine two"
```

**Whitespace is never collapsed**

Unlike HTML, two spaces are two spaces and a line break is written as \n. What you write is what you see.

## Selecting and copying

In the optional web preview (slml serve --web) and Firefox extension, drag across text to select it and press `Ctrl+C` (`Cmd+C` on macOS) to copy. `Ctrl+A` selects the document when you are not editing a field or the source.

Copying keeps literal spaces, code indentation and explicit newlines. A line wrapped only to fit the screen remains part of the same paragraph; separate text blocks are joined by a newline. Editable form fields keep their normal browser selection and copy behavior.

The native browser selects text the same way: drag, double-click for a word, triple-click for a paragraph, Shift+click to extend, Ctrl+A for everything and Ctrl+C to copy. On Linux, copying uses wl-clipboard or xclip.

## Symbols and emoji

The bundled fonts cover Latin, Greek and Cyrillic, symbols, arrows, math and monochrome emoji: ★ ✓ → ∑ ♥ 😀. Pages look the same on every computer because the fonts travel with them.

[Syntax basics](syntax.md)

[Layout](layout-guide.md)

[SLML source](../../handbook/text.slml)
