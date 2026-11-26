# graphdown

A linter for markdown docs that keeps them pointing at real things.

- **Every link is checked.** Ordinary markdown links, docs-site style links, and `@path#fragment` references must land on a real file and heading.
- **Docs can point into code.** `@src/auth.rs#Session::refresh` must name a real definition. Rust, Python, TypeScript and JavaScript are parsed with tree-sitter, so references survive edits that move lines.
- **Code can point back.** A comment like `/// Spec: @docs/auth#token-refresh` is checked too.
- **Stale docs get flagged.** In a git repo, `check` lists doc lines that link to or name code you just changed but that your change didn't touch: the ones most likely to now be wrong.

Any existing markdown repo works as it is; `graphdown check` will find its broken links on the first run.

```markdown
## Token refresh

When a request gets a `401`, @src/auth.rs#Session::refresh swaps the refresh
token for a new pair. Network failures retry with @docs/sync#backoff-policy.
```

## Install

Releases are published to npm, PyPI and crates.io, with prebuilt binaries for macOS, Linux and Windows.

```sh
npx @cainus/graphdown check     # or: npm install -g @cainus/graphdown
uvx graphdown check             # or: pip install graphdown
cargo install graphdown
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/cainus/graphdown/releases/latest/download/graphdown-installer.sh | sh
```

Until the first release is tagged, install from source: `cargo install --git https://github.com/cainus/graphdown`.

### In CI

```yaml
- uses: actions/checkout@v4
- uses: cainus/graphdown@v0.1.0
```

Broken references fail the job and show up as annotations on the lines that contain them. Pass `args: --since origin/${{ github.base_ref }}` (with `fetch-depth: 0` on checkout) to also get notices on docs that name changed code.

## Commands

```sh
graphdown check                                 # every reference resolves, or exit 1; notes docs that name changed code
graphdown check --since origin/main             # in CI: compare against the base branch
graphdown refs src/auth.rs#Session::refresh     # who points at this, and which lines name it without a link?
graphdown fmt --to links                        # @refs -> clickable links (and --to at, back)
graphdown skill --install                       # tell your coding agent to run check before finishing
```

## Documentation

These docs are written in graphdown and checked in CI.

- @docs/syntax: headings, sections, the two reference forms, code fragments
- @docs/cli: every command and flag
- @docs/architecture: how the code is organised
- @docs/design: what was deliberately left out, and why
- @docs/benchmark: whether agents do better with graphdown (mostly not; see @docs/design#a-linter-not-a-navigator)
- @docs/releasing: how a tag becomes binaries, npm, PyPI and crates.io packages
- @AGENTS: how coding agents should work in this repo
- @skill/SKILL.md: the agent skill that `graphdown skill` prints, for any graphdown repo

## License

MIT
