# The SLML Handbook

SLML, the **Strict Layout Markup Language**, is a language for web pages and documents. It replaces HTML and CSS with one small, strict grammar in which layout is built in. You write `.slml` files and the `slml` compiler turns them into a website.

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
- **Real websites.** Multi-page sites, navigation, forms and responsive layouts compile to plain HTML that works in every browser.

## How it works

SLML documents can be shown in two ways:

1. **In a web browser.** `slml serve` runs SLML's own renderer through WebAssembly while you edit. The Firefox extension can also open .slml URLs directly. For browsers without an extension, `slml site/` exports a separate HTML and CSS site you can upload anywhere.
2. **In the native SLML viewer.** SLML also has its own renderer written in Rust, which draws pages without a web browser: `slml view page.slml`.

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
