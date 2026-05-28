#!/usr/bin/env bun
// Phase 25 PR 25.3 — PostToolUse user-app deploy artifact verifier hook.
//
// Fires after every `axhub deploy create` Bash invocation. Reads tool_input
// + tool_response from stdin, runs `verifyUserAppArtifact` on the captured
// stdout, and emits a `systemMessage` warning when sanity checks fail. The
// hook itself always exits 0 (fail-open per docs/HOOKS.md spec).
//
// Kill switch: respects `AXHUB_DISABLE_HOOKS` / `AXHUB_DISABLE_HOOK` (with
// the legacy `DISABLE_AXHUB` alias) through `hooks/_helpers.ts`. Identical
// kill-switch contract as the Rust hooks shipped in PR 25.2.

import {
  isAxhubDeployCommand,
  isHookDisabled,
  readStdin,
} from "./_helpers";
import { verifyUserAppArtifact } from "../scripts/verify-user-app-artifact";

const HOOK_NAME = "post-tool-verify-deploy-artifacts";

async function main(): Promise<void> {
  if (isHookDisabled(HOOK_NAME)) return;

  let raw: string;
  try {
    raw = await readStdin();
  } catch {
    return; // fail-open: never block on stdin failure
  }

  let payload: Record<string, unknown>;
  try {
    payload = JSON.parse(raw);
  } catch {
    return; // fail-open: invalid JSON envelope = nothing to verify
  }

  const toolInput = (payload as { tool_input?: unknown }).tool_input;
  if (!isAxhubDeployCommand(toolInput)) return;

  const toolResponse = (payload as { tool_response?: { exit_code?: unknown; stdout?: unknown } }).tool_response;
  if (!toolResponse || toolResponse.exit_code !== 0) return;

  const stdout = typeof toolResponse.stdout === "string" ? toolResponse.stdout : "";
  if (stdout.trim().length === 0) return;

  const { passed, violations } = verifyUserAppArtifact(stdout);
  if (!passed && violations.length > 0) {
    const systemMessage = `⚠️ 배포 artifact 검증에서 의심 신호를 발견했어요: ${violations.join(", ")}. 라이브 결과를 한 번 더 확인해주세요.`;
    const additionalContext = [
      "<axhub-deploy-verify>",
      "[axhub hook | deploy artifact verification]",
      `Observed: ${violations.join("; ")}`,
      "Suggested: run axhub-helpers verify --app-id <app> or inspect axhub deploy logs before claiming the app is live.",
      "Skip: AXHUB_DISABLE_HOOK=post-tool-verify-deploy-artifacts",
      "</axhub-deploy-verify>",
    ].join("\n");
    console.log(JSON.stringify({
      systemMessage,
      hookSpecificOutput: { hookEventName: "PostToolUse", additionalContext },
    }));
  }
}

main().catch(() => {
  // Defensive: any thrown error from helpers must not propagate. The hook is
  // fail-open, so we swallow and exit 0.
});
