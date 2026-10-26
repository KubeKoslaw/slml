# Understanding errors

SLML is strict on purpose. A browser that guesses what broken HTML meant hides mistakes; SLML shows them, with everything you need to fix them.

## Reading an error

```slml
error: `t` has no attribute `gp`
 --> about.slml:12:7
   |
12 |     t{gp8 "Warning"}
   |       ^^
   = hint: `t` accepts: w, h, nw, mw, p, px, py, bg, r, b, bc, sh, id, c, s, i, mono/sans, u, a
```

- The first line says **what** is wrong, with the names exactly as you wrote them.
- The arrow gives the **file, line and column**.
- The source line is shown with the problem underlined.
- The **hint** suggests a fix: a likely spelling, the attributes an element accepts, or the SLML way to write an HTML tag.

All independent errors in a file are reported at once, so you can fix them in one go. A syntax error (such as a missing `}`) stops at the first one, because everything after it could mean something else.

## Common errors

| Name | Values | Description |
| --- | --- | --- |
| `unknown element` | cc{…}, div{…} | A typo, or an HTML name. The hint names the SLML element. |
| `has no attribute` | t{g8 …} | The key exists, but not on this element. The hint lists what it accepts. |
| `expects …` | g"big" | A value of the wrong kind: a string where a number belongs, an unknown word. |
| `` requires `alt` `` | img{/a.png} | Describe the image with a string. Use "" for decoration only. |
| `` requires `label` `` | in{n:email} | Every form field needs a visible label string. |
| `` `*` is never closed `` | "5 * 3" | A formatting character without its pair. Write \* for the character itself. |
| `` `[` starts a link `` | "a[0]" | Brackets are for [links](…) and [styled text]{…}; write \[ and \]. |
| `cannot be placed inside` | t{r{…}} | Only strings go inside text. Boxes go into boxes. |
| `inside a link or button` | btn{"[x](y)"} | Interactive elements can't be nested. |
| `no element has id` | >#faq | An anchor link points to an id that doesn't exist. |
| `broken link` | >abuot.slml | The linked file doesn't exist in the site. |
| `invalid path` | "../x.png" | Paths can't leave the site. Start at the root with /. |
| `unterminated string` | "Hello | A string must end on the same line; use \n or a """ block. |
| ``missing `slml` version line`` |  | The file must start with slml 0.2. |

## Why so strict?

HTML browsers accept almost anything and try to repair it, each in a slightly different way. That is why the same page can look different in two browsers, and why accessibility problems go unnoticed. SLML rejects the page instead and tells you exactly what to change. A page that opens is one that every SLML tool understands the same way. See the [design principles](https://github.com/KubeKoslaw/slml/blob/main/docs/principles.md) for the full reasoning.

[Command line](cli.md)

[Coming from HTML](from-html.md)

[SLML source](../../handbook/errors.slml)
