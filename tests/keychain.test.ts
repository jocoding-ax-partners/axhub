// Tests for parseKeyringValue go-keyring-base64 envelope decoder + 4-part format parity.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { parseKeyringValue } from "../src/axhub-helpers/keychain";

const REPO_ROOT = join(import.meta.dir, "..");
const KEYCHAIN_TS = join(REPO_ROOT, "src/axhub-helpers/keychain.ts");

describe("parseKeyringValue (go-keyring-base64 decoder)", () => {
  test("strips 'go-keyring-base64:' prefix + decodes JSON + extracts access_token", () => {
    const json = JSON.stringify({
      schema_version: 2,
      access_token: "test_access_token_value_long_enough_to_pass",
      token_type: "bearer",
      expires_at: "2030-01-01T00:00:00Z",
      scopes: ["read", "write"],
    });
    const b64 = Buffer.from(json, "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    const result = parseKeyringValue(raw);
    expect(result).toBe("test_access_token_value_long_enough_to_pass");
  });

  test("works without 'go-keyring-base64:' prefix (raw base64)", () => {
    const json = JSON.stringify({ access_token: "another_test_token_value_for_assertion" });
    const b64 = Buffer.from(json, "utf8").toString("base64");
    const result = parseKeyringValue(b64);
    expect(result).toBe("another_test_token_value_for_assertion");
  });

  test("returns null on empty input", () => {
    expect(parseKeyringValue("")).toBeNull();
  });

  test("returns null on invalid base64", () => {
    expect(parseKeyringValue("go-keyring-base64:!!!not-valid-base64@@@")).toBeNull();
  });

  test("returns null when decoded JSON has no access_token field", () => {
    const json = JSON.stringify({ token_type: "bearer", scopes: ["read"] });
    const b64 = Buffer.from(json, "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    expect(parseKeyringValue(raw)).toBeNull();
  });

  test("returns null when access_token is too short (< 16 chars)", () => {
    const json = JSON.stringify({ access_token: "short" });
    const b64 = Buffer.from(json, "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    expect(parseKeyringValue(raw)).toBeNull();
  });

  test("returns null when decoded payload is not valid JSON", () => {
    const b64 = Buffer.from("not json at all", "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    expect(parseKeyringValue(raw)).toBeNull();
  });

  test("returns null when decoded JSON is array (not object)", () => {
    const b64 = Buffer.from(JSON.stringify(["a", "b"]), "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    expect(parseKeyringValue(raw)).toBeNull();
  });
});

// Plan v2 (D2 downgrade) said 6 lines → 4-part, line 65 catch NOTE only ("unreachable
// per Bun.spawnSync ENOENT semantics"). Executor expanded to all 7 because catch can
// fire on non-ENOENT spawn failures (OOM, SELinux/AppArmor deny, signal). UX consistency
// outweighs strict count. Architect APPROVED deviation; semantic kernel preserved.
describe("keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1)", () => {
  const source = readFileSync(KEYCHAIN_TS, "utf8");

  test("source contains exactly 7 4-part error blocks (1 macOS miss + 1 macOS parse + 1 macOS catch + 1 Linux miss + 1 Linux parse + 1 Linux catch + 1 platform fallback)", () => {
    // Each 4-part block has 원인:, 해결:, 다음: markers
    const causeMarkers = source.match(/원인:/g);
    const solveMarkers = source.match(/해결:/g);
    const nextMarkers = source.match(/다음:/g);
    expect(causeMarkers?.length).toBe(7);
    expect(solveMarkers?.length).toBe(7);
    expect(nextMarkers?.length).toBe(7);
  });

  test("each error starts with emotion word (감정 prefix per error-empathy-catalog)", () => {
    const emotions = source.match(/(잠깐만요|아이고|죄송해요)\.\\n/g);
    expect(emotions?.length).toBeGreaterThanOrEqual(7);
  });

  test("macOS keychain miss preserves 'axhub auth login' kernel keyword", () => {
    const macSection = source.slice(source.indexOf('platform === "darwin"'), source.indexOf('platform === "linux"'));
    expect(macSection).toContain("macOS keychain");
    expect(macSection).toContain("axhub auth login");
  });

  test("macOS parse failure preserves '--force' kernel keyword + axhub CLI version mention", () => {
    const macSection = source.slice(source.indexOf('platform === "darwin"'), source.indexOf('platform === "linux"'));
    expect(macSection).toContain("--force");
    expect(macSection).toContain("axhub CLI");
  });

  test("macOS catch preserves 'security' command + PATH kernel keyword", () => {
    const macSection = source.slice(source.indexOf('platform === "darwin"'), source.indexOf('platform === "linux"'));
    expect(macSection).toContain("'security'");
    expect(macSection).toContain("PATH");
  });

  test("Linux miss preserves 'libsecret-tools' + 'secret-tool' kernel keywords", () => {
    const linuxSection = source.slice(source.indexOf('platform === "linux"'), source.indexOf('platform === "win32"'));
    expect(linuxSection).toContain("libsecret-tools");
    expect(linuxSection).toContain("secret-tool");
  });

  test("Linux parse failure preserves 'axhub auth login --force' kernel", () => {
    const linuxSection = source.slice(source.indexOf('platform === "linux"'), source.indexOf('platform === "win32"'));
    expect(linuxSection).toContain("axhub auth login --force");
  });

  test("Linux catch preserves 'D-Bus' + 'dbus-launch' kernel keywords (architect-flagged systemd-keyring concern)", () => {
    const linuxSection = source.slice(source.indexOf('platform === "linux"'), source.indexOf('platform === "win32"'));
    expect(linuxSection).toContain("D-Bus");
    expect(linuxSection).toContain("dbus-launch");
  });

  test("platform fallback preserves AXHUB_TOKEN env var escape kernel + interpolates platform name", () => {
    const fallbackSection = source.slice(source.indexOf("readWindowsKeychain()"));
    expect(fallbackSection).toContain("AXHUB_TOKEN");
    expect(fallbackSection).toContain("${platform}");
  });
});
