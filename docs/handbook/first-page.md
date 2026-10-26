# Your first page

In this chapter you write a small page and open it directly in SLML's own browser.

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

Open a terminal in the folder and start SLML:

```sh
slml index.slml
```

The native SLML browser opens your document. Change the text in `index.slml`, save, and the page updates by itself. Errors are drawn as an SLML error page in the same window. Close the window to exit.

## 3. Open the site folder

You can also give SLML the whole folder:

```sh
slml .
```

This opens `index.slml` and makes the folder the site root. Links to other `.slml` pages stay in SLML. Imported components and images reload when you save them. No HTML files are generated.

Use `slml render index.slml -o preview.png` when you want a picture of the page.

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
