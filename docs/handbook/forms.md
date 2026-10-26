# Forms

Contact forms, sign-ups and surveys are written with `form` and a few field elements. Every field must have a visible label.

## A contact form

```slml
form(to: "https://formspree.io/f/your-id", method: post, gap: 14) {
  input(name: "name", label: "Your name", required: true)
  input(name: "email", label: "E-mail", kind: email,
        placeholder: "you@example.com", required: true)
  select(name: "topic", label: "Topic") {
    option "Question"
    option "Feedback"
  }
  textarea(name: "message", label: "Message", rows: 4)
  checkbox(name: "newsletter", label: "Send me news")
  row { submit "Send" }
}
```

## Where the data goes

`to` is the address that receives the form, and `method` is `get` (the default, for searches) or `post` (for messages). A static website can't store messages itself, so use a form service such as Formspree, Basin or Netlify Forms, or your own server.

**Native viewer**

In the native SLML viewer forms are shown but can't be typed into yet. The optional web adapter (`slml serve --web --open`) and HTML exports support editing and submission. Native form interaction remains planned.

## Fields

| Name | Values | Description |
| --- | --- | --- |
| `input` | kind, placeholder, value | One line of text. kind is text, email, password, number, tel, url, date or search. |
| `textarea` | rows, placeholder, value | Several lines of text (rows: 1–50, default 4). |
| `checkbox` | checked | A yes/no box. Its label is shown next to it. |
| `select` | option … | A drop-down list of options. option takes a label, an optional value and selected: true. |
| `submit` | label | The button that sends the form. Styled like button. |

All fields need `name` (what the value is sent as) and `label` (what people see). `required: true` marks a field that must be filled in; its label gets a `*`.

**Rules**

- Fields and submit only work inside a form, and forms can't be nested.
- A field without a label is an error: screen readers and people both need it.
- A select needs at least one option, and only one can be selected.

[Components and imports](components.md)

[Building a website](sites.md)

[SLML source](../../handbook/forms.slml)
