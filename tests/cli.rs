//! End-to-end tests: build a small project in a temp folder, run the binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Project {
    dir: PathBuf,
}

impl Project {
    fn new(files: &[(&str, &str)]) -> Project {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "graphdown-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("graphdown.toml"), "").unwrap();
        for (path, body) in files {
            let full = dir.join(path);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(full, body).unwrap();
        }
        Project { dir }
    }

    fn run(&self, args: &[&str]) -> (i32, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_graphdown"))
            .args(args)
            .current_dir(&self.dir)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        (out.status.code().unwrap_or(-1), text)
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap()
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

const AUTH: &str = "# Auth session

Short-lived tokens with silent renewal.

## Token refresh

On a 401, @src/session.rs#Session::refresh swaps tokens. Network errors retry
with @docs/sync#backoff-policy.

### Token TTL

Tokens live 15 minutes.

## Session expiry

See [the TTL](#token-ttl).
";

const SYNC: &str = "# Sync engine

## Backoff policy

Exponential from 500ms, capped at 30s. Tokens come from @docs/auth#token-refresh.

## Conflicts

Server wins.
";

const SESSION_RS: &str = "pub struct Session;

impl Session {
    /// Swaps the refresh token for a new pair.
    pub fn refresh(&mut self) {}
}
";

fn sample() -> Project {
    Project::new(&[
        ("docs/auth.md", AUTH),
        ("docs/sync.md", SYNC),
        ("src/session.rs", SESSION_RS),
    ])
}

#[test]
fn check_passes_on_valid_project() {
    let (code, out) = sample().run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("ok: 4 refs across 2 docs"), "{out}");
}

#[test]
fn check_reports_broken_refs_with_suggestions() {
    let p = Project::new(&[
        (
            "docs/a.md",
            "# A\n\nSee @docs/b#backof and @docs/missing and @src/x.rs#Nope.\n",
        ),
        ("docs/b.md", "# B\n\n## Backoff\n"),
        ("src/x.rs", "fn yes() {}\n"),
    ]);
    let (code, out) = p.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("docs/a.md:3:5: @docs/b#backof: no heading `#backof` in docs/b.md (did you mean `#backoff`?)"), "{out}");
    assert!(
        out.contains("@docs/missing: no such doc or file `docs/missing`"),
        "{out}"
    );
    assert!(out.contains("no symbol `Nope` in src/x.rs"), "{out}");
    assert!(out.contains("3 broken"), "{out}");
}

#[test]
fn show_prints_one_section_with_subsections() {
    let (code, out) = sample().run(&["show", "docs/auth#token-refresh"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.starts_with("<!-- @docs/auth#token-refresh  docs/auth.md L5-12 -->\n## Token refresh"),
        "{out}"
    );
    assert!(out.contains("Tokens live 15 minutes."), "{out}");
    assert!(!out.contains("Session expiry"), "{out}");
}

#[test]
fn show_prints_a_code_symbol_with_its_doc_comment() {
    let (code, out) = sample().run(&["show", "@src/session.rs#Session.refresh"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("L4-5"), "{out}");
    assert!(out.contains("/// Swaps the refresh token"), "{out}");
}

#[test]
fn refs_finds_incoming_including_subsections() {
    let (_, out) = sample().run(&["refs", "docs/auth#token-refresh"]);
    assert!(
        out.contains("docs/sync.md:5 (#backoff-policy)  @docs/auth#token-refresh"),
        "{out}"
    );
    // `[the TTL](#token-ttl)` points at a subsection of token-refresh.
    assert!(
        out.contains("docs/auth.md:16 (#session-expiry)  [the TTL](#token-ttl)"),
        "{out}"
    );

    let (_, out) = sample().run(&["refs", "src/session.rs"]);
    assert!(out.contains("docs/auth.md:7"), "{out}");
}

#[test]
fn slice_follows_references_once_each() {
    let (code, out) = sample().run(&["slice", "docs/auth#token-refresh", "--format", "xml"]);
    assert_eq!(code, 0, "{out}");
    let headers: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("<doc") || l.starts_with("<code"))
        .collect();
    assert_eq!(
        headers,
        vec![
            r#"<doc ref="@docs/auth#token-refresh" file="docs/auth.md" lines="5-12">"#,
            r#"<code ref="@src/session.rs#Session::refresh" file="src/session.rs" lines="4-5">"#,
            r#"<doc ref="@docs/sync#backoff-policy" file="docs/sync.md" lines="3-5">"#,
        ],
        "{out}"
    );
}

#[test]
fn slice_respects_depth_and_no_code() {
    let (_, out) = sample().run(&["slice", "docs/auth#token-refresh", "--depth", "0"]);
    assert_eq!(out.matches("<!-- ").count(), 1, "{out}");
    let (_, out) = sample().run(&["slice", "docs/auth#token-refresh", "--no-code"]);
    assert!(!out.contains("<!-- @src/session.rs"), "{out}");
    assert!(
        out.contains("@docs/sync#backoff-policy  docs/sync.md"),
        "{out}"
    );
}

#[test]
fn outline_lists_headings_and_symbols() {
    let (_, out) = sample().run(&["outline", "docs/auth"]);
    assert!(
        out.starts_with(
            "@docs/auth  (16 lines, 2 incoming)\n  Short-lived tokens with silent renewal.\n"
        ),
        "{out}"
    );
    assert!(out.contains("#token-ttl"), "{out}");
    let (_, out) = sample().run(&["outline", "src/session.rs"]);
    assert!(out.contains("fn Session::refresh"), "{out}");
}

#[test]
fn fmt_round_trips_between_forms() {
    let p = sample();
    let (code, out) = p.run(&["fmt", "--to", "links"]);
    assert_eq!(code, 0, "{out}");
    let auth = p.read("docs/auth.md");
    assert!(
        auth.contains("[`Session::refresh`](../src/session.rs#Session::refresh)"),
        "{auth}"
    );
    assert!(
        auth.contains("[Backoff policy](sync.md#backoff-policy)"),
        "{auth}"
    );
    // Hand-written link text is kept.
    assert!(auth.contains("[the TTL](#token-ttl)"), "{auth}");
    assert_eq!(p.run(&["check"]).0, 0);

    p.run(&["fmt", "--to", "at"]);
    assert_eq!(p.read("docs/auth.md"), AUTH);
    assert_eq!(p.read("docs/sync.md"), SYNC);
}

#[test]
fn fmt_check_does_not_write() {
    let p = sample();
    let (code, out) = p.run(&["fmt", "--to", "links", "--check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("would rewrite docs/auth.md"), "{out}");
    assert_eq!(p.read("docs/auth.md"), AUTH);
}

#[test]
fn graphdownignore_excludes_docs() {
    let p = Project::new(&[
        (".graphdownignore", "fixtures/\n"),
        ("fixtures/bad.md", "@nowhere\n"),
        ("ok.md", "# Ok\n"),
    ]);
    let (code, out) = p.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(p.path().join("fixtures/bad.md").exists());
}

#[test]
fn line_anchors_and_plain_files() {
    let p = Project::new(&[
        (
            "a.md",
            "# A\n\n[l](src/x.rs#L1-L2) @src/x.rs#L9 ![img](logo.png)\n",
        ),
        ("src/x.rs", "fn a() {}\nfn b() {}\n"),
        ("logo.png", "png"),
    ]);
    let (code, out) = p.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("@src/x.rs#L9: `#L9` is outside src/x.rs (2 lines)"),
        "{out}"
    );
    assert!(out.contains("1 broken"), "{out}");
}

#[test]
fn fmt_leaves_claude_md_imports_alone() {
    let p = Project::new(&[
        ("CLAUDE.md", "@AGENTS.md\n"),
        ("AGENTS.md", "# Agents\n\nSee @CLAUDE.md\n"),
    ]);
    p.run(&["fmt", "--to", "links"]);
    assert_eq!(p.read("CLAUDE.md"), "@AGENTS.md\n");
    assert_eq!(p.read("AGENTS.md"), "# Agents\n\nSee [CLAUDE](CLAUDE.md)\n");
}
