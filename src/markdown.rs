//! Parsing a markdown file into headings (symbols) and references (edges).

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::lines::LineIndex;
use crate::slug::Slugger;

/// One markdown file, parsed.
#[derive(Debug)]
pub struct Doc {
    /// Path relative to the project root, with `/` separators, including `.md`.
    pub path: String,
    pub source: String,
    pub lines: LineIndex,
    /// Text of the first level-1 heading.
    pub title: Option<String>,
    /// Plain text of the first paragraph.
    pub summary: Option<String>,
    pub headings: Vec<Heading>,
    pub refs: Vec<Ref>,
}

/// A heading and the section it owns.
#[derive(Debug, Clone)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub slug: String,
    /// 1-based line of the heading itself.
    pub line: usize,
    /// 1-based last line of the section, before the next heading of the same
    /// or a higher level (trailing blank lines excluded).
    pub end_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefForm {
    /// `@path#fragment` written in prose.
    At,
    /// A markdown link `[text](path#fragment)`.
    Link,
    /// A markdown image `![alt](path)`.
    Image,
}

/// Where a reference points, before it is checked against the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    /// Normalised, root-relative path. Doc paths may lack the `.md` extension.
    pub path: String,
    pub fragment: Option<String>,
}

/// A reference found in a document.
#[derive(Debug, Clone)]
pub struct Ref {
    pub form: RefForm,
    /// Byte range of the whole construct (`@...` or `[text](dest)`).
    pub span: Range<usize>,
    pub line: usize,
    pub col: usize,
    /// The reference exactly as written.
    pub raw: String,
    /// `Err` when the path itself is malformed (for example escapes the root).
    pub target: Result<TargetSpec, String>,
}

impl Doc {
    /// Path without the `.md` extension, the form used in `@` references.
    pub fn id(&self) -> &str {
        self.path.strip_suffix(".md").unwrap_or(&self.path)
    }

    pub fn heading(&self, slug: &str) -> Option<&Heading> {
        self.headings.iter().find(|h| h.slug == slug)
    }

    /// The innermost heading whose section contains `line`.
    pub fn section_at(&self, line: usize) -> Option<&Heading> {
        self.headings
            .iter()
            .filter(|h| h.line <= line && line <= h.end_line)
            .max_by_key(|h| h.line)
    }

    pub fn last_line(&self) -> usize {
        self.lines.line_count()
    }
}

pub fn parse(path: &str, source: String) -> Doc {
    let lines = LineIndex::new(&source);
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;

    let mut headings: Vec<Heading> = Vec::new();
    let mut slugger = Slugger::default();
    let mut text_ranges: Vec<Range<usize>> = Vec::new();
    let mut links: Vec<(RefForm, String, Range<usize>)> = Vec::new();

    let mut heading: Option<(u8, Option<String>, usize, String)> = None;
    let mut paragraph: Option<String> = None;
    let mut summary: Option<String> = None;
    let mut in_code_block = false;
    let mut in_metadata = false;
    let mut link_depth = 0usize;

    for (event, range) in Parser::new_ext(&source, opts).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, id, .. }) => {
                heading = Some((
                    level as u8,
                    id.map(|s| s.to_string()),
                    range.start,
                    String::new(),
                ));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, id, start, text)) = heading.take() {
                    let text = text.trim().to_string();
                    let slug = match id {
                        Some(id) => id,
                        None => slugger.slug(&text),
                    };
                    let line = lines.line(start);
                    headings.push(Heading {
                        level,
                        text,
                        slug,
                        line,
                        end_line: line,
                    });
                }
            }
            Event::Start(Tag::Paragraph) => paragraph = Some(String::new()),
            Event::End(TagEnd::Paragraph) => {
                if let Some(p) = paragraph.take() {
                    let p = p.split_whitespace().collect::<Vec<_>>().join(" ");
                    if summary.is_none() && !p.is_empty() {
                        summary = Some(p);
                    }
                }
            }
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Start(Tag::MetadataBlock(_)) => in_metadata = true,
            Event::End(TagEnd::MetadataBlock(_)) => in_metadata = false,
            Event::Start(Tag::Link { dest_url, .. }) => {
                if link_depth == 0 {
                    links.push((RefForm::Link, dest_url.to_string(), range.clone()));
                }
                link_depth += 1;
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                if link_depth == 0 {
                    links.push((RefForm::Image, dest_url.to_string(), range.clone()));
                }
                link_depth += 1;
            }
            Event::End(TagEnd::Link) | Event::End(TagEnd::Image) => {
                link_depth = link_depth.saturating_sub(1)
            }
            Event::Text(t) => {
                if let Some(h) = heading.as_mut() {
                    h.3.push_str(&t);
                }
                if let Some(p) = paragraph.as_mut() {
                    p.push_str(&t);
                }
                if !in_code_block && !in_metadata && link_depth == 0 {
                    match text_ranges.last_mut() {
                        Some(last) if last.end == range.start => last.end = range.end,
                        _ => text_ranges.push(range),
                    }
                }
            }
            Event::Code(t) => {
                if let Some(h) = heading.as_mut() {
                    h.3.push_str(&t);
                }
                if let Some(p) = paragraph.as_mut() {
                    p.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(p) = paragraph.as_mut() {
                    p.push(' ');
                }
            }
            _ => {}
        }
    }

    compute_section_ends(&mut headings, &source, &lines);

    let mut refs = Vec::new();
    for range in &text_ranges {
        scan_at_refs(&source, range.clone(), &lines, false, &mut refs);
    }
    for (form, dest, span) in links {
        if let Some(target) = link_target(path, &dest) {
            let (line, col) = lines.line_col(span.start);
            refs.push(Ref {
                form,
                raw: source[span.clone()].to_string(),
                span,
                line,
                col,
                target,
            });
        }
    }
    refs.sort_by_key(|r| r.span.start);

    let title = headings
        .iter()
        .find(|h| h.level == 1)
        .map(|h| h.text.clone());
    if let Some(s) = summary.as_mut() {
        if s.chars().count() > 200 {
            *s = s.chars().take(197).collect::<String>() + "...";
        }
    }

    Doc {
        path: path.to_string(),
        source,
        lines,
        title,
        summary,
        headings,
        refs,
    }
}

/// Spec: @docs/syntax#sections
fn compute_section_ends(headings: &mut [Heading], source: &str, lines: &LineIndex) {
    let last = lines.line_count();
    for i in 0..headings.len() {
        let level = headings[i].level;
        let next = headings[i + 1..]
            .iter()
            .find(|h| h.level <= level)
            .map(|h| h.line);
        let mut end = next.map(|n| n - 1).unwrap_or(last).max(headings[i].line);
        while end > headings[i].line && lines.slice(source, end, end).trim().is_empty() {
            end -= 1;
        }
        headings[i].end_line = end;
    }
}

fn is_path_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'/' | b'-')
}

fn is_fragment_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b':' | b'.')
}

/// Finds `@path#fragment` references inside a run of prose text, or inside
/// a code comment. With `qualified_only`, a reference must contain `/` or `#`,
/// which keeps tags like `@param` or `@property` in code comments out.
///
/// Spec: @docs/syntax#at-references
pub fn scan_at_refs(
    source: &str,
    range: Range<usize>,
    lines: &LineIndex,
    qualified_only: bool,
    out: &mut Vec<Ref>,
) {
    let bytes = source.as_bytes();
    let mut i = range.start;
    while i < range.end {
        if bytes[i] != b'@' {
            i += 1;
            continue;
        }
        // `user@host`, `\@`, `@@` and the like are not references.
        let prev_ok = i == 0 || {
            let p = bytes[i - 1];
            p.is_ascii()
                && !(p.is_ascii_alphanumeric()
                    || matches!(p, b'_' | b'\\' | b'@' | b'.' | b'/' | b'-' | b'`'))
        };
        let start_ok =
            i + 1 < range.end && (bytes[i + 1].is_ascii_alphanumeric() || bytes[i + 1] == b'_');
        if !prev_ok || !start_ok {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < range.end && is_path_byte(bytes[j]) {
            j += 1;
        }
        if j < range.end && bytes[j] == b'#' {
            let mut k = j + 1;
            while k < range.end && is_fragment_byte(bytes[k]) {
                k += 1;
            }
            j = k;
        }
        // Sentence punctuation directly after a reference is not part of it.
        let body = source[i + 1..j].trim_end_matches(['.', ':', '#']);
        let end = i + 1 + body.len();
        if qualified_only && !body.contains(['/', '#']) {
            i = end.max(i + 1);
            continue;
        }
        let (path, fragment) = match body.split_once('#') {
            Some((p, f)) => (p, Some(f.to_string())),
            None => (body, None),
        };
        let path = path.trim_end_matches(['.', '/']);
        let target = normalize(path).map(|path| TargetSpec { path, fragment });
        let (line, col) = lines.line_col(i);
        out.push(Ref {
            form: RefForm::At,
            span: i..end,
            line,
            col,
            raw: source[i..end].to_string(),
            target,
        });
        i = end.max(i + 1);
    }
}

/// Resolves a link destination relative to the linking doc. Returns `None`
/// for links that graphdown does not check (URLs, mail links).
fn link_target(doc_path: &str, dest: &str) -> Option<Result<TargetSpec, String>> {
    let dest = dest.trim();
    if dest.is_empty()
        || dest.contains("://")
        || dest.starts_with("mailto:")
        || dest.starts_with("tel:")
    {
        return None;
    }
    let (path, fragment) = match dest.split_once('#') {
        Some((p, f)) => (p, Some(f.to_string()).filter(|f| !f.is_empty())),
        None => (dest, None),
    };
    let path = percent_decode(path.split('?').next().unwrap_or(""));
    let joined = if path.is_empty() {
        doc_path.to_string()
    } else if let Some(abs) = path.strip_prefix('/') {
        abs.to_string()
    } else {
        match doc_path.rsplit_once('/') {
            Some((dir, _)) => format!("{dir}/{path}"),
            None => path,
        }
    };
    Some(normalize(&joined).map(|path| TargetSpec { path, fragment }))
}

/// Collapses `.` and `..` segments; refuses paths that leave the project root.
pub fn normalize(path: &str) -> Result<String, String> {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(format!("path `{path}` leaves the project root"));
                }
            }
            s => parts.push(s),
        }
    }
    if parts.is_empty() {
        return Err("empty path".to_string());
    }
    Ok(parts.join("/"))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(b) = s
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(src: &str) -> Doc {
        parse("docs/a.md", src.to_string())
    }

    #[test]
    fn headings_and_sections() {
        let d = doc("# Title\n\nIntro.\n\n## One\n\nx\n\n### Sub\n\ny\n\n## Two\n\nz\n");
        let slugs: Vec<_> = d
            .headings
            .iter()
            .map(|h| (h.slug.as_str(), h.line, h.end_line))
            .collect();
        assert_eq!(
            slugs,
            vec![
                ("title", 1, 15),
                ("one", 5, 11),
                ("sub", 9, 11),
                ("two", 13, 15)
            ]
        );
        assert_eq!(d.title.as_deref(), Some("Title"));
        assert_eq!(d.summary.as_deref(), Some("Intro."));
    }

    #[test]
    fn at_refs_in_prose_only() {
        let d = doc("See @docs/b#token-ttl. Mail me@example.com, or `@not/this`.\n\n```\n@nor/this\n```\n\nAnd @src/lib.rs#Graph::slice: done \\@escaped\n");
        let raws: Vec<_> = d.refs.iter().map(|r| r.raw.as_str()).collect();
        assert_eq!(raws, vec!["@docs/b#token-ttl", "@src/lib.rs#Graph::slice"]);
        let t = d.refs[1].target.as_ref().unwrap();
        assert_eq!(t.path, "src/lib.rs");
        assert_eq!(t.fragment.as_deref(), Some("Graph::slice"));
    }

    #[test]
    fn links_resolve_relative_to_doc() {
        let d = doc("[x](../src/a.rs#run) [y](#local) [z](https://example.com) [w](b.md)\n");
        let targets: Vec<_> = d.refs.iter().map(|r| r.target.clone().unwrap()).collect();
        assert_eq!(
            targets[0],
            TargetSpec {
                path: "src/a.rs".into(),
                fragment: Some("run".into())
            }
        );
        assert_eq!(
            targets[1],
            TargetSpec {
                path: "docs/a.md".into(),
                fragment: Some("local".into())
            }
        );
        assert_eq!(
            targets[2],
            TargetSpec {
                path: "docs/b.md".into(),
                fragment: None
            }
        );
        assert_eq!(targets.len(), 3);
    }

    #[test]
    fn frontmatter_is_ignored() {
        let d = doc("---\ntitle: @nope\n---\n# Real\n");
        assert!(d.refs.is_empty());
        assert_eq!(d.title.as_deref(), Some("Real"));
    }
}
