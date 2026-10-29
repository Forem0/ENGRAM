use engram_core::{
    demo_case, render_report, write_report, Case, Evidence, ReportFormat, ReportOptions,
    ReviewAnnotation, ReviewStatus, MAX_REPORT_BYTES, MAX_REPORT_FINDINGS,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;

fn report_case(count: usize) -> Case {
    let mut case = demo_case();
    let template = case.findings.first().unwrap().clone();
    case.findings = (0..count)
        .map(|index| {
            let mut finding = template.clone();
            finding.id = format!("report-finding-{index}");
            finding.title = format!("Selected observation {index}");
            finding.path = format!("private-parent/nested/artifact-{index}.txt");
            finding.origin = "private-origin-parent/origin.txt".into();
            finding.locator = "private-locator-parent/locator.txt".into();
            finding.summary = "DETAILS_PRIVATE summary".into();
            finding.evidence = vec![Evidence {
                label: "Recorded observation".into(),
                value: "DETAILS_PRIVATE evidence".into(),
            }];
            finding.related_ids.clear();
            finding
        })
        .collect();
    case.review.clear();
    case.source_root = "synthetic://PRIVATE_SOURCE_ROOT".into();
    case.created_at = 1_760_000_000;
    case.completed_at = 1_760_000_001;
    case
}

fn options(case: &Case, format: ReportFormat) -> ReportOptions {
    ReportOptions {
        format,
        finding_ids: case.findings.iter().map(|finding| finding.id.clone()).collect(),
        include_notes: false,
        include_paths: false,
        include_details: false,
    }
}

#[test]
fn report_options_fail_closed_and_default_optional_content_to_omitted() {
    let value = json!({"format":"markdown","finding_ids":["one"]});
    let parsed: ReportOptions = serde_json::from_value(value).unwrap();
    assert!(!parsed.include_notes && !parsed.include_paths && !parsed.include_details);
    for invalid in [
        json!({"format":"html","finding_ids":["one"]}),
        json!({"format":"csv","finding_ids":["one"],"content":"untrusted"}),
        json!({"format":"csv","finding_ids":["one"],"include_notes":"yes"}),
        json!({"format":"csv","finding_ids":"one"}),
    ] {
        assert!(serde_json::from_value::<ReportOptions>(invalid).is_err());
    }
}

#[test]
fn default_exports_omit_notes_details_parent_paths_and_source_root() {
    let mut case = report_case(1);
    case.review.insert(case.findings[0].id.clone(), ReviewAnnotation {
        bookmarked: true,
        status: ReviewStatus::NeedsReview,
        note: "ANALYST_PRIVATE note".into(),
    });
    for format in [ReportFormat::Markdown, ReportFormat::Csv] {
        let report = render_report(&case, &options(&case, format)).unwrap();
        assert_eq!(report.finding_count, 1);
        assert!(report.content.contains("artifact"));
        for absent in ["ANALYST_PRIVATE", "DETAILS_PRIVATE", "private-parent", "private-origin-parent", "private-locator-parent", "PRIVATE_SOURCE_ROOT"] {
            assert!(!report.content.contains(absent), "unexpected field: {absent}");
        }
        assert!(report.content.contains(match format {
            ReportFormat::Markdown => "needs\\_review",
            ReportFormat::Csv => "needs_review",
        }));
        assert!(report.content.contains("true"));
    }
}

#[test]
fn explicit_content_options_include_current_notes_and_original_path_fields() {
    let mut case = report_case(1);
    case.review.insert(case.findings[0].id.clone(), ReviewAnnotation {
        bookmarked: false,
        status: ReviewStatus::Reviewed,
        note: "ANALYST_PRIVATE note".into(),
    });
    let mut selected = options(&case, ReportFormat::Csv);
    selected.include_notes = true;
    selected.include_paths = true;
    selected.include_details = true;
    let report = render_report(&case, &selected).unwrap();
    for present in ["ANALYST_PRIVATE", "DETAILS_PRIVATE", "private-parent/nested/", "private-origin-parent/origin.txt", "private-locator-parent/locator.txt", "synthetic://PRIVATE_SOURCE_ROOT"] {
        assert!(report.content.contains(present), "missing opted-in field: {present}");
    }
}

#[test]
fn arbitrary_inert_source_roots_cannot_rewrite_identity_hash_labels_or_notes() {
    let mut case = report_case(1);
    case.id = "case-d1".into();
    case.findings[0].id = "finding-d1".into();
    case.findings[0].confidence = engram_core::Confidence::Observed;
    case.findings[0].sha256 = Some("d1".repeat(32));
    case.review.insert("finding-d1".into(), ReviewAnnotation {
        bookmarked: true,
        status: ReviewStatus::Reviewed,
        note: "Analyst d1: preserve 1 and Observed exactly.".into(),
    });
    for format in [ReportFormat::Markdown, ReportFormat::Csv] {
        let mut selected = options(&case, format);
        selected.include_notes = true;
        let expected = render_report(&case, &selected).unwrap();
        for root in ["Observed", "d", "1", "case", "true"] {
            case.source_root = root.into();
            let actual = render_report(&case, &selected).unwrap();
            assert_eq!(actual.content, expected.content, "source root changed report: {root}");
            assert_eq!(actual.sha256, expected.sha256);
        }
    }
}

#[test]
fn unicode_format_characters_do_not_collapse_identities_or_rewrite_opted_in_notes() {
    let mut case = report_case(2);
    case.id = "case\u{200b}identity".into();
    case.findings[0].id = "f\u{200b}a".into();
    case.findings[1].id = "f a".into();
    let note = "Analyst 👩‍💻 note with a zero\u{200b}width separator.";
    case.review.insert(case.findings[0].id.clone(), ReviewAnnotation {
        bookmarked: true,
        status: ReviewStatus::Reviewed,
        note: note.into(),
    });
    let mut selected = options(&case, ReportFormat::Csv);
    selected.include_notes = true;
    let report = render_report(&case, &selected).unwrap();
    assert!(report.content.contains("\"f\u{200b}a\""));
    assert!(report.content.contains("\"f a\""));
    assert!(report.content.contains("\"case\u{200b}identity\""));
    assert!(report.content.contains(note));
    selected.format = ReportFormat::Markdown;
    let report = render_report(&case, &selected).unwrap();
    assert!(report.content.contains("**ID:** f\u{200b}a\n"));
    assert!(report.content.contains("**ID:** f a\n"));
    assert!(report.content.contains("case\u{200b}identity"));
    assert!(report.content.contains("👩‍💻"));
}

#[test]
fn csv_rejects_formula_like_identities_instead_of_collapsing_distinct_references() {
    let mut case = report_case(2);
    case.findings[0].id = "=x".into();
    case.findings[1].id = "'=x".into();
    let mut selected = options(&case, ReportFormat::Csv);
    let error = render_report(&case, &selected).unwrap_err().to_string();
    assert!(error.contains("Choose Markdown"));
    selected.format = ReportFormat::Markdown;
    let report = render_report(&case, &selected).unwrap();
    assert!(report.content.contains("**ID:** \\=x\n"));
    assert!(report.content.contains("**ID:** \\'\\=x\n"));
    selected.format = ReportFormat::Csv;
    selected.finding_ids = vec![case.findings[1].id.clone()];
    assert!(render_report(&case, &selected).is_ok(), "unselected identities are not exported");
    for identity in ["=case", " +case", "\u{feff}\u{200b}@case", "\u{202e}-case"] {
        case.id = identity.into();
        assert!(render_report(&case, &selected).is_err());
    }
    case.id = "ordinary-case".into();
    case.findings[1].rule_id = "=rule".into();
    assert!(render_report(&case, &selected).is_err());
}

#[test]
fn imported_windows_and_posix_paths_become_filenames_on_every_host() {
    let mut case = report_case(2);
    case.findings[0].path = "C:\\private-windows-parent\\child\\windows.txt".into();
    case.findings[1].path = "/private-posix-parent/child/posix.txt".into();
    let mut selected = options(&case, ReportFormat::Csv);
    selected.include_details = true;
    let report = render_report(&case, &selected).unwrap();
    assert!(report.content.contains("\"windows.txt\""));
    assert!(report.content.contains("\"posix.txt\""));
    assert!(report.content.contains("\"origin.txt\""));
    assert!(report.content.contains("\"locator.txt\""));
    assert!(!report.content.contains("private-windows-parent"));
    assert!(!report.content.contains("private-posix-parent"));
    assert!(!report.content.contains("private-origin-parent"));
    assert!(!report.content.contains("private-locator-parent"));
}

#[test]
fn reports_preserve_engine_order_and_are_deterministic_for_the_same_snapshot() {
    let case = report_case(3);
    let mut selected = options(&case, ReportFormat::Csv);
    selected.finding_ids = vec![case.findings[2].id.clone(), case.findings[0].id.clone()];
    let first = render_report(&case, &selected).unwrap();
    let second = render_report(&case, &selected).unwrap();
    assert_eq!(first.content, second.content);
    assert_eq!(first.sha256, second.sha256);
    assert_eq!(first.bytes, first.content.len());
    assert_eq!(first.sha256, format!("{:x}", Sha256::digest(first.content.as_bytes())));
    assert!(first.content.find("Selected observation 0").unwrap() < first.content.find("Selected observation 2").unwrap());
    assert!(!first.content.contains("Selected observation 1"));
    assert_eq!(first.finding_count, 2);
}

#[test]
fn invalid_selection_never_becomes_an_all_findings_export() {
    let case = report_case(2);
    for ids in [
        Vec::new(),
        vec!["missing".into()],
        vec![case.findings[0].id.clone(), case.findings[0].id.clone()],
        vec!["\ncontrol".into()],
        vec!["a".repeat(257)],
        vec![case.findings[0].id.clone(); MAX_REPORT_FINDINGS + 1],
    ] {
        let mut selected = options(&case, ReportFormat::Markdown);
        selected.finding_ids = ids;
        assert!(render_report(&case, &selected).is_err());
    }
}

#[test]
fn direct_case_callers_are_subject_to_shape_review_and_identity_validation() {
    let original = report_case(2);
    let selected = options(&original, ReportFormat::Markdown);
    let mut invalid = original.clone();
    invalid.schema_version = 2;
    assert!(render_report(&invalid, &selected).is_err());
    invalid = original.clone();
    invalid.findings[1].id = invalid.findings[0].id.clone();
    assert!(render_report(&invalid, &selected).is_err());
    invalid = original.clone();
    invalid.findings[0].sha256 = Some("not-a-hash".into());
    assert!(render_report(&invalid, &selected).is_err());
    invalid = original.clone();
    invalid.id = "control\0case".into();
    assert!(render_report(&invalid, &selected).is_err());
    invalid = original.clone();
    invalid.findings[1].id = "control\0unselected".into();
    let mut one_selected = selected.clone();
    one_selected.finding_ids.truncate(1);
    assert!(render_report(&invalid, &one_selected).is_err());
    invalid = original.clone();
    invalid.review.insert("missing".into(), ReviewAnnotation::default());
    assert!(render_report(&invalid, &selected).is_err());
    invalid = original.clone();
    invalid.review.insert(invalid.findings[0].id.clone(), ReviewAnnotation {
        note: "invalid\0note".into(), ..ReviewAnnotation::default()
    });
    assert!(render_report(&invalid, &selected).is_err());
}

#[test]
fn markdown_values_cannot_create_html_links_fences_or_extra_headings() {
    let mut case = report_case(1);
    case.findings[0].title = "<script>alert(1)</script>\n# forged\n![pixel](https://example.test/a) ```".into();
    case.findings[0].summary = "https://example.test/x?click=yes".into();
    let mut selected = options(&case, ReportFormat::Markdown);
    selected.include_details = true;
    let report = render_report(&case, &selected).unwrap();
    assert!(report.content.contains("&lt;script&gt;"));
    assert!(!report.content.contains("<script>"));
    assert!(!report.content.contains("\n# forged"));
    assert!(!report.content.contains("![pixel]"));
    assert!(!report.content.contains("](https"));
    assert!(!report.content.contains("```"));
    assert!(!report.content.contains("https://example.test"));
    assert!(report.content.contains("https\\:\\/\\/example\\.test"));
}

#[test]
fn csv_is_utf8_rfc4180_and_neutralizes_formula_and_invisible_prefixes() {
    let mut case = report_case(1);
    let selected = options(&case, ReportFormat::Csv);
    for payload in ["=SUM(1,2)", "+SUM(1,2)", "-SUM(1,2)", "@SUM(1,2)", " \t=SUM(1,2)", "\r\n+SUM(1,2)", "\u{feff}\u{200b}=SUM(1,2)", "\u{202e}=SUM(1,2)", "\0=SUM(1,2)"] {
        case.findings[0].title = payload.into();
        let report = render_report(&case, &selected).unwrap();
        assert!(report.content.starts_with('\u{feff}'));
        assert!(report.content.ends_with("\r\n"));
        assert!(report.content.contains(",\"'"), "unprotected formula: {payload:?}");
        assert!(!report.content.contains('\0'));
    }
    case.findings[0].title = "Quoted \"value\", next\nline".into();
    let report = render_report(&case, &selected).unwrap();
    assert!(report.content.contains("\"Quoted \"\"value\"\", next\nline\""));
}

#[test]
fn selected_count_limit_accepts_its_exact_boundary() {
    let case = report_case(MAX_REPORT_FINDINGS);
    let report = render_report(&case, &options(&case, ReportFormat::Csv)).unwrap();
    assert_eq!(report.finding_count, MAX_REPORT_FINDINGS);
    assert!(report.bytes <= MAX_REPORT_BYTES);
}

#[test]
fn oversized_render_fails_before_creating_any_output_directory() {
    let mut case = report_case(5);
    for finding in &mut case.findings {
        finding.evidence = (0..100).map(|index| Evidence {
            label: format!("Fact {index}"),
            value: "&".repeat(8192),
        }).collect();
    }
    let mut selected = options(&case, ReportFormat::Markdown);
    selected.include_details = true;
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("must-not-exist/report.md");
    let result = write_report(&case, &selected, &output);
    assert!(result.unwrap_err().to_string().contains("16 MiB"));
    assert!(!output.parent().unwrap().exists());
}

#[test]
fn derived_report_is_written_once_with_exact_receipt_and_without_source_reads() {
    let mut case = report_case(1);
    // This imported root must remain inert, even on Windows where it is a UNC path.
    case.source_root = "\\\\unreachable.invalid\\not-a-real-share\\source".into();
    case.findings[0].path = "never-open-this-file/artifact.txt".into();
    let selected = options(&case, ReportFormat::Csv);
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("report.csv");
    let rendered = render_report(&case, &selected).unwrap();
    let receipt = write_report(&case, &selected, &output).unwrap();
    let bytes = fs::read(&output).unwrap();
    assert_eq!(bytes, rendered.content.as_bytes());
    assert_eq!(receipt.sha256, rendered.sha256);
    assert_eq!(receipt.bytes, bytes.len());
    assert_eq!(receipt.finding_count, 1);
    assert_eq!(receipt.format, ReportFormat::Csv);
    assert!(write_report(&case, &selected, &output).is_err());
    assert_eq!(fs::read(&output).unwrap(), bytes);
}

#[test]
fn output_inside_source_traversal_and_control_names_are_rejected_without_writes() {
    let mut case = report_case(1);
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    fs::create_dir(&source).unwrap();
    let evidence = source.join("evidence.txt");
    fs::write(&evidence, b"synthetic evidence must remain unchanged").unwrap();
    case.source_root = source.canonicalize().unwrap().to_str().unwrap().into();
    let selected = options(&case, ReportFormat::Markdown);
    for output in [
        source.join("report.md"),
        directory.path().join("source/../traversal.md"),
        directory.path().join("newline\nreport.md"),
        directory.path().join("control\u{0085}report.md"),
        directory.path().join("x".repeat(32_769)),
    ] {
        assert!(write_report(&case, &selected, &output).is_err());
        assert!(!output.exists());
    }
    assert_eq!(fs::read(&evidence).unwrap(), b"synthetic evidence must remain unchanged");
}

#[cfg(unix)]
#[test]
fn output_aliases_cannot_hide_a_write_inside_the_source() {
    let mut case = report_case(1);
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    fs::create_dir(&source).unwrap();
    case.source_root = source.canonicalize().unwrap().to_str().unwrap().into();
    let alias = directory.path().join("alias");
    std::os::unix::fs::symlink(&source, &alias).unwrap();
    let selected = options(&case, ReportFormat::Markdown);
    assert!(write_report(&case, &selected, &alias.join("report.md")).is_err());
    assert!(!source.join("report.md").exists());
}

#[cfg(unix)]
#[test]
fn existing_output_symlinks_and_non_unicode_destinations_are_rejected() {
    use std::os::unix::ffi::OsStringExt;
    let case = report_case(1);
    let selected = options(&case, ReportFormat::Csv);
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("retained.txt");
    fs::write(&target, b"retain this synthetic file").unwrap();
    let alias = directory.path().join("report.csv");
    std::os::unix::fs::symlink(&target, &alias).unwrap();
    assert!(write_report(&case, &selected, &alias).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"retain this synthetic file");
    let invalid = directory.path().join(std::ffi::OsString::from_vec(vec![0xff]));
    assert!(write_report(&case, &selected, &invalid).is_err());
    assert!(!invalid.exists());
}
