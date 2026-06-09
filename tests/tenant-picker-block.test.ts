/**
 * A2: L1 블록 bun-test 골든 fixture harness (Decision 2C — Rust 무변경).
 *
 * CANONICAL_TENANT_PICKER_BLOCK 을 임시 스크립트로 실행하고 fake-axhub PATH 스텁 +
 * mock 캐시로 golden stdout 을 assert 해요.
 *
 * 커버 분기:
 *   Precedence 1 — explicit AXHUB_TENANT override
 *   Precedence 2 — valid cache re-read / TTL-stale clear
 *   Precedence 3 — count=1 (auto) / count≥2 non-TTY (warning) / count=0 (fallback)
 *   Precedence 4 — preflight current_team_id fallback
 *   Cache read-after-write (AC7-c) — 2회 실행으로 캐시 상속 실행 검증
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

const REPO_ROOT = join(import.meta.dir, "..");
const FIXTURE_BIN = join(REPO_ROOT, "tests/fixtures/tenant-picker/bin");

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
  const dir = join(tmpdir(), `tp-${label}-${Date.now()}`);
  mkdirSync(dir, { recursive: true });
  return dir;
}

function writeTenantCache(cwd: string, content: Record<string, unknown>): void {
  const stateDir = join(cwd, ".axhub/state");
  mkdirSync(stateDir, { recursive: true });
  writeFileSync(join(stateDir, "tenant.json"), JSON.stringify(content));
}

interface RunOpts {
  cwd: string;
  env?: Record<string, string>;
  /** fake-axhub がreturnするtenantリスト。undefined → fixture stub なし */
  fakeTenants?: unknown[];
  preflightJson?: Record<string, unknown>;
}

function runPicker(opts: RunOpts): { stdout: string; exitCode: number } {
  const {
    cwd,
    env = {},
    fakeTenants,
    preflightJson = {},
  } = opts;

  const prelude = `PREFLIGHT_JSON='${JSON.stringify(preflightJson)}'\nexport PREFLIGHT_JSON\n`;
  const trailer = `\necho "AXHUB_TENANT=$AXHUB_TENANT"\necho "NEEDS_PICK=$NEEDS_PICK"\n`;
  const scriptPath = join(cwd, "_tp_test.sh");
  writeFileSync(
    scriptPath,
    `#!/bin/sh\n${prelude}${L1_BASH}${trailer}`,
    { mode: 0o755 },
  );

  const pathPrefix = fakeTenants !== undefined ? `${FIXTURE_BIN}:` : "";
  const result = spawnSync("sh", [scriptPath], {
    cwd,
    env: {
      ...process.env,
      PATH: `${pathPrefix}${process.env.PATH ?? ""}`,
      FAKE_AXHUB_TENANTS_JSON: fakeTenants !== undefined
        ? JSON.stringify(fakeTenants)
        : "[]",
      // subprocess는 non-TTY이므로 CI/CLAUDE_NON_INTERACTIVE 를 명시 unset 해도
      // `! [ -t 1 ]` 가 true → 항상 non-TTY 경로로 진행 (예상 동작)
      CI: "",
      CLAUDE_NON_INTERACTIVE: "",
      ...env,
    },
    encoding: "utf8",
  });

  return {
    stdout: (result.stdout as string) ?? "",
    exitCode: result.status ?? 0,
  };
}

// 재사용 fixture 데이터
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

  test("L1 block references .axhub/state/tenant.json (write-back marker)", () => {
    expect(CANONICAL_TENANT_PICKER_BLOCK).toContain(".axhub/state/tenant.json");
  });

  test("L1 bash block is extractable and non-trivial", () => {
    expect(L1_BASH.length).toBeGreaterThan(200);
    expect(L1_BASH).toContain("AXHUB_TENANT");
    expect(L1_BASH).toContain("axhub tenants list --json");
    expect(L1_BASH).toContain("TENANT_CACHE_TTL");
  });

  test("L1 bash includes non-TTY predicate reusing D1 pattern", () => {
    expect(L1_BASH).toContain('[ -t 1 ]');
    expect(L1_BASH).toContain('CI');
    expect(L1_BASH).toContain('CLAUDE_NON_INTERACTIVE');
  });

  test("L1 bash includes multi-tenant fallback warning line (R4 guard)", () => {
    expect(L1_BASH).toContain("여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant");
  });
});

// ════════════════════════════════════════════════════════════════
// 골든 stdout: precedence 분기
// ════════════════════════════════════════════════════════════════
describe("tenant-picker-block golden stdout — precedence branches", () => {
  test("Precedence 1: explicit AXHUB_TENANT env skips picker entirely", () => {
    const cwd = makeTmpDir("p1");
    try {
      const { stdout } = runPicker({
        cwd,
        env: { AXHUB_TENANT: "explicit-tenant-id" },
        fakeTenants: [TENANT_A, TENANT_B], // ignored: override takes precedence
      });
      expect(stdout).toContain("AXHUB_TENANT=explicit-tenant-id");
      expect(stdout).toContain("NEEDS_PICK=false");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("Precedence 2 (valid cache): cache re-read used without calling axhub", () => {
    const cwd = makeTmpDir("p2-valid");
    try {
      writeTenantCache(cwd, {
        tenant: "cached-tenant-id",
        source: "auto",
        ts: NOW_SECS - 60, // 60초 전 = TTL 내 유효
      });
      // fakeTenants 없음 → fake axhub PATH 없음 → axhub 호출 시 실패
      // but cache hit at Precedence 2 means axhub is never reached
      const { stdout } = runPicker({ cwd });
      expect(stdout).toContain("AXHUB_TENANT=cached-tenant-id");
      expect(stdout).toContain("NEEDS_PICK=false");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("Precedence 2 (TTL-stale): stale cache cleared, falls through to list (count=1)", () => {
    const cwd = makeTmpDir("p2-stale");
    try {
      writeTenantCache(cwd, {
        tenant: "stale-tenant-id",
        source: "auto",
        ts: 0, // epoch → 수십 년 경과 → stale
      });
      const { stdout } = runPicker({ cwd, fakeTenants: [TENANT_A] });
      expect(stdout).toContain(`AXHUB_TENANT=${TENANT_A.id}`);
      expect(stdout).not.toContain("AXHUB_TENANT=stale-tenant-id");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("Precedence 3 (count=1): single tenant auto-selected", () => {
    const cwd = makeTmpDir("p3-one");
    try {
      const { stdout } = runPicker({ cwd, fakeTenants: [TENANT_A] });
      expect(stdout).toContain(`AXHUB_TENANT=${TENANT_A.id}`);
      expect(stdout).toContain("NEEDS_PICK=false");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("Precedence 3 (count≥2, non-TTY): warning line emitted + fallback to first tenant", () => {
    const cwd = makeTmpDir("p3-multi");
    try {
      const { stdout } = runPicker({ cwd, fakeTenants: [TENANT_A, TENANT_B] });
      // R4 fail-wrong guard: 경고 라인 필수
      expect(stdout).toContain("여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant");
      // 첫 번째 tenant 로 fallback
      expect(stdout).toContain(TENANT_A.id);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("Precedence 3 (count≥2, CLAUDE_NON_INTERACTIVE=1): warning line emitted", () => {
    const cwd = makeTmpDir("p3-ni");
    try {
      const { stdout } = runPicker({
        cwd,
        fakeTenants: [TENANT_A, TENANT_B],
        env: { CLAUDE_NON_INTERACTIVE: "1" },
      });
      expect(stdout).toContain("여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("Precedence 4 (count=0): preflight current_team_id fallback used", () => {
    const cwd = makeTmpDir("p4");
    try {
      const { stdout } = runPicker({
        cwd,
        fakeTenants: [], // empty → count=0 → Precedence 4
        preflightJson: { current_team_id: "preflight-team-id" },
      });
      expect(stdout).toContain("AXHUB_TENANT=preflight-team-id");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("Precedence 1 beats valid cache: explicit env wins over cache", () => {
    const cwd = makeTmpDir("p1-beats-cache");
    try {
      writeTenantCache(cwd, {
        tenant: "cached-tenant-id",
        source: "auto",
        ts: NOW_SECS - 60,
      });
      const { stdout } = runPicker({
        cwd,
        env: { AXHUB_TENANT: "override-wins" },
      });
      expect(stdout).toContain("AXHUB_TENANT=override-wins");
      expect(stdout).not.toContain("AXHUB_TENANT=cached-tenant-id");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });
});

// ════════════════════════════════════════════════════════════════
// Cache read-after-write (AC7-c): 상속을 실행으로 증명
// ════════════════════════════════════════════════════════════════
describe("tenant-picker-block cache read-after-write (AC7-c)", () => {
  test("second run inherits cache written by first run (no fake-axhub needed)", () => {
    const cwd = makeTmpDir("raw");
    try {
      // Run 1: count=1 → TENANT_A auto-select + cache write
      const run1 = runPicker({ cwd, fakeTenants: [TENANT_A] });
      expect(run1.stdout).toContain(`AXHUB_TENANT=${TENANT_A.id}`);

      // 캐시 파일이 실제로 기록됐는지 검증
      const cacheFile = join(cwd, ".axhub/state/tenant.json");
      expect(existsSync(cacheFile)).toBe(true);

      // Run 2: fake-axhub PATH 없음 → Precedence 2 (cache) 가 먼저 히트
      // axhub list 는 실행되지 않으므로 real axhub 설치 여부 무관
      const scriptPath = join(cwd, "_tp_test2.sh");
      const prelude2 = `PREFLIGHT_JSON='{}'\nexport PREFLIGHT_JSON\n`;
      const trailer2 = `\necho "AXHUB_TENANT=$AXHUB_TENANT"\necho "NEEDS_PICK=$NEEDS_PICK"\n`;
      writeFileSync(scriptPath, `#!/bin/sh\n${prelude2}${L1_BASH}${trailer2}`, { mode: 0o755 });

      const run2 = spawnSync("sh", [scriptPath], {
        cwd,
        env: {
          ...process.env,
          // FIXTURE_BIN 제외 → fake axhub 없음
          PATH: (process.env.PATH ?? "").split(":").filter((p) => p !== FIXTURE_BIN).join(":"),
          CI: "",
          CLAUDE_NON_INTERACTIVE: "",
        },
        encoding: "utf8",
      });

      const stdout2 = (run2.stdout as string) ?? "";
      // 캐시 상속: run1이 쓴 tenant 를 run2가 axhub 없이 재사용
      expect(stdout2).toContain(`AXHUB_TENANT=${TENANT_A.id}`);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("TTL-expired cache is not inherited (cleared before axhub call)", () => {
    const cwd = makeTmpDir("raw-stale");
    try {
      writeTenantCache(cwd, { tenant: "stale-id", source: "auto", ts: 0 });

      const run = runPicker({ cwd, fakeTenants: [TENANT_B] });
      expect(run.stdout).toContain(`AXHUB_TENANT=${TENANT_B.id}`);
      expect(run.stdout).not.toContain("stale-id");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });
});

// ════════════════════════════════════════════════════════════════
// cache-write ALL branches — L1 이 모든 분기에서 캐시를 영속화해야
// command fence 의 re-read 가 빈 값이 아니에요 (re-read 와 패키지로 동작)
// ════════════════════════════════════════════════════════════════
describe("tenant-picker-block cache-write all branches (cross-fence source of truth)", () => {
  const readCacheTenant = (cwd: string): string =>
    (JSON.parse(
      readFileSync(join(cwd, ".axhub/state/tenant.json"), "utf8"),
    ) as { tenant: string }).tenant;

  test("count=1 auto branch persists cache (.tenant = sole tenant)", () => {
    const cwd = makeTmpDir("w-auto");
    try {
      runPicker({ cwd, fakeTenants: [TENANT_A] });
      expect(existsSync(join(cwd, ".axhub/state/tenant.json"))).toBe(true);
      expect(readCacheTenant(cwd)).toBe(TENANT_A.id);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("count≥2 non-TTY fallback branch persists cache (.tenant = first tenant)", () => {
    const cwd = makeTmpDir("w-multi");
    try {
      runPicker({ cwd, fakeTenants: [TENANT_A, TENANT_B] });
      expect(existsSync(join(cwd, ".axhub/state/tenant.json"))).toBe(true);
      expect(readCacheTenant(cwd)).toBe(TENANT_A.id);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("count=0 preflight fallback branch persists cache (.tenant = current_team_id)", () => {
    const cwd = makeTmpDir("w-pre");
    try {
      runPicker({
        cwd,
        fakeTenants: [],
        preflightJson: { current_team_id: "preflight-team-id" },
      });
      expect(existsSync(join(cwd, ".axhub/state/tenant.json"))).toBe(true);
      expect(readCacheTenant(cwd)).toBe("preflight-team-id");
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
      writeTenantCache(cwd, {
        tenant: "picked-tenant-id",
        source: "picker",
        ts: NOW_SECS,
      });
      const stdout = runReread(cwd);
      expect(stdout).toContain("AXHUB_TENANT=picked-tenant-id");
      // 빈 값(--tenant "") 회귀 가드: 명시적으로 non-empty 라인 확인
      expect(stdout).not.toContain("AXHUB_TENANT=\n");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("explicit env override wins over cache in re-read", () => {
    const cwd = makeTmpDir("rr-ov");
    try {
      writeTenantCache(cwd, {
        tenant: "cached-id",
        source: "picker",
        ts: NOW_SECS,
      });
      const stdout = runReread(cwd, { AXHUB_TENANT: "explicit-id" });
      expect(stdout).toContain("AXHUB_TENANT=explicit-id");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("L1 resolve → re-read inherits across simulated fences (end-to-end)", () => {
    const cwd = makeTmpDir("rr-e2e");
    try {
      // Fence 1: L1 이 count=1 로 resolve → 모든 분기 캐시 영속화
      runPicker({ cwd, fakeTenants: [TENANT_A] });
      // Fence 2: re-read 만 (env 없음, fake-axhub 없음) → 캐시에서 TENANT_A 복원해야 해요
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
    // 상단 resolver 없이 inline 치환식만으로 --tenant 값이 채워지는지 확인해요
    writeFileSync(
      scriptPath,
      `#!/bin/sh\necho "--tenant \\"${CANONICAL_TENANT_PICKER_INLINE}\\""\n`,
      { mode: 0o755 },
    );
    const r = spawnSync("sh", [scriptPath], {
      cwd,
      env: { ...process.env },
      encoding: "utf8",
    });
    return { stdout: (r.stdout as string) ?? "", status: r.status ?? 0 };
  }

  test("inline $(jq ...) resolves cached tenant into a --tenant flag (no top resolver needed)", () => {
    const cwd = makeTmpDir("inline-hit");
    try {
      writeTenantCache(cwd, {
        tenant: "menu-tenant-id",
        source: "picker",
        ts: NOW_SECS,
      });
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
      // 캐시 없으면 빈 값 — crash 하지 않고 graceful (정상 경로는 L1 이 preflight 에서 채움)
      expect(stdout).toContain('--tenant ""');
      expect(status).toBe(0);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });
});
