# SLML Viewer for Firefox

Opens HTTP(S) `.slml` addresses with the SLML parser, validator, layout engine
and software painter compiled to WebAssembly. Documents are not translated
into HTML elements and CSS layouts. A small extension page supplies the
canvas, navigation, source editor and interactive form controls. The same
runtime is used by `slml serve`.

## Build and load

Requires Rust 1.88+, its `wasm32-unknown-unknown` standard-library target,
Python 3, and Firefox 140 or newer on desktop.

```sh
rustup target add wasm32-unknown-unknown
make web
```

This creates:

- `target/slml-firefox-0.1.0.xpi`: the extension archive;
- `target/firefox/manifest.json`: the unpacked extension's manifest.

In Firefox, open `about:debugging#/runtime/this-firefox`, choose **Load
Temporary Add-on**, and select the XPI or the unpacked manifest. Allow its
website access when Firefox requests it. Visit an address ending in `.slml`,
or use the toolbar button to enter a page address.

This is an **unsigned development build**. Temporary extensions are removed
when Firefox closes. A persistent installation in standard Firefox needs
Mozilla signing; the project does not yet publish a signed add-on. Do not
disable Firefox's signing or security protections. See Mozilla's
[temporary installation guide](https://extensionworkshop.com/documentation/develop/temporary-installation-in-firefox/)
and [signing guide](https://extensionworkshop.com/documentation/publish/signing-and-distribution-overview/).

## Try it locally

```sh
make install
slml serve examples/starter --open
```

The default preview already uses the WASM renderer, even without an extension.
With the extension installed, opening
`http://127.0.0.1:4000/index.slml` directly also invokes the SLML viewer.
Use the actual port printed by `slml serve`.

For an ordinary static HTTP server, serve the original `.slml` files and
images, not the generated `.html` output. The extension handles `.slml`
navigations before the server's MIME type can cause a download.

## Supported behavior

- The complete SLML 0.1 syntax, styles, components and imports.
- Responsive layout, scrolling, clickable links, anchors and browser history.
- The bundled SLML fonts and same-origin PNG/JPEG images.
- Labeled form inputs, textareas, checkboxes, selects, required-field and
  input-type browser validation. GET forms targeting `.slml` pages stay in
  the viewer; other GET and POST submissions navigate to their endpoint.
- A **Source** editor with live validation. Edits stay in the tab; they do
  not write files or upload the document. Imported files still come from
  the original document origin.
- Errors include source file, line, column and hints. Invalid documents
  clear the previous rendering rather than showing partial output.

The origin (`scheme://host:port`) is the site root for `/` paths. Imports
and images are read only from that origin. Requests omit credentials and
reject redirects. Authenticated sites that need cookies, remote images,
local `file://` documents, text selection and a full screen-reader text
tree are not supported yet. Unavailable images use their `alt` placeholder.
The canvas is limited to 32 million device pixels per viewport.

## Permissions and privacy

The extension asks for HTTP(S) website access and `webRequest` /
`webRequestBlocking` so it can redirect top-level GET navigations ending
in `.slml` into its viewer. It does not intercept POST submissions, image
requests, normal HTML pages or its own source fetches. Broad host permission
is necessary to open a user-chosen SLML origin and its imports; the loader
then confines each document to that origin. There are no telemetry,
third-party rendering services, remote scripts or document scripts.

The [manifest's WebAssembly policy](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/content_security_policy)
allows the packaged WASM module; it does not allow JavaScript string
evaluation. Native browser form submissions send data only when the user
submits a form, to the destination specified by that document.

## Verify

```sh
make web
node web/test-wasm.mjs
npx web-ext lint --source-dir target/firefox --warnings-as-errors
cargo test --all
```

The Node test executes the actual WASM binary and checks pixels, imports,
images, diagnostics, control geometry, URL boundaries and Firefox request
interception. `web-ext` is an optional Mozilla development tool, not a
runtime dependency. Rust owns the parser and renderer; no JavaScript parser
or layout engine is maintained alongside it.
