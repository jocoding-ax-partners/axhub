/**
 * Spec 004 — AC 8: helper binary auto-download runs UNCONDITIONALLY in all
 * projects, independent of the axhub.yaml project marker.
 *
 * Why this exists (brick prevention):
 *   The routing decouple work gates eager session-start infra (token-init,
 *   Gatekeeper warmup, quality-context) behind the axhub.yaml marker so that
 *   non-axhub projects get a zero-footprint pass-through. The helper binary
 *   auto-download is the ONE step that MUST stay outside that gate: the marker
 *   walk-up itself is computed by the helper, so gating the download on the
 *   marker would brick the plugin in any project where the helper is not yet
 *   installed (chicken-and-egg). See constraint
 *   "Helper binary download excluded from marker gate (brick prevention)".
 *
 * This is a regression LOCK, not a structural guarantee. It runs the real
 * committed hook (hooks/session-start.sh) with the helper binary missing, in
 * BOTH a non-marker cwd and a marker cwd, and asserts install.sh (the
 * downloader) ran in each case + the hook still exits 0. If a future edit
 * moves the download below/inside the marker gate, the non-marker case goes
 * red here.
 *
 * Mirrors the harness style of tests/session-start-warmup.test.ts.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { spawn } from "node:child_process";
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const HOOK_PATH = join(process.cwd(), "hooks", "session-start.sh");

interface RunResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

/**
 * Runs the real session-start hook with the helper binary ABSENT and a stub
 * install.sh that records it ran and materializes the helper (so the hook's
 * final `exec "$HELPER" session-start` succeeds).
 *
 * @param withMarker  when true, writes axhub.yaml into the project cwd so the
 *                     walk-up marker check (sibling work) would resolve present.
 */
async function runHookMissingHelper(args: {
  rootDir: string;
  projectDir: string;
  withMarker: boolean;
}): Promise<{ result: RunResult; installRan: boolean; helperCreated: boolean }> {
  const binDir = join(args.rootDir, "bin");
  await mkdir(binDir, { recursive: true });
  const tracePath = join(args.rootDir, "install-trace.log");

  // Stub install.sh — the downloader. Records that it ran, then writes a stub
  // helper into bin/ so the hook can proceed to exec. Uses $CLAUDE_PLUGIN_ROOT
  // (inherited from the hook) so it stays path-agnostic, mirroring the real
  // install.sh contract.
  const installSh = join(binDir, "install.sh");
  const installScript = `#!/usr/bin/env bash
printf 'INSTALL_SH_RAN\\n' >> "${tracePath}"
cat > "$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers" <<'HELPER'
#!/usr/bin/env bash
case "$1" in
  path) echo "$CLAUDE_PLUGIN_ROOT/dummy-token-file" ;;
  session-start) echo '{"systemMessage":"stub session-start"}' ;;
  *) ;;
esac
exit 0
HELPER
chmod +x "$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers"
exit 0
`;
  await writeFile(installSh, installScript);
  await chmod(installSh, 0o755);

  // Marker presence is a property of the project cwd, not the plugin root.
  if (args.withMarker) {
    await writeFile(join(args.projectDir, "axhub.yaml"), "name: test-app\n");
  }

  const env: NodeJS.ProcessEnv = {
    ...process.env,
    CLAUDE_PLUGIN_ROOT: args.rootDir,
    // Restricted PATH: excludes any real `axhub` CLI so the token-init /
    // auth-refresh branches stay inert and the test is hermetic.
    PATH: `${binDir}:/usr/bin:/bin`,
    // Keep eager infra inert — this test isolates the download step.
    AXHUB_GATEKEEPER_WARMUP: "0",
    AXHUB_AUTH_BG_REFRESH: "0",
    AXHUB_DISABLE_MEGASKILL: "1",
    // AXHUB_SKIP_AUTODOWNLOAD intentionally left UNSET — download must run.
  };
  delete env.AXHUB_SKIP_AUTODOWNLOAD;

  const result = await new Promise<RunResult>((resolve) => {
    const child = spawn("bash", [HOOK_PATH], {
      env,
      cwd: args.projectDir,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (b) => (stdout += b.toString()));
    child.stderr.on("data", (b) => (stderr += b.toString()));
    child.on("exit", (code) => resolve({ exitCode: code ?? -1, stdout, stderr }));
    child.on("error", () => resolve({ exitCode: 127, stdout, stderr }));
  });

  let installRan = false;
  try {
    const trace = await readFile(tracePath, "utf8");
    installRan = trace.includes("INSTALL_SH_RAN");
  } catch {
    installRan = false;
  }

  let helperCreated = false;
  try {
    await readFile(join(binDir, "axhub-helpers"), "utf8");
    helperCreated = true;
  } catch {
    helperCreated = false;
  }

  return { result, installRan, helperCreated };
}

describe("SessionStart helper auto-download is marker-independent (AC 8)", () => {
  let rootDir: string;
  let projectDir: string;

  beforeEach(async () => {
    rootDir = await mkdtemp(join(tmpdir(), "axhub-dl-root-"));
    projectDir = await mkdtemp(join(tmpdir(), "axhub-dl-proj-"));
  });

  afterEach(async () => {
    await rm(rootDir, { recursive: true, force: true });
    await rm(projectDir, { recursive: true, force: true });
  });

  test("non-marker project: missing helper triggers install.sh and exits 0", async () => {
    const { result, installRan, helperCreated } = await runHookMissingHelper({
      rootDir,
      projectDir,
      withMarker: false,
    });
    // The core invariant: zero-footprint projects still get the helper.
    expect(installRan).toBe(true);
    expect(helperCreated).toBe(true);
    // Fail-open contract: hook always exits 0.
    expect(result.exitCode).toBe(0);
  });

  test("marker project: missing helper triggers install.sh and exits 0", async () => {
    const { result, installRan, helperCreated } = await runHookMissingHelper({
      rootDir,
      projectDir,
      withMarker: true,
    });
    expect(installRan).toBe(true);
    expect(helperCreated).toBe(true);
    expect(result.exitCode).toBe(0);
  });

  test("download decision does not depend on the marker (both cwd variants run install)", async () => {
    const noMarker = await runHookMissingHelper({
      rootDir,
      projectDir,
      withMarker: false,
    });
    // Fresh dirs for the second variant so traces never bleed across runs.
    const rootDir2 = await mkdtemp(join(tmpdir(), "axhub-dl-root2-"));
    const projectDir2 = await mkdtemp(join(tmpdir(), "axhub-dl-proj2-"));
    try {
      const withMarker = await runHookMissingHelper({
        rootDir: rootDir2,
        projectDir: projectDir2,
        withMarker: true,
      });
      // Marker present vs absent must yield the SAME download outcome.
      expect(noMarker.installRan).toBe(withMarker.installRan);
      expect(noMarker.installRan).toBe(true);
    } finally {
      await rm(rootDir2, { recursive: true, force: true });
      await rm(projectDir2, { recursive: true, force: true });
    }
  });
});
