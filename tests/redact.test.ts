import { describe, expect, test } from "bun:test";
import { redact } from "../src/axhub-helpers/redact.ts";

describe("redact()", () => {
  test("NFKC normalize: Cyrillic а (U+0430) survives but is normalized", () => {
    // Cyrillic 'а' is already in NFKC form — it stays; NFKC does not remove it.
    // This test documents that NFKC preserves Cyrillic (homoglyph detection is
    // a UI-layer concern; redact() only normalizes).
    const input = "pаydrop"; // Cyrillic 'а' at position 1
    const result = redact(input);
    // After NFKC the Cyrillic char is still there (no homoglyph substitution).
    expect(result).toBe("pаydrop");
  });

  test("NFKC normalize: fullwidth chars collapse to ASCII", () => {
    // U+FF41 = ａ (fullwidth a) → NFKC → 'a'
    const input = "ａxhub";
    expect(redact(input)).toBe("axhub");
  });

  test("Zero-width joiner (U+200D) stripped: 'pay‍drop' → 'paydrop'", () => {
    const input = "pay‍drop";
    expect(redact(input)).toBe("paydrop");
  });

  test("Zero-width non-joiner (U+200C) stripped", () => {
    const input = "pay‌drop";
    expect(redact(input)).toBe("paydrop");
  });

  test("Zero-width space (U+200B) stripped", () => {
    const input = "pay​drop";
    expect(redact(input)).toBe("paydrop");
  });

  test("Bidi LRE (U+202A) stripped", () => {
    const input = "‪hello‬";
    expect(redact(input)).toBe("hello");
  });

  test("Bidi RLE (U+202B) stripped", () => {
    const input = "‫hello‬";
    expect(redact(input)).toBe("hello");
  });

  test("Bearer token redacted: ≥20 char token", () => {
    const input = "Authorization: Bearer abcdef1234567890abcdef";
    const result = redact(input);
    expect(result).toBe("Authorization: Bearer ***");
  });

  test("Bearer token NOT redacted when shorter than 20 chars", () => {
    const input = "Authorization: Bearer short123";
    // 'short123' is only 8 chars — below the 20-char threshold
    const result = redact(input);
    expect(result).toContain("Bearer short123");
  });

  test("AXHUB_TOKEN redacted: ≥20 char token", () => {
    const input = "AXHUB_TOKEN=abcdef1234567890abcdef extra";
    const result = redact(input);
    expect(result).toBe("AXHUB_TOKEN=*** extra");
  });

  test("AXHUB_TOKEN NOT redacted when shorter than 20 chars", () => {
    const input = "AXHUB_TOKEN=short123";
    expect(redact(input)).toContain("AXHUB_TOKEN=short123");
  });

  test("ANSI escape sequences stripped: colour codes", () => {
    const input = "\x1b[31mred\x1b[0m text";
    expect(redact(input)).toBe("red text");
  });

  test("ANSI bold stripped", () => {
    const input = "\x1b[1mbold\x1b[0m";
    expect(redact(input)).toBe("bold");
  });

  test("Multiple redactions in one string", () => {
    const input =
      "\x1b[32mOK\x1b[0m Bearer abcdef1234567890abcdef AXHUB_TOKEN=xyz1234567890abcdef1234";
    const result = redact(input);
    expect(result).toBe("OK Bearer *** AXHUB_TOKEN=***");
  });

  test("Plain text with no secrets passes through unchanged", () => {
    const input = "axhub deploy status dep_123 --app paydrop --json";
    expect(redact(input)).toBe(input);
  });

  test("Empty string returns empty string", () => {
    expect(redact("")).toBe("");
  });
});
