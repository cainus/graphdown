//! Which lines changed since a git revision, so `check` can point at docs
//! that describe changed code but weren't touched.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::Path;
use std::process::Command;

/// Changed line ranges in the working tree, by path relative to the project
/// root, in the new version of each file.
pub type Changes = BTreeMap<String, Vec<RangeInclusive<usize>>>;

/// Lines changed since `since` (staged or not), from `git diff`. `None` when
/// the root isn't in a git repository, git isn't installed, or the revision
/// doesn't exist; then there is nothing to compare against. Untracked files
/// aren't included.
///
/// Spec: @docs/cli#changed-code
pub fn since(root: &Path, since: &str) -> Option<Changes> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "diff",
            "--unified=0",
            "--no-color",
            "--no-ext-diff",
            "--relative",
            since,
            "--",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(parse_diff(&String::from_utf8_lossy(&out.stdout)))
}

/// Reads the `+++ b/path` and `@@ -a,b +c,d @@` lines of a unified diff.
/// A pure deletion is recorded as the line it happened after, so the
/// definition it sat in still counts as changed.
pub fn parse_diff(diff: &str) -> Changes {
    let mut changes = Changes::new();
    let mut file: Option<String> = None;
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            file = path.strip_prefix("b/").map(str::to_string);
            continue;
        }
        let (Some(path), Some(hunk)) = (&file, line.strip_prefix("@@ ")) else {
            continue;
        };
        let Some(new) = hunk.split_whitespace().find(|w| w.starts_with('+')) else {
            continue;
        };
        let new = &new[1..];
        let (start, count) = match new.split_once(',') {
            Some((s, c)) => (s.parse().unwrap_or(0), c.parse().unwrap_or(0)),
            None => (new.parse().unwrap_or(0), 1),
        };
        let start: usize = start;
        let range = if count == 0 {
            start.max(1)..=start.max(1)
        } else {
            start..=start + count - 1
        };
        changes.entry(path.clone()).or_default().push(range);
    }
    changes
}

/// Whether `line` falls in one of `ranges`.
pub fn touches(ranges: &[RangeInclusive<usize>], line: usize) -> bool {
    ranges.iter().any(|r| r.contains(&line))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_hunks() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -3 +3 @@ fn x\n-a\n+b\n@@ -10,2 +10,0 @@\n-c\n-d\n@@ -20,0 +19,3 @@\n+e\n+f\n+g\ndiff --git a/old.md b/old.md\n--- a/old.md\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";
        let c = parse_diff(diff);
        assert_eq!(c.len(), 1);
        assert_eq!(c["src/a.rs"], vec![3..=3, 10..=10, 19..=21]);
    }
}
