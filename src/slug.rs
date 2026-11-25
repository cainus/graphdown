//! GitHub-compatible heading slugs.

use std::collections::{HashMap, HashSet};

/// Slug for a single heading, matching GitHub's anchor algorithm:
/// lowercase, keep letters/digits/`_`/`-`, spaces become `-`, drop the rest.
///
/// Spec: @docs/syntax#symbols-are-headings
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.trim().chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            out.extend(c.to_lowercase());
        } else if c == ' ' {
            out.push('-');
        }
    }
    out
}

/// Assigns unique slugs within one document the way GitHub does:
/// the second "Usage" heading becomes `usage-1`, the third `usage-2`.
#[derive(Default)]
pub struct Slugger {
    used: HashSet<String>,
    counts: HashMap<String, usize>,
}

impl Slugger {
    pub fn slug(&mut self, text: &str) -> String {
        let base = slugify(text);
        let mut slug = base.clone();
        let mut n = self.counts.get(&base).copied().unwrap_or(0);
        while self.used.contains(&slug) {
            n += 1;
            slug = format!("{base}-{n}");
        }
        self.counts.insert(base, n);
        self.used.insert(slug.clone());
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_style() {
        assert_eq!(slugify("Token TTL"), "token-ttl");
        assert_eq!(slugify("What's `@ref` syntax?"), "whats-ref-syntax");
        assert_eq!(slugify("Retry & Backoff"), "retry--backoff");
    }

    #[test]
    fn duplicates() {
        let mut s = Slugger::default();
        assert_eq!(s.slug("Usage"), "usage");
        assert_eq!(s.slug("Usage"), "usage-1");
        assert_eq!(s.slug("Usage"), "usage-2");
    }
}
