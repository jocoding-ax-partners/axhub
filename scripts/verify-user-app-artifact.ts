// Phase 25 PR 25.3 — user-app deploy artifact verifier.
//
// CRITICAL boundary (see .plan/matrix-absorption/00-overview.md §10.2 #9):
// This is NOT `scripts/release-check.ts`. release-check verifies the **axhub
// CLI** cross-arch binary release artifacts (5 binaries). This module
// verifies the **user-app deploy artifact** — what `axhub deploy create`
// returned when a user shipped their app. Two completely separate domains;
// any future expansion must keep code-share-zero with release-check.
//
// Scope (MVP for PR 25.3):
//   - Parse `axhub deploy create --json` stdout (best-effort; non-JSON skips)
//   - Sanity-check plausible signals when present:
//       · manifest_hash       → must be sha256 hex (64 lowercase hex chars)
//       · state               → must be one of live/running/deployed/active/ok/succeeded
//       · url                 → must look like http(s)://
//       · deployment_id / id  → must be a non-empty string
//   - Return structured { passed, violations[] } so the caller emits a
//     systemMessage. Never raises — verifier itself is fail-soft.
//
// Out of scope here (deferred per plan §risk table R25-3):
//   - Network probes to axhub status endpoint
//   - User-app health endpoint discovery + GET
//   - Cross-call manifest digest comparison
// These can land as `verify-user-app-artifact-deep.ts` once the backend
// response schema is locked.

export interface VerifyResult {
  passed: boolean;
  violations: string[];
}

const SHA256_HEX = /^[a-f0-9]{64}$/i;
const HTTPS_PREFIX = /^https?:\/\//i;
const LIVE_STATES = new Set([
  "live",
  "running",
  "deployed",
  "active",
  "ok",
  "succeeded",
  "success",
]);

export function verifyUserAppArtifact(deployStdout: string): VerifyResult {
  const violations: string[] = [];
  const response = parseDeployResponse(deployStdout);
  if (response === null) {
    // No structured payload to verify — fail-open. Plan §10 explicit:
    // verifier never blocks the deploy; absence of JSON means no signal.
    return { passed: true, violations };
  }

  const manifestHash = response["manifest_hash"];
  if (manifestHash !== undefined) {
    if (typeof manifestHash !== "string" || !SHA256_HEX.test(manifestHash)) {
      violations.push(`manifest_hash 형식이 sha256 hex (64자 hex) 가 아니에요`);
    }
  }

  const state = response["state"];
  if (state !== undefined) {
    const normalized = String(state).toLowerCase();
    if (!LIVE_STATES.has(normalized)) {
      violations.push(`state="${state}" — live/running/deployed 가 아니에요`);
    }
  }

  const url = response["url"];
  if (url !== undefined) {
    if (typeof url !== "string" || !HTTPS_PREFIX.test(url)) {
      violations.push(`url="${url}" 가 http(s):// 로 시작 안 해요`);
    }
  }

  for (const idKey of ["deployment_id", "deploy_id", "id"] as const) {
    const value = response[idKey];
    if (value !== undefined) {
      if (typeof value !== "string" || value.trim().length === 0) {
        violations.push(`${idKey} 가 비어 있어요`);
      }
      break;
    }
  }

  return { passed: violations.length === 0, violations };
}

function parseDeployResponse(stdout: string): Record<string, unknown> | null {
  const trimmed = stdout.trim();
  if (!trimmed.startsWith("{")) return null;
  try {
    const parsed = JSON.parse(trimmed);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    // not JSON — fail-open
  }
  return null;
}

// CLI entry — `bun run scripts/verify-user-app-artifact.ts < stdin` so the
// hook (or a CI step) can pipe `axhub deploy create --json` output and read
// a structured result. Existing `release-check.ts` writes JSON to stdout; we
// mirror that contract.
if (import.meta.main) {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk as Buffer);
  }
  const result = verifyUserAppArtifact(Buffer.concat(chunks).toString("utf8"));
  process.stdout.write(JSON.stringify(result) + "\n");
  process.exit(result.passed ? 0 : 1);
}
