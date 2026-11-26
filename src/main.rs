use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};

use graphdown::ops::{self, Style};
use graphdown::project::{find_root, Project, Resolved};

/// Navigate markdown docs like code: checked @references, outlines,
/// find-references and context slices.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Project root. Defaults to the nearest ancestor holding graphdown.toml or .git.
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check that every reference in every doc points at something real.
    Check {
        /// Shorthand for `--format json`.
        #[arg(long)]
        json: bool,
        /// `github` prints workflow annotations, shown inline on pull requests.
        #[arg(long, value_enum, default_value_t = CheckFormat::Text)]
        format: CheckFormat,
        /// Compare against this git revision when listing docs that describe
        /// changed code.
        #[arg(long, value_name = "REV", default_value = "HEAD")]
        since: String,
        /// Don't list docs that describe changed code.
        #[arg(long)]
        no_changes: bool,
    },
    /// List every reference that points at a doc, section, file or symbol.
    Refs {
        target: String,
        /// Also list doc lines that contain this word without a reference.
        /// A code symbol's own name is always searched for. Repeatable.
        #[arg(long = "mention", value_name = "WORD")]
        mention: Vec<String>,
        /// Leave out unlinked mentions.
        #[arg(long)]
        no_mentions: bool,
        #[arg(long)]
        json: bool,
    },
    /// Print the agent skill for this version of graphdown, or install it.
    Skill {
        /// Write it to .claude/skills/graphdown/SKILL.md under the project root.
        #[arg(long)]
        install: bool,
    },
    /// Rewrite references between `@ref` form and clickable markdown links.
    Fmt {
        #[arg(long, value_enum)]
        to: Style,
        /// Report files that would change and exit non-zero, without writing.
        #[arg(long)]
        check: bool,
        /// Docs to rewrite. Defaults to every doc.
        files: Vec<String>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CheckFormat {
    Text,
    Json,
    Github,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("graphdown: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let root = match cli.root {
        Some(r) => r,
        None => find_root(&std::env::current_dir()?),
    };
    // The skill needs no docs loaded, and must work outside any project.
    if let Command::Skill { install } = cli.command {
        return skill(&root, install);
    }
    let project = Project::load(&root)?;
    let target =
        |s: &str| -> Result<Resolved> { project.parse_target(s).map_err(anyhow::Error::msg) };
    let mut out = std::io::stdout().lock();

    match cli.command {
        Command::Check {
            json,
            format,
            since,
            no_changes,
        } => {
            let report = ops::check(&project);
            let stale = if no_changes {
                Vec::new()
            } else {
                graphdown::changes::since(&project.root, &since)
                    .map(|c| ops::stale(&project, &c))
                    .unwrap_or_default()
            };
            let format = if json { CheckFormat::Json } else { format };
            if format == CheckFormat::Json {
                let body = serde_json::json!({ "problems": report.problems, "stale": stale });
                writeln!(out, "{}", serde_json::to_string_pretty(&body)?)?;
            } else {
                if format == CheckFormat::Github {
                    for p in &report.problems {
                        writeln!(out, "{}", ops::github_annotation(p))?;
                    }
                    for h in &stale {
                        writeln!(out, "{}", ops::github_stale_notice(h))?;
                    }
                }
                for p in &report.problems {
                    writeln!(
                        out,
                        "{}:{}:{}: {}: {}",
                        p.file, p.line, p.col, p.reference, p.message
                    )?;
                }
                let n = report.problems.len();
                let mut summary = format!("{} refs across {} docs", report.refs, report.docs);
                if report.sources > 0 {
                    summary.push_str(&format!(" and {} source files", report.sources));
                }
                if !stale.is_empty() {
                    writeln!(
                        out,
                        "changed code is named in lines the change didn't touch (check these by hand):"
                    )?;
                    for h in &stale {
                        let section = h
                            .within
                            .as_ref()
                            .map(|s| format!(" (#{s})"))
                            .unwrap_or_default();
                        let kind = match h.kind {
                            ops::IncomingKind::Reference => "references",
                            ops::IncomingKind::Mention => "mentions",
                        };
                        writeln!(
                            out,
                            "{}:{}{section}  {kind} {}  {}",
                            h.file, h.line, h.target, h.reference
                        )?;
                    }
                }
                if n == 0 {
                    writeln!(out, "ok: {summary}")?;
                } else {
                    writeln!(out, "{n} broken: {summary}")?;
                }
            }
            return Ok(if report.problems.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        }
        Command::Refs {
            target: t,
            mention,
            no_mentions,
            json,
        } => {
            let t = target(&t)?;
            let mut hits = ops::incoming(&project, &t);
            let mentions = if no_mentions {
                Vec::new()
            } else {
                ops::mentions(&project, &t, &ops::mention_terms(&t, &mention))
            };
            if json {
                hits.extend(mentions);
                writeln!(out, "{}", serde_json::to_string_pretty(&hits)?)?;
            } else {
                let line = |h: &ops::Incoming| {
                    let section = h
                        .within
                        .as_ref()
                        .map(|s| format!(" (#{s})"))
                        .unwrap_or_default();
                    format!("{}:{}{section}  {}", h.file, h.line, h.reference)
                };
                for h in &hits {
                    writeln!(out, "{}", line(h))?;
                }
                if hits.is_empty() {
                    writeln!(out, "no references to {t}")?;
                }
                if !mentions.is_empty() {
                    writeln!(
                        out,
                        "\nmentioned without a reference (check these by hand):"
                    )?;
                    for h in &mentions {
                        writeln!(out, "{}", line(h))?;
                    }
                }
            }
        }
        Command::Skill { .. } => unreachable!("handled before loading"),
        Command::Fmt { to, check, files } => {
            let docs: Vec<&graphdown::markdown::Doc> = if files.is_empty() {
                project.docs.values().collect()
            } else {
                let mut v = Vec::new();
                for f in &files {
                    let rel = f.trim_start_matches("./");
                    match project.doc(rel) {
                        Some(d) => v.push(d),
                        None => bail!("not a doc under the project root: {f}"),
                    }
                }
                v
            };
            let mut changed = 0;
            for doc in docs {
                if let Some(new) = ops::format_doc(&project, doc, to) {
                    changed += 1;
                    if check {
                        writeln!(out, "would rewrite {}", doc.path)?;
                    } else {
                        std::fs::write(project.root.join(&doc.path), new)?;
                        writeln!(out, "rewrote {}", doc.path)?;
                    }
                }
            }
            if check && changed > 0 {
                return Ok(ExitCode::FAILURE);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The agent skill, built into the binary so it always matches this version.
///
/// Spec: @docs/cli#skill
const SKILL: &str = include_str!("../skill/SKILL.md");

fn skill(root: &std::path::Path, install: bool) -> Result<ExitCode> {
    if !install {
        print!("{SKILL}");
        return Ok(ExitCode::SUCCESS);
    }
    let dir = root.join(".claude/skills/graphdown");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("SKILL.md");
    std::fs::write(&path, SKILL)?;
    println!("wrote {}", path.display());
    Ok(ExitCode::SUCCESS)
}
