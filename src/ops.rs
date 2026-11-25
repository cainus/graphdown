//! The operations behind each CLI command.

use std::collections::{BTreeSet, VecDeque};

use serde::Serialize;

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
    pub refs: usize,
}

/// Resolves every reference in every doc and reports the broken ones.
pub fn check(project: &Project) -> CheckReport {
    let mut problems = Vec::new();
    let mut refs = 0;
    for doc in project.docs.values() {
        for r in &doc.refs {
            refs += 1;
            if let Err(message) = resolve_ref(project, r) {
                problems.push(Problem {
                    file: doc.path.clone(),
                    line: r.line,
                    col: r.col,
                    reference: r.raw.clone(),
                    message,
                });
            }
        }
    }
    CheckReport {
        problems,
        docs: project.docs.len(),
        refs,
    }
}

pub fn resolve_ref(project: &Project, r: &Ref) -> Result<Resolved, String> {
    let spec = r.target.as_ref().map_err(|e| e.clone())?;
    project.resolve(spec)
}

// ---------------------------------------------------------------- refs

#[derive(Debug, Serialize)]
pub struct Incoming {
    pub file: String,
    pub line: usize,
    /// Slug of the section the reference sits in, if any.
    pub section: Option<String>,
    pub reference: String,
    pub target: String,
}

/// Every reference that lands on `target`. A whole doc or file matches
/// references to any part of it; a section matches its subsections too.
pub fn incoming(project: &Project, target: &Resolved) -> Vec<Incoming> {
    let mut out = Vec::new();
    for doc in project.docs.values() {
        for r in &doc.refs {
            let Ok(res) = resolve_ref(project, r) else {
                continue;
            };
            if lands_on(project, &res, target) {
                out.push(Incoming {
                    file: doc.path.clone(),
                    line: r.line,
                    section: doc.section_at(r.line).map(|h| h.slug.clone()),
                    reference: r.raw.clone(),
                    target: res.to_string(),
                });
            }
        }
    }
    out
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

        if target.is_doc() && max_depth.is_none_or(|m| depth < m) {
            if let Some(doc) = project.doc(target.path()) {
                for r in refs_in(doc, content.start, content.end) {
                    if let Ok(next) = resolve_ref(project, r) {
                        queue.push_back((next, depth + 1));
                    }
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

fn refs_in(doc: &Doc, start: usize, end: usize) -> impl Iterator<Item = &Ref> {
    doc.refs
        .iter()
        .filter(move |r| r.line >= start && r.line <= end)
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
