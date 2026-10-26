# SLML

**SLML — Strict Layout Markup Language.** Ścisła alternatywa dla HTML, z układem strony wbudowanym w język i z własnym rendererem w Rust.

> 🇬🇧 [English version](README.md)

```slml
slml 0.2
@karta{p20 r12 bg#fff w:f}

page{bg#f6f6f9
  #"Cześć, SLML!"
  r{g16
    c.karta{"Układ jest *wbudowany*."}
    c.karta{"[Dlaczego nie HTML?](about.slml)"}
  }
}
```

Krótko do pisania: `c` to kolumna, `r` wiersz, `p20` odstęp wewnętrzny,
`bg#fff` tło, `#"…"` nagłówek, a zwykły napis to akapit z `*pogrubieniem*`,
`_kursywą_` i `[linkami](…)`. Pliki w starszej, długiej składni
(`slml 0.1`) nadal działają; `slml upgrade` je przepisuje.

## Dlaczego?

HTML był rozbudowywany przez 30 lat tak, żeby żadna stara strona nie przestała
działać. SLML zaczyna od nowa. Zachowuje to, co dobre (drzewo elementów, linki,
tekst), i odrzuca historyczne błędy:

- **Ścisłość:** błędny dokument jest odrzucany z dokładnym numerem linii i
  podpowiedzią. SLML nie „naprawia” niczego po cichu.
- **Jedna składnia** dla struktury i wyglądu, zamiast trzech osobnych języków
  (HTML, CSS, JS).
- **Układ w języku:** `r`, `c`, `grid`, `g`, `w:f`, `a` są od początku.
- **Bez semantycznych „regionów”:** nie ma `main`, `aside`, `nav`, `section` ani reguł, co może być w czym. Pudełko to tylko układ.
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
