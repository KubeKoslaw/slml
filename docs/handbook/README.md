# SLML Handbook

Read these Markdown chapters on GitHub, or open this folder in Obsidian.
The canonical source is the [SLML handbook](../../handbook/).

1. [The SLML Handbook](index.md)
2. [Installing the compiler](install.md)
3. [Your first page](first-page.md)
4. [Syntax basics](syntax.md)
5. [Text](text.md)
6. [Layout](layout-guide.md)
7. [Boxes and styles](styling.md)
8. [Lists, code and images](content.md)
9. [Links and navigation](links.md)
10. [Components and imports](components.md)
11. [Forms](forms.md)
12. [Building a website](sites.md)
13. [A complete website](website-tutorial.md)
14. [Command line](cli.md)
15. [Understanding errors](errors.md)
16. [Coming from HTML](from-html.md)
17. [Reference](reference.md)

To refresh this edition after changing the SLML sources:

```sh
cargo build -p slml-cli
python3 scripts/sync-handbook-markdown.py
```
