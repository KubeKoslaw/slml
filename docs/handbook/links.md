# Links and navigation

Anything can be a link: a piece of text, a button, an image or a whole box. The `to` attribute says where it goes.

## Kinds of links

```slml
text { "Read the " link(to: "text.slml") "text chapter" "." }
row(gap: 12) {
  button(to: "#targets") "A button"
  image(src: "/logo.png", alt: "Home", width: 40, to: "/index.slml")
}
column(to: "/layout-guide.slml", fill: #f4f4f8, padding: 14, radius: 10) {
  text(bold: true) "A whole box is a link"
  text "Click anywhere in it."
}
```

**No links inside links**

A link, button or linked box can't contain another link, button or form field. In HTML this is a common source of broken pages; in SLML it is an error.

## Where a link can go

| Name | Values | Description |
| --- | --- | --- |
| `about.slml` | a page | Relative to the current file. Compiled sites link to about.html automatically. |
| `/docs/a.slml` | from the root | A leading slash starts at the site root, so the link works from any folder. |
| `#contact` | an anchor | An element on this page with id: "contact". It must exist. |
| `a.slml#top` | page + anchor | A place on another page. |
| `https://…` | a website | Also http:, mailto: (e-mail) and tel: (phone). Other schemes are rejected. |

Paths can't contain `..`. To reach a parent folder, start at the root with `/`. This keeps every link inside your site.

Write local file names literally: `reports/annual report 100%.pdf` or `żółć.slml`. The compiler handles spaces, Unicode and URL escaping for you. Do not replace a space with `%20` in an SLML file path; that would mean a file whose name contains the characters %20. External URLs keep their usual escaping.

**Broken links are errors**

When you compile or check a page, every link to a file in your site is checked. A typo in a file name is reported like any other mistake, before your visitors find it.

## Anchors

Give any block element an `id`, then link to it with `#`. Ids start with a letter and must be unique on the page.

```slml
text { link(to: "#faq") "Jump to the questions" }
...
heading(level: 2, id: "faq") "Questions"
```

The chapter headings of this handbook are anchors: try [this link](#kinds-of-links).

## Site navigation

A menu is a row of links. To show the same menu on every page, put it in a [component](components.md) in a shared file:

```slml
// parts.slml
slml 0.1

define menu {
  row(gap: 20, padding: 16, align: center) {
    text(bold: true) { link(to: "/index.slml") "My site" }
    spacer
    text { link(to: "/blog/index.slml") "Blog" }
    text { link(to: "/contact.slml") "Contact" }
  }
}
```

```slml
// any page, in any folder
slml 0.1
use "/parts.slml"

page { menu  heading "Welcome" }
```

[Lists, code and images](content.md)

[Components and imports](components.md)

[SLML source](../../handbook/links.slml)
