// Windows compatibility docs guard — locks Phase 0 decision boundaries.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const read = (path: string) => readFileSync(join(REPO_ROOT, path), "utf8");

describe("Windows compatibility docs", () => {
  test("Phase 0 spike blocks Tier 3 automatic SessionStart until native evidence exists", () => {
    const doc = read("docs/pilot/windows-hook-packaging-spike.md");
    expect(doc).toContain("Stock Windows automatic SessionStart stays **deferred**");
    expect(doc).toContain("Do not add a universal `shell: \"powershell\"` SessionStart sibling");
    expect(doc).toContain("`${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers`");
    expect(doc).toContain("Bash tool");
    expect(doc).toContain("Exit criteria for Tier 3 promotion");
  });

  test("bin README narrows PATH and Windows installer claims", () => {
    const doc = read("bin/README.md");
    expect(doc).toContain("Bash tool");
    expect(doc).toContain("registered in the universal `hooks/hooks.json` today");
    expect(doc).toContain("Universal PowerShell SessionStart stays disabled");
    expect(doc).not.toContain("auto-download `windows-amd64.exe` on first session");
  });

  test("Windows smoke checklist no longer claims first-session install.ps1 auto-trigger", () => {
    const doc = read("docs/pilot/windows-vm-smoke-checklist.md");
    expect(doc).toContain("explicit install.ps1 downloads windows-amd64.exe");
    expect(doc).toContain("Do not record “first session auto-triggered install.ps1”");
    expect(doc).toContain("Hook paths are labeled accurately");
    expect(doc).not.toContain("First session should trigger install.ps1");
  });

  test("README separates Windows native from Git Bash and WSL fallback", () => {
    const doc = read("README.md");
    expect(doc).toContain("Windows native 는 명시적 PowerShell 설치·token-import·AXHUB_TOKEN 경로");
    expect(doc).toContain("Windows native 자동 SessionStart 는 platform-specific hook 검증 전까지 deferred 예요");
    expect(doc).toContain("PowerShell 은 $env:AXHUB_TOKEN / axhub-helpers.exe token-import");
  });

  test("Windows smoke executor mirrors explicit install and hook evidence boundaries", () => {
    const script = read("tests/smoke-windows-vm-checklist.ps1");
    expect(script).toContain("explicit install.ps1 downloads windows-amd64.exe");
    expect(script).toContain("powershell -NoProfile -ExecutionPolicy Bypass -File $installPs1");
    expect(script).toContain("do not record first-session auto-trigger");
    expect(script).toContain("Hook paths labeled accurately for native Windows evidence");
    expect(script).toContain("extensionless helper path observed");
    expect(script).toContain("cmd-launching-PowerShell/helper only");
  });

  test("troubleshooting token import examples avoid POSIX-only temp paths", () => {
    const doc = read("docs/troubleshooting.ko.md");
    expect(doc).toContain('Get-Content "$env:USERPROFILE\\axhub-token.json"');
    expect(doc).toContain('"$env:CLAUDE_PLUGIN_ROOT\\bin\\axhub-helpers.exe" token-import');
    expect(doc).toContain("Get-Command axhub");
    expect(doc).not.toContain("/tmp/axhub-token.json");
  });

  test("auth and headless references include PowerShell token-import lanes", () => {
    const auth = read("skills/auth/SKILL.md");
    const headless = read("skills/deploy/references/headless-flow.md");
    const recovery = read("skills/deploy/references/recovery-flows.md");

    expect(auth).toContain("$env:AXHUB_TOKEN='axhub_pat_...'");
    expect(auth).toContain('"$env:CLAUDE_PLUGIN_ROOT\\bin\\axhub-helpers.exe" token-import');
    expect(headless).toContain("$env:AXHUB_TOKEN='axhub_pat_...'");
    expect(headless).toContain('"$env:CLAUDE_PLUGIN_ROOT\\bin\\axhub-helpers.exe" token-import');
    expect(recovery).toContain("$env:AXHUB_TOKEN='axhub_pat_...'");
    expect(recovery).toContain('"$env:CLAUDE_PLUGIN_ROOT\\bin\\axhub-helpers.exe" token-import');
  });

  test("skill guidance separates POSIX statusline/cache from native Windows", () => {
    const deploy = read("skills/deploy/SKILL.md");
    const recover = read("skills/recover/SKILL.md");
    const doctor = read("skills/doctor/SKILL.md");
    const installCli = read("skills/install-cli/SKILL.md");
    const admin = read("docs/pilot/admin-rollout.ko.md");

    expect(deploy).toContain("POSIX/Git Bash/WSL tool execution");
    expect(recover).toContain("native Windows statusLine wiring");
    expect(doctor).toContain("Get-Command axhub");
    const windowsDoctorRow = doctor
      .split("\n")
      .find((line) => line.includes("helper missing on Windows")) ?? "";
    expect(windowsDoctorRow).toContain("Windows native 는 자동 SessionStart 다운로드가 아직 deferred 예요");
    expect(windowsDoctorRow).not.toContain("CC 재시작");
    expect(installCli).toContain("Get-Command axhub");
    expect(admin).toContain("Get-Command axhub");
  });

});
