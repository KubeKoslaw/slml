# Command line

`slml` opens original documents in its own native browser. Run `slml --help` or `man slml` for a summary.

## Opening documents

| Name | Values | Description |
| --- | --- | --- |
| `slml page.slml` | native browser | Read the original SLML document and draw it directly. |
| `slml site/` | site/index.slml | Open the site's home page. The folder is the root for absolute paths. |
| `slml handbook/` | native handbook | Read this guide in SLML's own browser. |

## Explicit exports

| Name | Values | Description |
| --- | --- | --- |
| `slml build page.slml` | page.html | Optional HTML compatibility export. |
| `slml build site/ -o public/` | HTML folder | Export a site for an existing HTML web host. |
| `slml page.slml -o out.html` | HTML file | An explicit output path also selects export mode. |
| `slml render page.slml -o page.png` | native PNG | Save a picture with SLML's renderer. |

## Commands

| Name | Values | Description |
| --- | --- | --- |
| `slml check …` | files or folders | Report errors without writing anything. Exit status 1 on errors. |
| `slml serve [folder]` | --port, --open, --web, --html | Serve original SLML sources. --open launches the native browser; --web or --html explicitly selects a web-browser adapter. |
| `slml view page-or-folder` | --scale | Explicit form of the default native browser action. |
| `slml render page.slml` | -o, --width, --scale | Render to PNG (same as -o page.png). |
| `slml upgrade …` | files or folders, --check | Rewrite slml 0.1 files in the short slml 0.2 syntax and re-format 0.2 files. Nothing is written unless every page stays exactly the same. --check only lists files that would change. |

## Upgrading older files

Pages written in the long syntax of SLML 0.1 still open, but the current syntax is 0.2. Convert a whole site at once:

```sh
slml upgrade my-site/          # rewrite every .slml file in place
slml upgrade --check my-site/  # only list files that would change
```

Before writing, `slml upgrade` compiles every page from the old and the new files, imports included, and refuses to write anything if a single page would differ. Comments are kept. Running it on 0.2 files gives them the standard formatting, so it also works as a formatter.

## Options

| Name | Values | Description |
| --- | --- | --- |
| `-o, --output` | path | Explicitly export a file or folder; .png uses the native image renderer, other output paths select HTML. |
| `--root` | folder | What paths starting with / refer to. Default: the current folder if the file is inside it, otherwise the file's folder. When building a folder, that folder. |
| `--width` | pixels | Page width for PNG output (default 900). |
| `--scale` | number | Pixel density for PNG and the viewer, e.g. 2 for sharp screens. |
| `--port` | number | First port serve tries (default 4000). |
| `--open` |  | serve: open the native SLML browser. With --web or --html, open the explicitly selected web adapter. |
| `--web` |  | serve: opt into the optional WebAssembly viewer hosted in a web browser. |
| `--html` |  | Explicit HTML export or HTML server preview. |
| `-q, --quiet` |  | Print only errors. |
| `-V, --version` |  | Compiler and language version. |

## The native browser

`slml` shows a page with SLML's own renderer in its own native window. Scroll with the mouse wheel, arrow keys or Page Up/Down, click links, press Backspace to go back and F5 to reload; the page also reloads when you save it, its imports or its images. External links open in your normal browser.

## Exit status

`0` on success; `1` if checking, export or opening a window fails. In the interactive browser an invalid page appears as an SLML error page. Use `slml check` for automated validation.

[A complete website](website-tutorial.md)

[Understanding errors](errors.md)

[SLML source](../../handbook/cli.slml)
