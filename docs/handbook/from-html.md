# Coming from HTML

SLML does what HTML and CSS do in a fraction of the characters, and without the parts that cause trouble. This page maps what you know to SLML.

```slml
c{bg#fff p20 r12 g8
  ##"Welcome"
  "Read *more* [here](/syntax.slml)."
}
```

The same in HTML and CSS needs a `div` with a `style` (or a class in a separate stylesheet), an `h2`, a `p`, a `strong` and an `a`: about four times as many characters.

## Element by element

**HTML**

**SLML**

`<!DOCTYPE html>`

`slml 0.2`

`<title>Home</title>`

`meta{"Home"}`

`<body>…</body>`

`page{…}`

`<div>, <main>, <aside>, <nav>, <section>`

`c{…} or r{…}`

`<h1>Title</h1>`

`#"Title"`

`<h2>Title</h2>`

`##"Title"`

`<p>Text</p>`

`"Text"`

`<b>, <strong>`

`*text*`

`<i>, <em>`

`_text_`

`<code>`

`` `text` ``

`<a href="x.html">text</a>`

`[text](x.slml)`

`<span style="color:red">`

`[text]{c#f00}`

`<img src="a.png" alt="A">`

`img{./a.png "A"}`

`<ul><li>One</li></ul>`

`-{"One"}`

`<ol>`

`+{…}`

`<pre>`

`code"…"`

`<hr>`

`|`

`<br>`

`\n inside a string`

`&amp; &lt; &nbsp;`

`just type & < and spaces`

`<form action=… method=post>`

`form{>… post …}`

`<label>Name <input name=n></label>`

`in{n:n "Name"}`

`<button type=submit>`

`send"Send"`

`<table>`

`grid{colsN …} or rows`

## CSS to attributes

`display: flex; flex-direction: column`

`c{…}`

`display: flex`

`r{…}`

`flex-wrap: wrap`

`wrap`

`grid-template-columns: repeat(3, 1fr)`

`grid{cols3 …}`

`repeat(auto-fill, minmax(200px, 1fr))`

`grid{cw200 …}`

`gap: 16px`

`g16`

`align-items: center`

`a:c`

`justify-content: space-between`

`j:b`

`flex: 1`

`w:f`

`max-width: 760px; margin: 0 auto`

`mw760 in a c{a:c …}`

`padding: 8px 16px`

`py8 px16`

`background: #fff`

`bg#fff`

`border-radius: 12px`

`r12`

`border: 1px solid #ddd`

`b1 bc#ddd`

`box-shadow: …`

`sh:s / sh:l`

`font-weight: bold`

`b`

`font-size: 18px`

`s18`

`color: #333`

`c#333`

`text-align: center`

`a:c (on text)`

`.card { … } + class="card"`

`@card{…} + c.card{…}`

## Things that work differently

- **No landmark elements.** There is no `main`, `aside`, `nav`, `header` or `section`, and no rules about which may contain which. A box is a box; place it wherever the layout needs it.
- **No margins.** Space between elements is the parent's `g`. There are no collapsing margins to fight.
- **No cascade.** A style applies only where you write `.name`. Only text properties (color, size, …) are inherited.
- **No JavaScript.** SLML pages are documents. Exported sites contain no scripts, which also makes them fast and safe.
- **Literal text.** Spaces and line breaks in strings are kept as written; only `*`, `_`, backticks and brackets format text.
- **Errors instead of repairs.** See [Understanding errors](errors.md).

[Understanding errors](errors.md)

[Reference](reference.md)

[SLML source](../../handbook/from-html.slml)
