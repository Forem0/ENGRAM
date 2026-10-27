use hmac::{Hmac, Mac};
use regex::Regex;
use sha2::Sha256;
use std::sync::OnceLock;

/// The raw value deliberately never leaves this module. Only a keyed, case-local
/// matching identifier and detector metadata reach the rest of the application.
pub(crate) struct SecretCandidate {
    pub line: usize,
    pub column: usize,
    pub kind: &'static str,
    pub rule: &'static str,
    pub match_id: String,
}

struct Detector {
    expression: Regex,
    kind: &'static str,
    rule: &'static str,
    capture: usize,
}

fn detectors() -> &'static [Detector] {
    static DETECTORS: OnceLock<Vec<Detector>> = OnceLock::new();
    DETECTORS.get_or_init(|| {
        vec![
            Detector {
                expression: Regex::new(r#"(?im)\b(?:password|passwd|pwd|api[_-]?key|access[_-]?token|auth[_-]?token|client[_-]?secret|secret[_-]?key|token)\b[ \t]*[=:][ \t]*["']?([^\s"'`,;<>}{]{6,256})"#).expect("static assignment detector"),
                kind: "Credential assignment", rule: "secret.assignment", capture: 1,
            },
            Detector {
                expression: Regex::new(r"\b(?:gh[pousr]_[A-Za-z0-9_]{20,255}|github_pat_[A-Za-z0-9_]{20,255})\b").expect("static GitHub detector"),
                kind: "Token-shaped string", rule: "secret.token_shape", capture: 0,
            },
            Detector {
                expression: Regex::new(r"\bAKIA[0-9A-Z]{16}\b").expect("static access-key detector"),
                kind: "Access-key-shaped string", rule: "secret.access_key_shape", capture: 0,
            },
            Detector {
                expression: Regex::new(r"\beyJ[A-Za-z0-9_-]{9,1024}\.[A-Za-z0-9_-]{12,2048}\.[A-Za-z0-9_-]{12,2048}\b").expect("static JWT detector"),
                kind: "JWT-shaped string", rule: "secret.jwt_shape", capture: 0,
            },
            Detector {
                expression: Regex::new(r"(?s)-----BEGIN (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----[\s\S]{1,8192}?-----END (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----").expect("static private-key detector"),
                kind: "Private-key block", rule: "secret.private_key_block", capture: 0,
            },
        ]
    })
}

fn is_placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("${")
        || lower.starts_with("your_")
        || lower.starts_with("your-")
        || lower.starts_with("example")
        || lower.starts_with("replace_")
        || matches!(
            lower.as_str(),
            "changeme" | "change_me" | "redacted" | "undefined" | "password"
        )
        || value
            .bytes()
            .all(|c| matches!(c, b'*' | b'x' | b'X' | b'-'))
}

pub(crate) fn scan(text: &str, case_key: &[u8; 32]) -> Vec<SecretCandidate> {
    let mut found = Vec::new();
    let mut ranges = Vec::<(usize, usize)>::new();
    for detector in detectors() {
        for captures in detector.expression.captures_iter(text) {
            if found.len() >= 64 {
                return found;
            }
            let Some(value) = captures.get(detector.capture) else {
                continue;
            };
            if is_placeholder(value.as_str())
                || ranges
                    .iter()
                    .any(|(start, end)| value.start() < *end && value.end() > *start)
            {
                continue;
            }
            let mut mac =
                Hmac::<Sha256>::new_from_slice(case_key).expect("HMAC accepts a 32-byte key");
            mac.update(value.as_str().as_bytes());
            let match_id = format!("{:x}", mac.finalize().into_bytes());
            found.push(SecretCandidate {
                line: text[..value.start()]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    + 1,
                column: value.start()
                    - text[..value.start()]
                        .rfind('\n')
                        .map(|index| index + 1)
                        .unwrap_or(0)
                    + 1,
                kind: detector.kind,
                rule: detector.rule,
                match_id,
            });
            ranges.push((value.start(), value.end()));
        }
    }
    found.sort_by_key(|candidate| candidate.line);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_and_links_values_only_within_a_case() {
        let input = "password=synthetic-secret-value\napi_key=synthetic-secret-value\n";
        let one = scan(input, &[7; 32]);
        let other = scan(input, &[8; 32]);
        assert_eq!(one.len(), 2);
        assert_eq!(one[0].line, 1);
        assert_eq!(one[0].match_id, one[1].match_id);
        assert_ne!(one[0].match_id, other[0].match_id);
        assert!(!one[0].match_id.contains("synthetic-secret-value"));
    }

    #[test]
    fn skips_placeholders_and_caps_noisy_files() {
        assert!(scan("password=changeme\ntoken=your_token_here\n", &[1; 32]).is_empty());
        let noisy = "password=synthetic-only-123\n".repeat(500);
        assert_eq!(scan(&noisy, &[1; 32]).len(), 64);
    }

    #[test]
    fn token_inside_assignment_is_not_counted_twice() {
        let result = scan(
            "token=ghp_ENGRAM_SYNTHETIC_NOT_A_REAL_KEY_000000\n",
            &[2; 32],
        );
        assert_eq!(result.len(), 1);
    }
}
