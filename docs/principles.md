# Design principles

These principles decide arguments. A change that breaks one needs a written
reason in its pull request and an update to this file.

## 1. Strict over forgiving

HTML's "be liberal in what you accept" produced decades of incompatible error
recovery. SLML rejects invalid documents with a precise, helpful error.

- ✅ Unknown element → error with a suggestion.
- ❌ Ignoring unknown attributes "for forward compatibility". Use the version
  header instead.

## 2. One obvious way

Each concept has one spelling. When we add a feature, we check whether an
existing construct already covers it.

- Styles are attribute sets, not a second language with selectors.
- There are no named colors. Hex is the only color syntax.

## 3. Layout is the language

`row`, `column`, `gap`, `fill`, `align` and `justify` form the layout model.
We never add positioning tricks (floats, negative margins, absolute
positioning) to work around missing layout features. If a layout is hard to
express, extend the model in the spec.

## 4. Predictable rendering

The same file looks the same on every machine and every implementation:
fonts are bundled, every default is written in SPEC.md, and text is
literal.

## 5. Accessible by construction

If information is needed for accessibility (`alt`, link targets, later
labels for inputs), it is required by the grammar or the validator, not
recommended by a linter.

## 6. Efficient to parse and render

- LL(1) grammar, single pass, no backtracking.
- Layout is computed top-down with local measurement only. There are no
  global passes that can be invalidated by later content.
- No feature may require re-parsing a document.

## 7. Safe by default

Documents are data. No scripting in 0.x. When interactivity arrives, it must
be declarative and sandboxed. A document can never read local files outside
its own directory.

## 8. Small core, clear growth path

Prefer a few composable primitives over many special elements. New features
go through [ROADMAP.md](../ROADMAP.md) and the spec first, then the code.
