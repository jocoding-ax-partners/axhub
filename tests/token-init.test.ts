// Phase 8 US-802: tests for cmdTokenInit keychain bridge.
//
// Phase 6/7 had a broken assumption — `axhub auth login --print-token` is not
// a real CLI flag. Phase 8 rewrote cmdTokenInit to read straight from the OS
// keychain that ax-hub-cli already populates via `axhub auth login`.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { parseKeyringValue } from "../src/axhub-helpers/keychain";

const REPO_ROOT = join(import.meta.dir, "..");
const INDEX_TS = join(REPO_ROOT, "src/axhub-helpers/index.ts");
const KEYCHAIN_TS = join(REPO_ROOT, "src/axhub-helpers/keychain.ts");

describe("cmdTokenInit subcommand registration (US-603)", () => {
  test("dispatch switch includes 'token-init' case", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toMatch(/case "token-init":\s*\n\s*return cmdTokenInit\(args\);/);
  });

  test("USAGE documents token-init subcommand", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toContain("token-init");
    expect(content).toContain("1-step setup");
  });

  test("cmdTokenInit function is defined", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toMatch(/async function cmdTokenInit\(_args: string\[\]\): Promise<number>/);
  });

  test("Phase 8 rewrite: NO axhub --print-token call (CLI flag does not exist)", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    const initIdx = content.indexOf("async function cmdTokenInit");
    const initBody = content.slice(initIdx, initIdx + 4000);
    expect(initBody).not.toContain("--print-token");
    expect(initBody).not.toContain("auth\", \"login\"");
  });

  test("token storage path matches token-import (XDG_CONFIG_HOME-aware)", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toContain("axhub-plugin");
    expect(content).toContain("XDG_CONFIG_HOME");
  });

  test("file mode 0600 + dir mode 0700 enforced (security parity with token-import)", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    const initIdx = content.indexOf("async function cmdTokenInit");
    expect(initIdx).toBeGreaterThan(-1);
    const initBody = content.slice(initIdx, initIdx + 4000);
    expect(initBody).toContain("0o600");
    expect(initBody).toContain("0o700");
  });
});

describe("Phase 8 US-801: AXHUB_TOKEN env var precedence + keychain bridge", () => {
  test("AXHUB_TOKEN env var path is checked before keychain", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    const initIdx = content.indexOf("async function cmdTokenInit");
    const initBody = content.slice(initIdx, initIdx + 4000);
    expect(initBody).toContain("AXHUB_TOKEN");
    expect(initBody.indexOf("AXHUB_TOKEN")).toBeLessThan(initBody.indexOf("readKeychainToken"));
  });

  test("readKeychainToken handles macOS via 'security find-generic-password'", () => {
    const content = readFileSync(KEYCHAIN_TS, "utf8");
    expect(content).toContain('"security"');
    expect(content).toContain('"find-generic-password"');
    expect(content).toContain('"-s"');
    expect(content).toContain('"axhub"');
  });

  test("readKeychainToken handles Linux via secret-tool", () => {
    const content = readFileSync(KEYCHAIN_TS, "utf8");
    expect(content).toContain('"secret-tool"');
    expect(content).toContain('"lookup"');
    expect(content).toContain("libsecret-tools");
  });

  test("Windows is explicitly deferred with Korean message", () => {
    const content = readFileSync(KEYCHAIN_TS, "utf8");
    expect(content).toContain("win32");
    expect(content).toContain("Windows");
    expect(content).toContain("AXHUB_TOKEN");
  });
});

describe("Phase 8 US-801: parseKeyringValue (go-keyring-base64 decoder)", () => {
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
