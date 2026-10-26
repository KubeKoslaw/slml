# Boxes and styles

Every block element is a box. Its appearance is set with attributes, and sets of attributes you use often become **styles**.

## The box

| Name | Values | Description |
| --- | --- | --- |
| `p20` | number | Padding: space inside the box on all sides. |
| `px24  py8` | number | Horizontal or vertical padding only; they override p. |
| `bg#fff` | color | Background color. |
| `r12` | number | Rounded corners. |
| `b1  bc#ddd` | number, color | Border width and color (light gray by default). |
| `sh:s  sh:l` | small, large | A soft drop shadow. |

```slml
c{bg#fff p20 r12 b1 bc#e4e4ea sh:s
  ###"A card"
  "Padding, fill, radius, border and shadow."
}
```

**Sizes include padding and border**

A box with `w200 p20` is 200 pixels wide in total. There is no box-model arithmetic to remember.

## Styles

`@name{…}` gives a name to a set of attributes. Apply it with a dot after the element name. The element's own attributes win over the style's:

```slml
@card{bg#fff p20 r12 b1 bc#e4e4ea g6}
@highlight{bg#fff4d6 bc#f0d48a}

r{g12
  c.card{w:f "Plain card"}
  c.card.highlight{w:f "Card + highlight"}
  c.card{w:f r0 "Square card"}
}
```

- Styles are applied left to right, then the element's own attributes: `c.card.highlight{r0}`.
- A style may only contain attributes that fit every element it is used on; `t.card` is an error here, because a text has no gap (`g`).
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
