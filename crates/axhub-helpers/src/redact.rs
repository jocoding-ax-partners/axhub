use regex::Regex;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

static BIDI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\u{202A}-\u{202E}\u{2066}-\u{2069}]").unwrap());
static ZW_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\u{200B}-\u{200D}]").unwrap());
static ANSI_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*m").unwrap());
static BEARER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Bearer [A-Za-z0-9_\-.]{20,}").unwrap());
static AXHUB_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"AXHUB_TOKEN=[A-Za-z0-9_\-.]{20,}").unwrap());
static AXHUB_PAT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"axhub_pat_[A-Za-z0-9_-]{16,}").unwrap());

pub fn redact(text: &str) -> String {
    let normalized: String = text.nfkc().collect();
    let s = BIDI_RE.replace_all(&normalized, "");
    let s = ZW_RE.replace_all(&s, "");
    let s = BEARER_RE.replace_all(&s, "Bearer ***");
    let s = AXHUB_TOKEN_RE.replace_all(&s, "AXHUB_TOKEN=***");
    let s = AXHUB_PAT_RE.replace_all(&s, "axhub_pat_[redacted]");
    ANSI_RE.replace_all(&s, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_unicode_and_redacts_secrets() {
        assert_eq!(redact("ａxhub"), "axhub");
        assert_eq!(redact("pay\u{200d}drop"), "paydrop");
        assert_eq!(redact("\u{202a}hello\u{202c}"), "hello");
        assert_eq!(
            redact("Authorization: Bearer abcdef1234567890abcdef"),
            "Authorization: Bearer ***"
        );
        assert_eq!(
            redact("AXHUB_TOKEN=abcdef1234567890abcdef extra"),
            "AXHUB_TOKEN=*** extra"
        );
        assert_eq!(
            redact("test axhub_pat_a1b2c3d4e5f6g7h8i9j0 leak"),
            "test axhub_pat_[redacted] leak"
        );
        assert_eq!(redact("\x1b[31mred\x1b[0m text"), "red text");
    }
}
