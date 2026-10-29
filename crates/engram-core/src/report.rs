use crate::{
    digest, storage, Case, CaseReview, Confidence, EngramError, Finding, ReviewAnnotation,
    ReviewStatus,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

pub const MAX_REPORT_FINDINGS: usize = 10_000;
pub const MAX_REPORT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    Markdown,
    Csv,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportOptions {
    pub format: ReportFormat,
    pub finding_ids: Vec<String>,
    #[serde(default)]
    pub include_notes: bool,
    #[serde(default)]
    pub include_paths: bool,
    #[serde(default)]
    pub include_details: bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            format: ReportFormat::Markdown,
            finding_ids: Vec::new(),
            include_notes: false,
            include_paths: false,
            include_details: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderedReport {
    pub content: String,
    pub format: ReportFormat,
    pub finding_count: usize,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportReceipt {
    pub path: String,
    pub format: ReportFormat,
    pub finding_count: usize,
    pub sha256: String,
    pub bytes: usize,
}

struct ReportWriter {
    content: String,
}

impl ReportWriter {
    fn new() -> Self {
        Self {
            content: String::with_capacity(4096),
        }
    }

    fn push(&mut self, value: &str) -> Result<(), EngramError> {
        if value.len() > MAX_REPORT_BYTES.saturating_sub(self.content.len()) {
            return Err(EngramError::Input(
                "Report would exceed 16 MiB. Select fewer findings or omit details and notes. No report was written.".into(),
            ));
        }
        self.content.push_str(value);
        Ok(())
    }
}

fn selection<'a>(case: &'a Case, options: &ReportOptions) -> Result<Vec<&'a Finding>, EngramError> {
    if options.finding_ids.is_empty() || options.finding_ids.len() > MAX_REPORT_FINDINGS {
        return Err(EngramError::Input(format!(
            "Select between 1 and {MAX_REPORT_FINDINGS} findings for a report."
        )));
    }
    let known: HashSet<&str> = case
        .findings
        .iter()
        .map(|finding| finding.id.as_str())
        .collect();
    let mut selected = HashSet::new();
    for id in &options.finding_ids {
        if id.is_empty()
            || id.len() > 256
            || id.chars().any(char::is_control)
            || !known.contains(id.as_str())
            || !selected.insert(id.as_str())
        {
            return Err(EngramError::Input(
                "Report selection contains an invalid, unknown or duplicate finding identity."
                    .into(),
            ));
        }
    }
    // Request order cannot reorder or fabricate native evidence.
    Ok(case
        .findings
        .iter()
        .filter(|finding| selected.contains(finding.id.as_str()))
        .collect())
}

/// Build a deterministic, bounded report from a validated snapshot. This is a
/// derived report, not a restorable case; no described source path is opened.
pub fn render_report(case: &Case, options: &ReportOptions) -> Result<RenderedReport, EngramError> {
    let review = storage::validate_case(case)?;
    let selected = selection(case, options)?;
    if case.id.chars().any(char::is_control)
        || case
            .findings
            .iter()
            .any(|finding| finding.id.chars().any(char::is_control))
        || selected
            .iter()
            .any(|finding| finding.rule_id.chars().any(char::is_control))
    {
        return Err(EngramError::Input(
            "Report identities cannot contain control characters.".into(),
        ));
    }
    if options.format == ReportFormat::Csv
        && (formula_like(&case.id)
            || selected
                .iter()
                .any(|finding| formula_like(&finding.id) || formula_like(&finding.rule_id)))
    {
        return Err(EngramError::Input(
            "CSV cannot preserve case, finding or rule identities with spreadsheet-formula prefixes. Choose Markdown instead; no report was written.".into(),
        ));
    }
    let mut writer = ReportWriter::new();
    match options.format {
        ReportFormat::Markdown => markdown(&mut writer, case, options, &selected, &review)?,
        ReportFormat::Csv => csv(&mut writer, case, options, &selected, &review)?,
    }
    let bytes = writer.content.len();
    let sha256 = digest(writer.content.as_bytes());
    Ok(RenderedReport {
        content: writer.content,
        format: options.format,
        finding_count: selected.len(),
        sha256,
        bytes,
    })
}

/// Save a derived report to a new file outside the described source. Report
/// export never changes review annotations or a case's saved/unsaved state.
pub fn write_report(
    case: &Case,
    options: &ReportOptions,
    destination: &Path,
) -> Result<ReportReceipt, EngramError> {
    let report = render_report(case, options)?;
    let destination = storage::write_new_artifact(
        case,
        destination,
        report.content.as_bytes(),
        MAX_REPORT_BYTES,
    )?;
    Ok(ReportReceipt {
        path: destination.to_string_lossy().into_owned(),
        format: report.format,
        finding_count: report.finding_count,
        sha256: report.sha256,
        bytes: report.bytes,
    })
}

fn filename(value: &str) -> &str {
    value
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or("(unnamed file)")
}

fn path_text(value: &str, options: &ReportOptions) -> String {
    if options.include_paths || value.is_empty() {
        value.to_owned()
    } else {
        filename(value).to_owned()
    }
}

fn invisible_format(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2069}'
            | '\u{feff}'
    )
}

fn clean_text(value: &str) -> String {
    let mut clean = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    let _ = characters.next();
                }
                clean.push('\n');
            }
            '\n' | '\t' => clean.push(character),
            value if value.is_control() => clean.push(' '),
            _ => clean.push(character),
        }
    }
    clean
}

fn markdown_text(value: &str) -> String {
    let mut escaped = String::new();
    for character in clean_text(value).chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\n' => escaped.push_str(" ↵ "),
            '\t' => escaped.push_str(" ⇥ "),
            value if value.is_ascii_punctuation() => {
                escaped.push('\\');
                escaped.push(value);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn utc_time(value: Option<i64>) -> String {
    value
        .map(|seconds| {
            chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, 0)
                .map(|time| time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
                .unwrap_or_else(|| format!("Out-of-range timestamp: {seconds}"))
        })
        .unwrap_or_default()
}

fn status_label(status: ReviewStatus) -> &'static str {
    match status {
        ReviewStatus::Unreviewed => "unreviewed",
        ReviewStatus::NeedsReview => "needs_review",
        ReviewStatus::Reviewed => "reviewed",
    }
}

fn markdown_field(writer: &mut ReportWriter, label: &str, value: &str) -> Result<(), EngramError> {
    let label = markdown_text(label);
    writer.push(&format!("- **{label}:** {}\n", markdown_text(value)))
}

fn markdown(
    writer: &mut ReportWriter,
    case: &Case,
    options: &ReportOptions,
    selected: &[&Finding],
    review: &CaseReview,
) -> Result<(), EngramError> {
    writer.push("# ENGRAM investigation report\n\nFind what remains.\n\n")?;
    writer.push("This report contains selected observations from one offline snapshot. It is not a restorable case. A missing finding is not proof of deletion, credential validity or application access.\n\n## Snapshot and export scope\n\n")?;
    for (label, value) in [
        ("Exporter version", env!("CARGO_PKG_VERSION").to_owned()),
        ("Case ID", case.id.clone()),
        ("Source kind", case.source_kind.label().to_owned()),
        ("Captured", utc_time(Some(case.created_at))),
        ("Completed", utc_time(Some(case.completed_at))),
        ("Selected findings", selected.len().to_string()),
        ("Snapshot findings", case.findings.len().to_string()),
        ("Files examined", case.stats.files_examined.to_string()),
        ("Bytes examined", case.stats.bytes_examined.to_string()),
        ("Skipped files", case.stats.skipped_files.to_string()),
        ("Coverage notes", case.warnings.len().to_string()),
    ] {
        markdown_field(writer, label, &value)?;
    }
    markdown_field(
        writer,
        "Path fields",
        if options.include_paths {
            "Original paths included"
        } else {
            "Filename only; source root omitted"
        },
    )?;
    markdown_field(
        writer,
        "Evidence details",
        if options.include_details {
            "Included"
        } else {
            "Omitted"
        },
    )?;
    markdown_field(
        writer,
        "Analyst notes",
        if options.include_notes {
            "Included as entered; not automatically redacted"
        } else {
            "Omitted"
        },
    )?;
    if options.include_paths {
        markdown_field(writer, "Source root", &case.source_root)?;
    }
    writer.push("\nReview states and bookmarks are analyst metadata; they do not change the engine's Observed / Linked / Candidate confidence. Exporting this report does not save an edited case.\n\n## Reader coverage\n\n")?;
    if case.modules.is_empty() {
        writer.push("No module coverage was recorded.\n")?;
    }
    for module in &case.modules {
        markdown_field(writer, &module.name, module.state.label())?;
        if options.include_details {
            markdown_field(writer, "Coverage detail", &module.detail)?;
        }
    }
    if options.include_details {
        for warning in &case.warnings {
            markdown_field(writer, "Coverage note", warning)?;
        }
    } else if !case.warnings.is_empty() {
        writer.push("\nCoverage-note text is omitted with evidence details. Review the full case before interpreting these findings.\n")?;
    }
    let observed = selected
        .iter()
        .filter(|finding| finding.confidence == Confidence::Observed)
        .count();
    let linked = selected
        .iter()
        .filter(|finding| finding.confidence == Confidence::Linked)
        .count();
    writer.push(&format!(
        "\n## Selected findings\n\nObserved: {observed}. Linked: {linked}. Candidate: {}.\n",
        selected.len() - observed - linked
    ))?;
    for (index, finding) in selected.iter().enumerate() {
        writer.push(&format!("\n### Finding {}\n\n", index + 1))?;
        let default_review = ReviewAnnotation::default();
        let annotation = review.get(&finding.id).unwrap_or(&default_review);
        for (label, value) in [
            ("ID", finding.id.clone()),
            ("Title", finding.title.clone()),
            ("Category", finding.category.label().to_owned()),
            ("Confidence", finding.confidence.label().to_owned()),
            ("Rule", finding.rule_id.clone()),
            ("Path", path_text(&finding.path, options)),
            (
                "SHA-256",
                finding
                    .sha256
                    .clone()
                    .unwrap_or_else(|| "Not recorded".into()),
            ),
            (
                "Source timestamp",
                if finding.observed_at.is_some() {
                    utc_time(finding.observed_at)
                } else {
                    "Not recorded".into()
                },
            ),
            ("Bytes", finding.bytes.to_string()),
            ("Review status", status_label(annotation.status).to_owned()),
            ("Bookmarked", annotation.bookmarked.to_string()),
        ] {
            markdown_field(writer, label, &value)?;
        }
        if options.include_notes {
            markdown_field(writer, "Analyst note", &annotation.note)?;
        }
        if options.include_details {
            markdown_field(writer, "Origin", &path_text(&finding.origin, options))?;
            markdown_field(writer, "Summary", &finding.summary)?;
            markdown_field(writer, "Locator", &path_text(&finding.locator, options))?;
            for fact in &finding.evidence {
                markdown_field(writer, &format!("Evidence — {}", fact.label), &fact.value)?;
            }
        }
    }
    writer.push("\n---\nGenerated offline by ENGRAM. Line breaks and tabs in Markdown text are shown as ↵ and ⇥. Keep reports private when they contain identifying metadata or analyst notes.\n")
}

fn formula_like(value: &str) -> bool {
    matches!(
        value
            .trim_start_matches(|character: char| {
                character.is_whitespace() || character.is_control() || invisible_format(character)
            })
            .chars()
            .next(),
        Some('=' | '+' | '-' | '@')
    )
}

fn csv_cell(writer: &mut ReportWriter, value: &str) -> Result<(), EngramError> {
    let clean = clean_text(value);
    writer.push("\"")?;
    if formula_like(&clean) {
        writer.push("'")?;
    }
    for character in clean.chars() {
        if character == '"' {
            writer.push("\"\"")?;
        } else {
            let mut bytes = [0; 4];
            writer.push(character.encode_utf8(&mut bytes))?;
        }
    }
    writer.push("\"")
}

fn csv_row(writer: &mut ReportWriter, values: &[String]) -> Result<(), EngramError> {
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            writer.push(",")?;
        }
        csv_cell(writer, value)?;
    }
    writer.push("\r\n")
}

fn csv(
    writer: &mut ReportWriter,
    case: &Case,
    options: &ReportOptions,
    selected: &[&Finding],
    review: &CaseReview,
) -> Result<(), EngramError> {
    // BOM makes the UTF-8 encoding explicit to common spreadsheet applications.
    writer.push("\u{feff}")?;
    let mut header: Vec<String> = [
        "exporter_version",
        "case_id",
        "source_kind",
        "case_created_at_utc",
        "case_completed_at_utc",
        "case_findings",
        "case_warning_count",
        "case_skipped_files",
        "path_fields",
        "details_included",
        "notes_included",
        "finding_id",
        "title",
        "category",
        "confidence",
        "rule_id",
        "path",
        "sha256",
        "observed_at_utc",
        "bytes",
        "review_status",
        "bookmarked",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if options.include_paths {
        header.push("source_root".into());
    }
    if options.include_notes {
        header.push("analyst_note".into());
    }
    if options.include_details {
        header.extend(["origin", "summary", "locator", "evidence"].map(str::to_owned));
    }
    csv_row(writer, &header)?;
    for finding in selected {
        let default_review = ReviewAnnotation::default();
        let annotation = review.get(&finding.id).unwrap_or(&default_review);
        let mut row = vec![
            env!("CARGO_PKG_VERSION").into(),
            case.id.clone(),
            case.source_kind.label().into(),
            utc_time(Some(case.created_at)),
            utc_time(Some(case.completed_at)),
            case.findings.len().to_string(),
            case.warnings.len().to_string(),
            case.stats.skipped_files.to_string(),
            if options.include_paths {
                "original"
            } else {
                "filename_only"
            }
            .into(),
            options.include_details.to_string(),
            options.include_notes.to_string(),
            finding.id.clone(),
            finding.title.clone(),
            finding.category.label().into(),
            finding.confidence.label().into(),
            finding.rule_id.clone(),
            path_text(&finding.path, options),
            finding.sha256.clone().unwrap_or_default(),
            utc_time(finding.observed_at),
            finding.bytes.to_string(),
            status_label(annotation.status).into(),
            annotation.bookmarked.to_string(),
        ];
        if options.include_paths {
            row.push(case.source_root.clone());
        }
        if options.include_notes {
            row.push(annotation.note.clone());
        }
        if options.include_details {
            row.extend([
                path_text(&finding.origin, options),
                finding.summary.clone(),
                path_text(&finding.locator, options),
                finding
                    .evidence
                    .iter()
                    .map(|fact| format!("{}: {}", fact.label, fact.value))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ]);
        }
        csv_row(writer, &row)?;
    }
    Ok(())
}
