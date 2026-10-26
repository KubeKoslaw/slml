# Your first page

In this chapter you write a small page, preview it in your browser and compile it to HTML.

## 1. Write the page

Create a folder for your site and a file named `index.slml` in it:

```slml
slml 0.1

meta(title: "My first page")

page {
  heading "Hello, world!"
  text "This is my first SLML page."
  button(to: "#write") "Click me"
}
```

Every SLML file has the same three parts:

- `slml 0.1` — the version line. It is always first, so a page can never be misread by a newer compiler.
- `meta(...)` — information about the page: its title, language, description and icon. Optional.
- `page { ... }` — the visible content. Exactly one per file.

## 2. Preview it

Open a terminal in the folder and start the preview server:

```sh
slml serve --open
```

Your browser opens `http://127.0.0.1:4000/`. Change the text in `index.slml`, save, and the page updates by itself. If you make a mistake, the browser shows the error and where it is.

## 3. Compile it

When you are happy, compile the page:

```sh
slml index.slml
```

This writes `index.html` next to your file, plus a `_slml` folder with the stylesheet and fonts. Open `index.html` in any browser, or upload both to a web server.

Like a C compiler, `-o` chooses the output:

```sh
slml index.slml -o public/index.html    # HTML somewhere else
slml index.slml -o preview.png          # a picture of the page
```

## 4. Make a mistake on purpose

Change `heading` to `headng` and run `slml check index.slml`:

```slml
error: unknown element `headng`
 --> index.slml:6:3
  |
6 |   headng "Hello, world!"
  |   ^^^^^^
  = hint: did you mean `heading`?
```

SLML never guesses. Each mistake is reported with its place and a suggestion, and nothing is written until the file is correct.

[Installing](install.md)

[Syntax basics](syntax.md)

[SLML source](../../handbook/first-page.slml)
