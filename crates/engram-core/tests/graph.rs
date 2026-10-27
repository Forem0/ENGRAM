use engram_core::{demo_case, scan_path, ScanOptions};
use std::fs;

#[test]
fn synthetic_graph_and_inspector_connections_agree_in_both_directions() {
    let case = demo_case();
    assert!(case
        .findings
        .iter()
        .any(|finding| !finding.related_ids.is_empty()));
    for finding in &case.findings {
        for related in &finding.related_ids {
            let neighbour = case
                .findings
                .iter()
                .find(|candidate| &candidate.id == related)
                .unwrap();
            assert!(neighbour.related_ids.contains(&finding.id));
        }
    }
}

#[test]
fn large_duplicate_groups_have_bounded_reciprocal_connections() {
    let source = tempfile::tempdir().unwrap();
    for index in 0..40 {
        fs::write(
            source.path().join(format!("copy-{index}.txt")),
            "identical content",
        )
        .unwrap();
    }
    let case = scan_path(source.path(), &ScanOptions::default()).unwrap();
    for finding in &case.findings {
        assert!(finding.related_ids.len() <= 16);
        for identity in &finding.related_ids {
            assert!(case
                .findings
                .iter()
                .find(|other| &other.id == identity)
                .unwrap()
                .related_ids
                .contains(&finding.id));
        }
    }
}
