# Architecture

Graphdown loads every markdown file in the project into memory, parses each one into headings and references, and answers every command from that in-memory graph. There is no cache and no generated index; parsing a few hundred docs takes milliseconds, so the answer is always current. See @docs/design#no-committed-index.

## Data flow

1. @src/project.rs#find_root picks the project root.
2. @src/project.rs#Project::load walks it (respecting ignore files) and hands each `.md` file to the parser.
3. @src/markdown.rs#parse turns a file into a @src/markdown.rs#Doc: its headings with section line ranges, and its references with their positions.
4. @src/project.rs#Project::resolve checks a reference's target against the loaded docs, or against a source file's definitions, giving a @src/project.rs#Resolved.
5. The commands in @src/ops.rs combine those: check every reference, find incoming ones, walk outgoing ones for a slice, rewrite them for fmt.
6. @src/main.rs only parses arguments and prints.

## Modules

### markdown

@src/markdown.rs reads markdown with `pulldown-cmark`, which gives each event a byte range in the source. Headings become @src/markdown.rs#Heading values; their slugs come from @src/slug.rs#Slugger and their section ends from @src/markdown.rs#compute_section_ends.

`@` references are found by @src/markdown.rs#scan_at_refs, which scans only the source ranges of prose text events. That is what keeps code spans, code blocks, link text and frontmatter out. Link destinations are resolved against the linking file by @src/markdown.rs#link_target and normalised by @src/markdown.rs#normalize, so both reference forms end up as the same @src/markdown.rs#TargetSpec.

### code

@src/code.rs parses source files with tree-sitter and lists their named definitions as @src/code.rs#Symbol values with qualified names and line ranges. Each language has its own walk over the syntax tree in @src/code.rs#Walker, and @src/code.rs#comment_ranges collects the comments in the same parse.

At load time, @src/project.rs#Project::load parses only the source files that contain an `@` at all, and runs @src/markdown.rs#scan_at_refs over their comments to build `code_refs`. Any other source file is parsed only when a reference points into it. Either way the result is cached for the run by @src/project.rs#Project::code_file. @src/ops.rs#all_refs is the single place that merges doc and code references, so every command treats them alike. @src/ops.rs#mentions is the one text search in graphdown: it scans doc lines for a target's name and reports the lines with no reference, kept apart from real references.

### changes

@src/changes.rs runs `git diff` and reads which lines changed since a revision. @src/ops.rs#changed_symbols maps them to the innermost symbols, using @src/code.rs#enclosing_name for edits inside unnamed-as-symbol code like object-literal methods, and @src/ops.rs#stale lists the references and mentions those edits didn't reach. It is the only place graphdown calls another program.

### project

@src/project.rs holds the loaded docs and resolves targets. @src/project.rs#Project::content turns any resolved target into the exact lines it covers; `show`, `slice` and the containment rules in `refs` all go through it. @src/project.rs#Project::parse_target reads a target typed on the command line.

### ops

@src/ops.rs implements the commands described in @docs/cli. Output structs derive `Serialize`, so every command's `--json` is the same data as its text output.

## Testing

Unit tests sit beside the code they test. @tests/cli.rs runs the built binary against small projects written to a temporary folder, one test per behaviour a user would notice. This repository's own docs are a test as well: CI runs `graphdown check` on them.
