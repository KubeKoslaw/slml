# Building a website

A website is a folder of `.slml` files, images and other files. One command turns it into a folder of plain HTML that any web host can serve.

## A site folder

```slml
my-site/
  layout.slml        styles and components (a library: no page)
  index.slml         → index.html, the home page
  about.slml         → about.html
  blog/
    index.slml       → blog/index.html
    first-post.slml  → blog/first-post.html
  img/
    logo.png         copied as it is
```

- Files with a `page` become HTML pages. Files with only `style` and `define` are libraries for `use`.
- Every other file (images, PDFs, downloads) is copied unchanged.
- Folders whose name starts with `_` or `.` are skipped, so you can keep drafts in `_drafts/`.
- Hidden files, `target/`, `node_modules/` and common working notes are excluded from builds and direct preview requests. Symbolic links inside the site are skipped during builds and rejected by the preview server and imports; keep shared components inside the site root.
- Paths starting with `/` mean the folder you build: `/img/logo.png` works from every page.

## Previewing

```sh
slml serve my-site --open
```

The browser loads the original SLML sources and runs the same parser, layout engine, painter and fonts as the native viewer through WebAssembly. It draws the document on a canvas instead of compiling its layout to HTML. Errors are shown in the browser, and pages reload when you save a file. The server only listens on your own computer (127.0.0.1).

Use `slml serve my-site --html` to explicitly preview the separate HTML export. In the default viewer, Source opens an editor; edits there stay in the tab. If files change on disk while you have browser edits, Reload discards the browser edits and reloads the files.

The default port is 4000. Use `slml serve my-site --port 8080` to choose another. If it is busy, SLML tries the next 19 ports and prints the address it actually uses. Stop the server with Ctrl+C.

**Previewing forms**

Fields and browser validation work in the preview. A GET form can navigate to a local page, preserving its query string. The preview server serves files only; it does not process POST submissions or store messages. For those, point your form at a working HTTPS endpoint.

## Building

```sh
slml my-site                 # into my-site/_site
slml my-site -o public       # into public/
```

The output contains your pages as `.html`, your other files, and a `_slml` folder with the stylesheet and fonts. Links to `.slml` pages are rewritten to `.html`, and paths are made relative, so the site also works when opened straight from disk or from a sub-folder of a server.

If any page has an error or a broken link, `slml` reports all of them and exits with status 1, which stops automated deployments before a broken site goes live.

## Publishing

The output is a static site: upload the output folder to any web host.

| Name | Values | Description |
| --- | --- | --- |
| `GitHub Pages` | free for public repos | Build in a GitHub Actions workflow and deploy the output folder with actions/deploy-pages. |
| `Netlify / Cloudflare` | free tiers | Build command: slml . -o public. Publish directory: public. |
| `Any server` | FTP, rsync | Copy the output folder to the server's web root. |

**Example: GitHub Pages workflow**

```yaml
# .github/workflows/pages.yml
on: { push: { branches: [main] } }
permissions: { pages: write, id-token: write, contents: read }
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: github-pages
    steps:
      - uses: actions/checkout@v5
      - run: curl -fsSL https://raw.githubusercontent.com/KubeKoslaw/slml/main/install.sh | sh
      - run: ~/.local/bin/slml . -o public
      - uses: actions/upload-pages-artifact@v3
        with: { path: public }
      - uses: actions/deploy-pages@v4
```

## What the HTML looks like

The compiler writes semantic HTML: headings are `h1`–`h4`, lists are `ul`/`ol`, forms are real forms. Layout uses CSS flexbox and grid, so pages are responsive. Built pages contain **no JavaScript**; the live-reload script exists only in the preview server.

[Forms](forms.md)

[A complete website](website-tutorial.md)

[SLML source](../../handbook/sites.slml)
