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
    },
    /// List the headings of docs (or the symbols of code files) with line ranges.
    #[command(alias = "ls")]
    Outline {
        /// Docs or code files. Defaults to every doc.
        targets: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Print one section or symbol, e.g. `graphdown show docs/syntax#references`.
    Show { target: String },
    /// List every reference that points at a doc, section, file or symbol.
    Refs {
        target: String,
        #[arg(long)]
        json: bool,
    },
    /// Print the given sections plus everything they reference, transitively.
    Slice {
        #[arg(required = true)]
        targets: Vec<String>,
        /// How many reference hops to follow (default: no limit).
        #[arg(long)]
        depth: Option<usize>,
        /// Leave out referenced code.
        #[arg(long)]
        no_code: bool,
        #[arg(long, value_enum, default_value_t = Format::Markdown)]
        format: Format,
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

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Markdown,
    Xml,
    Json,
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
        Command::Check { json, format } => {
            let report = ops::check(&project);
            let format = if json { CheckFormat::Json } else { format };
            if format == CheckFormat::Json {
                writeln!(out, "{}", serde_json::to_string_pretty(&report.problems)?)?;
            } else {
                if format == CheckFormat::Github {
                    for p in &report.problems {
                        writeln!(out, "{}", ops::github_annotation(p))?;
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
        Command::Outline { targets, json } => {
            let targets: Vec<Resolved> = if targets.is_empty() {
                project
                    .docs
                    .keys()
                    .map(|p| Resolved::Doc { path: p.clone() })
                    .collect()
            } else {
                targets.iter().map(|t| target(t)).collect::<Result<_>>()?
            };
            let outlines = targets
                .iter()
                .map(|t| ops::outline(&project, t, true).map_err(anyhow::Error::msg))
                .collect::<Result<Vec<_>>>()?;
            if json {
                writeln!(out, "{}", serde_json::to_string_pretty(&outlines)?)?;
            } else {
                for (i, o) in outlines.iter().enumerate() {
                    if i > 0 {
                        writeln!(out)?;
                    }
                    write!(out, "{}", ops::render_outline(o))?;
                }
            }
        }
        Command::Show { target: t } => {
            let t = target(&t)?;
            let c = project.content(&t).map_err(anyhow::Error::msg)?;
            writeln!(out, "<!-- {t}  {} L{}-{} -->", c.path, c.start, c.end)?;
            writeln!(out, "{}", c.text)?;
        }
        Command::Refs { target: t, json } => {
            let t = target(&t)?;
            let hits = ops::incoming(&project, &t);
            if json {
                writeln!(out, "{}", serde_json::to_string_pretty(&hits)?)?;
            } else {
                for h in &hits {
                    let section = h
                        .within
                        .as_ref()
                        .map(|s| format!(" (#{s})"))
                        .unwrap_or_default();
                    writeln!(out, "{}:{}{section}  {}", h.file, h.line, h.reference)?;
                }
                if hits.is_empty() {
                    writeln!(out, "no references to {t}")?;
                }
            }
        }
        Command::Slice {
            targets,
            depth,
            no_code,
            format,
        } => {
            let entries = targets
                .iter()
                .map(|t| target(t))
                .collect::<Result<Vec<_>>>()?;
            let pieces =
                ops::slice(&project, &entries, depth, !no_code).map_err(anyhow::Error::msg)?;
            match format {
                Format::Markdown => write!(out, "{}", ops::render_slice_markdown(&pieces))?,
                Format::Xml => write!(out, "{}", ops::render_slice_xml(&pieces))?,
                Format::Json => writeln!(out, "{}", serde_json::to_string_pretty(&pieces)?)?,
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
