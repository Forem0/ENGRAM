use crate::{finding_id, Category, Confidence, Evidence, Finding, MAX_DB_ROWS};
use rusqlite::Connection;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub(crate) struct PhotoAsset {
    pub filename: String,
    pub directory: String,
    pub finding_id: String,
}

#[derive(Default)]
pub(crate) struct ParsedDatabase {
    pub findings: Vec<Finding>,
    pub photos: Vec<PhotoAsset>,
    pub warnings: Vec<String>,
}

pub(crate) fn is_regular_table(connection: &Connection, table: &str) -> bool {
    let Ok(Some(definition)) = connection.query_row(
        "SELECT sql FROM sqlite_schema WHERE type='table' AND name=?1",
        [table],
        |row| row.get::<_, Option<String>>(0),
    ) else {
        return false;
    };
    // A familiar-looking view or virtual table may execute arbitrary expensive
    // expressions. This preview's parsers accept ordinary CREATE TABLE sources.
    let regular_table =
        regex::Regex::new(r"(?i)^\s*CREATE\s+TABLE\b").expect("static schema detector");
    regular_table.is_match(&definition)
}

fn columns(connection: &Connection, table: &str) -> HashSet<String> {
    // `table` is always a literal from this module, never source-controlled text.
    if !is_regular_table(connection, table) {
        return HashSet::new();
    }
    let Ok(mut statement) = connection.prepare(&format!("PRAGMA table_info(\"{table}\")")) else {
        return HashSet::new();
    };
    let Ok(rows) = statement.query_map([], |row| row.get::<_, String>(1)) else {
        return HashSet::new();
    };
    rows.filter_map(Result::ok).collect()
}

fn record(
    path: &str,
    origin: &str,
    category: Category,
    rule: &str,
    locator: String,
    title: String,
    observed_at: Option<i64>,
    summary: &str,
    evidence: Vec<Evidence>,
) -> Finding {
    Finding {
        id: finding_id(&format!("{origin}:{path}"), rule, &locator),
        category,
        title,
        path: path.into(),
        origin: origin.into(),
        summary: summary.into(),
        locator,
        observed_at,
        bytes: 0,
        sha256: None,
        confidence: Confidence::Observed,
        rule_id: rule.into(),
        sensitive: true,
        evidence,
        related_ids: Vec::new(),
    }
}

fn fact(label: &str, value: impl ToString) -> Evidence {
    Evidence {
        label: label.into(),
        value: value.to_string(),
    }
}

fn sane_timestamp(timestamp: i64) -> Option<i64> {
    (0..=4_102_444_800)
        .contains(&timestamp)
        .then_some(timestamp)
}

fn chromium_time(value: i64) -> Option<i64> {
    sane_timestamp(value / 1_000_000 - 11_644_473_600)
}

fn apple_time(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let seconds = if value.abs() > 10_000_000_000.0 {
        value / 1_000_000_000.0
    } else {
        value
    };
    sane_timestamp((seconds + 978_307_200.0) as i64)
}

/// Preserve only the origin of a web visit. URL userinfo, paths, query strings,
/// fragments and page titles can contain credentials or private search terms.
fn web_origin(value: &str) -> Option<String> {
    let (scheme, rest) = value.split_once("://")?;
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?.rsplit('@').next()?;
    if authority.is_empty()
        || authority.len() > 255
        || authority
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
    {
        return None;
    }
    Some(format!("{}://{}", scheme.to_ascii_lowercase(), authority))
}

pub(crate) fn inspect(connection: &Connection, path: &str, origin: &str) -> ParsedDatabase {
    let mut result = ParsedDatabase::default();
    let chromium = columns(connection, "urls");
    let safari_items = columns(connection, "history_items");
    let safari_visits = columns(connection, "history_visits");
    let messages = columns(connection, "message");
    let assets = columns(connection, "ZASSET");

    if ["id", "url", "last_visit_time"]
        .iter()
        .all(|name| chromium.contains(*name))
    {
        parse_chromium(connection, path, origin, &mut result);
    }
    if ["id", "url"]
        .iter()
        .all(|name| safari_items.contains(*name))
        && ["history_item", "visit_time"]
            .iter()
            .all(|name| safari_visits.contains(*name))
    {
        parse_safari(connection, path, origin, &mut result);
    }
    if messages.contains("date") {
        parse_messages(connection, path, origin, &messages, &mut result);
    }
    if assets.contains("ZFILENAME") {
        parse_photos(connection, path, origin, &assets, &mut result);
    }
    if result.findings.is_empty() && result.warnings.is_empty() {
        result.warnings.push(format!("No supported activity or Photos schema was found in {path}; database-file metadata is still available."));
    }
    result
}

fn parse_chromium(connection: &Connection, path: &str, origin: &str, output: &mut ParsedDatabase) {
    let query = format!(
        "SELECT id, url, last_visit_time FROM urls ORDER BY last_visit_time DESC LIMIT {}",
        MAX_DB_ROWS + 1
    );
    let operation = (|| -> rusqlite::Result<()> {
        let mut statement = connection.prepare(&query)?;
        let mut rows = statement.query([])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            if count >= MAX_DB_ROWS {
                output.warnings.push(format!(
                    "Chromium history in {path} was limited to {MAX_DB_ROWS} rows."
                ));
                break;
            }
            count += 1;
            let id: i64 = row.get(0)?;
            let url: String = row.get(1)?;
            let timestamp: Option<i64> = row.get(2).unwrap_or(None);
            let Some(site) = web_origin(&url) else {
                continue;
            };
            output.findings.push(record(path, origin, Category::Browser, "browser.chromium_visit", format!("urls row {id}"), format!("Visit: {site}"), timestamp.and_then(chromium_time), "A visit record is present in this supplied database snapshot. Page titles, paths and query parameters are omitted.", vec![fact("Website origin", site), fact("Parser", "Chromium urls"), fact("Privacy", "URL path, query, fragment and title omitted")]));
        }
        Ok(())
    })();
    if let Err(error) = operation {
        output
            .warnings
            .push(format!("Chromium parser could not finish {path}: {error}"));
    }
}

fn parse_safari(connection: &Connection, path: &str, origin: &str, output: &mut ParsedDatabase) {
    let query = format!("SELECT v.ROWID, i.url, v.visit_time FROM history_visits v JOIN history_items i ON i.id = v.history_item ORDER BY v.visit_time DESC LIMIT {}", MAX_DB_ROWS + 1);
    let operation = (|| -> rusqlite::Result<()> {
        let mut statement = connection.prepare(&query)?;
        let mut rows = statement.query([])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            if count >= MAX_DB_ROWS {
                output.warnings.push(format!(
                    "Safari history in {path} was limited to {MAX_DB_ROWS} rows."
                ));
                break;
            }
            count += 1;
            let id: i64 = row.get(0)?;
            let url: String = row.get(1)?;
            let timestamp: Option<f64> = row.get(2).unwrap_or(None);
            let Some(site) = web_origin(&url) else {
                continue;
            };
            output.findings.push(record(path, origin, Category::Browser, "browser.safari_visit", format!("history_visits row {id}"), format!("Visit: {site}"), timestamp.and_then(apple_time), "A Safari visit row exists in this supplied snapshot. It does not establish current browser state or deleted-history recovery.", vec![fact("Website origin", site), fact("Parser", "Safari history_items + history_visits"), fact("Privacy", "URL path, query, fragment and title omitted")]));
        }
        Ok(())
    })();
    if let Err(error) = operation {
        output
            .warnings
            .push(format!("Safari parser could not finish {path}: {error}"));
    }
}

fn parse_messages(
    connection: &Connection,
    path: &str,
    origin: &str,
    columns: &HashSet<String>,
    output: &mut ParsedDatabase,
) {
    let direction = if columns.contains("is_from_me") {
        "is_from_me"
    } else {
        "NULL"
    };
    let query = format!(
        "SELECT ROWID, date, {direction} FROM message ORDER BY date DESC LIMIT {}",
        MAX_DB_ROWS + 1
    );
    let operation = (|| -> rusqlite::Result<()> {
        let mut statement = connection.prepare(&query)?;
        let mut rows = statement.query([])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            if count >= MAX_DB_ROWS {
                output.warnings.push(format!(
                    "Message metadata in {path} was limited to {MAX_DB_ROWS} rows."
                ));
                break;
            }
            count += 1;
            let id: i64 = row.get(0)?;
            let timestamp: Option<f64> = row.get(1).unwrap_or(None);
            let direction: Option<i64> = row.get(2).unwrap_or(None);
            let label = match direction {
                Some(1) => "Outgoing message record",
                Some(0) => "Incoming message record",
                _ => "Message record",
            };
            output.findings.push(record(path, origin, Category::Message, "message.row_metadata", format!("message row {id}"), label.into(), timestamp.and_then(apple_time), "Message-row metadata is present. Body text, contacts and attachments are not copied into this case. This is not deleted-message recovery.", vec![fact("Parser", "Apple message table metadata"), fact("Content", "Message body and contact details omitted")]));
        }
        Ok(())
    })();
    if let Err(error) = operation {
        output
            .warnings
            .push(format!("Message parser could not finish {path}: {error}"));
    }
}

fn parse_photos(
    connection: &Connection,
    path: &str,
    origin: &str,
    columns: &HashSet<String>,
    output: &mut ParsedDatabase,
) {
    let directory = if columns.contains("ZDIRECTORY") {
        "ZDIRECTORY"
    } else {
        "NULL"
    };
    let trashed = if columns.contains("ZTRASHEDSTATE") {
        "ZTRASHEDSTATE"
    } else {
        "NULL"
    };
    let date = if columns.contains("ZDATECREATED") {
        "ZDATECREATED"
    } else {
        "NULL"
    };
    let query = format!(
        "SELECT ROWID, ZFILENAME, {directory}, {trashed}, {date} FROM ZASSET LIMIT {}",
        MAX_DB_ROWS + 1
    );
    let operation = (|| -> rusqlite::Result<()> {
        let mut statement = connection.prepare(&query)?;
        let mut rows = statement.query([])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            if count >= MAX_DB_ROWS {
                output.warnings.push(format!("Photos catalogue in {path} was limited to {MAX_DB_ROWS} rows; unmatched media remains inconclusive."));
                break;
            }
            count += 1;
            let row_id: i64 = row.get(0)?;
            let filename: Option<String> = row.get(1)?;
            let Some(filename) = filename.filter(|name| name.len() <= 1024) else {
                continue;
            };
            let directory: Option<String> = row
                .get::<_, Option<String>>(2)
                .unwrap_or(None)
                .filter(|value| value.len() <= 4096);
            let trashed: Option<i64> = row.get(3).unwrap_or(None);
            let date: Option<f64> = row.get(4).unwrap_or(None);
            let status = match trashed {
                Some(0) => "Not marked trashed in this snapshot",
                Some(_) => "Trashed-state flag is set in this snapshot",
                None => "Trashed state unavailable",
            };
            let item = record(path, origin, Category::Media, "media.photos_catalogue", format!("ZASSET row {row_id}"), "Photos catalogue entry".into(), date.and_then(apple_time), "An asset row is present in the supplied Photos database. A trash flag is not proof of permanent deletion, and schema interpretation is experimental.", vec![fact("Filename", &filename), fact("Directory", directory.as_deref().unwrap_or("Unavailable")), fact("Catalogue state", status), fact("Parser", "Experimental ZASSET schema")]);
            output.photos.push(PhotoAsset {
                filename,
                directory: directory.unwrap_or_default(),
                finding_id: item.id.clone(),
            });
            output.findings.push(item);
        }
        Ok(())
    })();
    if let Err(error) = operation {
        output
            .warnings
            .push(format!("Photos parser could not finish {path}: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_origins_omit_credentials_and_search_terms() {
        assert_eq!(
            web_origin("https://name:password@example.invalid/private/token?q=secret#fragment")
                .as_deref(),
            Some("https://example.invalid")
        );
        assert_eq!(web_origin("file:///secret/file"), None);
        assert_eq!(web_origin("https://bad host/path"), None);
    }

    #[test]
    fn timestamps_use_each_platform_epoch() {
        assert_eq!(chromium_time(13_370_745_600_000_000), Some(1_726_272_000));
        assert_eq!(apple_time(747_964_800.0), Some(1_726_272_000));
        assert_eq!(apple_time(747_964_800_000_000_000.0), Some(1_726_272_000));
        assert_eq!(apple_time(f64::NAN), None);
    }

    #[test]
    fn parses_rows_without_persisting_private_url_parts_or_message_bodies() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE urls(id INTEGER, url TEXT, last_visit_time INTEGER); INSERT INTO urls VALUES (1,'https://example.invalid/reset?token=do-not-export',13370745600000000); CREATE TABLE message(date INTEGER,is_from_me INTEGER,text TEXT); INSERT INTO message VALUES(747964800000000000,1,'password=never-export-this');").unwrap();
        let parsed = inspect(&connection, "History", "Folder");
        assert_eq!(parsed.findings.len(), 2);
        let json = serde_json::to_string(&parsed.findings).unwrap();
        assert!(!json.contains("do-not-export"));
        assert!(!json.contains("never-export-this"));
    }

    #[test]
    fn a_recursive_view_cannot_impersonate_a_history_table() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE VIEW urls AS WITH RECURSIVE r(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM r) SELECT n AS id, 'https://fake.invalid' AS url, n AS last_visit_time FROM r;").unwrap();
        assert!(!is_regular_table(&connection, "urls"));
        let parsed = inspect(&connection, "History", "Folder");
        assert!(parsed.findings.is_empty());
        assert!(!parsed.warnings.is_empty());
    }

    #[test]
    fn absent_timestamps_are_not_fabricated_as_january_2001() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE history_items(id INTEGER,url TEXT); CREATE TABLE history_visits(history_item INTEGER,visit_time REAL); INSERT INTO history_items VALUES(1,'https://demo.invalid'); INSERT INTO history_visits VALUES(1,NULL); CREATE TABLE message(date INTEGER); INSERT INTO message VALUES(NULL);").unwrap();
        let parsed = inspect(&connection, "History", "Folder");
        assert_eq!(parsed.findings.len(), 2);
        assert!(parsed
            .findings
            .iter()
            .all(|finding| finding.observed_at.is_none()));
    }
}
