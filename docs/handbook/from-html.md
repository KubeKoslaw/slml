# Coming from HTML

If you know HTML and CSS, most of SLML will feel familiar. This page maps what you know to SLML.

## Element by element

**HTML**

**SLML**

`<!DOCTYPE html>`

`slml 0.1`

`<title>Home</title>`

`meta(title: "Home")`

`<body>…</body>`

`page { … }`

`<div>, <section>, <header>`

`column { … } or row { … }`

`<h1>Title</h1>`

`heading "Title"`

`<h2>Title</h2>`

`heading(level: 2) "Title"`

`<p>Text</p>`

`text "Text"`

`<b>, <strong>`

`bold "…"`

`<i>, <em>`

`italic "…"`

`<a href="x.html">`

`link(to: "x.slml") "…"`

`<img src="a.png" alt="A">`

`image(src: "a.png", alt: "A")`

`<ul><li>One</li></ul>`

`list { item "One" }`

`<ol>`

`list(marker: number)`

`<pre>`

`codeblock`

`<hr>`

`divider`

`<br>`

`\n inside a string`

`&amp; &lt; &nbsp;`

`just type & < and spaces`

`<form action=… method=post>`

`form(to: …, method: post)`

`<label>Name <input name=n></label>`

`input(name: "n", label: "Name")`

`<button type=submit>`

`submit "Send"`

`<table>`

`grid(columns: N) or rows`

## CSS to attributes

`display: flex; flex-direction: column`

`column`

`display: flex`

`row`

`flex-wrap: wrap`

`row(wrap: true)`

`display: grid; grid-template-columns: repeat(3, 1fr)`

`grid(columns: 3)`

`repeat(auto-fill, minmax(200px, 1fr))`

`grid(cell-width: 200)`

`gap: 16px`

`gap: 16`

`align-items: center`

`align: center`

`justify-content: space-between`

`justify: between`

`flex: 1`

`width: fill`

`max-width: 760px; margin: 0 auto`

`max-width: 760 inside a column(align: center)`

`padding: 8px 16px`

`padding-y: 8, padding-x: 16`

`background: #fff`

`fill: #fff`

`border-radius: 12px`

`radius: 12`

`border: 1px solid #ddd`

`border: 1, border-color: #ddd`

`box-shadow: …`

`shadow: small / large`

`font-weight: bold`

`bold: true`

`font-size: 18px`

`size: 18`

`color: #333`

`color: #333`

`text-align: center`

`align: center (on text)`

`.card { … } + class="card"`

`style card(…) + column.card`

## Things that work differently

- **No margins.** Space between elements is the parent's `gap`. There are no collapsing margins to fight.
- **No cascade.** A style applies only where you write `.name`. Only text properties (color, size, …) are inherited.
- **No JavaScript.** SLML pages are documents. Built sites contain no scripts, which also makes them fast and safe.
- **Literal text.** Spaces and line breaks in strings are kept as written.
- **Errors instead of repairs.** See [Understanding errors](errors.md).

[Understanding errors](errors.md)

[Reference](reference.md)

[SLML source](../../handbook/from-html.slml)
