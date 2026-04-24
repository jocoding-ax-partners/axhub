// Tests for cmdTokenInit OS-keychain bridge + Windows source assertions.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INDEX_TS = join(REPO_ROOT, "src/axhub-helpers/index.ts");
const KEYCHAIN_TS = join(REPO_ROOT, "src/axhub-helpers/keychain.ts");

describe("cmdTokenInit subcommand registration", () => {
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

  test("NO axhub --print-token call (CLI flag does not exist)", () => {
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

describe("AXHUB_TOKEN env var precedence + keychain bridge", () => {
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

  test("Windows uses PowerShell + Add-Type PInvoke against advapi32!CredReadW", () => {
    const KEYCHAIN_WIN = join(REPO_ROOT, "src/axhub-helpers/keychain-windows.ts");
    const content = readFileSync(KEYCHAIN_WIN, "utf8");
    expect(content).toContain("powershell.exe");
    expect(content).toContain("Add-Type");
    expect(content).toContain("advapi32.dll");
    expect(content).toContain("CredReadW");
    expect(content).toContain("AXHUB_OK:");
    expect(content).toContain("ERR:NOT_FOUND");
    expect(content).toContain("ERR:LOAD_FAIL");
  });
});
