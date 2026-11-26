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

const SPEC: &str = "# Auth

## Token refresh

Swap tokens on a 401.
";

const REFRESH_RS: &str = "pub struct Session;

impl Session {
    /// Spec: @docs/auth#token-refresh
    pub fn refresh(&mut self) {
        let _not_a_ref = \"@docs/nowhere#x\";
    }
}
";

const CLIENT_TS: &str = "/**
 * Retries a request.
 * @param attempts how many times, see @docs/auth#token-refresh
 * @returns the response
 */
export function retry(attempts: number) {}
";

const WORKER_PY: &str = "def run():
    # see @docs/auth#missing-section
    pass
";

fn backlinked() -> Project {
    Project::new(&[
        ("docs/auth.md", SPEC),
        ("src/session.rs", REFRESH_RS),
        ("web/client.ts", CLIENT_TS),
        ("worker.py", WORKER_PY),
    ])
}

#[test]
fn check_covers_references_in_code_comments() {
    let (code, out) = backlinked().run(&["check"]);
    assert_eq!(code, 1, "{out}");
    // Only the Python reference is broken; JSDoc tags and string literals are ignored.
    assert!(
        out.contains("worker.py:2:11: @docs/auth#missing-section: no heading `#missing-section`"),
        "{out}"
    );
    assert!(
        out.contains("1 broken: 3 refs across 1 docs and 3 source files"),
        "{out}"
    );
}

#[test]
fn refs_lists_code_that_points_at_a_section() {
    let (_, out) = backlinked().run(&["refs", "docs/auth#token-refresh"]);
    assert!(
        out.contains("src/session.rs:4 (#Session::refresh)  @docs/auth#token-refresh"),
        "{out}"
    );
    assert!(out.contains("web/client.ts:3 (#retry)"), "{out}");
}

#[test]
fn skill_prints_and_installs() {
    let p = Project::new(&[("README.md", "# Hi\n")]);
    let (code, out) = p.run(&["skill"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("---\nname: graphdown\n"), "{out}");
    let (code, out) = p.run(&["skill", "--install"]);
    assert_eq!(code, 0, "{out}");
    assert!(p
        .read(".claude/skills/graphdown/SKILL.md")
        .contains("graphdown check"));
    // The installed copy is hidden, so it is not checked as a project doc.
    assert!(p.run(&["check"]).1.contains("across 1 docs"));
}

#[test]
fn check_can_print_github_annotations() {
    let p = Project::new(&[("docs/a.md", "# A\n\nSee @docs/a#nope, then stop.\n")]);
    let (code, out) = p.run(&["check", "--format", "github"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.starts_with(
            "::error file=docs/a.md,line=3,col=5,title=broken reference @docs/a#nope::no heading `#nope` in docs/a.md\n"
        ),
        "{out}"
    );
    assert!(out.contains("1 broken"), "{out}");
}

#[test]
fn docs_site_links_resolve() {
    let p = Project::new(&[
        ("docs/index.md", "# Home\n\n[API](/guide/api#hooks) and [setup](guide/setup) and [old](guide/api.html)\n"),
        ("docs/guide/api.md", "# API\n\n## Hooks\n\nBack [home](/).\n"),
        ("docs/guide/setup/index.md", "# Setup\n"),
    ]);
    let (code, out) = p.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let (_, out) = p.run(&["refs", "docs/guide/api#hooks"]);
    assert!(out.contains("docs/index.md:3"), "{out}");
}

#[test]
fn comment_tags_that_look_like_paths_are_ignored() {
    let p = Project::new(&[
        ("docs/a.md", "# A\n"),
        ("src/x.ts", "// eslint-disable-next-line @typescript-eslint/ban-ts-comment\n// see @docs/a#nope\nexport const x = 1;\n"),
    ]);
    let (code, out) = p.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(!out.contains("typescript-eslint"), "{out}");
    assert!(out.contains("src/x.ts:2"), "{out}");
}

#[test]
fn refs_lists_unlinked_mentions_separately() {
    let p = Project::new(&[
        ("src/server.ts", "export interface DevServer {\n  ssrFixStacktrace(e: Error): void\n}\n"),
        (
            "docs/api.md",
            "# API\n\n## Stack traces\n\nSee @src/server.ts#DevServer.ssrFixStacktrace for details.\n\n## Migration\n\nYou no longer need `server.ssrFixStacktrace` here.\nNor ssrFixStacktraceExtra, a different word.\n",
        ),
    ]);
    let (code, out) = p.run(&["refs", "src/server.ts#DevServer.ssrFixStacktrace"]);
    assert_eq!(code, 0, "{out}");
    let (refs, mentions) = out.split_once("mentioned without a reference").expect(&out);
    assert!(refs.contains("docs/api.md:5 (#stack-traces)"), "{out}");
    assert!(
        mentions.contains(
            "docs/api.md:9 (#migration)  You no longer need `server.ssrFixStacktrace` here."
        ),
        "{out}"
    );
    assert!(!mentions.contains(":10"), "{out}");
    assert!(!mentions.contains(":5 "), "{out}");

    let (_, out) = p.run(&[
        "refs",
        "src/server.ts#DevServer.ssrFixStacktrace",
        "--no-mentions",
    ]);
    assert!(!out.contains("mentioned without"), "{out}");

    let (_, out) = p.run(&[
        "refs",
        "docs/api#migration",
        "--mention",
        "no longer",
        "--json",
    ]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(
        v.as_array()
            .unwrap()
            .iter()
            .all(|h| h["kind"] == "mention" || h["kind"] == "reference"),
        "{out}"
    );
}

#[test]
fn check_lists_docs_that_name_changed_code() {
    let p = Project::new(&[
        (
            "src/server.ts",
            "export function createServer() {\n  const server = {\n    ssrFixStacktrace(e: Error) {\n      return e\n    },\n    listen() {\n      return 1\n    },\n  }\n  return server\n}\n",
        ),
        (
            "docs/api.md",
            "# API\n\n## Errors\n\nCall `server.ssrFixStacktrace` on SSR errors.\n\n## Server\n\nSee @src/server.ts#createServer.\n",
        ),
        ("docs/other.md", "# Other\n\nThe main thing is `listen`.\n"),
    ]);
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(p.path())
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "init"]);

    // Nothing changed yet: nothing to list.
    let (code, out) = p.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("changed code"), "{out}");

    // Edit inside the object-literal method; the doc line isn't touched.
    let src = p
        .read("src/server.ts")
        .replace("return e\n", "return new Error(e.message)\n");
    fs::write(p.path().join("src/server.ts"), src).unwrap();
    let (code, out) = p.run(&["check"]);
    assert_eq!(code, 0, "notes don't fail the check: {out}");
    assert!(
        out.contains(
            "docs/api.md:5 (#errors)  mentions `ssrFixStacktrace` (@src/server.ts#createServer)"
        ),
        "{out}"
    );
    assert!(
        out.contains("docs/api.md:9 (#server)  references @src/server.ts#createServer"),
        "{out}"
    );
    assert!(!out.contains("other.md"), "{out}");

    // Once the doc line is edited too, it drops off the list.
    let doc = p
        .read("docs/api.md")
        .replace("on SSR errors", "on any SSR error");
    fs::write(p.path().join("docs/api.md"), doc).unwrap();
    let (_, out) = p.run(&["check"]);
    assert!(!out.contains("docs/api.md:5"), "{out}");

    let (_, out) = p.run(&["check", "--json"]);
    assert!(out.contains("\"stale\""), "{out}");
    let (_, out) = p.run(&["check", "--no-changes"]);
    assert!(!out.contains("changed code"), "{out}");
    let (_, out) = p.run(&["check", "--format", "github"]);
    assert!(out.contains("::notice file=docs/api.md,line=9,"), "{out}");
}
