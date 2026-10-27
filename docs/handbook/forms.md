# Forms

Contact forms, sign-ups and surveys are written with `form` and a few field elements. Every field must have a visible label: the field's string.

## A contact form

```slml
form{>https://formspree.io/f/your-id post g14
  in{n:name "Your name" req}
  in{n:email "E-mail" k:email ph"you@example.com" req}
  sel{n:topic "Topic" opt"Question" opt"Feedback"}
  area{n:message "Message" rows4}
  check{n:newsletter "Send me news"}
  r{send"Send"}
}
```

## Where the data goes

`>target` is the address that receives the form; the flag `post` sends it as a message (the default `get` is for searches). A static website can't store messages itself, so use a form service such as Formspree, Basin or Netlify Forms, or your own server.

**In the native browser**

Click a field or Tab to it and type; Backspace, Delete, the arrow keys, Home, End and Ctrl+V work as usual, and Enter sends the form (in an `area` it starts a new line). Space ticks a checkbox; a `sel` opens a list. Before sending, required fields, e-mail addresses, numbers and URLs are checked and the first problem is focused. A `get` form for a page of your site opens that page; a form for a website is handed to your normal web browser, which sends it.

## Fields

| Name | Values | Description |
| --- | --- | --- |
| `in{…}` | k, ph, v | One line of text. k:text, k:email, k:password, k:number, k:tel, k:url, k:date or k:search. |
| `area{…}` | rows, ph, v | Several lines of text (rows1 to rows50, default 4). |
| `check{…}` | on | A yes/no box. Its label is shown next to it; on ticks it. |
| `sel{…}` | opt… | A drop-down list. opt"Label" or opt{v"value" on "Label"} for one choice. |
| `send"…"` | label | The button that sends the form. Styled like btn. |

All fields need `n:name` (what the value is sent as) and a label string (what people see). The flag `req` marks a field that must be filled in; its label gets a `*`.

**Rules**

- Fields and send only work inside a form, and forms can't be nested.
- A field without a label is an error: screen readers and people both need it.
- A sel needs at least one opt, and only one can be on.

[Components and imports](components.md)

[Building a website](sites.md)

[SLML source](../../handbook/forms.slml)
