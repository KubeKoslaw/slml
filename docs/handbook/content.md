# Lists, code and images

The building blocks of real content: lists, code samples, images and dividers.

## Lists

```slml
list {
  item "Bullets are the default"
  item { "Items can contain " bold "inline" " elements" }
  list(marker: number) {
    item "A nested list follows an item"
    item "and is indented under it"
  }
}
```

| Name | Values | Description |
| --- | --- | --- |
| `marker` | bullet / number / none | The symbol before each item. Numbers count 1, 2, 3 … |
| `gap` | number | Space between items (default 6). |

A list contains only `item`s and nested `list`s; an `item` can only be inside a list.

## Code blocks

`codeblock` shows text in a monospaced font on a light background, keeping every space. It is ideal with a [block string](syntax.md#strings):

```slml
codeblock """
    fn main() {
        println!("Hello");
    }
    """
```

For a short piece of code inside a sentence use the inline `code` element.

## Images

```slml
row(gap: 16, align: center) {
  image(src: "/logo.png", alt: "SLML logo", width: 64, radius: 14)
  image(src: "/logo.png", alt: "Small SLML logo", width: 32)
  image(src: "missing.png", alt: "This image is missing", width: 160, height: 64)
}
```

- `src` is a path to a PNG or JPEG file (or an https address). `alt` describes the image for people who can't see it, and it is **required**.
- Purely decorative images say so with `alt: ""`.
- Without `width` and `height`, an image has its natural size but never overflows its container. Give one of them and the other follows the image's proportions.
- `radius`, `border`, `padding` and `shadow` work on images too, and `to` turns an image into a link.

## Dividers and spacers

```slml
text "Above"
divider
text "Below"
row(height: 40) { text "Left" divider(color: #2457d6, thickness: 2) text "Right" }
```

In a row a divider is vertical. A `spacer` is empty space: without a size it takes all free space, with `height: 24` it is a fixed gap.

[Boxes and styles](styling.md)

[Links and navigation](links.md)

[SLML source](../../handbook/content.slml)
