# Lists, code and images

The building blocks of real content: lists, code samples, images and dividers.

## Lists

```slml
-{
  "Bullets: every string is an item"
  "Items can contain *formatting*"
  +{"A nested list follows an item" "and is indented under it"}
}
-"One item" -"after another"
```

| Name | Values | Description |
| --- | --- | --- |
| `-{…}` | bullets | • before each item. |
| `+{…}` | numbers | 1. 2. 3. — nested lists count separately. |
| `={…}` | no marker | Items without a marker. |
| `g8` | number | Space between items (default 6). |

`-"a" -"b"` written one after the other form a single list, so short lists need no braces.

## Code blocks

`code` shows text in a monospaced font on a light background, keeping every space. It is ideal with a [block string](syntax.md#strings-and-text):

```slml
code"""
    fn main() {
        println!("Hello");
    }
    """
```

For a short piece of code inside a sentence use backticks: `` `like this` ``.

## Images

```slml
r{g16 a:c
  img{/logo.png w64 r14 "SLML logo"}
  img{/logo.png w32 "Small SLML logo"}
  img{./missing.png w160 h64 "This image is missing"}
}
```

- The path comes first: `/img/a.png` from the site root, `./a.png` next to the page, or an https address. PNG and JPEG are supported.
- The string describes the image for people who can't see it, and it is **required**. Purely decorative images say so with an empty string `""`.
- Without `w` and `h`, an image has its natural size but never overflows its box. Give one of them and the other follows the image's proportions.
- `r`, `b`, `p` and `sh` work on images too, and `>target` turns an image into a link.

## Dividers and spacers

```slml
"Above"
|
"Below"
r{h40 g8 "Left" |{c#2457d6 th2} "Right"}
```

In a row a divider is vertical. A spacer `~` is empty space: without a size it takes all free space, `~{h24}` is a fixed gap.

[Boxes and styles](styling.md)

[Links and navigation](links.md)

[SLML source](../../handbook/content.slml)
