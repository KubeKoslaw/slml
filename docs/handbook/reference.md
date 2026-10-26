# Reference

Every element and attribute of SLML 0.1 on one page. The complete rules, including layout details, are in [SPEC.md](https://github.com/KubeKoslaw/slml/blob/main/SPEC.md).

Jump to: [containers](#containers) · [text](#text-blocks) · [inline](#inline-elements) · [media](#images-and-spacing) · [forms](#form-fields) · [attributes](#attribute-groups) · [top level](#top-level)

## Containers

| Name | Values | Description |
| --- | --- | --- |
| `page` | frame, flow, text | The visible document. One per file. Defaults: padding 32, gap 16, white, 16 px text. |
| `column` | frame, flow, text, to | Children top to bottom. |
| `row` | frame, flow, text, to, wrap | Children side by side; wrap: true moves overflow to new lines. |
| `grid` | frame, flow, text, columns, cell-width | Equal columns: a fixed number (1–12), or as many of at least cell-width as fit. |
| `form` | frame, flow, text, to, method | A column that sends its fields to `to` (get or post). Gap 16. |
| `list` | frame, text, gap, marker | items and nested lists. marker: bullet, number or none. |

## Text blocks

| Name | Values | Description |
| --- | --- | --- |
| `text` | frame, text, align | A paragraph. |
| `heading` | frame, text, align, level | Level 1–4: 32, 24, 20 and 17 px, bold. |
| `item` | text, align, id | An entry of a list. |
| `codeblock` | frame, text, align | Monospaced text on a light background, spaces kept. |
| `button` | frame, text, align, to | A link that looks like a button. |
| `submit` | frame, text, align | The button that sends a form. |

## Inline elements

| Name | Values | Description |
| --- | --- | --- |
| `bold` | text | Bold text. |
| `italic` | text | Italic text. |
| `code` | text | Monospaced text. |
| `link` | text, to | A link; blue and underlined by default. |
| `span` | text | Any text attributes for part of a sentence. |

## Images and spacing

| Name | Values | Description |
| --- | --- | --- |
| `image` | frame, src, alt, to | PNG or JPEG. alt is required; alt: "" for decoration. |
| `divider` | width, color, thickness, id | A line; vertical inside a row. |
| `spacer` | width, height | Empty space; without a size it takes all free space. |

## Form fields

All fields take `name`, `label` (both required) and `required`. They must be inside a `form`.

| Name | Values | Description |
| --- | --- | --- |
| `input` | frame, text, kind, placeholder, value | kind: text, email, password, number, tel, url, date, search. |
| `textarea` | frame, text, rows, placeholder, value | rows: 1–50 (default 4). |
| `checkbox` | text, checked, id | A box with its label next to it. |
| `select` | frame, text | Contains option elements. |
| `option` | value, selected | One choice of a select, with a text label. |

## Attribute groups

**frame**

| Name | Values | Description |
| --- | --- | --- |
| `width, height` | number, fill, auto | Size including padding and border. |
| `min-width, max-width` | number | Limits for the width. |
| `padding` | number | Inner space; padding-x and padding-y set one axis. |
| `fill` | color | Background. |
| `radius` | number | Corner rounding. |
| `border, border-color` | number, color | Border width and color. |
| `shadow` | none, small, large | Drop shadow. |
| `id` | string | Anchor name for #links; unique per page. |

**flow**

| Name | Values | Description |
| --- | --- | --- |
| `gap` | number | Space between children. |
| `align` | start, center, end, stretch | Placement across the main direction. |
| `justify` | start, center, end, between | Distribution along the main direction. |

**text (inherited)**

| Name | Values | Description |
| --- | --- | --- |
| `color` | color | Text color. |
| `size` | number | Font size. |
| `bold, italic, underline` | true, false | Text decoration. |
| `font` | sans, mono | Font family. |

## Top level

| Name | Values | Description |
| --- | --- | --- |
| `slml 0.1` |  | Version line; always first. |
| `meta` | title, lang, description, icon | Page information. |
| `use` | "path.slml" | Import styles and components. |
| `style` | name(attributes) | A named set of attributes, applied with .name. |
| `define` | name(params) { … } | A component; $param inside, slot for content. |

[Coming from HTML](from-html.md)

[Back to the start](index.md)

[SLML source](../../handbook/reference.slml)
