# A complete website

Start with a working four-page website, then make it yours. The starter in examples/starter includes a home page, an about page, a contact form and its local preview destination.

## 1. Copy and preview

From a checkout of the SLML repository:

```sh
cp -R examples/starter my-site
slml serve my-site --open
```

On Windows, copy the examples/starter folder in File Explorer and run the same slml serve command. Keep the terminal open while editing. The browser reloads after each save; an invalid page shows an error with the file, line and a hint.

```slml
my-site/
  layout.slml     shared styles, site frame and service cards
  index.slml      home page and responsive service grid
  about.slml      studio story and process
  contact.slml    labeled fields and required-field validation
  preview.slml    local destination for the demo GET form
```

## 2. Make the layout yours

Open `layout.slml`. Change the studio name, colors and menu once; every page imports this file with `use "layout.slml"`. The `site` component provides the frame, and `slot` receives the content of each page.

```slml
slml 0.1
use "layout.slml"

site(title: "About — My studio", description: "What we do and how we work.") {
  heading "A useful, welcoming website."
  text "Replace this with your own story."
}
```

Set the right language in the shared `meta` element. Each page supplies its own title and search description through the component parameters. Add images with `image(src: "photo.jpg", alt: "A description of the photo")`. Put photo.jpg next to the page.

The service grid uses `grid(columns: 3, cell-width: 260, gap: 16)`: up to three equal columns, fewer when the window is narrow. The menu uses a wrapping row, so links move to another line instead of disappearing on a phone.

## 3. Try the form, then connect a real endpoint

Open Contact from the menu. Submit it empty first: the browser should focus a required field. Enter made-up details, select a service, write a message and check the demo box. Submitting should take you to the local preview page.

**What the demo does**

The starter uses GET so you can test navigation without a server or account. Values are visible in the URL. The destination does not read them, store them or send a message. Use made-up details only.

For real messages, replace the form's `to` with your HTTPS endpoint and change `method` to `post`:

```slml
form.card(to: "https://example.com/contact", method: post, gap: 18) {
  input(name: "name", label: "Your name", required: true)
  input(name: "email", label: "Email address", kind: email, required: true)
  textarea(name: "message", label: "Message", required: true)
  submit.action "Send message"
}
```

Use an endpoint you control or the address supplied by a form service; example.com is only a placeholder. The endpoint must accept URL-encoded POST data and return a confirmation or redirect. Replace the demo checkbox and explanatory copy with wording appropriate to your actual service. Test the full submission before publishing.

See [Forms](forms.md) for the available field types and validation rules.

## 4. Check the complete site

1. Follow every menu link from every page, including the Services anchor.
2. Make the browser narrow: the menu should wrap and the services should form one column.
3. Use Tab, Shift+Tab and Enter to navigate links and fields without a mouse.
4. Try an empty form, an invalid email and a complete submission.
5. Save an invalid element name and confirm that the preview shows a useful error. Fix it and confirm that the page returns.

```sh
slml check my-site
```

This checks every page and local link. A failed check exits with status 1. It does not contact external websites or verify that a form service accepts your data.

## 5. Build and publish

```sh
slml my-site -o public
```

Upload the contents of `public/`, including `_slml/`, to your static web host. Your shared layout file is a library and does not become an extra page. The output contains HTML, CSS and bundled fonts, with no JavaScript.

See [Building a website](sites.md#publishing) for hosting examples. Keep the source folder: edit the .slml files and rebuild whenever you update the site.

[Building a website](sites.md)

[Command line](cli.md)

[SLML source](../../handbook/website-tutorial.slml)
