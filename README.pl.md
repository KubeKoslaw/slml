# SLML

**SLML — Strict Layout Markup Language.** Ścisła alternatywa dla HTML, z układem strony wbudowanym w język i z własnym rendererem w Rust.

> 🇬🇧 [English version](README.md)

```slml
slml 0.1

style karta(padding: 20, radius: 12, fill: #ffffff, width: fill)

page(fill: #f6f6f9) {
  heading "Cześć, SLML!"
  row(gap: 16) {
    column.karta { text { "Układ jest " bold "wbudowany" "." } }
    column.karta { text { link(to: "about.slml") "Dlaczego nie HTML?" } }
  }
}
```

## Dlaczego?

HTML był rozbudowywany przez 30 lat tak, żeby żadna stara strona nie przestała
działać. SLML zaczyna od nowa. Zachowuje to, co dobre (drzewo elementów, linki,
tekst), i odrzuca historyczne błędy:

- **Ścisłość:** błędny dokument jest odrzucany z dokładnym numerem linii i
  podpowiedzią. SLML nie „naprawia” niczego po cichu.
- **Jedna składnia** dla struktury i wyglądu, zamiast trzech osobnych języków
  (HTML, CSS, JS).
- **Układ w języku:** `row`, `column`, `gap`, `fill`, `align` są od początku.
- **Przewidywalność:** fonty są wbudowane, a wszystkie wartości domyślne są
  opisane w specyfikacji.
- **Dostępność:** `image` wymaga opisu `alt`.

## Uruchomienie

```bash
cargo run --release -- examples/site/index.slml
```

Okno odświeża stronę samo po zapisaniu pliku. Linki działają, a Backspace
wraca do poprzedniej strony.

## Współpraca

Projekt jest otwarty dla ludzi i agentów AI. Dokumentacja jest po angielsku,
żeby mogło pomagać jak najwięcej osób:
[CONTRIBUTING.md](CONTRIBUTING.md), [AGENTS.md](AGENTS.md),
[SPEC.md](SPEC.md), [ROADMAP.md](ROADMAP.md).
