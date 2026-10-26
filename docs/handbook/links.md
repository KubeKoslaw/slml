# Links and navigation

Anything can be a link: a piece of text, a button, an image or a whole box. In text a link is `[text](target)`; on an element it is `>target`.

## Kinds of links

```slml
"Read the [text chapter](text.slml)."
r{g12
  btn{>#targets "A button"}
  img{/logo.png w40 >/index.slml "Home"}
}
c{>/layout-guide.slml bg#f4f4f8 p14 r10
  t{b "A whole box is a link"}
  "Click anywhere in it."
}
```

**No links inside links**

A link, button or linked box can't contain another link, button or form field. In HTML this is a common source of broken pages; in SLML it is an error.

## Where a link can go

| Name | Values | Description |
| --- | --- | --- |
| `about.slml` | a page | Relative to the current file. HTML exports link to about.html automatically. |
| `/docs/a.slml` | from the root | A leading slash starts at the site root, so the link works from any folder. |
| `#contact` | an anchor | An element on this page with id:contact. It must exist. |
| `a.slml#top` | page + anchor | A place on another page. |
| `https://…` | a website | Also http:, mailto: (e-mail) and tel: (phone). Other schemes are rejected. |

After `>` a target runs until a space or brace. A target with spaces is quoted: `>"reports/annual report.pdf"`. In `[text](target)` everything up to the closing parenthesis is the target.

Paths can't contain `..`. To reach a parent folder, start at the root with `/`. This keeps every link inside your site.

Write local file names literally: `reports/annual report 100%.pdf` or `żółć.slml`. The tools handle spaces, Unicode and URL escaping for you. Do not replace a space with `%20` in an SLML file path; that would mean a file whose name contains the characters %20. External URLs keep their usual escaping.

**Broken links are errors**

When you check or export a page, every link to a file in your site is checked. A typo in a file name is reported like any other mistake, before your readers find it.

## Anchors

Give any block element an `id`, then link to it with `#`. Ids start with a letter and must be unique on the page.

```slml
"[Jump to the questions](#faq)"
…
##{id:faq "Questions"}
```

The chapter headings of this handbook are anchors: try [this link](#kinds-of-links).

## Site navigation

A menu is a row of links. To show the same menu on every page, put it in a [component](components.md) in a shared file:

```slml
// parts.slml
slml 0.2
@menu(){
  r{g20 p16 a:c
    t{b "[My site](/index.slml)"}
    ~
    "[Blog](/blog/index.slml)"
    "[Contact](/contact.slml)"
  }
}
```

```slml
// any page, in any folder
slml 0.2
use"/parts.slml"
page{menu{} #"Welcome"}
```

[Lists, code and images](content.md)

[Components and imports](components.md)

[SLML source](../../handbook/links.slml)
