# Boxes and styles

Every block element is a box. Its appearance is set with attributes, and sets of attributes you use often become **styles**.

## The box

| Name | Values | Description |
| --- | --- | --- |
| `padding` | number | Space inside the box on all sides. |
| `padding-x / -y` | number | Horizontal or vertical padding only; they override padding. |
| `fill` | color | Background color. |
| `radius` | number | Rounded corners. |
| `border` | number | Border width. Its color is border-color (light gray by default). |
| `shadow` | none / small / large | A soft drop shadow. |

```slml
column(fill: #ffffff, padding: 20, radius: 12,
       border: 1, border-color: #e4e4ea, shadow: small) {
  heading(level: 3) "A card"
  text "Padding, fill, radius, border and shadow."
}
```

**Sizes include padding and border**

A box with `width: 200` and `padding: 20` is 200 pixels wide in total. There is no box-model arithmetic to remember.

## Styles

A `style` gives a name to a set of attributes. Apply it with a dot after the element name. The element's own attributes win over the style's:

```slml
style card(fill: #ffffff, padding: 20, radius: 12,
           border: 1, border-color: #e4e4ea, gap: 6)
style highlight(fill: #fff4d6, border-color: #f0d48a)

row(gap: 12) {
  column.card(width: fill) { text "Plain card" }
  column.card.highlight(width: fill) { text "Card + highlight" }
  column.card(width: fill, radius: 0) { text "Square card" }
}
```

- Styles are applied left to right, then the element's own attributes: `column.card.highlight(radius: 0)`.
- A style may only contain attributes that fit every element it is used on; `text.card` is an error here, because a text has no `gap`.
- Styles never cascade and have no selectors. An element looks the way its own line says.
- Put styles you share between pages into a library file and `use` it. See [imports](components.md#imports).

## Colors

Colors are hexadecimal: `#rgb`, `#rrggbb`, or with transparency `#rgba` / `#rrggbbaa`. There are no color names, so a color always means the same thing.

#2457d6

#146c43

#b42318

#00000014

[Layout](layout-guide.md)

[Lists, code and images](content.md)

[SLML source](../../handbook/styling.slml)
