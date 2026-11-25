# graphdown

Navigate markdown docs the way agents navigate code.

Code has imports and function names, so an agent can jump straight to the definition it needs and find everything that uses it. Docs usually have neither: an agent reads whole files and follows links blind. Graphdown gives prose the same structure without inventing a new format:

- **Headings are symbols.** `## Token TTL` in `docs/auth.md` is addressable as `@docs/auth#token-ttl`.
- **References are edges.** Write `@docs/auth#token-ttl` in prose, or an ordinary markdown link. Point into code too: `@src/auth.rs#Session::refresh`.
- **The CLI is the compiler.** `graphdown check` fails on any reference that doesn't land, and the other commands let you and your agents move around the graph.

Any existing markdown repo is already valid graphdown; `graphdown check` will find its broken links on the first run.

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

Broken references fail the job and show up as annotations on the lines that contain them.

## Commands

```sh
graphdown check                                 # every reference resolves, or exit 1
graphdown outline                               # every doc: summary, headings, line ranges
graphdown show docs/auth#token-refresh          # print one section
graphdown refs src/auth.rs#Session::refresh     # who points at this?
graphdown slice docs/auth#token-refresh         # the section plus everything it depends on
graphdown fmt --to links                        # @refs -> clickable links (and --to at, back)
graphdown skill --install                       # teach your coding agent to use all of the above
```

## Documentation

These docs are written in graphdown and checked in CI.

- @docs/syntax: headings, sections, the two reference forms, code fragments
- @docs/cli: every command and flag
- @docs/architecture: how the code is organised
- @docs/design: what was deliberately left out, and why
- @docs/benchmark: how we'll test whether agents actually do better with graphdown
- @docs/releasing: how a tag becomes binaries, npm, PyPI and crates.io packages
- @AGENTS: how coding agents should work in this repo
- @skill/SKILL.md: the agent skill that `graphdown skill` prints, for any graphdown repo

## License

MIT
