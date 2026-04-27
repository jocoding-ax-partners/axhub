/**
 * redact.ts — NFKC normalize + Unicode hardening + secret redaction
 *
 * Implements PLAN §16.11 (Unicode hardening: NFKC + Bidi/ZWJ strip)
 * and §16.17 (privacy: bearer token + axhub token redaction).
 */

// Bidi override characters: U+202A–U+202E (LRE, RLE, PDF, LRO, RLO)
// and U+2066–U+2069 (LRI, RLI, FSI, PDI)
const BIDI_RE = /[‪-‮⁦-⁩]/g;

// Zero-width joiners / non-joiners / space: U+200D (ZWJ), U+200C (ZWNJ), U+200B (ZWSP)
const ZW_RE = /[​-‍]/g;

// ANSI escape sequences (colour/formatting codes)
const ANSI_RE = /\x1b\[[0-9;]*m/g;

// Bearer token: "Bearer " followed by ≥20 base64url/dot chars
const BEARER_RE = /Bearer [A-Za-z0-9_\-.]{20,}/g;

// axhub env-var token: "AXHUB_TOKEN=" followed by ≥20 base64url/dot chars
const AXHUB_TOKEN_RE = /AXHUB_TOKEN=[A-Za-z0-9_\-.]{20,}/g;

// axhub raw PAT token: "axhub_pat_" followed by ≥16 alnum/_/- chars.
// Phase docs (PLAN E7 + skills/deploy/references/headless-flow.md §3) require
// this masking — caught missing by live plugin smoke 2026-04-27.
const AXHUB_PAT_RE = /axhub_pat_[A-Za-z0-9_-]{16,}/g;

/**
 * Sanitize and redact a text string:
 * 1. NFKC normalize (collapses homoglyphs, compatibility chars)
 * 2. Strip Bidi override chars (‪–‮, ⁦–⁩)
 * 3. Strip ZWJ / ZWNJ / ZWSP
 * 4. Redact Bearer tokens
 * 5. Redact AXHUB_TOKEN= values
 * 6. Redact raw axhub_pat_* tokens
 * 7. Strip ANSI escape sequences
 */
export function redact(text: string): string {
  return text
    .normalize("NFKC")
    .replace(BIDI_RE, "")
    .replace(ZW_RE, "")
    .replace(BEARER_RE, "Bearer ***")
    .replace(AXHUB_TOKEN_RE, "AXHUB_TOKEN=***")
    .replace(AXHUB_PAT_RE, "axhub_pat_[redacted]")
    .replace(ANSI_RE, "");
}
