use engram_core::{
    compare_cases, demo_case, read_case, validate_review, write_case, Case, CaseReview,
    ReviewAnnotation, ReviewStatus, MAX_REVIEW_ANNOTATIONS, MAX_REVIEW_NOTE_BYTES,
    MAX_REVIEW_TOTAL_NOTE_BYTES,
};
use serde_json::json;
use std::fs;

fn review_case(count: usize) -> Case {
    let mut case = demo_case();
    let template = case.findings.first().expect("synthetic fixture finding").clone();
    case.findings = (0..count)
        .map(|index| {
            let mut finding = template.clone();
            finding.id = format!("review-finding-{index}");
            finding.related_ids.clear();
            finding
        })
        .collect();
    case.review.clear();
    case
}

fn annotation(note: &str) -> ReviewAnnotation {
    ReviewAnnotation {
        bookmarked: false,
        status: ReviewStatus::Unreviewed,
        note: note.into(),
    }
}

#[test]
fn old_schema_one_cases_need_no_review_field() {
    let case = review_case(1);
    let mut value = serde_json::to_value(&case).unwrap();
    assert!(value.get("review").is_none());
    value.as_object_mut().unwrap().remove("review");
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("old-case.json");
    fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
    let reopened = read_case(&file).unwrap();
    assert!(reopened.review.is_empty());
    assert!(serde_json::to_value(&reopened).unwrap().get("review").is_none());
    assert_eq!(ReviewAnnotation::default(), annotation(""));
}

#[test]
fn canonical_review_discards_only_default_or_whitespace_only_entries() {
    let case = review_case(3);
    let mut review = CaseReview::new();
    review.insert(case.findings[0].id.clone(), ReviewAnnotation::default());
    review.insert(case.findings[1].id.clone(), annotation(" \t\n\u{feff}\u{2003}"));
    review.insert(case.findings[2].id.clone(), annotation("  Meaningful\n\tnote  "));
    let normalized = validate_review(&case.findings, &review).unwrap();
    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[&case.findings[2].id].note, "  Meaningful\n\tnote  ");
    assert_eq!(review.len(), 3, "validation must not mutate the caller's map");
}

#[test]
fn bookmarks_status_and_notes_roundtrip_through_native_case_storage() {
    let mut case = review_case(3);
    case.review.insert(case.findings[0].id.clone(), ReviewAnnotation {
        bookmarked: true,
        status: ReviewStatus::Reviewed,
        note: "Checked the source snapshot.\nRetain this observation.".into(),
    });
    case.review.insert(case.findings[1].id.clone(), ReviewAnnotation {
        bookmarked: false,
        status: ReviewStatus::NeedsReview,
        note: "Compare with the earlier copy.".into(),
    });
    case.review.insert(case.findings[2].id.clone(), ReviewAnnotation::default());
    let expected = validate_review(&case.findings, &case.review).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("reviewed-case.json");
    write_case(&case, &output).unwrap();
    let reopened = read_case(&output).unwrap();
    assert_eq!(reopened.review, expected);
    assert_eq!(case.review.len(), 3, "saving must not mutate the supplied snapshot");
    assert_eq!(reopened.id, case.id);
    assert_eq!(reopened.source_root, case.source_root);
}

#[test]
fn imported_default_annotations_are_normalized_and_omitted_on_resave() {
    let case = review_case(1);
    let mut value = serde_json::to_value(&case).unwrap();
    let mut review = serde_json::Map::new();
    review.insert(case.findings[0].id.clone(), json!({"bookmarked":false,"status":"unreviewed","note":" \t\n"}));
    value["review"] = serde_json::Value::Object(review);
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("defaults.json");
    fs::write(&input, serde_json::to_vec(&value).unwrap()).unwrap();
    let reopened = read_case(&input).unwrap();
    assert!(reopened.review.is_empty());
    let output = directory.path().join("normalized.json");
    write_case(&reopened, &output).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    assert!(saved.get("review").is_none());
}

#[test]
fn dangling_annotations_fail_even_when_the_annotation_is_default() {
    let mut case = review_case(1);
    case.review.insert("not-an-active-finding".into(), ReviewAnnotation::default());
    assert!(validate_review(&case.findings, &case.review).is_err());
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("dangling.json");
    fs::write(&input, serde_json::to_vec(&case).unwrap()).unwrap();
    assert!(read_case(&input).is_err());
    let output = directory.path().join("must-not-create/case.json");
    assert!(write_case(&case, &output).is_err());
    assert!(!directory.path().join("must-not-create").exists());
}

#[test]
fn malformed_review_shapes_are_rejected_instead_of_defaulted() {
    let case = review_case(1);
    let identity = &case.findings[0].id;
    let malformed_annotations = [
        json!(null),
        json!([]),
        json!({}),
        json!({"bookmarked":false,"status":"unreviewed"}),
        json!({"bookmarked":"false","status":"unreviewed","note":""}),
        json!({"bookmarked":false,"status":"unknown","note":""}),
        json!({"bookmarked":false,"status":{"reviewed":null},"note":""}),
        json!({"bookmarked":false,"status":"reviewed","note":42}),
        json!({"bookmarked":false,"status":"reviewed","note":"","extra":true}),
    ];
    for invalid in malformed_annotations {
        let mut value = serde_json::to_value(&case).unwrap();
        let mut review = serde_json::Map::new();
        review.insert(identity.clone(), invalid);
        value["review"] = serde_json::Value::Object(review);
        assert!(serde_json::from_value::<Case>(value).is_err());
    }
    for malformed_map in [json!(null), json!([]), json!(false)] {
        let mut value = serde_json::to_value(&case).unwrap();
        value["review"] = malformed_map;
        assert!(serde_json::from_value::<Case>(value).is_err());
    }
}

#[test]
fn review_limits_count_utf8_bytes_and_raw_input_entries() {
    let case = review_case(MAX_REVIEW_ANNOTATIONS + 1);
    let mut review: CaseReview = case.findings.iter().take(MAX_REVIEW_ANNOTATIONS)
        .map(|finding| (finding.id.clone(), ReviewAnnotation::default())).collect();
    assert!(validate_review(&case.findings, &review).unwrap().is_empty());
    review.insert(case.findings[MAX_REVIEW_ANNOTATIONS].id.clone(), ReviewAnnotation::default());
    assert!(validate_review(&case.findings, &review).is_err());
    let identity = case.findings[0].id.clone();
    let exact = "é".repeat(MAX_REVIEW_NOTE_BYTES / 2);
    assert_eq!(exact.len(), MAX_REVIEW_NOTE_BYTES);
    let review = CaseReview::from([(identity.clone(), annotation(&exact))]);
    assert!(validate_review(&case.findings, &review).is_ok());
    let review = CaseReview::from([(identity, annotation(&format!("{exact}é")))]);
    assert!(validate_review(&case.findings, &review).is_err());
}

#[test]
fn total_review_text_limit_is_enforced_before_normalization() {
    let case = review_case(257);
    let note = "x".repeat(MAX_REVIEW_NOTE_BYTES);
    let mut review: CaseReview = case.findings.iter().take(256)
        .map(|finding| (finding.id.clone(), annotation(&note))).collect();
    assert_eq!(note.len() * review.len(), MAX_REVIEW_TOTAL_NOTE_BYTES);
    assert!(validate_review(&case.findings, &review).is_ok());
    review.insert(case.findings[256].id.clone(), annotation("x"));
    assert!(validate_review(&case.findings, &review).is_err());
    let whitespace = " ".repeat(MAX_REVIEW_NOTE_BYTES);
    let review: CaseReview = case.findings.iter()
        .map(|finding| (finding.id.clone(), annotation(&whitespace))).collect();
    assert!(validate_review(&case.findings, &review).is_err());
}

#[test]
fn notes_reject_controls_but_preserve_tabs_newlines_and_unicode() {
    let case = review_case(1);
    let identity = case.findings[0].id.clone();
    for value in (0..=31_u32).chain(127..=159) {
        if matches!(value, 9 | 10) { continue; }
        let note = format!("note{}end", char::from_u32(value).unwrap());
        let review = CaseReview::from([(identity.clone(), annotation(&note))]);
        assert!(validate_review(&case.findings, &review).is_err(), "accepted control {value}");
    }
    let note = "Проверено 🧠\n\tEvidence remains separate.";
    let review = CaseReview::from([(identity.clone(), annotation(note))]);
    assert_eq!(validate_review(&case.findings, &review).unwrap()[&identity].note, note);
}

#[test]
fn review_identities_reject_all_controls_even_when_the_finding_exists() {
    let mut case = review_case(1);
    for identity in ["finding\tidentity", "finding\nidentity", "finding\u{0085}identity"] {
        case.findings[0].id = identity.into();
        let review = CaseReview::from([(identity.into(), annotation("A valid note."))]);
        assert!(validate_review(&case.findings, &review).is_err());
    }
}

#[test]
fn prototype_like_finding_identities_remain_plain_data() {
    let mut case = review_case(3);
    let mut review = CaseReview::new();
    for (finding, identity) in case.findings.iter_mut().zip(["__proto__", "constructor", "prototype"]) {
        finding.id = identity.into();
        review.insert(identity.into(), annotation("Analyst metadata."));
    }
    assert_eq!(validate_review(&case.findings, &review).unwrap(), review);
}

#[test]
fn annotation_only_edits_never_change_the_evidence_diff() {
    let before = review_case(2);
    let mut after = before.clone();
    after.review.insert(after.findings[0].id.clone(), ReviewAnnotation {
        bookmarked: true,
        status: ReviewStatus::Reviewed,
        note: "Analyst note, not source evidence.".into(),
    });
    let difference = compare_cases(&before, &after);
    assert!(difference.added.is_empty());
    assert!(difference.removed.is_empty());
    assert!(difference.changed.is_empty());
    assert_eq!(difference.unchanged.len(), before.findings.len());
}
