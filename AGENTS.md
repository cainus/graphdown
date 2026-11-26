# Working in this repository

This repo's docs are written in graphdown (@docs/syntax): `@path#fragment` references are checked links between docs and code. Before changing a definition or a section, run `graphdown refs <target>` to find the docs that describe it. If the binary isn't built, `cargo run -q -- <command>` works the same.

## When you change things

- Keep @docs/architecture and @docs/cli in step with the code. If you add a command, flag, module or language, document it in the same change.
- Reference code by symbol (`@src/ops.rs#check`), never by line number.
- A function that implements a documented behaviour ends its doc comment with `/// Spec: @docs/...#section`. Add one when you write such a function.
- Record a decision that rules something out in @docs/design.
- Before finishing: `cargo test`, `cargo clippy --all-targets`, and `cargo run -q -- check` must all pass.

## Conventions

- Rust, edition 2021, formatted with `cargo fmt`.
- Errors that reach the user are plain sentences naming the file and what was expected.
- Behaviour a user can see gets a test in @tests/cli.rs.
