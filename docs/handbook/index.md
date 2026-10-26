# The SLML Handbook

SLML, the **Strict Layout Markup Language**, is a language for web pages and documents. It replaces HTML and CSS with one small, strict grammar in which layout is built in. You write `.slml` files and the `slml` native browser reads and draws them directly.

```slml
slml 0.1

page {
  heading "Hello!"
  row(gap: 12) {
    button(to: "about.slml") "About me"
    text { "Made with " bold "SLML" }
  }
}
```

## Why SLML?

- **Strict.** A mistake is an error with a line number and a hint, never a silently broken page.
- **Layout first.** Rows, columns, grids, gaps and alignment are part of the language; there are no floats or positioning tricks.
- **One syntax.** Structure, appearance and reusable components share one grammar instead of three languages.
- **Predictable.** Fonts are bundled and every default is written down, so a page looks the same everywhere.
- **Accessible.** Images must describe themselves and form fields must have labels.
- **Real websites.** Multi-page documents, navigation, reusable components and responsive layouts are written directly in SLML.

## How it works

The native browser is the normal way to use SLML:

1. **Open a page or site.** Run `slml page.slml` or `slml my-site/`. SLML opens its own window, parses the original source and draws it with its Rust renderer.
2. **Edit and save.** The window reloads when the page, an import or an image changes. Local links stay in SLML.
3. **Use adapters only when you choose them.** `slml build` explicitly exports HTML. The optional `slml serve --web` runtime and Firefox extension host SLML in existing web browsers.

## Where to start

### [1. Install](install.md)

Get the slml compiler for Linux, macOS or Windows.

### [2. First page](first-page.md)

Write, preview and compile your first page in five minutes.

### [3. Build a site](sites.md)

Share a header across pages and publish the result.

**This handbook is written in SLML**

Every page you are reading is an `.slml` file in the `handbook/` folder of the repository. Open them to see real-world SLML.

[Reference](reference.md)

[Install the compiler](install.md)

[SLML source](../../handbook/index.slml)
