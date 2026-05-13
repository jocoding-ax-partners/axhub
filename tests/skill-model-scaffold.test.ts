// Phase 25 PR 25.5a — scaffold + skill-doctor `model:` frontmatter support.
//
// Two surfaces under test:
//   1. `scripts/skill-new.ts --model <name>` emits `model: <name>` in
//      frontmatter; default = sonnet; invalid value rejected.
//   2. `scripts/skill-doctor.ts` keeps the 19 production SKILLs passing
//      (no `model:` declared yet — bulk migration arrives in 25.5b/25.5c)
//      and flags an invalid model value when one is declared.

import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const root = join(import.meta.dir, "..");
const registryPath = join(root, "tests/fixtures/ask-defaults/registry.json");

function uniqueSlug(label: string) {
  return `axhub-25-5a-${label}-${Date.now().toString(36)}`;
}

function readRegistry(): Record<string, unknown> {
  return JSON.parse(readFileSync(registryPath, "utf8"));
}

function writeRegistry(data: Record<string, unknown>) {
  writeFileSync(registryPath, JSON.stringify(data, null, 2) + "\n");
}

function cleanupScaffold(slug: string) {
  const dir = join(root, "skills", slug);
  if (existsSync(dir)) {
    rmSync(dir, { recursive: true, force: true });
  }
  const reg = readRegistry();
  if (slug in reg) {
    const { [slug]: _, ...rest } = reg;
    writeRegistry(rest);
  }
}

function runScaffold(args: string[]) {
  return spawnSync("bun", ["run", "scripts/skill-new.ts", ...args], {
    cwd: root,
    encoding: "utf8",
  });
}

describe("Phase 25 PR 25.5a scaffold --model flag", () => {
  const cleanupSlugs: string[] = [];
  afterEach(() => {
    while (cleanupSlugs.length > 0) {
      cleanupScaffold(cleanupSlugs.pop()!);
    }
  });

  test("--model haiku emits model: haiku in frontmatter", () => {
    const slug = uniqueSlug("haiku");
    cleanupSlugs.push(slug);
    const out = runScaffold([slug, "--model", "haiku"]);
    expect(out.status).toBe(0);
    expect(out.stdout).toContain("model: haiku");

    const fm = readFileSync(join(root, "skills", slug, "SKILL.md"), "utf8");
    expect(fm).toContain("model: haiku\n");
  });

  test("default (no --model) emits model: sonnet", () => {
    const slug = uniqueSlug("default");
    cleanupSlugs.push(slug);
    const out = runScaffold([slug]);
    expect(out.status).toBe(0);
    const fm = readFileSync(join(root, "skills", slug, "SKILL.md"), "utf8");
    expect(fm).toContain("model: sonnet\n");
  });

  test("--model opus emits model: opus (allowed value even if rarely used)", () => {
    const slug = uniqueSlug("opus");
    cleanupSlugs.push(slug);
    const out = runScaffold([slug, "--model", "opus"]);
    expect(out.status).toBe(0);
    const fm = readFileSync(join(root, "skills", slug, "SKILL.md"), "utf8");
    expect(fm).toContain("model: opus\n");
  });

  test("invalid --model value rejected with exit 1", () => {
    const slug = uniqueSlug("invalid");
    const out = runScaffold([slug, "--model", "ultra"]);
    expect(out.status).toBe(1);
    expect(out.stderr).toContain("must be one of haiku|sonnet|opus");
    expect(existsSync(join(root, "skills", slug))).toBe(false);
  });
});

describe("Phase 25 PR 25.5a skill-doctor model validation", () => {
  test("strict mode keeps current 19 production SKILLs passing (no-op)", () => {
    // 핵심 약속: 기존 SKILL.md 변경 0. doctor 가 model 미선언 상태도 통과해야 해요.
    const result = spawnSync("bun", ["scripts/skill-doctor.ts", "--strict"], {
      cwd: root,
      encoding: "utf8",
    });
    expect(result.status).toBe(0);
    expect(result.stdout).not.toContain("model routing");
  });

  test("non-strict mode marks model field as exempt for existing SKILLs", () => {
    const result = spawnSync("bun", ["scripts/skill-doctor.ts"], {
      cwd: root,
      encoding: "utf8",
    });
    expect(result.status).toBe(0);
    // 보고서 본문에 "model" 라벨이 적어도 한 번은 등장해야 해요 (exempt 또는 valid).
    expect(result.stdout).toContain("model routing");
    // 어떤 SKILL 도 "missing" 으로 마킹되면 안 됩니다 (no-op effective).
    expect(result.stdout).not.toContain("model routing missing");
  });

  test("invalid model value triggers strict failure", () => {
    // 임시 SKILL 을 만들어서 invalid model 을 가진 상태에서 doctor 가 실패하는지 확인해요.
    const slug = uniqueSlug("invalid-model");
    try {
      // scaffold 로 valid SKILL 만들고 frontmatter 의 model 값을 손상시켜요.
      const make = runScaffold([slug, "--model", "haiku"]);
      expect(make.status).toBe(0);

      const skillPath = join(root, "skills", slug, "SKILL.md");
      const content = readFileSync(skillPath, "utf8").replace(
        "model: haiku",
        "model: ultra"
      );
      writeFileSync(skillPath, content);

      const result = spawnSync("bun", ["scripts/skill-doctor.ts", "--strict"], {
        cwd: root,
        encoding: "utf8",
      });
      expect(result.status).toBe(1);
      expect(result.stdout).toContain(`skills/${slug}/SKILL.md: missing model routing`);
    } finally {
      cleanupScaffold(slug);
    }
  });
});
