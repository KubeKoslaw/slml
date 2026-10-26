# The SLML Handbook

SLML, the **Strict Layout Markup Language**, is a short, strict language for web pages and documents. One small grammar replaces HTML and CSS, and layout is built in. You write `.slml` files and the `slml` native browser reads and draws them directly.

```slml
slml 0.2
page{
  #"Hello!"
  r{g12 a:c
    btn{>#why "About me"}
    "Made with *SLML*"
  }
}
```

## Why SLML?

- **Short.** `c{p20 bg#fff "Hi"}` is a padded white box with a paragraph. Frequent things cost one or two characters.
- **Strict.** A mistake is an error with a line number and a hint, never a silently broken page.
- **Layout, not semantics.** Rows, columns, grids, gaps and alignment are the language. There is no `main`, `aside` or `section` with rules about where each may go.
- **One syntax.** Structure, appearance and reusable components share one grammar instead of three languages.
- **Predictable.** Fonts are bundled and every default is written down, so a page looks the same everywhere.
- **Accessible.** Images must describe themselves and form fields must have labels.

## How it works

The native browser is the normal way to use SLML:

1. **Open a page or site.** Run `slml page.slml` or `slml my-site/`. SLML opens its own window, parses the original source and draws it with its Rust renderer.
2. **Edit and save.** The window reloads when the page, an import or an image changes. Local links stay in SLML.
3. **Use adapters only when you choose them.** `slml build` explicitly exports HTML. The optional `slml serve --web` runtime and the Firefox extension host SLML in existing web browsers.

## Where to start

### [1. Install](install.md)

Get the slml program for Linux, macOS or Windows.

### [2. First page](first-page.md)

Write and open your first page in five minutes.

### [3. Learn the syntax](syntax.md)

Everything SLML can say, on one page.

**This handbook is written in SLML**

Every page you are reading is an `.slml` file in the `handbook/` folder of the repository. Open them to see real-world SLML.

[Reference](reference.md)

[Install SLML](install.md)

[SLML source](../../handbook/index.slml)
