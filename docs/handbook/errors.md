# Understanding errors

SLML is strict on purpose. A browser that guesses what broken HTML meant hides mistakes; SLML shows them, with everything you need to fix them.

## Reading an error

```slml
error: `text` has no attribute `colr`
 --> about.slml:12:10
   |
12 |     text(colr: #b42318) "Warning"
   |          ^^^^
   = hint: did you mean `color`?
```

- The first line says **what** is wrong.
- The arrow gives the **file, line and column**.
- The source line is shown with the problem underlined.
- The **hint** suggests a fix: a likely spelling, the attributes an element accepts, or the SLML equivalent of an HTML tag.

All independent errors in a file are reported at once, so you can fix them in one go.

## Common errors

| Name | Values | Description |
| --- | --- | --- |
| `unknown element` | colum, div, p | A typo, or an HTML name. The hint names the SLML element. |
| `has no attribute` | text(gap: …) | The attribute exists, but not on this element. The hint lists what it accepts. |
| `expects …` | gap: "big" | A value of the wrong kind: a string where a number belongs, an unknown word. |
| `` requires `alt` `` | image | Describe the image. Use alt: "" for decoration only. |
| `` requires `label` `` | input, select … | Every form field needs a visible label. |
| `text cannot be placed directly` | column { "Hi" } | Wrap the string in text. |
| `cannot be placed inside` | text { row } | Only strings and inline elements go inside text. |
| `inside a link or button` | button { link } | Interactive elements can't be nested. |
| `no element has id` | link(to: "#x") | An anchor link points to an id that doesn't exist. |
| `broken link` | to: "abuot.slml" | The linked file doesn't exist in the site. |
| `invalid path` | "../x.png" | Paths can't leave the site. Start at the root with /. |
| `unterminated string` | "Hello | A string must end on the same line; use \n or a """ block. |
| ``missing `slml` version line`` |  | The file must start with slml 0.1. |

## Why so strict?

HTML browsers accept almost anything and try to repair it, each in a slightly different way. That is why the same page can look different in two browsers, and why accessibility problems go unnoticed. SLML rejects the page instead and tells you exactly what to change. A page that compiles is one that every SLML tool understands the same way. See the [design principles](https://github.com/KubeKoslaw/slml/blob/main/docs/principles.md) for the full reasoning.

[Command line](cli.md)

[Coming from HTML](from-html.md)

[SLML source](../../handbook/errors.slml)
