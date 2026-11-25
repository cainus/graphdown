//! The set of docs under a project root, and resolution of references
//! against it.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};

use crate::code::{self, Lang, Symbol};
use crate::lines::LineIndex;
use crate::markdown::{self, Doc, Ref, TargetSpec};

/// What a reference points at, once checked.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Resolved {
    /// A whole markdown document.
    Doc { path: String },
    /// One heading's section of a document.
    Section { path: String, slug: String },
    /// A whole source file in a supported language.
    CodeFile { path: String },
    /// A named definition in a source file.
    Symbol { path: String, name: String },
    /// A line range, from a GitHub-style `#L10-L20` fragment.
    Lines {
        path: String,
        start: usize,
        end: usize,
    },
    /// Any other file (an image, a config file).
    File { path: String },
}

impl Resolved {
    pub fn path(&self) -> &str {
        match self {
            Resolved::Doc { path }
            | Resolved::Section { path, .. }
            | Resolved::CodeFile { path }
            | Resolved::Symbol { path, .. }
            | Resolved::Lines { path, .. }
            | Resolved::File { path } => path,
        }
    }

    pub fn is_doc(&self) -> bool {
        matches!(self, Resolved::Doc { .. } | Resolved::Section { .. })
    }
}

/// Prints the canonical `@` form of the target.
impl fmt::Display for Resolved {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let id = |p: &str| p.strip_suffix(".md").unwrap_or(p).to_string();
        match self {
            Resolved::Doc { path } => write!(f, "@{}", id(path)),
            Resolved::Section { path, slug } => write!(f, "@{}#{slug}", id(path)),
            Resolved::CodeFile { path } | Resolved::File { path } => write!(f, "@{path}"),
            Resolved::Symbol { path, name } => write!(f, "@{path}#{name}"),
            Resolved::Lines { path, start, end } => write!(f, "@{path}#L{start}-L{end}"),
        }
    }
}

/// The text a target covers.
pub struct Content {
    pub path: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

pub struct CodeFile {
    pub source: String,
    pub lines: LineIndex,
    pub symbols: Vec<Symbol>,
    pub comments: Vec<std::ops::Range<usize>>,
}

pub struct Project {
    pub root: PathBuf,
    pub docs: BTreeMap<String, Doc>,
    /// References written in code comments, by source file. Files without
    /// any are absent.
    pub code_refs: BTreeMap<String, Vec<Ref>>,
    code: RefCell<HashMap<String, Result<Rc<CodeFile>, String>>>,
}

/// The nearest ancestor of `start` containing `graphdown.toml` or `.git`.
pub fn find_root(start: &Path) -> PathBuf {
    for dir in start.ancestors() {
        if dir.join("graphdown.toml").exists() || dir.join(".git").exists() {
            return dir.to_path_buf();
        }
    }
    start.to_path_buf()
}

impl Project {
    /// Loads every `.md` file under `root`, and the comment references of
    /// every source file in a supported language, honouring `.gitignore` and
    /// `.graphdownignore`.
    pub fn load(root: &Path) -> Result<Project> {
        let root = root
            .canonicalize()
            .with_context(|| format!("no such directory {}", root.display()))?;
        let mut docs = BTreeMap::new();
        let mut sources: Vec<(String, String)> = Vec::new();
        let walker = ignore::WalkBuilder::new(&root)
            .add_custom_ignore_filename(".graphdownignore")
            .parents(false)
            .require_git(false)
            .build();
        for entry in walker {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let rel = rel_path(&root, path);
            if path.extension().is_some_and(|e| e == "md") {
                let source =
                    std::fs::read_to_string(path).with_context(|| format!("reading {rel}"))?;
                docs.insert(rel.clone(), markdown::parse(&rel, source));
            } else if Lang::for_path(&rel).is_some() {
                // Unreadable or non-UTF-8 source is skipped, not fatal.
                if let Ok(source) = std::fs::read_to_string(path) {
                    if source.contains('@') {
                        sources.push((rel, source));
                    }
                }
            }
        }
        let mut project = Project {
            root,
            docs,
            code_refs: BTreeMap::new(),
            code: RefCell::new(HashMap::new()),
        };
        for (rel, source) in sources {
            let Ok(file) = project.parse_code(&rel, source) else {
                continue;
            };
            let mut refs = Vec::new();
            for range in &file.comments {
                markdown::scan_at_refs(&file.source, range.clone(), &file.lines, true, &mut refs);
            }
            project.code.borrow_mut().insert(rel.clone(), Ok(file));
            if !refs.is_empty() {
                project.code_refs.insert(rel, refs);
            }
        }
        Ok(project)
    }

    /// References made from inside `path`, whether it is a doc or source.
    pub fn refs_from(&self, path: &str) -> &[Ref] {
        match self.doc(path) {
            Some(doc) => &doc.refs,
            None => self.code_refs.get(path).map(Vec::as_slice).unwrap_or(&[]),
        }
    }

    /// The innermost section slug (doc) or symbol name (code) containing
    /// `line` of `path`.
    pub fn within(&self, path: &str, line: usize) -> Option<String> {
        if let Some(doc) = self.doc(path) {
            return doc.section_at(line).map(|h| h.slug.clone());
        }
        let file = self.code_file(path).ok()?;
        file.symbols
            .iter()
            .filter(|s| s.start_line <= line && line <= s.end_line)
            .max_by_key(|s| s.start_line)
            .map(|s| s.name.clone())
    }

    fn parse_code(&self, path: &str, source: String) -> Result<Rc<CodeFile>, String> {
        let lang = Lang::for_path(path).ok_or_else(|| {
            format!(
                "can't look up symbols in `{path}` (supported: {})",
                code::SUPPORTED
            )
        })?;
        let parsed = code::parse(lang, &source)?;
        Ok(Rc::new(CodeFile {
            lines: LineIndex::new(&source),
            source,
            symbols: parsed.symbols,
            comments: parsed.comments,
        }))
    }

    /// Finds the doc for a path written with or without `.md`.
    pub fn doc(&self, path: &str) -> Option<&Doc> {
        self.docs
            .get(path)
            .or_else(|| self.docs.get(&format!("{path}.md")))
    }

    pub fn code_file(&self, path: &str) -> Result<Rc<CodeFile>, String> {
        if let Some(cached) = self.code.borrow().get(path) {
            return cached.clone();
        }
        let result = std::fs::read_to_string(self.root.join(path))
            .map_err(|e| format!("reading {path}: {e}"))
            .and_then(|source| self.parse_code(path, source));
        self.code
            .borrow_mut()
            .insert(path.to_string(), result.clone());
        result
    }

    /// Checks a target against the project.
    pub fn resolve(&self, spec: &TargetSpec) -> Result<Resolved, String> {
        if let Some(doc) = self.doc(&spec.path) {
            let path = doc.path.clone();
            return match &spec.fragment {
                None => Ok(Resolved::Doc { path }),
                Some(frag) => match doc.heading(frag) {
                    Some(h) => Ok(Resolved::Section {
                        path,
                        slug: h.slug.clone(),
                    }),
                    None => {
                        let slugs: Vec<&str> =
                            doc.headings.iter().map(|h| h.slug.as_str()).collect();
                        Err(format!(
                            "no heading `#{frag}` in {}{}",
                            doc.path,
                            suggest(frag, &slugs, "#")
                        ))
                    }
                },
            };
        }
        let path = spec.path.clone();
        let full = self.root.join(&path);
        if !full.is_file() {
            return Err(if full.is_dir() {
                format!("`{path}` is a directory")
            } else {
                format!("no such doc or file `{path}`")
            });
        }
        let Some(frag) = &spec.fragment else {
            return Ok(if Lang::for_path(&path).is_some() {
                Resolved::CodeFile { path }
            } else {
                Resolved::File { path }
            });
        };
        if let Some((start, end)) = line_anchor(frag) {
            let count = match self.code_file(&path) {
                Ok(c) => c.lines.line_count(),
                Err(_) => std::fs::read_to_string(&full)
                    .map(|s| LineIndex::new(&s).line_count())
                    .unwrap_or(0),
            };
            if start == 0 || end < start || end > count {
                return Err(format!("`#{frag}` is outside {path} ({count} lines)"));
            }
            return Ok(Resolved::Lines { path, start, end });
        }
        let file = self.code_file(&path)?;
        match code::find(&file.symbols, frag) {
            Some(sym) => Ok(Resolved::Symbol {
                path,
                name: sym.name.clone(),
            }),
            None => {
                let names: Vec<&str> = file.symbols.iter().map(|s| s.name.as_str()).collect();
                Err(format!(
                    "no symbol `{frag}` in {path}{}",
                    suggest(frag, &names, "#")
                ))
            }
        }
    }

    /// Parses a target typed on the command line: `docs/a#x`, `@docs/a#x`
    /// or `docs/a.md#x`, relative to the root.
    pub fn parse_target(&self, input: &str) -> Result<Resolved, String> {
        let input = input.trim().trim_start_matches('@');
        let (path, fragment) = match input.split_once('#') {
            Some((p, f)) => (p, Some(f.to_string()).filter(|f| !f.is_empty())),
            None => (input, None),
        };
        let path = markdown::normalize(path.trim_start_matches("./"))?;
        self.resolve(&TargetSpec { path, fragment })
    }

    /// The lines a target covers.
    pub fn content(&self, target: &Resolved) -> Result<Content, String> {
        let path = target.path().to_string();
        let (source, lines, start, end): (String, LineIndex, usize, usize) = match target {
            Resolved::Doc { .. } | Resolved::Section { .. } => {
                let doc = self
                    .doc(&path)
                    .ok_or_else(|| format!("unknown doc {path}"))?;
                let (s, e) = match target {
                    Resolved::Section { slug, .. } => {
                        let h = doc
                            .heading(slug)
                            .ok_or_else(|| format!("unknown heading #{slug}"))?;
                        (h.line, h.end_line)
                    }
                    _ => (1, doc.last_line()),
                };
                (doc.source.clone(), doc.lines.clone(), s, e)
            }
            _ => {
                let source = std::fs::read_to_string(self.root.join(&path))
                    .map_err(|e| format!("reading {path}: {e}"))?;
                let lines = LineIndex::new(&source);
                let (s, e) = match target {
                    Resolved::Symbol { name, .. } => {
                        let file = self.code_file(&path)?;
                        let sym = code::find(&file.symbols, name)
                            .ok_or_else(|| format!("unknown symbol {name}"))?;
                        (sym.start_line, sym.end_line)
                    }
                    Resolved::Lines { start, end, .. } => (*start, *end),
                    _ => (1, lines.line_count()),
                };
                (source, lines, s, e)
            }
        };
        let text = if lines.line_count() == 0 {
            String::new()
        } else {
            lines.slice(&source, start, end).to_string()
        };
        Ok(Content {
            path,
            start,
            end,
            text,
        })
    }

    /// Text to use when turning a reference into a markdown link.
    pub fn link_text(&self, target: &Resolved) -> String {
        match target {
            Resolved::Doc { path } => {
                self.doc(path)
                    .and_then(|d| d.title.clone())
                    .unwrap_or_else(|| {
                        path.rsplit('/')
                            .next()
                            .unwrap_or(path)
                            .trim_end_matches(".md")
                            .to_string()
                    })
            }
            Resolved::Section { path, slug } => self
                .doc(path)
                .and_then(|d| d.heading(slug))
                .map(|h| h.text.clone())
                .unwrap_or_else(|| slug.clone()),
            Resolved::Symbol { name, .. } => format!("`{name}`"),
            Resolved::CodeFile { path }
            | Resolved::File { path }
            | Resolved::Lines { path, .. } => format!("`{path}`"),
        }
    }
}

fn rel_path(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Parses GitHub line anchors: `L12` or `L12-L20`.
fn line_anchor(frag: &str) -> Option<(usize, usize)> {
    let num = |s: &str| s.strip_prefix('L').and_then(|n| n.parse::<usize>().ok());
    match frag.split_once('-') {
        Some((a, b)) => Some((num(a)?, num(b).or_else(|| b.parse().ok())?)),
        None => num(frag).map(|n| (n, n)),
    }
}

/// " (did you mean #x?)" for the closest candidate, if any is close.
fn suggest(want: &str, candidates: &[&str], prefix: &str) -> String {
    let want_l = want.to_lowercase();
    let best = candidates
        .iter()
        .map(|c| {
            let cl = c.to_lowercase();
            let score = if cl.ends_with(&want_l) || cl.contains(&want_l) {
                0
            } else {
                distance(&want_l, &cl)
            };
            (score, *c)
        })
        .min_by_key(|(d, _)| *d);
    match best {
        Some((d, c)) if d <= 3.max(want.len() / 3) => format!(" (did you mean `{prefix}{c}`?)"),
        _ => String::new(),
    }
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == *cb { 0 } else { 1 };
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}
