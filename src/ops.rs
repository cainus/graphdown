//! The operations behind each CLI command.

use std::collections::{BTreeSet, VecDeque};

use serde::Serialize;

use crate::changes::Changes;
use crate::markdown::{Doc, Ref, RefForm};
use crate::project::{Project, Resolved};

// ---------------------------------------------------------------- check

#[derive(Debug, Serialize)]
pub struct Problem {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub reference: String,
    pub message: String,
}

pub struct CheckReport {
    pub problems: Vec<Problem>,
    pub docs: usize,
    /// Source files containing at least one comment reference.
    pub sources: usize,
    pub refs: usize,
}

/// Every reference in the project: from docs, then from code comments.
pub fn all_refs(project: &Project) -> impl Iterator<Item = (&str, &Ref)> {
    let from_docs = project
        .docs
        .values()
        .flat_map(|d| d.refs.iter().map(move |r| (d.path.as_str(), r)));
    let from_code = project
        .code_refs
        .iter()
        .flat_map(|(path, refs)| refs.iter().map(move |r| (path.as_str(), r)));
    from_docs.chain(from_code)
}

/// Resolves every reference in every doc and code comment, and reports the
/// broken ones.
///
/// Spec: @docs/cli#check
pub fn check(project: &Project) -> CheckReport {
    let mut problems = Vec::new();
    let mut refs = 0;
    for (file, r) in all_refs(project) {
        refs += 1;
        if let Err(message) = resolve_ref(project, r) {
            problems.push(Problem {
                file: file.to_string(),
                line: r.line,
                col: r.col,
                reference: r.raw.clone(),
                message,
            });
        }
    }
    CheckReport {
        problems,
        docs: project.docs.len(),
        sources: project.code_refs.len(),
        refs,
    }
}

/// A GitHub Actions workflow command that shows `p` as an error annotation
/// on its line in a pull request.
///
/// Spec: @docs/cli#check
pub fn github_annotation(p: &Problem) -> String {
    let data = |s: &str| {
        s.replace('%', "%25")
            .replace('\r', "%0D")
            .replace('\n', "%0A")
    };
    let prop = |s: &str| data(s).replace(':', "%3A").replace(',', "%2C");
    format!(
        "::error file={},line={},col={},title={}::{}",
        prop(&p.file),
        p.line,
        p.col,
        prop(&format!("broken reference {}", p.reference)),
        data(&p.message)
    )
}

/// A GitHub Actions notice for a line that names changed code; notices show
/// on the pull request without failing the check.
///
/// Spec: @docs/cli#changed-code
pub fn github_stale_notice(h: &Incoming) -> String {
    let data = |s: &str| {
        s.replace('%', "%25")
            .replace('\r', "%0D")
            .replace('\n', "%0A")
    };
    let prop = |s: &str| data(s).replace(':', "%3A").replace(',', "%2C");
    format!(
        "::notice file={},line={},title={}::{}",
        prop(&h.file),
        h.line,
        prop(&format!("names changed code {}", h.target)),
        data("This line names a definition the change edited. Check it still holds.")
    )
}

pub fn resolve_ref(project: &Project, r: &Ref) -> Result<Resolved, String> {
    let spec = r.target.as_ref().map_err(|e| e.clone())?;
    project.resolve(spec)
}

// ---------------------------------------------------------------- refs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IncomingKind {
    /// A checked `@` reference or markdown link.
    Reference,
    /// The target's name in a doc's text with no reference beside it.
    Mention,
}

#[derive(Debug, Serialize)]
pub struct Incoming {
    pub kind: IncomingKind,
    pub file: String,
    pub line: usize,
    /// The section slug (in a doc) or symbol name (in code) that the
    /// reference sits in, if any.
    pub within: Option<String>,
    pub reference: String,
    pub target: String,
}

/// Every reference that lands on `target`. A whole doc or file matches
/// references to any part of it; a section matches its subsections too.
///
/// Spec: @docs/cli#refs
pub fn incoming(project: &Project, target: &Resolved) -> Vec<Incoming> {
    let mut out = Vec::new();
    for (file, r) in all_refs(project) {
        let Ok(res) = resolve_ref(project, r) else {
            continue;
        };
        if lands_on(project, &res, target) {
            out.push(Incoming {
                kind: IncomingKind::Reference,
                file: file.to_string(),
                line: r.line,
                within: project.within(file, r.line),
                reference: r.raw.clone(),
                target: res.to_string(),
            });
        }
    }
    out
}

/// Words that name `target` in prose: the last part of a symbol's name
/// (`ssrFixStacktrace` for `ViteDevServer.ssrFixStacktrace`), plus any the
/// caller adds. Short names are skipped because they match too much.
pub fn mention_terms(target: &Resolved, extra: &[String]) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    if let Resolved::Symbol { name, .. } = target {
        let last = name.rsplit(['.', ':']).next().unwrap_or(name);
        if last.chars().count() >= 4 {
            terms.push(last.to_string());
        }
    }
    for t in extra {
        if !t.is_empty() && !terms.contains(t) {
            terms.push(t.clone());
        }
    }
    terms
}

/// Lines in docs that contain one of `terms` as a whole word but carry no
/// reference to `target`: the prose that restates something without linking
/// it, which `check` and plain references can't see.
///
/// Spec: @docs/cli#refs
pub fn mentions(project: &Project, target: &Resolved, terms: &[String]) -> Vec<Incoming> {
    let mut out = Vec::new();
    if terms.is_empty() {
        return out;
    }
    for doc in project.docs.values() {
        if target.is_doc() && doc.path == target.path() {
            continue;
        }
        let linked: BTreeSet<usize> = doc
            .refs
            .iter()
            .filter(|r| resolve_ref(project, r).is_ok_and(|res| lands_on(project, &res, target)))
            .map(|r| r.line)
            .collect();
        for (i, line) in doc.source.lines().enumerate() {
            let n = i + 1;
            if linked.contains(&n) {
                continue;
            }
            if let Some(term) = terms.iter().find(|t| contains_word(line, t)) {
                out.push(Incoming {
                    kind: IncomingKind::Mention,
                    file: doc.path.clone(),
                    line: n,
                    within: doc.section_at(n).map(|h| h.slug.clone()),
                    reference: snippet(line, term),
                    target: target.to_string(),
                });
            }
        }
    }
    out
}

/// A definition that a change touched, and the name to search docs for.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Changed {
    pub symbol: Resolved,
    /// Usually the symbol's last name part. When the edit sits inside
    /// something named that isn't a symbol (a method of an object literal
    /// inside a function), that name, since docs describe it by that name.
    pub name: String,
}

/// The innermost definitions that a change touches: for each changed range
/// in a source file, the symbols it overlaps that have no overlapping symbol
/// nested inside them. Editing one method names the method, not its class.
pub fn changed_symbols(project: &Project, changes: &Changes) -> Vec<Changed> {
    let mut out = BTreeSet::new();
    for (path, ranges) in changes {
        let Some(lang) = crate::code::Lang::for_path(path) else {
            continue;
        };
        let Ok(file) = project.code_file(path) else {
            continue;
        };
        for range in ranges {
            let (start, end) = (*range.start(), *range.end());
            let hit: Vec<_> = file
                .symbols
                .iter()
                .filter(|s| s.start_line <= end && start <= s.end_line)
                .collect();
            for s in &hit {
                let has_inner = hit.iter().any(|t| {
                    s.start_line <= t.start_line
                        && t.end_line <= s.end_line
                        && (t.start_line, t.end_line) != (s.start_line, s.end_line)
                });
                if has_inner {
                    continue;
                }
                let own = s.name.rsplit(['.', ':']).next().unwrap_or(&s.name);
                let lo = start.max(s.start_line);
                let hi = end.min(s.end_line);
                let name = crate::code::enclosing_name(lang, &file.source, lo, hi)
                    .unwrap_or_else(|| own.to_string());
                out.insert(Changed {
                    symbol: Resolved::Symbol {
                        path: path.clone(),
                        name: s.name.clone(),
                    },
                    name,
                });
            }
        }
    }
    out.into_iter().collect()
}

/// Whether `word` looks like an identifier rather than an English word:
/// it has a capital after the first letter, an underscore or a digit.
/// `ssrFixStacktrace` and `DEFAULT_TIMEOUT` pass; `main` and `refresh`
/// don't, because they would match ordinary prose.
fn looks_like_identifier(word: &str) -> bool {
    word.chars().count() >= 4
        && (word.chars().skip(1).any(char::is_uppercase)
            || word.contains('_')
            || word.chars().any(|c| c.is_ascii_digit()))
}

/// References to and mentions of changed definitions that sit on lines the
/// change didn't touch: the docs that may now describe the old behaviour.
///
/// Spec: @docs/cli#changed-code
pub fn stale(project: &Project, changes: &Changes) -> Vec<Incoming> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for changed in changed_symbols(project, changes) {
        let target = &changed.symbol;
        let terms: Vec<String> = Some(changed.name.clone())
            .filter(|n| looks_like_identifier(n))
            .into_iter()
            .collect();
        let found = incoming(project, target)
            .into_iter()
            .chain(mentions(project, target, &terms));
        for mut hit in found {
            let edited = changes
                .get(&hit.file)
                .is_some_and(|r| crate::changes::touches(r, hit.line));
            if !edited && seen.insert((hit.file.clone(), hit.line)) {
                if hit.kind == IncomingKind::Mention {
                    hit.target = format!("`{}` ({target})", changed.name);
                }
                out.push(hit);
            }
        }
    }
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out
}

fn contains_word(line: &str, word: &str) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(word).any(|(i, _)| {
        let before = line[..i].chars().next_back();
        let after = line[i + word.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

/// The line, trimmed to about 100 characters around `term`.
fn snippet(line: &str, term: &str) -> String {
    let line = line.trim();
    if line.chars().count() <= 100 {
        return line.to_string();
    }
    let at = line.find(term).unwrap_or(0);
    let start = line[..at]
        .char_indices()
        .rev()
        .nth(40)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let tail: String = line[start..].chars().take(100).collect();
    format!("{}{tail}...", if start > 0 { "..." } else { "" })
}

fn lands_on(project: &Project, hit: &Resolved, target: &Resolved) -> bool {
    if hit.path() != target.path() {
        return false;
    }
    match target {
        Resolved::Doc { .. } | Resolved::CodeFile { .. } | Resolved::File { .. } => true,
        Resolved::Section { .. } | Resolved::Symbol { .. } | Resolved::Lines { .. } => {
            if hit == target {
                return true;
            }
            // A reference to a subsection (or nested symbol) also lands on its parent.
            match (project.content(hit), project.content(target)) {
                (Ok(h), Ok(t)) => {
                    !matches!(hit, Resolved::Doc { .. } | Resolved::CodeFile { .. })
                        && t.start <= h.start
                        && h.end <= t.end
                }
                _ => false,
            }
        }
    }
}

// ---------------------------------------------------------------- slice

#[derive(Debug, Serialize)]
pub struct Piece {
    pub target: String,
    pub file: String,
    pub start: usize,
    pub end: usize,
    pub depth: usize,
    pub kind: &'static str,
    pub text: String,
}

/// The entry targets plus everything they reference, transitively, each
/// piece included once. Pieces contained in an earlier piece are skipped.
///
/// Spec: @docs/cli#slice
pub fn slice(
    project: &Project,
    entries: &[Resolved],
    max_depth: Option<usize>,
    with_code: bool,
) -> Result<Vec<Piece>, String> {
    let mut queue: VecDeque<(Resolved, usize)> = entries.iter().cloned().map(|e| (e, 0)).collect();
    let mut seen: BTreeSet<Resolved> = BTreeSet::new();
    let mut pieces: Vec<Piece> = Vec::new();

    while let Some((target, depth)) = queue.pop_front() {
        if !seen.insert(target.clone()) || matches!(target, Resolved::File { .. }) {
            continue;
        }
        if !with_code && !target.is_doc() && depth > 0 {
            continue;
        }
        let content = project.content(&target)?;
        let covered = pieces
            .iter()
            .any(|p| p.file == content.path && p.start <= content.start && content.end <= p.end);
        if covered {
            continue;
        }
        // Drop earlier pieces that this one swallows whole.
        pieces.retain(|p| {
            !(p.file == content.path && content.start <= p.start && p.end <= content.end)
        });

        // Follow references made inside this piece: prose in a doc, or
        // comments in code.
        if max_depth.is_none_or(|m| depth < m) {
            let inside = project
                .refs_from(target.path())
                .iter()
                .filter(|r| r.line >= content.start && r.line <= content.end);
            for r in inside {
                if let Ok(next) = resolve_ref(project, r) {
                    queue.push_back((next, depth + 1));
                }
            }
        }
        pieces.push(Piece {
            target: target.to_string(),
            file: content.path,
            start: content.start,
            end: content.end,
            depth,
            kind: if target.is_doc() { "doc" } else { "code" },
            text: content.text,
        });
    }
    Ok(pieces)
}

pub fn render_slice_markdown(pieces: &[Piece]) -> String {
    let mut out = String::new();
    for p in pieces {
        out.push_str(&format!(
            "<!-- {}  {} L{}-{} -->\n",
            p.target, p.file, p.start, p.end
        ));
        if p.kind == "code" {
            let lang = p.file.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
            out.push_str(&format!("```{lang}\n{}\n```\n\n", p.text));
        } else {
            out.push_str(&p.text);
            out.push_str("\n\n");
        }
    }
    out
}

pub fn render_slice_xml(pieces: &[Piece]) -> String {
    let mut out = String::new();
    for p in pieces {
        let attr = |s: &str| {
            s.replace('&', "&amp;")
                .replace('"', "&quot;")
                .replace('<', "&lt;")
        };
        out.push_str(&format!(
            "<{tag} ref=\"{}\" file=\"{}\" lines=\"{}-{}\">\n{}\n</{tag}>\n",
            attr(&p.target),
            attr(&p.file),
            p.start,
            p.end,
            p.text,
            tag = p.kind
        ));
    }
    out
}

// ---------------------------------------------------------------- outline

#[derive(Debug, Serialize)]
pub struct OutlineEntry {
    pub level: u8,
    pub name: String,
    pub target: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize)]
pub struct Outline {
    pub file: String,
    pub target: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub lines: usize,
    pub incoming: usize,
    pub entries: Vec<OutlineEntry>,
}

/// Spec: @docs/cli#outline
pub fn outline(
    project: &Project,
    target: &Resolved,
    count_incoming: bool,
) -> Result<Outline, String> {
    let incoming = if count_incoming {
        incoming(project, target).len()
    } else {
        0
    };
    let file = target.path().to_string();
    if let Some(doc) = project.doc(&file).filter(|_| target.is_doc()) {
        let entries = doc
            .headings
            .iter()
            .map(|h| OutlineEntry {
                level: h.level,
                name: h.text.clone(),
                target: format!("@{}#{}", doc.id(), h.slug),
                start: h.line,
                end: h.end_line,
            })
            .collect();
        return Ok(Outline {
            target: format!("@{}", doc.id()),
            file,
            title: doc.title.clone(),
            summary: doc.summary.clone(),
            lines: doc.last_line(),
            incoming,
            entries,
        });
    }
    let code = project.code_file(&file)?;
    let entries = code
        .symbols
        .iter()
        .map(|s| OutlineEntry {
            level: (s.name.matches("::").count() + s.name.matches('.').count() + 1) as u8,
            name: format!("{} {}", s.kind, s.name),
            target: format!("@{file}#{}", s.name),
            start: s.start_line,
            end: s.end_line,
        })
        .collect();
    Ok(Outline {
        target: format!("@{file}"),
        file,
        title: None,
        summary: None,
        lines: code.lines.line_count(),
        incoming,
        entries,
    })
}

/// One line for a doc: its reference, title, summary and size, so a whole
/// repo's docs fit in a few thousand tokens.
///
/// Spec: @docs/cli#outline
pub fn render_outline_line(o: &Outline) -> String {
    let mut line = o.target.clone();
    let mut about = String::new();
    if let Some(t) = &o.title {
        about.push_str(t);
    }
    if let Some(s) = &o.summary {
        if !about.is_empty() {
            about.push_str(": ");
        }
        about.push_str(s);
    }
    if about.chars().count() > 110 {
        about = about.chars().take(107).collect::<String>() + "...";
    }
    if !about.is_empty() {
        line.push_str("  ");
        line.push_str(&about);
    }
    line.push_str(&format!("  ({} lines", o.lines));
    if o.incoming > 0 {
        line.push_str(&format!(", {} in", o.incoming));
    }
    line.push(')');
    line
}

pub fn render_outline(o: &Outline) -> String {
    let mut out = format!("{}  ({} lines", o.target, o.lines);
    if o.incoming > 0 {
        out.push_str(&format!(", {} incoming", o.incoming));
    }
    out.push(')');
    if let Some(s) = &o.summary {
        out.push_str(&format!("\n  {s}"));
    }
    out.push('\n');
    let width = o
        .entries
        .iter()
        .map(|e| e.name.len() + 2 * e.level as usize)
        .max()
        .unwrap_or(0);
    for e in &o.entries {
        let label = format!("{}{}", "  ".repeat(e.level as usize), e.name);
        let fragment = e.target.rsplit_once('#').map(|(_, f)| f).unwrap_or("");
        let lines = format!("L{}-{}", e.start, e.end);
        out.push_str(&format!("{label:<width$}  {lines:<10} #{fragment}\n"));
    }
    out
}

// ---------------------------------------------------------------- fmt

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Style {
    /// Rewrite `@refs` as markdown links that GitHub and editors can click.
    Links,
    /// Rewrite plain markdown links to docs and code as `@refs`.
    At,
}

/// The rewritten source of `doc`, if anything changed.
///
/// Only references that resolve are rewritten, and a link is turned back
/// into an `@ref` only when its text is exactly what `--to links` would have
/// produced, so the two directions round-trip without losing words.
///
/// `CLAUDE.md` files are left alone: Claude Code reads `@path` there as an
/// import, which must stay in `@` form.
///
/// Spec: @docs/cli#fmt
pub fn format_doc(project: &Project, doc: &Doc, style: Style) -> Option<String> {
    let name = doc.path.rsplit('/').next().unwrap_or(&doc.path);
    if name == "CLAUDE.md" || name == "CLAUDE.local.md" {
        return None;
    }
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for r in &doc.refs {
        let Ok(target) = resolve_ref(project, r) else {
            continue;
        };
        if matches!(target, Resolved::File { .. }) {
            continue;
        }
        let link = as_link(project, doc, &target);
        match (style, r.form) {
            (Style::Links, RefForm::At) => edits.push((r.span.clone(), link)),
            (Style::At, RefForm::Link) if r.raw == link => {
                edits.push((r.span.clone(), target.to_string()))
            }
            _ => {}
        }
    }
    if edits.is_empty() {
        return None;
    }
    let mut out = doc.source.clone();
    for (span, text) in edits.into_iter().rev() {
        out.replace_range(span, &text);
    }
    Some(out)
}

fn as_link(project: &Project, doc: &Doc, target: &Resolved) -> String {
    // A section of the same doc links as `#slug`; anything else by relative path.
    let same_doc_section = target.path() == doc.path && matches!(target, Resolved::Section { .. });
    let dest = if same_doc_section {
        String::new()
    } else {
        relative(&doc.path, target.path())
    };
    let fragment = match target {
        Resolved::Section { slug, .. } => format!("#{slug}"),
        Resolved::Symbol { name, .. } => format!("#{name}"),
        Resolved::Lines { start, end, .. } => format!("#L{start}-L{end}"),
        _ => String::new(),
    };
    let dest = if dest.contains(' ') {
        dest.replace(' ', "%20")
    } else {
        dest
    };
    format!("[{}]({dest}{fragment})", project.link_text(target))
}

/// Path of `to` relative to the directory containing `from`.
fn relative(from: &str, to: &str) -> String {
    let from_dir: Vec<&str> = from
        .split('/')
        .collect::<Vec<_>>()
        .split_last()
        .map(|(_, d)| d.to_vec())
        .unwrap_or_default();
    let to_parts: Vec<&str> = to.split('/').collect();
    let common = from_dir
        .iter()
        .zip(&to_parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<&str> = vec![".."; from_dir.len() - common];
    parts.extend(&to_parts[common..]);
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::relative;

    #[test]
    fn relative_paths() {
        assert_eq!(relative("docs/a.md", "docs/b.md"), "b.md");
        assert_eq!(relative("docs/a.md", "src/lib.rs"), "../src/lib.rs");
        assert_eq!(relative("README.md", "docs/x/y.md"), "docs/x/y.md");
        assert_eq!(relative("docs/x/a.md", "docs/y.md"), "../y.md");
    }
}
