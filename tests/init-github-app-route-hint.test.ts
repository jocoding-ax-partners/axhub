import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// Regression lock for the init GitHub App surface (PR #173 follow-up).
//
// PR #173 added a read-only "GitHub App install state" step (Step 2.5) to the
// init SKILL body, but the prompt-route init route hint — the contract an
// NL-routed "새 앱 만들어줘" prompt actually follows — still described
// templates → app name and skipped it. The two sources of truth disagreed, so
// the surface never showed for NL-routed init. This locks the route hint to
// mention the GitHub App surface between the template and app-name steps, so it
// stays consistent with the SKILL body.

const repoRoot = join(import.meta.dir, "..");
const helperBinary = join(repoRoot, "target", "debug", "axhub-helpers");

function ensureHelperBuilt() {
  const build = spawnSync("cargo", ["build", "-p", "axhub-helpers"], {
    cwd: repoRoot,
    encoding: "utf8",
    timeout: 120_000,
  });
  expect(build.status).toBe(0);
}

function fakeAxhub(dir: string): string {
  const bin = join(dir, "axhub");
  writeFileSync(
    bin,
    `#!/usr/bin/env bash
set -euo pipefail
if [ "\${1:-}" = "--version" ]; then
  echo "axhub 0.18.0"
  exit 0
fi
if [ "\${1:-}" = "auth" ] && [ "\${2:-}" = "status" ] && [ "\${3:-}" = "--json" ]; then
  echo '{"user_email":"qa@example.test","expires_at":"2099-01-01T00:00:00Z","scopes":["read","deploy"]}'
  exit 0
fi
echo '{"ok":true}'
`,
  );
  chmodSync(bin, 0o755);
  return bin;
}

function promptRoute(prompt: string): string {
  ensureHelperBuilt();
  const dir = mkdtempSync(join(tmpdir(), "axhub-init-github-route-"));
  const out = spawnSync(helperBinary, ["prompt-route"], {
    cwd: dir,
    input: JSON.stringify({ hook_event_name: "UserPromptSubmit", prompt }),
    env: {
      ...process.env,
      AXHUB_BIN: fakeAxhub(dir),
      CLAUDE_PLUGIN_ROOT: repoRoot,
      XDG_STATE_HOME: join(dir, "state"),
    },
    encoding: "utf8",
    timeout: 10_000,
  });
  expect(out.status).toBe(0);
  return out.stdout;
}

describe("init route hint surfaces the GitHub App install state", () => {
  test("NL new-app prompt carries the GitHub App surface step", () => {
    const stdout = promptRoute("새 앱 만들어줘");
    const payload = JSON.parse(stdout);
    const additionalContext = payload.hookSpecificOutput?.additionalContext ?? "";
    const systemMessage = payload.systemMessage ?? "";
    const routedText = [additionalContext, systemMessage].join("\n");

    // still the init contract (did not break the existing flow)
    expect(routedText).toContain("새 앱을 만들 수 있는 템플릿을 확인할게요");
    expect(routedText).toContain("axhub apps templates list --json");

    // the GitHub App surface step is present in the agent-facing route hint.
    // PR #178 moved route control out of user-visible systemMessage, so this
    // regression now locks both properties: the surface remains, and internals
    // do not leak into `UserPromptSubmit says:`.
    expect(additionalContext).toContain("axhub github accounts list --json");
    expect(additionalContext).toContain("GitHub App 계정 설치 상태");
    expect(systemMessage).not.toContain("GitHub App 계정 설치 상태");

    // install_url must be shown even for already-installed accounts (the
    // add-another-org entry point). The "그냥 넘어가요" / "just move on"
    // wording suppressed the link for installed accounts — lock against it.
    expect(additionalContext).toContain("Always show `install_url`");
    expect(additionalContext).toContain("already-installed");
    expect(additionalContext).not.toContain("just move on");
    expect(additionalContext).toContain("항상 같이 보여줘요");
    expect(systemMessage).not.toContain("그냥 넘어가요");

    // the surface sits before the app-name step, not after approval
    const ghIndex = additionalContext.indexOf("GitHub App install state");
    const nameIndex = additionalContext.indexOf("ask for the app name");
    expect(ghIndex).toBeGreaterThanOrEqual(0);
    expect(nameIndex).toBeGreaterThan(ghIndex);
  });
});
