# Graphdown syntax

Graphdown is plain CommonMark plus one convention: a reference written `@path#fragment`. Headings are the symbols, references are the edges, and `graphdown check` makes sure every edge lands somewhere real.

## Symbols are headings

Every heading is addressable by its GitHub-style slug: lowercase, letters, digits, `_` and `-` kept, spaces turned into `-`, everything else dropped. `## Token TTL` is `#token-ttl`. A second heading with the same text gets `-1`, a third `-2`, exactly as GitHub numbers its anchors, so the same fragment works in `graphdown` and in a browser.

An explicit `{#id}` after a heading overrides the slug when a stable name matters more than the wording.

### Sections

A heading owns the lines from itself down to the next heading of the same or a higher level, minus trailing blank lines. So a `##` section includes its `###` subsections. This is the unit that @docs/cli#show prints and @docs/cli#slice collects.

The whole document is addressed by its path with no fragment. Its title is the first `#` heading and its summary is the first paragraph, which is what @docs/cli#outline shows. There are no other declarations to keep in sync; see @docs/design#no-import-or-export-lists.

## References

A reference can be written two ways, and both are checked.

### At references

`@` followed by a path relative to the project root, then optionally `#` and a fragment:

| Written                          | Points at                                    |
| -------------------------------- | -------------------------------------------- |
| `@docs/syntax`                   | the whole doc `docs/syntax.md`               |
| `@docs/syntax#sections`          | one section of it                            |
| `@src/project.rs`                | a source file                                |
| `@src/project.rs#Project::resolve` | one definition in it                       |
| `@src/project.rs#L10-L20`        | a line range (GitHub style)                  |

The project root is the nearest folder holding `graphdown.toml` or `.git`. Doc paths may leave off `.md`. Paths are always root-relative, never relative to the current file, so a reference means the same thing wherever it is pasted.

A reference ends at the first character that cannot be part of a path or fragment, and trailing `.` or `:` are treated as sentence punctuation. So "see @docs/syntax." works.

Not references:

- anything in an inline code span or a fenced code block, which is how this page shows examples
- an `@` straight after a letter, digit, `.`, `/`, `-` or backslash, so `me@example.com` and `\@` are left alone
- anything in YAML frontmatter

An `@` reference only counts when the first part of its path exists in the project, so `@vitejs/plugin-react` or `@alice` in someone's prose or comments is left alone. The price is that a typo in that first part (`@doc/syntax` for `@docs/syntax`) goes unreported, so backticks are still the clearer way to write a package name or a handle: `` `@scope/package` ``.

### Markdown links

Ordinary links are references too: `[Sections](syntax.md#sections)`, `[the resolver](../src/project.rs#Project::resolve)`. Their paths are relative to the linking file, as everywhere else in markdown, and a leading `/` means the project root. Links to URLs are not checked.

Docs sites spell links in ways GitHub doesn't, so when a link's path is not itself a file, graphdown also tries the spellings they use. No configuration is needed.

- `guide/api` for `guide/api.md`, and a folder for its `index.md` or `README.md`
- `page.html` for `page.md`, as mdBook writes it
- `/guide/api`: the repo root first, then each folder above the linking file, which finds `docs/guide/api.md` for VitePress, Docusaurus or MkDocs sites. Static files are also looked for in a `public/` or `static/` folder there.
- a fragment that differs from the GitHub slug only in punctuation, such as `#build-assetsinlinelimit` for `## build.assetsInlineLimit`

Links to mail addresses, written `<me@example.com>`, are not checked. The candidates are listed by @src/markdown.rs#TargetSpec::candidates.

Links are clickable on GitHub and in editors; `@` references are shorter to write and read. @docs/cli#fmt converts between them without losing anything, so a repo can author in one form and publish in the other.

## Code fragments

After a source path, the fragment names a definition rather than a line, so references survive edits that move code around and break only when the code is renamed or deleted, which is exactly when the docs need attention.

Names are qualified by their container: `Type::method` for Rust, `Class.method` for TypeScript, JavaScript and Python. Either separator is accepted for any language. A definition's range starts at the doc comments, attributes or decorators directly above it.

| Language                  | Extensions                     | Definitions found                                                     |
| ------------------------- | ------------------------------ | --------------------------------------------------------------------- |
| Rust                      | `.rs`                          | fn, struct, enum, union, trait, type, const, static, macro, mod, impl methods |
| TypeScript                | `.ts` `.tsx` `.mts` `.cts`     | function, class and its methods and fields, interface and its members, type, enum, const, namespace |
| JavaScript                | `.js` `.jsx` `.mjs` `.cjs`     | function, class and its methods and fields, const                     |
| Python                    | `.py` `.pyi`                   | def, class and its methods, module-level assignments                  |

Symbol extraction is @src/code.rs#parse; adding a language means one more walker there.

## References from code

Code can point back at docs. Any `@` reference inside a comment in a supported source file is checked like one in a doc, shows up in @docs/cli#refs, and is followed by @docs/cli#slice:

```rust
/// Swaps the refresh token for a new pair.
///
/// Spec: @docs/auth#token-refresh
pub fn refresh(&mut self) { /* ... */ }
```

The prose rules apply, with one more: in code, a reference must contain `/` or `#`. That keeps comment tags such as JSDoc's `@param` and `@returns` or Python's `@property` from being read as references. Backticked references and string literals are ignored, and Python docstrings count as strings, not comments.

There is no required marker. `Spec:` is just a readable convention, used throughout this repo's own source. Back-links are optional and checked one way: a doc pointing at a function does not oblige the function to point back (see @docs/design#back-links-from-code).

## Frontmatter

Graphdown neither needs nor reads frontmatter. It is skipped when parsing and left untouched by @docs/cli#fmt, so tools that rely on it keep working.
