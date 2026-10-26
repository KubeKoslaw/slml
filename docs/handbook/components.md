# Components and imports

Components are your own elements. You define them once with `define` and use them like built-in elements. Imports share them between files.

## Defining a component

```slml
define badge(label, color: #2457d6) {
  text(fill: $color, color: #ffffff, size: 13, bold: true,
       padding-x: 10, padding-y: 3, radius: 99) $label
}

row(gap: 8) {
  badge(label: "New")
  badge(label: "Sale", color: #c2410c)
}
```

- `define name(param, other: default) { ... }` declares a component. Parameters without a default are required.
- Inside the body, `$param` is replaced by the value given when the component is used, either as an attribute value or as text.
- Forgetting a required parameter, giving an unknown one, or using a name of a built-in element are errors.

## Passing content with slot

`slot` marks where the content written inside the component's braces goes:

```slml
define feature(title) {
  column(fill: #ffffff, padding: 16, radius: 12, border: 1,
         border-color: #e4e4ea, gap: 6, width: fill) {
    heading(level: 4) $title
    slot
  }
}

row(gap: 12) {
  feature(title: "Fast") { text "Compiles in milliseconds." }
  feature(title: "Strict") { text "No silent mistakes." }
}
```

Components can use other components. A component is replaced by its content before the page is checked, so everything it produces is validated like hand-written SLML.

## Page layouts

A component can produce the whole `meta` and `page`. Then every page of a site is just its content:

```slml
// layout.slml
slml 0.1

define site-page(title) {
  meta(title: $title, lang: "en")
  page(padding: 0, gap: 0) {
    menu
    column(padding: 32, gap: 16) {
      heading $title
      slot
    }
    footer
  }
}
```

```slml
// about.slml
slml 0.1
use "/layout.slml"

site-page(title: "About us") {
  text "We make things."
}
```

This handbook works exactly like that: look at `handbook/layout.slml` in the repository.

## Imports

`use "file.slml"` loads the `style`s and `define`s of another file. An imported file contains no `meta` or `page`; it is a **library**, and `slml site/` does not turn it into a page.

- Paths work like links: relative to the file, or from the site root with `/`.
- Libraries can import other libraries. Each file is loaded once, and import loops are errors.
- Errors in an imported file are reported with that file's name and line.

[Links and navigation](links.md)

[Forms](forms.md)

[SLML source](../../handbook/components.slml)
