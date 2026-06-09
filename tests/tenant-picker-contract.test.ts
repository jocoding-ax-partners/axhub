/**
 * A7: tenant-picker contract test (migrate-skill-contract 패턴).
 *
 * 두 계층 검증:
 *   (a) Structural — tenant-target-skills.json 등재 skill 마다 L1/L2 sentinel + marker 존재.
 *   (b) Structural — AC2 per-call --tenant thread: 각 axhub tenant-scoped 명령 라인에
 *       --tenant 존재 (whole-file toContain 금지 — 라인 단위 regex).
 *   (c) Behavioral — doctor --strict 가 manifest 등재 skill 에서 0 exit (실행 검증).
 *
 * Note: behavioral 골든-stdout (캐시 read-after-write, 멤버십 분기) 은 A2 의
 *       tenant-picker-block.test.ts 가 커버해요 — 이 파일은 SKILL.md 통합을 잠가요.
 */

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const REPO_ROOT = join(import.meta.dir, "..");
const read = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

// ── manifest ────────────────────────────────────────────────────────────────

interface TenantTargetManifest {
  tenant_picker_targets: string[];
}

const manifest = JSON.parse(read("scripts/tenant-target-skills.json")) as TenantTargetManifest;
const TARGETS = manifest.tenant_picker_targets;

// ── bash fence 추출 헬퍼 ─────────────────────────────────────────────────────

/** SKILL.md 에서 모든 bash fence 안의 라인을 추출해요 (prose 제외). */
function extractBashLines(content: string): string[] {
  const lines = content.split("\n");
  const result: string[] = [];
  let inBash = false;
  for (const line of lines) {
    if (line.trim() === "```bash") { inBash = true; continue; }
    if (inBash && line.trim() === "```") { inBash = false; continue; }
    if (inBash) result.push(line);
  }
  return result;
}

/** SKILL.md 에서 bash fence 를 fence 단위로 묶어 추출해요 (cross-fence 검증용). */
function extractBashFences(content: string): string[][] {
  const lines = content.split("\n");
  const fences: string[][] = [];
  let cur: string[] | null = null;
  for (const line of lines) {
    const t = line.trim();
    if (t === "```bash") { cur = []; continue; }
    if (cur !== null && t === "```") { fences.push(cur); cur = null; continue; }
    if (cur !== null) cur.push(line);
  }
  return fences;
}

/**
 * tenant-scoped axhub 서브명령 패턴 — 이 라인들은 반드시 --tenant 를 가져야 해요.
 *
 * 포함 (tenant-level 리소스 생성·조회):
 *   axhub deploy create, axhub apps bootstrap, axhub apps bootstrap-status, axhub apps templates list
 *
 * 제외 (--app "$APP_ID" 로 이미 앱 레벨 스코프 확정 — tenant implicit):
 *   deploy status, deploy cancel, deploy list, apps git connect, apps git status
 * 제외 (tenant-neutral 조회):
 *   axhub --version, axhub tenants list, axhub github accounts list, axhub auth, axhub open
 */
const TENANT_SCOPED_PATTERN =
  /^\s*axhub\s+(deploy\s+create\b|apps\s+(bootstrap|bootstrap-status|templates\s+list)\b)/;

// ════════════════════════════════════════════════════════════════
// (a) Structural — L1/L2 sentinel + marker
// ════════════════════════════════════════════════════════════════

describe("tenant-picker structural contract — L1/L2 sentinel + marker", () => {
  for (const slug of TARGETS) {
    const skillPath = `skills/${slug}/SKILL.md`;

    test(`${slug}: L1 sentinel (axhub-tenant-picker:L1) present`, () => {
      const content = read(skillPath);
      expect(content).toContain("axhub-tenant-picker:L1");
    });

    test(`${slug}: L1 has .axhub/state/tenant.json re-read marker`, () => {
      const content = read(skillPath);
      expect(content).toContain(".axhub/state/tenant.json");
    });

    test(`${slug}: L2 sentinel (axhub-tenant-picker:L2) present`, () => {
      const content = read(skillPath);
      expect(content).toContain("axhub-tenant-picker:L2");
    });

    test(`${slug}: L2 has .axhub/state/tenant.json write-back marker`, () => {
      const content = read(skillPath);
      // L2 stanza는 write-back 지시를 포함해요 (L1 re-read marker와 동일 문자열)
      expect(content).toContain(".axhub/state/tenant.json");
    });
  }
});

// ════════════════════════════════════════════════════════════════
// (b) Structural — AC2 per-call --tenant thread (라인 단위 regex)
// ════════════════════════════════════════════════════════════════

describe("tenant-picker structural contract — AC2 per-call --tenant thread", () => {
  for (const slug of TARGETS) {
    const skillPath = `skills/${slug}/SKILL.md`;

    test(`${slug}: every tenant-scoped axhub command line carries --tenant`, () => {
      const content = read(skillPath);
      const bashLines = extractBashLines(content);
      const tenantScopedLines = bashLines.filter((line) => TENANT_SCOPED_PATTERN.test(line));

      // tenant-level 명령(TENANT_SCOPED_PATTERN)이 있으면 모두 --tenant 를 가져야 해요.
      // app-scoped-only skill(env/logs/rollback/publish/app-lifecycle 등)은 tenant-level
      // 명령이 없어도 통과해요 — picker 블록(structural a)은 session-cache tenant 연속성을
      // 위해 존재하지만, --app "$APP_ID" 명령은 tenant 가 implicit 이라 --tenant 를 강제하지
      // 않아요 (Phase B 2-tier: tenant-level 은 thread, app-scoped 은 block-only).
      for (const line of tenantScopedLines) {
        // 라인 단위 assert — whole-file toContain 이 아니에요
        expect(line).toContain("--tenant");
      }
    });
  }
});

// ════════════════════════════════════════════════════════════════
// (b2) Cross-fence invariant — $AXHUB_TENANT 를 쓰는 fence 는 같은 fence 에서 대입
// ════════════════════════════════════════════════════════════════
//
// Bash tool 계약상 fence 간 shell env (export 포함) 는 휘발해요. 따라서 L1 블록이
// export 한 AXHUB_TENANT 는 다음 fence 에서 사라져요. tenant-scoped 명령 fence 가
// $AXHUB_TENANT 를 bare 참조하면 빈 값(`--tenant ""`)으로 실행돼요 — picker 선택 증발.
// 이 invariant 는 "$AXHUB_TENANT 를 참조하는 모든 bash fence 는 같은 fence 안에서
// AXHUB_TENANT= 대입(L1 resolver 또는 re-read snippet)을 가져야 한다"를 강제해요.
// 이 테스트가 곧 그 버그 클래스를 잡는 behavioral 보증의 정적 짝이에요.

// tenant 변수 2종을 모두 검사해요. AXHUB_TENANT (picker 정규 변수) 와 TENANT
// (grounding fence 가 파생하는 별칭) 둘 다 cross-fence 로 휘발하므로, 둘 중 하나라도
// 같은 fence 에서 대입 없이 참조되면 '--tenant ""' 가 돼요.
// word-boundary 로 TENANT_CACHE / _TENANTS_JSON / AXHUB_TENANT 오탐을 막아요.
const TENANT_VARS = [
  { name: "AXHUB_TENANT", use: /\$\{?AXHUB_TENANT\b/, assign: /(^|\s)AXHUB_TENANT=/ },
  { name: "TENANT", use: /\$\{?TENANT\b/, assign: /(^|\s)TENANT=/ },
] as const;

describe("tenant-picker cross-fence invariant — no bare tenant var", () => {
  for (const slug of TARGETS) {
    const skillPath = `skills/${slug}/SKILL.md`;

    test(`${slug}: every bash fence using a tenant var assigns it in-fence`, () => {
      const content = read(skillPath);
      const fences = extractBashFences(content);

      for (const fence of fences) {
        for (const v of TENANT_VARS) {
          const uses = fence.some((l) => v.use.test(l));
          if (!uses) continue;

          // 같은 fence 에 VAR= 대입(L1 resolver / re-read / re-derive)이 있어야 해요.
          // `export AXHUB_TENANT`(=없음)는 대입이 아니에요. inline `--tenant "$(jq ...)"`
          // 형태는 $VAR 참조가 없으므로 애초에 여기 걸리지 않아요 (메뉴 fence 의 정답).
          const assigns = fence.some((l) => v.assign.test(l));
          const preview = fence.join("\n").slice(0, 500);
          expect(
            assigns,
            `${slug}: bash fence references $${v.name} without in-fence ${v.name}= assignment ` +
              `(fence env 휘발 → '--tenant \"\"' 위험). 메뉴 fence 는 inline \`--tenant "$(jq ...)"\`, ` +
              `sequential fence 는 re-read/re-derive 를 fence 상단에 추가해요.\n--- fence ---\n${preview}`,
          ).toBe(true);
        }
      }
    });
  }
});

// ════════════════════════════════════════════════════════════════
// (c) Behavioral — skill:doctor --strict 가 manifest skill 을 통과해요
// ════════════════════════════════════════════════════════════════

describe("tenant-picker behavioral contract — doctor --strict gate", () => {
  test("bun run skill:doctor --strict exits 0 (tenant-picker L1/L2 checks pass)", () => {
    const result = spawnSync(
      "bun",
      ["run", "scripts/skill-doctor.ts", "--strict"],
      { cwd: REPO_ROOT, timeout: 60_000, encoding: "utf8" },
    );
    if (result.status !== 0) {
      // 실패 시 doctor 출력을 그대로 노출해요
      throw new Error(
        `skill:doctor --strict exited ${result.status}:\n${result.stdout}\n${result.stderr}`,
      );
    }
    expect(result.status).toBe(0);
  });

  test("manifest targets include init and deploy (Phase A baseline)", () => {
    expect(TARGETS).toContain("init");
    expect(TARGETS).toContain("deploy");
  });
});
