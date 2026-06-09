/**
 * tenant-picker L1 thin-block bun-test (#189 Phase 2).
 *
 * L1 resolver 로직은 Rust axhub-helpers `tenant-resolve` 가 소유해요 (cargo 테스트가
 * precedence/TTL/non-numeric-ts 를 커버). 이 파일은 thin BASH 블록 통합을 검증해요:
 *   - 헬퍼 호출 결과를 받아 AXHUB_TENANT/NEEDS_PICK 설정
 *   - 모든 block-resolved 분기에서 캐시 영속화 (cross-fence source of truth)
 *   - no-loop (빈/부재 helper → NEEDS_PICK=false, 재프롬프트 없음)
 *   - R4 non-TTY 멀티테넌트 경고
 *   - command-fence re-read / inline 치환 (cache 소비)
 *
 * 헬퍼는 CLAUDE_PLUGIN_ROOT/bin/axhub-helpers 스텁으로 주입하고, presence 를
 * 토글해서 (resolve 인자 유무) 정상/skew(부재) 경로를 모두 검증해요.
 */

import { describe, expect, test } from "bun:test";
import {
  writeFileSync,
  readFileSync,
  mkdirSync,
  rmSync,
  existsSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import {
  CANONICAL_TENANT_PICKER_BLOCK,
  CANONICAL_TENANT_PICKER_REREAD,
  CANONICAL_TENANT_PICKER_INLINE,
} from "../scripts/tenant-picker-block";

// ── bash 추출 ────────────────────────────────────────────────────────────────
function extractBash(block: string): string {
  const lines = block.split("\n");
  const start = lines.findIndex((l) => l.trim() === "```bash");
  const end = lines.findIndex((l, i) => i > start && l.trim() === "```");
  if (start === -1 || end === -1) throw new Error("bash block not found in block constant");
  return lines.slice(start + 1, end).join("\n");
}

const L1_BASH = extractBash(CANONICAL_TENANT_PICKER_BLOCK);

// ── 헬퍼 ────────────────────────────────────────────────────────────────────
function makeTmpDir(label: string): string {
  const dir = join(tmpdir(), `tp-${label}-${Date.now()}-${Math.floor(performance.now())}`);
  mkdirSync(dir, { recursive: true });
  return dir;
}

function writeTenantCache(cwd: string, content: Record<string, unknown>): void {
  const stateDir = join(cwd, ".axhub/state");
  mkdirSync(stateDir, { recursive: true });
  writeFileSync(join(stateDir, "tenant.json"), JSON.stringify(content));
}

function cacheTenantOf(cwd: string): string | null {
  const cacheFile = join(cwd, ".axhub/state/tenant.json");
  if (!existsSync(cacheFile)) return null;
  return (JSON.parse(readFileSync(cacheFile, "utf8")) as { tenant: string }).tenant;
}

interface ThinOpts {
  cwd: string;
  env?: Record<string, string>;
  /** JSON the stubbed `tenant-resolve` returns. undefined → NO helper present (skew/fail-open). */
  resolve?: Record<string, unknown>;
}

/**
 * Run the thin L1 block with a stubbed `tenant-resolve` helper (or no helper).
 * HOME is pinned to the cwd and PATH is minimal so the HELPER-pick only finds
 * our CLAUDE_PLUGIN_ROOT stub — never a real axhub-helpers on the dev machine.
 */
function runThinBlock(opts: ThinOpts): { stdout: string; cacheTenant: string | null } {
  const { cwd, env = {}, resolve } = opts;

  let pluginRoot = "";
  if (resolve !== undefined) {
    pluginRoot = join(cwd, "_plugin");
    mkdirSync(join(pluginRoot, "bin"), { recursive: true });
    writeFileSync(
      join(pluginRoot, "bin", "axhub-helpers"),
      `#!/bin/sh\nif [ "$1" = "tenant-resolve" ]; then\ncat <<'TR_JSON'\n${JSON.stringify(resolve)}\nTR_JSON\nexit 0\nfi\nexit 0\n`,
      { mode: 0o755 },
    );
  }

  const trailer = `\necho "AXHUB_TENANT=$AXHUB_TENANT"\necho "NEEDS_PICK=$NEEDS_PICK"\n`;
  const scriptPath = join(cwd, "_tp_thin.sh");
  writeFileSync(scriptPath, `#!/bin/sh\n${L1_BASH}${trailer}`, { mode: 0o755 });

  const result = spawnSync("sh", [scriptPath], {
    cwd,
    env: {
      ...process.env,
      PATH: "/usr/bin:/bin",
      HOME: cwd, // no ~/.claude/plugins/cache so the glob fallback finds nothing
      CLAUDE_PLUGIN_ROOT: pluginRoot, // "" when no helper → HELPER stays empty → fail-open
      AXHUB_TENANT: "",
      CI: "",
      CLAUDE_NON_INTERACTIVE: "",
      ...env,
    },
    encoding: "utf8",
  });

  return {
    stdout: (result.stdout as string) ?? "",
    cacheTenant: cacheTenantOf(cwd),
  };
}

const NOW_SECS = Math.floor(Date.now() / 1000);
const TENANT_A = { id: "tenant-alpha", slug: "alpha", name: "Alpha Corp" };
const TENANT_B = { id: "tenant-beta", slug: "beta", name: "Beta Corp" };

// ════════════════════════════════════════════════════════════════
// 구조: sentinel / bash 추출
// ════════════════════════════════════════════════════════════════
describe("tenant-picker-block constants", () => {
  test("L1 block contains axhub-tenant-picker:L1 sentinel", () => {
    expect(CANONICAL_TENANT_PICKER_BLOCK).toContain("axhub-tenant-picker:L1");
  });

  test("L1 block references .axhub/state/tenant.json (cross-fence marker)", () => {
    expect(CANONICAL_TENANT_PICKER_BLOCK).toContain(".axhub/state/tenant.json");
  });

  test("L1 bash calls the Rust helper and carries NO bash arithmetic", () => {
    expect(L1_BASH.length).toBeGreaterThan(200);
    expect(L1_BASH).toContain("tenant-resolve --json");
    // 위험 로직(산술/TTL)은 Rust 로 이관 — 블록에 산술이 남으면 안 돼요
    expect(L1_BASH).not.toContain("$(( ");
    expect(L1_BASH).not.toContain("_AGE=");
  });

  test("L1 bash includes non-TTY predicate (R4 guard stays in bash)", () => {
    expect(L1_BASH).toContain("[ -t 1 ]");
    expect(L1_BASH).toContain("CI");
    expect(L1_BASH).toContain("CLAUDE_NON_INTERACTIVE");
  });

  test("L1 bash includes multi-tenant fallback warning line (R4 guard)", () => {
    expect(L1_BASH).toContain("여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant");
  });
});

// ════════════════════════════════════════════════════════════════
// thin block × helper 통합 — 모든 block-resolved 분기 + no-loop + persist
// ════════════════════════════════════════════════════════════════
describe("tenant-picker thin block — helper integration", () => {
  test("auto resolve → AXHUB_TENANT set, NEEDS_PICK=false, cache persisted", () => {
    const cwd = makeTmpDir("thin-auto");
    try {
      const { stdout, cacheTenant } = runThinBlock({
        cwd,
        resolve: { tenant: TENANT_A.id, source: "auto", needs_pick: false, candidates: [] },
      });
      expect(stdout).toContain(`AXHUB_TENANT=${TENANT_A.id}`);
      expect(stdout).toContain("NEEDS_PICK=false");
      expect(cacheTenant).toBe(TENANT_A.id);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("multi-tenant non-TTY → R4 warning + first candidate + persist + NEEDS_PICK=false (no-loop)", () => {
    const cwd = makeTmpDir("thin-multi");
    try {
      const { stdout, cacheTenant } = runThinBlock({
        cwd,
        resolve: { tenant: "", source: "list", needs_pick: true, candidates: [TENANT_A, TENANT_B] },
      });
      expect(stdout).toContain("여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant");
      expect(stdout).toContain(`AXHUB_TENANT=${TENANT_A.id}`);
      // no-loop: non-TTY 는 needs_pick 을 false 로 리셋 (재프롬프트 안 함)
      expect(stdout).toContain("NEEDS_PICK=false");
      // Finding 1: non-TTY 멀티 분기도 영속화 (이전 blind spot)
      expect(cacheTenant).toBe(TENANT_A.id);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("explicit AXHUB_TENANT env → helper skipped (Precedence 1), persisted", () => {
    const cwd = makeTmpDir("thin-override");
    try {
      const { stdout, cacheTenant } = runThinBlock({
        cwd,
        resolve: { tenant: "should-not-be-used", source: "auto", needs_pick: false, candidates: [] },
        env: { AXHUB_TENANT: "explicit-x" },
      });
      expect(stdout).toContain("AXHUB_TENANT=explicit-x");
      expect(stdout).not.toContain("should-not-be-used");
      // Finding 1: explicit-env 분기도 영속화 (이전 blind spot)
      expect(cacheTenant).toBe("explicit-x");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("empty resolve → empty tenant, NEEDS_PICK=false, NO cache (no empty persist)", () => {
    const cwd = makeTmpDir("thin-empty");
    try {
      const { stdout, cacheTenant } = runThinBlock({
        cwd,
        resolve: { tenant: "", source: "", needs_pick: false, candidates: [] },
      });
      expect(stdout).toContain("AXHUB_TENANT=\n");
      expect(stdout).toContain("NEEDS_PICK=false");
      expect(cacheTenant).toBeNull();
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("absent helper (skew) → fail-open empty, NEEDS_PICK=false, NO cache (no re-prompt loop)", () => {
    const cwd = makeTmpDir("thin-nohelper");
    try {
      const { stdout, cacheTenant } = runThinBlock({ cwd }); // resolve undefined → 헬퍼 없음
      expect(stdout).toContain("AXHUB_TENANT=\n");
      expect(stdout).toContain("NEEDS_PICK=false");
      expect(cacheTenant).toBeNull();
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("needs_pick=true under non-TTY never re-prompts and never leaves an unpersisted resolved tenant", () => {
    const cwd = makeTmpDir("thin-noloop");
    try {
      // 두 번 연속 실행해도 매번 동일하게 first candidate 로 영속화 (loop 없음)
      const r1 = runThinBlock({
        cwd,
        resolve: { tenant: "", source: "list", needs_pick: true, candidates: [TENANT_B, TENANT_A] },
      });
      expect(r1.stdout).toContain("NEEDS_PICK=false");
      expect(r1.cacheTenant).toBe(TENANT_B.id);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });
});

// ════════════════════════════════════════════════════════════════
// re-read snippet — command fence 가 캐시를 다시 읽어 AXHUB_TENANT 복원
// (이 테스트가 '--tenant ""' 회귀를 잡는 behavioral 가드예요)
// ════════════════════════════════════════════════════════════════
describe("tenant-picker re-read snippet — command fence resolves cache (bug-class guard)", () => {
  function runReread(cwd: string, env: Record<string, string> = {}): string {
    const scriptPath = join(cwd, "_reread.sh");
    writeFileSync(
      scriptPath,
      `#!/bin/sh\n${CANONICAL_TENANT_PICKER_REREAD}\necho "AXHUB_TENANT=$AXHUB_TENANT"\n`,
      { mode: 0o755 },
    );
    const r = spawnSync("sh", [scriptPath], {
      cwd,
      env: { ...process.env, AXHUB_TENANT: "", ...env },
      encoding: "utf8",
    });
    return (r.stdout as string) ?? "";
  }

  test("populated cache → re-read resolves non-empty AXHUB_TENANT equal to cached tenant", () => {
    const cwd = makeTmpDir("rr-hit");
    try {
      writeTenantCache(cwd, { tenant: "picked-tenant-id", source: "picker", ts: NOW_SECS });
      const stdout = runReread(cwd);
      expect(stdout).toContain("AXHUB_TENANT=picked-tenant-id");
      expect(stdout).not.toContain("AXHUB_TENANT=\n");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("explicit env override wins over cache in re-read", () => {
    const cwd = makeTmpDir("rr-ov");
    try {
      writeTenantCache(cwd, { tenant: "cached-id", source: "picker", ts: NOW_SECS });
      const stdout = runReread(cwd, { AXHUB_TENANT: "explicit-id" });
      expect(stdout).toContain("AXHUB_TENANT=explicit-id");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("thin block resolve → next fence re-read inherits the persisted tenant (Finding 1, e2e)", () => {
    const cwd = makeTmpDir("rr-e2e");
    try {
      // Fence 1: thin block auto-resolves + persists
      runThinBlock({
        cwd,
        resolve: { tenant: TENANT_A.id, source: "auto", needs_pick: false, candidates: [] },
      });
      // Fence 2: re-read only (no helper, no env) → 캐시에서 TENANT_A 복원
      const stdout = runReread(cwd);
      expect(stdout).toContain(`AXHUB_TENANT=${TENANT_A.id}`);
      expect(stdout).not.toContain("AXHUB_TENANT=\n");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });
});

// ════════════════════════════════════════════════════════════════
// inline 치환 — 메뉴 fence 가 cherry-pick 되어 standalone 실행돼도
// --tenant 값이 cache 에서 inline 으로 채워져야 해요 (resolution travels with command)
// ════════════════════════════════════════════════════════════════
describe("tenant-picker inline substitution — resolves cache at point of use (menu fence)", () => {
  function runInline(cwd: string): { stdout: string; status: number } {
    const scriptPath = join(cwd, "_inline.sh");
    writeFileSync(
      scriptPath,
      `#!/bin/sh\necho "--tenant \\"${CANONICAL_TENANT_PICKER_INLINE}\\""\n`,
      { mode: 0o755 },
    );
    const r = spawnSync("sh", [scriptPath], { cwd, env: { ...process.env }, encoding: "utf8" });
    return { stdout: (r.stdout as string) ?? "", status: r.status ?? 0 };
  }

  test("inline $(jq ...) resolves cached tenant into a --tenant flag (no top resolver needed)", () => {
    const cwd = makeTmpDir("inline-hit");
    try {
      writeTenantCache(cwd, { tenant: "menu-tenant-id", source: "picker", ts: NOW_SECS });
      const { stdout } = runInline(cwd);
      expect(stdout).toContain('--tenant "menu-tenant-id"');
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("inline $(jq ...) degrades to empty (no crash) when cache absent", () => {
    const cwd = makeTmpDir("inline-miss");
    try {
      const { stdout, status } = runInline(cwd);
      expect(stdout).toContain('--tenant ""');
      expect(status).toBe(0);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });
});
