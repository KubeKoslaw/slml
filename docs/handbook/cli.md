# Command line

`slml` works like a classic compiler: give it input files, get output files. Run `slml --help` or `man slml` for a summary.

## Compiling

| Name | Values | Description |
| --- | --- | --- |
| `slml page.slml` | → page.html | Compile one or more pages next to themselves. |
| `slml page.slml -o out.html` | -o file | Choose the output file. |
| `slml page.slml -o page.png` | .png | Render a picture of the page with the native renderer. |
| `slml site/` | → site/_site/ | Build a whole site folder. |
| `slml site/ -o public/` | -o folder | Build a site into a chosen folder. |

## Commands

| Name | Values | Description |
| --- | --- | --- |
| `slml check …` | files or folders | Report errors without writing anything. Exit status 1 on errors. |
| `slml serve [folder]` | --port, --open, --html | Live preview with the SLML WebAssembly renderer. --html explicitly selects the HTML export preview. |
| `slml view page.slml` | --scale | Open the page in the native SLML viewer window. |
| `slml render page.slml` | -o, --width, --scale | Render to PNG (same as -o page.png). |

## Options

| Name | Values | Description |
| --- | --- | --- |
| `-o, --output` | path | Output file or folder. |
| `--root` | folder | What paths starting with / refer to. Default: the current folder if the file is inside it, otherwise the file's folder. When building a folder, that folder. |
| `--width` | pixels | Page width for PNG output (default 900). |
| `--scale` | number | Pixel density for PNG and the viewer, e.g. 2 for sharp screens. |
| `--port` | number | First port serve tries (default 4000). |
| `--open` |  | serve: open the preview in your browser. |
| `--html` |  | serve: preview the HTML export instead of drawing SLML with WebAssembly. |
| `-q, --quiet` |  | Print only errors. |
| `-V, --version` |  | Compiler and language version. |

## The native viewer

`slml view` shows a page with SLML's own renderer, without a web browser. Scroll with the mouse wheel, arrow keys or Page Up/Down, click links, press Backspace to go back and F5 to reload; the page also reloads when you save it. External links open in your normal browser.

## Exit status

`0` when everything compiled, `1` when any file has errors. Scripts and CI can rely on it.

[A complete website](website-tutorial.md)

[Understanding errors](errors.md)

[SLML source](../../handbook/cli.slml)
