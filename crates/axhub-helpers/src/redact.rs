use regex::Regex;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

/// HITL capture byte cap per plan v6 §4.2.
pub const REDACT_BYTE_CAP: usize = 100 * 1024;

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
static SERVICE_BASE_URL_JSON_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""service_base_url"\s*:\s*"[^"]*""#).unwrap());
static SERVICE_BASE_URL_TEXT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)\bservice_base_url\s*:\s*https?://[^\s,"}]+"#).unwrap());

// Plan v6 §4.2 free-text secret regex set — for HITL capture redaction.
static OPENAI_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sk-[A-Za-z0-9_\-]{20,}").unwrap());
static GH_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{20,})").unwrap()
});
// Broader AWS access-key prefix set per AWS account-id prefix taxonomy
// (AKIA = long-lived, ASIA = STS temp, AGPA = group, AIDA = IAM user, etc.).
// Covers temporary creds in CI logs that the prior `AKIA[0-9A-Z]{16}` missed.
static AWS_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(AKIA|ASIA|AGPA|ANPA|ANVA|AROA|AIDA|AIPA)[0-9A-Z]{16}").unwrap());
// Slack token taxonomy: xoxb(bot)/xoxp(user)/xoxa(app)/xoxo(workspace)/xoxs(session)/xoxr(refresh)/xoxe(export)
static SLACK_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"xox[abeoprs]-[0-9A-Za-z-]{4,}").unwrap());
static SLACK_WEBHOOK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https://hooks\.slack\.com/services/[A-Za-z0-9/_-]+").unwrap()
});
static PRIVATE_KEY_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-----BEGIN [A-Z ]+PRIVATE KEY-----.*?-----END [A-Z ]+PRIVATE KEY-----")
        .unwrap()
});
static URL_CREDS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bhttps?://[A-Za-z0-9._~+\-]+:[^\s@/]+@").unwrap());

pub fn redact(text: &str) -> String {
    let normalized: String = text.nfkc().collect();
    let s = BIDI_RE.replace_all(&normalized, "");
    let s = ZW_RE.replace_all(&s, "");
    // Run secret-class redaction BEFORE byte cap so a token can never be
    // half-truncated and then partially exposed. Order matters: private key
    // blocks before single-line tokens to avoid mid-block matches.
    let s = PRIVATE_KEY_BLOCK_RE.replace_all(&s, "<REDACTED_PRIVATE_KEY>");
    let s = BEARER_RE.replace_all(&s, "Bearer ***");
    let s = AXHUB_TOKEN_RE.replace_all(&s, "AXHUB_TOKEN=***");
    let s = AXHUB_PAT_RE.replace_all(&s, "axhub_pat_[redacted]");
    let s = OPENAI_KEY_RE.replace_all(&s, "<REDACTED_OPENAI_KEY>");
    let s = GH_TOKEN_RE.replace_all(&s, "<REDACTED_GH_TOKEN>");
    let s = AWS_KEY_RE.replace_all(&s, "<REDACTED_AWS_KEY>");
    let s = SLACK_TOKEN_RE.replace_all(&s, "<REDACTED_SLACK_TOKEN>");
    let s = SLACK_WEBHOOK_RE.replace_all(&s, "<REDACTED_SLACK_WEBHOOK>");
    let s = URL_CREDS_RE.replace_all(&s, "https://<REDACTED_CREDS>@");
    let s = SERVICE_BASE_URL_JSON_RE.replace_all(&s, r#""service_base_url":"[redacted]""#);
    let s = SERVICE_BASE_URL_TEXT_RE.replace_all(&s, "service_base_url: [redacted]");
    ANSI_RE.replace_all(&s, "").into_owned()
}

/// Redact + enforce byte cap. Used by HITL capture path (plan v6 §4.2).
/// Returns (capped_text, was_truncated).
pub fn redact_for_handoff(text: &str) -> (String, bool) {
    let redacted = redact(text);
    if redacted.len() <= REDACT_BYTE_CAP {
        return (redacted, false);
    }
    // Truncate at a UTF-8 char boundary just before REDACT_BYTE_CAP.
    let mut cut = REDACT_BYTE_CAP;
    while cut > 0 && !redacted.is_char_boundary(cut) {
        cut -= 1;
    }
    (redacted[..cut].to_string(), true)
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

    // Plan v6 §4.2 — HITL free-text secret patterns.

    #[test]
    fn redacts_openai_key() {
        assert_eq!(
            redact("API: sk-abcdefghij0123456789ZZZZ end"),
            "API: <REDACTED_OPENAI_KEY> end"
        );
    }

    #[test]
    fn redacts_gh_pat() {
        let s = "git+https://ghp_abcdefghij0123456789ABCDEFGHIJ0123456789 ok";
        let r = redact(s);
        assert!(r.contains("<REDACTED_GH_TOKEN>"), "got: {r}");
        // ghs_/gho_/ghr_/ghu_ all match the same class.
        let s2 = "use ghs_ABCDEFGHIJ0123456789abcdefghij0123456789 here";
        let r2 = redact(s2);
        assert!(r2.contains("<REDACTED_GH_TOKEN>"), "got: {r2}");
        // Fine-grained GitHub PATs use the `github_pat_` prefix and include
        // underscores in the body, which the legacy gh[pousr]_ regex misses.
        let s3 = "token github_pat_11AA22BB33CC44DD55EE66_77FF88GG99HH00II11JJ22KK33LL44MM55NN";
        let r3 = redact(s3);
        assert!(r3.contains("<REDACTED_GH_TOKEN>"), "got: {r3}");
        assert!(
            !r3.contains("github_pat_11AA22BB33CC44DD55EE66"),
            "raw PAT leaked: {r3}"
        );
    }

    #[test]
    fn redacts_aws_key() {
        assert_eq!(
            redact("aws_access_key_id=AKIAIOSFODNN7EXAMPLE done"),
            "aws_access_key_id=<REDACTED_AWS_KEY> done"
        );
    }

    #[test]
    fn redacts_aws_temp_creds_asia() {
        // ASIA = STS temporary credentials — common in CI logs that pipe
        // `aws sts assume-role` output. The prior AKIA-only regex missed these.
        let r = redact("AWS_ACCESS_KEY_ID=ASIAIOSFODNN7EXAMPLE more");
        assert!(
            r.contains("<REDACTED_AWS_KEY>"),
            "ASIA must be redacted: {r}"
        );
        assert!(!r.contains("ASIAIOSFODNN7EXAMPLE"), "raw ASIA leaked: {r}");
    }

    #[test]
    fn redacts_aws_group_id_agpa() {
        let r = redact("group: AGPAIOSFODNN7EXAMPLE end");
        assert!(
            r.contains("<REDACTED_AWS_KEY>"),
            "AGPA must be redacted: {r}"
        );
    }

    #[test]
    fn redacts_private_key_block() {
        let pem = "before\n-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAKx\n-----END RSA PRIVATE KEY-----\nafter";
        let r = redact(pem);
        assert!(r.contains("<REDACTED_PRIVATE_KEY>"), "got: {r}");
        assert!(!r.contains("MIIBOgIBAAJBAKx"), "key body leaked: {r}");
        assert!(r.contains("before"));
        assert!(r.contains("after"));
    }

    #[test]
    fn redacts_slack_tokens_and_webhooks() {
        let input = "SLACK_BOT_TOKEN=xoxb-1073512345678-abcDEF123ghi SLACK_WEBHOOK_URL=https://hooks.slack.com/services/T0B6XAAAA/B0BBBB/secretpart123";
        let out = redact(input);
        assert!(!out.contains("xoxb-1073512345678"));
        assert!(out.contains("<REDACTED_SLACK_TOKEN>"));
        assert!(!out.contains("secretpart123"));
        assert!(out.contains("<REDACTED_SLACK_WEBHOOK>"));
    }

    #[test]
    fn redacts_url_creds() {
        let s = "git remote: https://user:hunter2@example.com/repo.git";
        let r = redact(s);
        assert!(r.contains("<REDACTED_CREDS>"), "got: {r}");
        assert!(!r.contains("hunter2"), "password leaked: {r}");
    }

    #[test]
    fn handoff_caps_byte_size() {
        let huge = "a".repeat(REDACT_BYTE_CAP + 5_000);
        let (out, truncated) = redact_for_handoff(&huge);
        assert!(truncated);
        assert!(out.len() <= REDACT_BYTE_CAP);
    }

    #[test]
    fn handoff_passes_under_cap() {
        let small = "no secrets here";
        let (out, truncated) = redact_for_handoff(small);
        assert!(!truncated);
        assert_eq!(out, small);
    }

    #[test]
    fn handoff_redacts_before_cap() {
        // Token near the end MUST be redacted, not half-truncated.
        let mut s = "x".repeat(REDACT_BYTE_CAP - 100);
        s.push_str(" Bearer abcdefghij0123456789ZZZZZZZZZZ tail");
        let (out, _truncated) = redact_for_handoff(&s);
        assert!(out.contains("Bearer ***"), "redaction must run before cap");
    }

    #[test]
    fn handoff_truncates_at_utf8_boundary() {
        // Build text that overflows the cap and lands a multi-byte char on the cap boundary.
        let mut s = "a".repeat(REDACT_BYTE_CAP - 2);
        // 가 = 3 bytes in UTF-8 — pushes us past the cap by 1 byte.
        s.push('가');
        s.push_str("rest");
        let (out, truncated) = redact_for_handoff(&s);
        assert!(truncated);
        // out.len() must NOT split a UTF-8 codepoint.
        assert!(out.is_char_boundary(out.len()));
    }
}
