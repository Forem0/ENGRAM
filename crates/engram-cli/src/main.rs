use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use engram_core::{
    compare_cases, create_demo_fixture, demo_case, read_case, scan_path, write_case, write_report,
    Case, Category, Confidence, EngramError, ReportFormat, ReportOptions, ScanOptions, SourceKind,
};
use serde_json::json;

#[derive(Parser)]
#[command(
    name = "engram",
    version,
    about = "Find what remains.",
    long_about = "ENGRAM examines explicitly selected offline files and iPhone backups. Findings describe observed evidence and candidates, not proof of deletion or credential validity. No network requests are made by the analyzer."
)]
struct Cli {
    /// Write structured JSON to stdout instead of the human-readable view.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the real engine against a generated, synthetic demonstration dataset.
    Demo {
        /// Save the resulting case. Existing files are never overwritten.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Analyze this folder or unencrypted iPhone backup, without modifying it.
    Scan {
        /// Explicit source folder. ENGRAM does not search your computer for sources.
        source: PathBuf,
        /// New case file outside the source directory.
        #[arg(long)]
        out: PathBuf,
        #[command(flatten)]
        limits: Limits,
    },
    /// Open a saved case and show its coverage, findings and module states.
    Inspect { case: PathBuf },
    /// Export selected findings to a new Markdown report or spreadsheet-safe CSV.
    Export {
        /// Saved ENGRAM case. Export does not read its original source again.
        case: PathBuf,
        /// New report file outside the source recorded in the case.
        #[arg(long)]
        out: PathBuf,
        /// Human-readable report or CSV table; neither format is a restorable case.
        #[arg(long, value_enum, default_value_t = ExportFormat::Markdown)]
        format: ExportFormat,
        /// Include this known finding ID. Repeat to choose multiple findings.
        #[arg(long = "finding", value_name = "ID", conflicts_with = "bookmarked", value_parser = parse_finding_id)]
        findings: Vec<String>,
        /// Export only findings bookmarked in this saved case.
        #[arg(long, conflicts_with = "findings")]
        bookmarked: bool,
        /// Include analyst notes as entered; notes are not automatically redacted.
        #[arg(long)]
        include_notes: bool,
        /// Include recorded source paths instead of filename-only locations.
        #[arg(long)]
        include_paths: bool,
        /// Include source-derived details, which can contain private text or paths.
        #[arg(long)]
        include_details: bool,
    },
    /// Compare two saved cases. A missing finding is not proof of secure deletion.
    Compare {
        before: PathBuf,
        after: PathBuf,
        /// Also save the comparison JSON to a new file.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Show this build's operating boundaries and default scan limits.
    Doctor,
    /// Generate synthetic files for reproducible local testing.
    Fixture { destination: PathBuf },
}

#[derive(Clone, Copy, ValueEnum)]
enum ExportFormat {
    Markdown,
    Csv,
}

impl From<ExportFormat> for ReportFormat {
    fn from(value: ExportFormat) -> Self {
        match value {
            ExportFormat::Markdown => Self::Markdown,
            ExportFormat::Csv => Self::Csv,
        }
    }
}

#[derive(Args)]
struct Limits {
    /// Maximum number of source files examined.
    #[arg(long, default_value_t = 25_000, value_parser = parse_positive_usize)]
    max_files: usize,
    /// Maximum bytes read from a single file (default: 8 MiB).
    #[arg(long, default_value_t = 8_388_608, value_parser = parse_positive_u64)]
    max_file_bytes: u64,
    /// Maximum total file bytes read (default: 512 MiB).
    #[arg(long, default_value_t = 536_870_912, value_parser = parse_positive_u64)]
    max_total_bytes: u64,
    /// Skip dotfiles. Hidden text such as .env is otherwise eligible for scanning.
    #[arg(long)]
    skip_hidden: bool,
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(|| "must be a positive integer".to_owned())
}

fn parse_positive_u64(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(|| "must be a positive integer".to_owned())
}

fn parse_finding_id(value: &str) -> Result<String, String> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(
            "must be a nonempty finding ID of at most 256 bytes without control characters".into(),
        );
    }
    Ok(value.to_owned())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ENGRAM: {}", visible(&error.to_string()));
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), EngramError> {
    match cli.command {
        Command::Demo { out } => {
            let case = demo_case();
            let out = out
                .map(|path| absolute_resolved_target(&path))
                .transpose()?;
            if let Some(path) = out.as_deref() {
                ensure_new_file(path)?;
                write_case(&case, path)?;
            }
            print_case(&case, cli.json)?;
            if !cli.json {
                if let Some(path) = out {
                    println!("\nSaved: {}", visible(&path.display().to_string()));
                }
            }
        }
        Command::Scan {
            source,
            out,
            limits,
        } => {
            let out = ensure_outside_source(&source, &out)?;
            ensure_new_file(&out)?;
            let options = ScanOptions {
                max_files: limits.max_files,
                max_file_bytes: limits.max_file_bytes,
                max_total_bytes: limits.max_total_bytes,
                include_hidden: !limits.skip_hidden,
            };
            let case = scan_path(&source, &options)?;
            write_case(&case, &out)?;
            print_case(&case, cli.json)?;
            if !cli.json {
                println!("\nSaved: {}", visible(&out.display().to_string()));
            }
        }
        Command::Inspect { case } => print_case(&read_case(&case)?, cli.json)?,
        Command::Export {
            case,
            out,
            format,
            findings,
            bookmarked,
            include_notes,
            include_paths,
            include_details,
        } => {
            let case = read_case(&case)?;
            let finding_ids = if !findings.is_empty() {
                findings
            } else {
                case.findings
                    .iter()
                    .filter(|finding| {
                        !bookmarked
                            || case
                                .review
                                .get(&finding.id)
                                .is_some_and(|review| review.bookmarked)
                    })
                    .map(|finding| finding.id.clone())
                    .collect()
            };
            let options = ReportOptions {
                format: format.into(),
                finding_ids,
                include_notes,
                include_paths,
                include_details,
            };
            let receipt = write_report(&case, &options, &out)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&receipt)?);
            } else {
                let format = match format {
                    ExportFormat::Markdown => "Markdown",
                    ExportFormat::Csv => "CSV",
                };
                println!("ENGRAM / EVIDENCE REPORT\n");
                println!("Saved:    {}", visible(&receipt.path));
                println!(
                    "Format:   {format}\nFindings: {}\nBytes:    {}",
                    receipt.finding_count, receipt.bytes
                );
                println!("SHA-256:  {}", visible(&receipt.sha256));
                println!("\nThe input case is unchanged. Reports cannot be reopened as cases.");
                if include_notes {
                    println!(
                        "Included analyst notes are plain text and are not automatically redacted."
                    );
                }
            }
        }
        Command::Compare { before, after, out } => {
            let out = out
                .map(|path| absolute_resolved_target(&path))
                .transpose()?;
            if let Some(path) = out.as_deref() {
                ensure_new_file(path)?;
            }
            let previous = read_case(&before)?;
            let current = read_case(&after)?;
            for case in [&previous, &current] {
                if let Some(path) = out.as_deref() {
                    ensure_outside_stored_source(&case.source_root, path)?;
                }
            }
            let diff = compare_cases(&previous, &current);
            let value = json!({
                "schema_version": 1,
                "before_case": previous.id,
                "after_case": current.id,
                "before_source_kind": previous.source_kind,
                "after_source_kind": current.source_kind,
                "interpretation": "Differences describe the two supplied snapshots only. Absence is not proof of secure deletion.",
                "diff": diff,
            });
            if let Some(path) = out {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent)?;
                }
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)?;
                file.write_all(&serde_json::to_vec_pretty(&value)?)?;
                file.write_all(b"\n")?;
                file.sync_all()?;
            }
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else {
                println!("ENGRAM / SNAPSHOT COMPARISON\n");
                println!("Before   {}", visible(&previous.title));
                println!("After    {}", visible(&current.title));
                println!(
                    "\nAdded      {:>6}\nRemoved    {:>6}\nChanged    {:>6}\nUnchanged  {:>6}",
                    diff.added.len(),
                    diff.removed.len(),
                    diff.changed.len(),
                    diff.unchanged.len()
                );
                println!("\nA removed finding is absent from the later supplied snapshot.\nIt does not establish that every copy was erased.");
                if previous.source_kind != current.source_kind
                    || previous.source_kind == SourceKind::Demo
                {
                    println!("\nNote: compare source types and coverage before interpreting these differences.");
                }
            }
        }
        Command::Doctor => {
            let defaults = ScanOptions::default();
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "application": "ENGRAM", "version": env!("CARGO_PKG_VERSION"),
                        "channel": "release", "operating_system": std::env::consts::OS,
                        "architecture": std::env::consts::ARCH, "network_required": false,
                        "source_selection": "explicit_path", "source_mutation": false,
                        "credential_validation": false, "keychain_recovery": false,
                        "default_limits": { "max_files": defaults.max_files, "max_file_bytes": defaults.max_file_bytes, "max_total_bytes": defaults.max_total_bytes },
                        "verification": "This command reports build settings; it does not scan personal files or test access to a device."
                    }))?
                );
            } else {
                println!("ENGRAM {} / LOCAL ANALYSIS", env!("CARGO_PKG_VERSION"));
                println!("{} / {}", std::env::consts::OS, std::env::consts::ARCH);
                println!("\nSource selection       Explicit folder or backup path\nAnalyzer network       None\nWrites to source       None\nCredential validation  None\nKeychain recovery      Not implemented");
                println!("\nDefault limits\n  Files: {}\n  Per-file read: {} bytes\n  Total read: {} bytes", defaults.max_files, defaults.max_file_bytes, defaults.max_total_bytes);
                println!("\nDoctor did not inspect personal data or connect to a device.");
            }
        }
        Command::Fixture { destination } => {
            create_demo_fixture(&destination)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &json!({ "synthetic": true, "destination": destination, "next": "engram scan <destination> --out <case-file-outside-source>" })
                    )?
                );
            } else {
                println!(
                    "Synthetic fixture created: {}",
                    visible(&destination.display().to_string())
                );
                println!("Contains generated test data only. Analyze it with `engram scan <folder> --out <case-file-outside-source>`.");
            }
        }
    }
    Ok(())
}

fn ensure_new_file(path: &Path) -> Result<(), EngramError> {
    if path.exists() || std::fs::symlink_metadata(path).is_ok() {
        return Err(EngramError::Input("Output already exists. Choose a new case filename; existing files are not overwritten.".into()));
    }
    Ok(())
}

fn ensure_outside_source(source: &Path, output: &Path) -> Result<PathBuf, EngramError> {
    let source = source.canonicalize()?;
    let output = absolute_resolved_target(output)?;
    if output.starts_with(&source) {
        return Err(EngramError::Input("The case output must be outside the selected source directory, so the source remains unchanged.".into()));
    }
    Ok(output)
}

// Imported source metadata is inert: resolving a stored UNC path could initiate
// SMB access. Genuine scan roots are already canonical when the case is created.
fn ensure_outside_stored_source(stored_source: &str, output: &Path) -> Result<(), EngramError> {
    let source = Path::new(stored_source);
    if source.is_absolute() && output.starts_with(source) {
        return Err(EngramError::Input(
            "Save comparison output outside the source recorded in the supplied case.".into(),
        ));
    }
    Ok(())
}

// Resolve an output with not-yet-created parent directories without treating lexical
// `..` components or an existing parent symlink as a different destination.
fn absolute_resolved_target(path: &Path) -> Result<PathBuf, EngramError> {
    use std::path::Component;
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    // Normalize before creating any parents. Otherwise a path such as
    // source/new/../../outside/case.json could create source/new on its way out.
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str())
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(EngramError::Input(
                        "Cannot resolve output above its filesystem root.".into(),
                    ));
                }
            }
        }
    }
    let mut suffix = Vec::new();
    let mut ancestor = normalized.as_path();
    while !ancestor.exists() {
        if std::fs::symlink_metadata(ancestor).is_ok() {
            return Err(EngramError::Input(
                "Output resolves through an unavailable link. Choose a regular output directory."
                    .into(),
            ));
        }
        let name = ancestor
            .file_name()
            .ok_or_else(|| EngramError::Input("Cannot resolve output path.".into()))?;
        suffix.push(name.to_owned());
        ancestor = ancestor
            .parent()
            .ok_or_else(|| EngramError::Input("Cannot resolve output parent.".into()))?;
    }
    let mut resolved = ancestor.canonicalize()?;
    for component in suffix.into_iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

fn print_case(case: &Case, as_json: bool) -> Result<(), EngramError> {
    if as_json {
        println!("{}", serde_json::to_string_pretty(case)?);
        return Ok(());
    }
    println!(
        "ENGRAM / {}",
        if case.source_kind == SourceKind::Demo {
            "SYNTHETIC DEMO"
        } else {
            "LOCAL INSPECTION"
        }
    );
    println!(
        "{}\nSource: {}",
        visible(&case.title),
        case.source_kind.label()
    );
    println!(
        "\n{:>6} files examined   {:>6} findings   {:>6} skipped",
        case.stats.files_examined,
        case.findings.len(),
        case.stats.skipped_files
    );
    println!(
        "{} bytes examined / {} ms",
        case.stats.bytes_examined, case.stats.elapsed_ms
    );
    println!("\nCATEGORY       FINDINGS    CANDIDATES");
    for category in Category::ALL {
        let total = case
            .findings
            .iter()
            .filter(|f| f.category == category)
            .count();
        let candidates = case
            .findings
            .iter()
            .filter(|f| f.category == category && f.confidence == Confidence::Candidate)
            .count();
        println!("{:<14} {:>8} {:>13}", category.label(), total, candidates);
    }
    println!("\nMODULES");
    for module in &case.modules {
        println!(
            "  {:<16} {:<13} {}",
            visible(&module.name),
            module.state.label(),
            visible(&module.detail)
        );
    }
    if !case.warnings.is_empty() {
        println!("\nCOVERAGE / LIMITATIONS");
        for warning in &case.warnings {
            println!("  - {}", visible(warning));
        }
    }
    println!("\nFINDINGS (first 30; use --json for all records)");
    for finding in case.findings.iter().take(30) {
        println!(
            "  {:<10} {:<10} {}",
            finding.category.label(),
            finding.confidence.label(),
            visible(&finding.title)
        );
        println!(
            "    {} {}",
            visible(&finding.path),
            visible(&finding.locator)
        );
    }
    println!("\nObserved data and candidate matches are evidence to review.\nThey do not prove permanent deletion, unauthorized access, or that a secret is active.");
    Ok(())
}

fn visible(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| {
            if c.is_control() {
                format!("\\u{{{:x}}}", c as u32).chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_control_sequences_are_escaped() {
        assert_eq!(visible("file\u{1b}[31m\n"), "file\\u{1b}[31m\\u{a}");
    }

    #[test]
    fn export_flags_have_a_valid_command_schema() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn export_selector_modes_are_mutually_exclusive() {
        assert!(Cli::try_parse_from([
            "engram",
            "export",
            "saved.json",
            "--out",
            "report.md",
            "--finding",
            "id-1",
            "--bookmarked",
        ])
        .is_err());
        assert!(Cli::try_parse_from([
            "engram",
            "export",
            "saved.json",
            "--out",
            "report.md",
            "--format",
            "markdown",
        ])
        .is_ok());
    }

    #[test]
    fn export_finding_ids_reject_controls_and_excess_utf8_bytes() {
        assert!(parse_finding_id("finding-1").is_ok());
        assert!(parse_finding_id("").is_err());
        assert!(parse_finding_id("finding\u{1b}[31m").is_err());
        assert!(parse_finding_id(&"é".repeat(129)).is_err());
    }

    #[test]
    fn imported_source_metadata_is_never_resolved() {
        let remote = "\\\\unreachable.invalid\\share\\evidence";
        assert!(ensure_outside_stored_source(remote, Path::new("comparison.json")).is_ok());
        assert!(ensure_outside_stored_source(
            "synthetic://engram-demo",
            Path::new("comparison.json")
        )
        .is_ok());
    }

    #[test]
    fn refuse_case_output_within_source_even_for_missing_subfolders() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir(&source).unwrap();
        assert!(ensure_outside_source(&source, &source.join("new/case.engram.json")).is_err());
        assert!(ensure_outside_source(&source, &temp.path().join("cases/new.engram.json")).is_ok());
        assert!(
            ensure_outside_source(&source, &temp.path().join("missing/../source/case.json"))
                .is_err()
        );
        let resolved = ensure_outside_source(&source, &source.join("new/../../case.json")).unwrap();
        assert_eq!(
            resolved,
            temp.path().canonicalize().unwrap().join("case.json")
        );
        assert!(!source.join("new").exists());
    }

    #[cfg(unix)]
    #[test]
    fn output_parent_link_cannot_disguise_source_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(&source, &alias).unwrap();
        assert!(ensure_outside_source(&source, &alias.join("new/case.json")).is_err());
        assert!(!source.join("new").exists());
    }
}
