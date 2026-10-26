# Contributing to SLML

Thanks for helping! This project is built by people and AI agents working
together, so it relies on clear rules. They are short.

## Ways to help

- **Try it and report problems.** A confusing error message is a bug.
- **Write example documents** in `examples/`. They show what's missing.
- **Pick a roadmap item.** Items marked *good first issue* in
  [ROADMAP.md](ROADMAP.md) are self-contained.
- **Propose language changes.** Open an issue with the *Language proposal*
  template before writing code.

## Workflow

1. Fork the repository and create a branch.
2. Make one focused change, with tests.
3. Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` and
   `cargo test --all`.
4. Open a pull request and fill in the template.

## Language changes

The language is defined by [SPEC.md](SPEC.md) and guided by
[docs/principles.md](docs/principles.md). A pull request that changes what
documents mean must:

- update `SPEC.md`,
- explain which principle it serves and whether it breaks existing
  documents (if it does, the language version must change),
- add tests and, for new features, an example.

## Using AI agents

You are welcome to use AI agents. The agent instructions live in
[AGENTS.md](AGENTS.md); `CLAUDE.md` points to the same file. You are
responsible for what you submit: read the diff before opening a pull
request.

## License

By contributing, you agree that your contributions are licensed under the
MIT and Apache-2.0 licenses, like the rest of the code.
