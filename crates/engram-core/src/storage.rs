use crate::{Case, CaseDiff, EngramError, MAX_FINDINGS};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const MAX_CASE_BYTES: u64 = 64 * 1024 * 1024;

pub fn write_case(case: &Case, destination: &Path) -> Result<(), EngramError> {
    let destination = validate_destination(case, destination)?;
    let parent = destination
        .parent()
        .ok_or_else(|| EngramError::Input("Choose a case filename.".into()))?;
    fs::create_dir_all(parent)?;
    // Resolve the now-existing parent a second time before creating the output.
    validate_destination(case, &destination)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temporary, case)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(&destination)
        .map_err(|error| EngramError::Io(error.error))?;
    Ok(())
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

fn validate_destination(case: &Case, destination: &Path) -> Result<PathBuf, EngramError> {
    if destination
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(EngramError::Input("Parent traversal (..) is not supported in case destinations; choose a direct output path.".into()));
    }
    let destination = if destination.is_absolute() {
        destination.to_owned()
    } else {
        std::env::current_dir()?.join(destination)
    };
    if destination.file_name().is_none() {
        return Err(EngramError::Input("Choose a case filename.".into()));
    }
    if destination.symlink_metadata().is_ok() {
        return Err(EngramError::Input(
            "Destination already exists; ENGRAM never replaces an existing file.".into(),
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| EngramError::Input("Choose a case filename.".into()))?;
    let mut nearest_existing = None;
    for ancestor in parent.ancestors() {
        match ancestor.symlink_metadata() {
            Ok(_) => {
                // Resolve an existing parent once and use its canonical path for
                // subsequent writes. This supports macOS /var and user aliases
                // without letting an alias hide an output inside the source.
                if !fs::metadata(ancestor)?.is_dir() {
                    return Err(EngramError::Input(
                        "Case output parent is not a directory.".into(),
                    ));
                }
                nearest_existing = Some(ancestor);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let existing = nearest_existing
        .ok_or_else(|| EngramError::Input("Cannot resolve the case output directory.".into()))?;
    let resolved_parent = existing.canonicalize()?.join(
        parent
            .strip_prefix(existing)
            .map_err(|_| EngramError::Input("Cannot resolve the output parent.".into()))?,
    );
    // scan_path stores a canonical root. An imported source_root is inert
    // metadata: resolving it could contact a UNC endpoint without user selection.
    let source = Path::new(&case.source_root);
    if source.is_absolute() && resolved_parent.starts_with(source) {
        return Err(EngramError::Input(
            "Save the case outside the source directory; source evidence is never modified.".into(),
        ));
    }
    Ok(resolved_parent.join(
        destination
            .file_name()
            .ok_or_else(|| EngramError::Input("Choose a case filename.".into()))?,
    ))
}

pub fn read_case(path: &Path) -> Result<Case, EngramError> {
    let metadata = path.symlink_metadata()?;
    if !metadata.is_file() || is_link(&metadata) || metadata.len() > MAX_CASE_BYTES {
        return Err(EngramError::Input(
            "Choose a regular ENGRAM case file smaller than 64 MiB.".into(),
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_CASE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_CASE_BYTES {
        return Err(EngramError::Input("Case size limit exceeded.".into()));
    }
    let case: Case = serde_json::from_slice(&bytes)?;
    if case.schema_version != 1 {
        return Err(EngramError::Input(format!(
            "Unsupported case schema {}. This build supports schema 1.",
            case.schema_version
        )));
    }
    if case.findings.len() > MAX_FINDINGS || case.events.len() > MAX_FINDINGS {
        return Err(EngramError::Input(
            "Case contains more records than this application supports.".into(),
        ));
    }
    if case.id.is_empty()
        || case.id.len() > 256
        || case.title.len() > 1024
        || case.source_root.len() > 16_384
        || case.warnings.len() > 512
        || case.warnings.iter().any(|value| value.len() > 8192)
        || case.modules.len() > 64
        || case
            .modules
            .iter()
            .any(|module| module.name.len() > 256 || module.detail.len() > 8192)
        || case.events.iter().any(|event| event.message.len() > 8192)
        || case.findings.iter().any(|finding| {
            finding.id.is_empty()
                || finding.id.len() > 256
                || finding.path.len() > 16_384
                || finding.title.len() > 2048
                || finding.origin.len() > 1024
                || finding.summary.len() > 8192
                || finding.locator.len() > 1024
                || finding.rule_id.len() > 256
                || finding.evidence.len() > 100
                || finding
                    .evidence
                    .iter()
                    .any(|fact| fact.label.len() > 256 || fact.value.len() > 8192)
                || finding.related_ids.len() > 128
                || finding
                    .related_ids
                    .iter()
                    .any(|id| id.is_empty() || id.len() > 256)
                || finding.sha256.as_ref().is_some_and(|digest| {
                    digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
        })
    {
        return Err(EngramError::Input(
            "Case record exceeds the supported shape limits.".into(),
        ));
    }
    let mut identities = HashSet::new();
    if case
        .findings
        .iter()
        .any(|finding| !identities.insert(finding.id.clone()))
    {
        return Err(EngramError::Input(
            "Case contains duplicate finding identities.".into(),
        ));
    }
    if case.findings.iter().any(|finding| {
        finding
            .related_ids
            .iter()
            .any(|id| id == &finding.id || !identities.contains(id.as_str()))
    }) {
        return Err(EngramError::Input(
            "Case contains a dangling or self-referencing evidence connection.".into(),
        ));
    }
    Ok(case)
}

/// Compare exported evidence and metadata. Case-local secret HMACs are deliberately
/// excluded: they cannot establish secret continuity across independent scans.
pub fn compare_cases(before: &Case, after: &Case) -> CaseDiff {
    let previous: BTreeMap<_, _> = before
        .findings
        .iter()
        .map(|finding| (&finding.id, finding))
        .collect();
    let current: BTreeMap<_, _> = after
        .findings
        .iter()
        .map(|finding| (&finding.id, finding))
        .collect();
    let mut difference = CaseDiff::default();
    for (id, finding) in &current {
        match previous.get(id) {
            None => difference.added.push((*id).clone()),
            Some(old) => {
                let same = old.category == finding.category
                    && old.title == finding.title
                    && old.path == finding.path
                    && old.bytes == finding.bytes
                    && old.observed_at == finding.observed_at
                    && old.sha256 == finding.sha256
                    && old.confidence == finding.confidence
                    && old.rule_id == finding.rule_id
                    && old.summary == finding.summary
                    && old.origin == finding.origin
                    && old.locator == finding.locator
                    && old.sensitive == finding.sensitive
                    && comparable_evidence(old) == comparable_evidence(finding)
                    && comparable_relations(old) == comparable_relations(finding);
                if same {
                    difference.unchanged.push((*id).clone());
                } else {
                    difference.changed.push((*id).clone());
                }
            }
        }
    }
    for id in previous.keys() {
        if !current.contains_key(id) {
            difference.removed.push((*id).clone());
        }
    }
    difference
}

fn comparable_evidence(finding: &crate::Finding) -> Vec<(&str, &str)> {
    let mut facts: Vec<_> = finding
        .evidence
        .iter()
        .filter(|fact| fact.label != "Case-local match")
        .map(|fact| (fact.label.as_str(), fact.value.as_str()))
        .collect();
    facts.sort_unstable();
    facts
}

fn comparable_relations(finding: &crate::Finding) -> Vec<&str> {
    let mut identities: Vec<_> = finding.related_ids.iter().map(String::as_str).collect();
    identities.sort_unstable();
    identities.dedup();
    identities
}
