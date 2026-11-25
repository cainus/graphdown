# Working in this repository

This repo's docs are written in graphdown (@docs/syntax): headings are symbols and `@path#fragment` references are checked edges between docs and code. Use the `graphdown` CLI to move around instead of reading whole files.

## Finding your way

- Start with `graphdown outline` for a map of every doc, its summary and its sections.
- Read one section with `graphdown show docs/cli#slice`, or one definition with `graphdown show src/ops.rs#slice`.
- For a task, `graphdown slice <section>` gives the section plus everything it references, in one read.
- Before changing a definition or a section, run `graphdown refs <target>` to find the docs that describe it.

If the binary isn't built, `cargo run -q -- <command>` works the same.

## When you change things

- Keep @docs/architecture and @docs/cli in step with the code. If you add a command, flag, module or language, document it in the same change.
- Reference code by symbol (`@src/ops.rs#slice`), never by line number.
- A function that implements a documented behaviour ends its doc comment with `/// Spec: @docs/...#section`. Add one when you write such a function.
- Record a decision that rules something out in @docs/design.
- Before finishing: `cargo test`, `cargo clippy --all-targets`, and `cargo run -q -- check` must all pass.

## Conventions

- Rust, edition 2021, formatted with `cargo fmt`.
- Errors that reach the user are plain sentences naming the file and what was expected.
- Behaviour a user can see gets a test in @tests/cli.rs.
