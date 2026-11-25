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

Write `` `@scope/package` `` in backticks when you mean an npm package or a person's handle.

### Markdown links

Ordinary links are references too: `[Sections](syntax.md#sections)`, `[the resolver](../src/project.rs#Project::resolve)`. Their paths are relative to the linking file, as everywhere else in markdown, and a leading `/` means the project root. Links to URLs are not checked.

Links are clickable on GitHub and in editors; `@` references are shorter to write and read. @docs/cli#fmt converts between them without losing anything, so a repo can author in one form and publish in the other.

## Code fragments

After a source path, the fragment names a definition rather than a line, so references survive edits that move code around and break only when the code is renamed or deleted, which is exactly when the docs need attention.

Names are qualified by their container: `Type::method` for Rust, `Class.method` for TypeScript, JavaScript and Python. Either separator is accepted for any language. A definition's range starts at the doc comments, attributes or decorators directly above it.

| Language                  | Extensions                     | Definitions found                                                     |
| ------------------------- | ------------------------------ | --------------------------------------------------------------------- |
| Rust                      | `.rs`                          | fn, struct, enum, union, trait, type, const, static, macro, mod, impl methods |
| TypeScript                | `.ts` `.tsx` `.mts` `.cts`     | function, class and its methods and fields, interface, type, enum, const, namespace |
| JavaScript                | `.js` `.jsx` `.mjs` `.cjs`     | function, class and its methods and fields, const                     |
| Python                    | `.py` `.pyi`                   | def, class and its methods, module-level assignments                  |

Symbol extraction is @src/code.rs#symbols; adding a language means one more walker there.

## Frontmatter

Graphdown neither needs nor reads frontmatter. It is skipped when parsing and left untouched by @docs/cli#fmt, so tools that rely on it keep working.
