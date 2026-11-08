use regex::Regex;
use std::sync::OnceLock;

fn patterns() -> &'static Vec<Regex> {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        let raw = [
            r"(?i)(aws_access_key_id|AKIA[0-9A-Z]{16})",
            r"ghp_[A-Za-z0-9]{20,}",
            r"gho_[A-Za-z0-9]{20,}",
            r"github_pat_[A-Za-z0-9_]{20,}",
            r"sk-[A-Za-z0-9]{20,}",
            r"sk_live_[A-Za-z0-9]{16,}",
            r"xox[baprs]-[A-Za-z0-9-]{10,}",
            r"(?i)-----BEGIN (RSA |EC |OPENSSH |PGP )?PRIVATE KEY-----",
            r"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
            r#"(?i)(api[_-]?key|access[_-]?token|secret[_-]?key)\s*[:=]\s*["']?[A-Za-z0-9_\-]{16,}["']?"#,
            r#"(?i)password\s*[:=]\s*["']?\S{8,}["']?"#,
            r#"(?i)passwd\s*[:=]\s*["']?\S{8,}["']?"#,
            r"Bearer\s+[A-Za-z0-9_\-\.]{20,}",
            r#"(?i)(client_secret)\s*[:=]\s*["']?\S{8,}["']?"#,
        ];
        raw.iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect()
    })
}

pub fn is_sensitive(text: &str) -> bool {
    if text.len() > 1_000_000 {
        return false;
    }
    patterns().iter().any(|re| re.is_match(text))
}

pub fn redact_placeholder() -> &'static str {
    "[Sensitive content withheld]"
}
