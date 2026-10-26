# Reference

Every element and attribute of SLML 0.2 on one page. The complete rules, including layout details, are in [SPEC.md](https://github.com/KubeKoslaw/slml/blob/main/SPEC.md).

Jump to: [boxes](#boxes) · [text](#text) · [inline](#formatting-inside-strings) · [media](#images-and-spacing) · [forms](#form-fields) · [attributes](#attributes) · [top level](#top-level)

## Boxes

| Name | Values | Description |
| --- | --- | --- |
| `page{…}` | box, flow, text | The visible document. One per file. Defaults: p32 g16, white, 16 px text. |
| `c{…}` | box, flow, text, > | Column: children top to bottom. |
| `r{…}` | box, flow, text, >, wrap | Row: children side by side; wrap moves overflow to new lines. |
| `grid{…}` | box, flow, text, cols, cw | Equal columns: cols1–cols12, or as many of at least cw as fit. |
| `form{…}` | box, flow, text, >, post | A column that sends its fields to >target (get by default). Gap 16. |
| `-{…} +{…} ={…}` | box, text, g | List: every string is an item; - bullets, + numbers, = no marker. |

A bare string in a box is a paragraph.

## Text

| Name | Values | Description |
| --- | --- | --- |
| `"…"  t"…"  t{…}` | box, text, a | A paragraph; t only to add attributes. |
| `#"…" … ####"…"` | box, text, a | Headings 1–4: 32, 24, 20 and 17 px, bold. |
| `code"…"` | box, text, a | Monospaced, spaces kept, no formatting. |
| `btn{>… "…"}` | box, text, a, > | A link that looks like a button. |
| `send"…"` | box, text, a | The button that sends a form. |

## Formatting inside strings

| Name | Values | Description |
| --- | --- | --- |
| `*text*  _text_` |  | Bold, italic. |
| `` `text` `` |  | Inline code, taken literally. |
| `[text](target)` |  | A link; blue and underlined by default. |
| `[text]{attrs}` |  | Text attributes for part of a sentence; {.style} applies a style. |
| `$name  ${name}` |  | A component parameter. |
| ``\* \_ \` \[ \] \$`` |  | The characters themselves. |

## Images and spacing

| Name | Values | Description |
| --- | --- | --- |
| `img{/a.png "…"}` | box, > | PNG or JPEG. The description is required; "" for decoration. |
| `\|  \|{…}` | w, c, th, id | A divider line; vertical inside a row. |
| `~  ~{h24}` | w, h | Empty space; without a size it takes all free space. |

## Form fields

Every field takes `n:name` and a label string (both required) and the flag `req`. Fields must be inside a `form`.

| Name | Values | Description |
| --- | --- | --- |
| `in{…}` | box, text, k, ph, v | k: text, email, password, number, tel, url, date, search. |
| `area{…}` | box, text, rows, ph, v | rows1–rows50 (default 4). |
| `check{…}` | text, on, id | A box with its label next to it; on ticks it. |
| `sel{…}` | box, text | Contains opt elements. |
| `opt"…"  opt{v"…" on "…"}` | v, on | One choice: a label, an optional value, on to select it. |

## Attributes

**box**

| Name | Values | Description |
| --- | --- | --- |
| `w h` | number, :f, :a | Width, height (including padding and border). |
| `nw mw` | number | Min-width, max-width. |
| `p px py` | number | Padding: all, horizontal, vertical. |
| `bg` | color | Background. |
| `r` | number | Corner radius. |
| `b bc` | number, color | Border width (b1) and color. |
| `sh` | :n :s :l | Shadow: none, small, large. |
| `id` | :name | Anchor name for #links; unique per page. |
| `>` | target | Makes the element a link (or a form's destination). |

**flow**

| Name | Values | Description |
| --- | --- | --- |
| `g` | number | Gap between children. |
| `a` | :s :c :e :x | Align across: start, center, end, stretch. On text: line alignment. |
| `j` | :s :c :e :b | Justify along: start, center, end, between. |
| `wrap` | flag | Rows wrap onto new lines. |

**text (inherited)**

| Name | Values | Description |
| --- | --- | --- |
| `c` | color | Text color. |
| `s` | number | Font size. |
| `b i u` | flags, :0 | Bold, italic, underline; b:0 switches off. |
| `mono sans` | flags | Font family. |

Full words work as values too: `a:center` is `a:c`.

## Top level

| Name | Values | Description |
| --- | --- | --- |
| `slml 0.2` |  | Version line; always first. |
| `use"…"` | path | Import styles and components. |
| `@name{…}` | attributes | A style, applied with .name. |
| `@name(a, b#fff){…}` | parameters | A component; $a inside, slot for content; called as name{$a"…" …}. |
| `meta{"Title" …}` | lang, desc, icon | Page information. |
| `page{…}` |  | The page. |

[Coming from HTML](from-html.md)

[Back to the start](index.md)

[SLML source](../../handbook/reference.slml)
