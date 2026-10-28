use crate::{CaseReview, EngramError, Finding, ReviewAnnotation, ReviewStatus};
use std::collections::HashSet;

pub const MAX_REVIEW_ANNOTATIONS: usize = 2_048;
pub const MAX_REVIEW_NOTE_BYTES: usize = 4_096;
pub const MAX_REVIEW_TOTAL_NOTE_BYTES: usize = 1024 * 1024;

fn forbidden_control(character: char) -> bool {
    (character <= '\u{001f}' && !matches!(character, '\t' | '\n'))
        || ('\u{007f}'..='\u{009f}').contains(&character)
}

/// Validate analyst metadata against its actual evidence snapshot and return its
/// canonical sparse representation. Limits apply to raw input, before defaults
/// are removed. No source files or paths are opened while reviewing a case.
pub fn validate_review(
    findings: &[Finding],
    review: &CaseReview,
) -> Result<CaseReview, EngramError> {
    if review.len() > MAX_REVIEW_ANNOTATIONS {
        return Err(EngramError::Input(format!(
            "Case review supports at most {MAX_REVIEW_ANNOTATIONS} annotations."
        )));
    }
    let identities: HashSet<&str> = findings.iter().map(|finding| finding.id.as_str()).collect();
    let mut normalized = CaseReview::new();
    let mut total_bytes = 0_usize;
    for (identity, annotation) in review {
        if identity.is_empty()
            || identity.len() > 256
            || identity.chars().any(char::is_control)
            || !identities.contains(identity.as_str())
        {
            return Err(EngramError::Input(
                "Case review contains an invalid or dangling finding identity.".into(),
            ));
        }
        let note_bytes = annotation.note.len();
        if note_bytes > MAX_REVIEW_NOTE_BYTES {
            return Err(EngramError::Input(format!(
                "Each review note must be at most {MAX_REVIEW_NOTE_BYTES} UTF-8 bytes."
            )));
        }
        total_bytes += note_bytes;
        if total_bytes > MAX_REVIEW_TOTAL_NOTE_BYTES {
            return Err(EngramError::Input(
                "Case review notes together must be at most 1 MiB of UTF-8 text.".into(),
            ));
        }
        if annotation.note.chars().any(forbidden_control) {
            return Err(EngramError::Input(
                "Review notes cannot contain control characters other than tabs and newlines."
                    .into(),
            ));
        }
        // Match textarea/JavaScript trim semantics for whitespace-only notes,
        // including the BOM character, while preserving useful note whitespace.
        let note = if annotation
            .note
            .chars()
            .all(|character| character.is_whitespace() || character == '\u{feff}')
        {
            String::new()
        } else {
            annotation.note.clone()
        };
        if annotation.bookmarked || annotation.status != ReviewStatus::Unreviewed || !note.is_empty()
        {
            normalized.insert(
                identity.clone(),
                ReviewAnnotation {
                    bookmarked: annotation.bookmarked,
                    status: annotation.status,
                    note,
                },
            );
        }
    }
    Ok(normalized)
}
