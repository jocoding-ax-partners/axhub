// Tests for readWindowsKeychain — 6 mocked-runner cases covering pre-mortem
// scenarios: success / ExecutionPolicy / NOT_FOUND / PInvoke load / EDR / spawn.

import { describe, expect, test } from "bun:test";

import {
  readWindowsKeychain,
  type WindowsRunner,
  type WindowsSpawnResult,
} from "../src/axhub-helpers/keychain-windows";

const makeResult = (over: Partial<WindowsSpawnResult> = {}): WindowsSpawnResult => ({
  exitCode: 0,
  signalCode: undefined,
  stdout: "",
  stderr: "",
  ...over,
});

const makeRunner = (result: WindowsSpawnResult): WindowsRunner => () => result;
const makeThrowingRunner = (): WindowsRunner => () => {
  throw new Error("powershell.exe not found in PATH");
};

const validBlob = (() => {
  const json = JSON.stringify({
    schema_version: 2,
    access_token: "windows_credential_manager_test_token_long_enough",
    token_type: "bearer",
    expires_at: "2030-01-01T00:00:00Z",
    scopes: ["read", "write"],
  });
  const envelope = "go-keyring-base64:" + Buffer.from(json, "utf8").toString("base64");
  return Buffer.from(envelope, "utf8").toString("base64");
})();

describe("readWindowsKeychain — mocked-runner pre-mortem coverage", () => {
  test("case 1: success — AXHUB_OK:<base64> sentinel returns extracted token", () => {
    const runner = makeRunner(
      makeResult({ exitCode: 0, stdout: "AXHUB_OK:" + validBlob + "\n" }),
    );
    const result = readWindowsKeychain(runner);
    expect(result.token).toBe("windows_credential_manager_test_token_long_enough");
    expect(result.source).toBe("windows-credential-manager");
    expect(result.error).toBeUndefined();
  });

  test("case 2: ExecutionPolicy block — exit 1 + stderr 'execution of scripts' → ExecutionPolicy 4-part Korean", () => {
    const runner = makeRunner(
      makeResult({
        exitCode: 1,
        stdout: "",
        stderr:
          "File C:\\Users\\... cannot be loaded because the execution of scripts is disabled on this system.",
      }),
    );
    const result = readWindowsKeychain(runner);
    expect(result.token).toBeUndefined();
    expect(result.error).toContain("ExecutionPolicy");
    expect(result.error).toContain("AXHUB_TOKEN");
    // 4-part 감정/원인/해결/다음액션 → at least 4 newlines (4 parts)
    expect((result.error ?? "").split("\n").length).toBeGreaterThanOrEqual(4);
  });

  test("case 3: NOT_FOUND — stdout ERR:NOT_FOUND → 4-part Korean missing-credential", () => {
    const runner = makeRunner(makeResult({ exitCode: 0, stdout: "ERR:NOT_FOUND\n" }));
    const result = readWindowsKeychain(runner);
    expect(result.token).toBeUndefined();
    expect(result.error).toContain("Credential Manager");
    expect(result.error).toContain("axhub auth login");
    expect((result.error ?? "").split("\n").length).toBeGreaterThanOrEqual(4);
  });

  test("case 4: PInvoke load failure — stdout ERR:LOAD_FAIL → 4-part Korean PInvoke error", () => {
    const runner = makeRunner(
      makeResult({
        exitCode: 1,
        stdout: "ERR:LOAD_FAIL\n",
        stderr: "Add-Type: Cannot add type. Compilation errors occurred.",
      }),
    );
    const result = readWindowsKeychain(runner);
    expect(result.token).toBeUndefined();
    expect(result.error).toContain("Add-Type");
    expect(result.error).toContain("AXHUB_TOKEN");
    expect((result.error ?? "").split("\n").length).toBeGreaterThanOrEqual(4);
  });

  test("case 5: EDR signal kill — signalCode set OR exit ∈ {-1, 0xC0000409} → EDR honesty (no IT-allowlist)", () => {
    const runner = makeRunner(
      makeResult({ exitCode: -1, signalCode: undefined, stdout: "", stderr: "" }),
    );
    const result = readWindowsKeychain(runner);
    expect(result.token).toBeUndefined();
    expect(result.error).toContain("보안 솔루션");
    expect(result.error).toContain("코드사이닝");
    expect(result.error).toContain("AXHUB_TOKEN");
    // Must NOT instruct user to ask IT for EDR allowlist (SOC will reject for unsigned binary)
    expect(result.error).not.toContain("관리자에게");
    expect((result.error ?? "").split("\n").length).toBeGreaterThanOrEqual(5);
  });

  test("case 6: spawnSync throws — runner rejects → 4-part Korean spawn-failure", () => {
    const runner = makeThrowingRunner();
    const result = readWindowsKeychain(runner);
    expect(result.token).toBeUndefined();
    expect(result.error).toContain("PowerShell");
    expect(result.error).toContain("AXHUB_TOKEN");
    expect((result.error ?? "").split("\n").length).toBeGreaterThanOrEqual(4);
  });
});
