# Layout

Layout is part of SLML. Four containers, `column`, `row`, `grid` and `page`, and a handful of attributes describe almost any page.

## Columns and rows

`column` stacks its children from top to bottom, `row` places them side by side. `gap` is the space between them.

```slml
column(gap: 8) {
  text.box "One"
  row(gap: 8) {
    text.box "Two"
    text.box "Three"
  }
}
```

The `page` itself is a column with 32 pixels of padding and a gap of 16.

## Sizes: auto, fill and pixels

`width` and `height` take three kinds of values:

| Name | Values | Description |
| --- | --- | --- |
| `auto` | default | As big as the content. In a column, auto width fills the column unless align says otherwise. |
| `fill` | share space | Take the free space. Several fill children share it equally. |
| `240` | pixels | Exactly this size, padding and border included. |

```slml
row(gap: 8) {
  text.box(width: 80) "80 px"
  text.box(width: fill) "fill"
  text.box(width: fill) "fill"
  text.box "auto"
}
```

`max-width` limits a size and `min-width` sets a lower bound. They are the key to readable pages: `column(width: fill, max-width: 760)` is a column that grows with the window but never gets too wide.

## Alignment

`align` places children across the container (horizontally in a column, vertically in a row): `start`, `center`, `end` or `stretch` (the default). `justify` distributes them along it: `start`, `center`, `end` or `between`.

```slml
row(justify: between, align: center, fill: #f4f4f8, padding: 10) {
  text.box "Left"
  text.box(padding: 20) "Middle"
  text.box "Right"
}
column(align: center) { text.box "Centered" }
```

A `spacer` absorbs free space; it is the easiest way to push things apart:

```slml
row { text "Logo"  spacer  text "Menu" }
```

## Responsive layouts

Pages are viewed on phones and wide screens. Two tools adapt a layout to the space available:

**Wrapping rows.** With `wrap: true`, children that don't fit move to the next line. Give `fill` children a `min-width` to say how narrow they may get:

```slml
row(wrap: true, gap: 8) {
  text.box(width: fill, min-width: 160) "Sidebar"
  text.box(width: fill, min-width: 260) "Content"
}
```

**Grids.** `grid(columns: 3)` makes three equal columns. `grid(cell-width: 200)` fits as many columns of at least 200 pixels as the width allows: four on a laptop, one on a phone.

```slml
grid(cell-width: 140, gap: 8) {
  text.box "A"  text.box "B"  text.box "C"
  text.box "D"  text.box "E"
}
```

**Try it**

Make this browser window narrower: the sidebar of this handbook moves above the text and the grid above drops columns. Both are just a wrapping row and a grid.

## Recipes

**Centered page content**

```slml
page(align: center) {
  column(width: fill, max-width: 760, gap: 20) { ... }
}
```

**Header with logo and menu**

```slml
row(align: center, gap: 16, padding: 16) {
  image(src: "/logo.png", alt: "Home", width: 32, to: "/index.slml")
  spacer
  text { link(to: "/about.slml") "About" }
  text { link(to: "/contact.slml") "Contact" }
}
```

**Sidebar layout that stacks on phones**

```slml
row(wrap: true, gap: 24) {
  column(width: 240) { ...menu... }
  column(width: fill, min-width: 320) { ...content... }
}
```

[Text](text.md)

[Boxes and styles](styling.md)

[SLML source](../../handbook/layout-guide.slml)
