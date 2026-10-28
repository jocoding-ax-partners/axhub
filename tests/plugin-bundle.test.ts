import { describe, expect, test } from "bun:test";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, relative } from "node:path";

import { POLICY_WRAPPER_SCRIPT, SESSION_WRAPPER_SCRIPTS } from "./fixtures/host-expectations";

const REPO_ROOT = join(import.meta.dir, "..");
const MARKETPLACE_BUNDLE_DIR = join(REPO_ROOT, "plugins", "axhub");
const SKILLS = ["onboarding", "bootstrap", "scaffold", "plugins", "deploy", "up", "import", "development", "diagnosis", "clarity", "update"] as const;
const FORBIDDEN_PARTS = new Set([
  ".DS_Store",
  ".axhub-state",
  ".claude",
  ".codegraph",
  ".git",
  ".github",
  ".gitnexus",
  ".omc",
  ".omx",
  ".qa-live",
  "AGENTS.md",
  "CHANGELOG.md",
  "CLAUDE.md",
  "dist",
  "graphify-out",
  "node_modules",
  "package.json",
  "scripts",
  "test-results.json",
  "tests",
]);

const walk = (dir: string): string[] => {
  const files: string[] = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      files.push(...walk(path));
    } else if (stat.isFile()) {
      files.push(path);
    }
  }
  return files;
};

describe("clean plugin bundle", () => {
  test("builds only the runtime plugin surface", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "axhub-plugin-bundle-"));
    const outDir = join(tempRoot, "bundle");
    try {
      const result = Bun.spawnSync({
        cmd: ["bun", "scripts/build-plugin-bundle.ts", "--out", outDir, "--json"],
        cwd: REPO_ROOT,
        stdout: "pipe",
        stderr: "pipe",
      });

      expect(result.exitCode, result.stderr.toString()).toBe(0);
      const stats = JSON.parse(result.stdout.toString()) as { bytes: number; files: number };
      expect(stats.files).toBeGreaterThan(8);
      // 개발 산출물(node_modules·dist 등)이 섞이면 MB 단위로 커져요 — 이 상한은 그걸 잡는
      // sanity 값이에요. 512 KiB → 544 KiB: cli-feedback 후속(#405·#387·#386·#830·#832·#455)의
      // 스킬·reference 안내가 늘어 실측 약 527 KB 가 됐어요. 544 KiB → 576 KiB: WP-I1
      // (P1-13·P2-33~P3-47) 의 skill·reference 안내(verify 대기 timeout, update 판정, 작업공간
      // 선택·scaffold placeholder reference 등)로 약 12 KB 늘었고, WP-H 가
      // skills/plugins/references/publish.md 를 더해요. 576 KiB → 592 KiB: 조직 정책 runtime
      // 추가 후 소스 실측 595,038B(54 files), HEAD 대비 +15,274B예요. POLICY +5,863B,
      // README +3,237B, policy wrapper +3,231B, deploy reference +1,695B, onboarding
      // reference +657B, hook wiring +587B, 나머지 prompt 순증 +4B이며 개발 산출물은 없어요.
      // 필수 배포 자산의 총량만 수용해요. prompt 총량·per-skill·8,000B gate 상한은 그대로예요.
      expect(stats.bytes).toBeLessThan(592 * 1024);

      expect(existsSync(join(outDir, ".claude-plugin", "plugin.json"))).toBe(true);
      expect(existsSync(join(outDir, "README.md"))).toBe(true);
      expect(existsSync(join(outDir, "LICENSE"))).toBe(true);
      expect(existsSync(join(outDir, "POLICY.md"))).toBe(true);
      expect(existsSync(join(outDir, "hooks", "hooks.json"))).toBe(true);
      expect(existsSync(join(outDir, "hooks", "update-router.sh"))).toBe(true);
      // KTD6: SessionStart 인라인 command 를 추출한 wrapper 5개도 번들에 실려요
      for (const wrapper of SESSION_WRAPPER_SCRIPTS) {
        expect(existsSync(join(outDir, "hooks", wrapper)), `missing bundled wrapper: ${wrapper}`).toBe(true);
      }
      expect(readFileSync(join(outDir, "hooks", POLICY_WRAPPER_SCRIPT), "utf8")).toBe(
        readFileSync(join(REPO_ROOT, "hooks", POLICY_WRAPPER_SCRIPT), "utf8"),
      );
      expect(readFileSync(join(outDir, "hooks", "hooks.json"), "utf8")).toBe(
        readFileSync(join(REPO_ROOT, "hooks", "hooks.json"), "utf8"),
      );
      expect(existsSync(join(outDir, "hooks", "import-router.sh"))).toBe(false);
      expect(existsSync(join(outDir, "hooks", "clarity-router.sh"))).toBe(false);
      expect(existsSync(join(outDir, "hooks", "status-resume-router.sh"))).toBe(false);
      expect(readFileSync(join(outDir, "POLICY.md"), "utf8")).toContain("플러그인 스킬 흐름은 그 도구를 우선 사용하지 않아요");
      // kill switch 본문은 스크립트로 추출됐어요 — hooks.json 은 위임만 가져요
      expect(readFileSync(join(outDir, "hooks", "update-router.sh"), "utf8")).toContain("AXHUB_NO_UPDATE_ROUTER");
      expect(readFileSync(join(outDir, "hooks", "hooks.json"), "utf8")).toContain("update-router.sh");
      expect(readFileSync(join(outDir, "hooks", "hooks.json"), "utf8")).not.toContain("import-router.sh");

      const rootManifest = JSON.parse(readFileSync(join(REPO_ROOT, ".claude-plugin", "plugin.json"), "utf8")) as { version: string };
      const bundledManifest = JSON.parse(readFileSync(join(outDir, ".claude-plugin", "plugin.json"), "utf8")) as { version: string };
      expect(bundledManifest.version).toBe(rootManifest.version);
      const bundledMarketplace = JSON.parse(
        readFileSync(join(outDir, ".claude-plugin", "marketplace.json"), "utf8"),
      ) as { plugins: Array<{ source?: string }> };
      expect(bundledMarketplace.plugins[0]?.source).toBe(".");

      for (const skill of SKILLS) {
        expect(existsSync(join(outDir, "skills", skill, "SKILL.md")), `missing bundled skill: ${skill}`).toBe(true);
      }

      const relativeFiles = walk(outDir).map((file) => relative(outDir, file));
      for (const file of relativeFiles) {
        const parts = file.split("/");
        expect(parts.some((part) => FORBIDDEN_PARTS.has(part)), `forbidden bundle file: ${file}`).toBe(false);
        expect(FORBIDDEN_PARTS.has(basename(file)), `forbidden bundle file: ${file}`).toBe(false);
      }
    } finally {
      rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("marketplace points at the committed clean runtime bundle", () => {
    const marketplace = JSON.parse(readFileSync(join(REPO_ROOT, ".claude-plugin", "marketplace.json"), "utf8")) as {
      plugins: Array<{ source?: string }>;
    };
    expect(marketplace.plugins[0]?.source).toBe("./plugins/axhub");
    expect(existsSync(join(MARKETPLACE_BUNDLE_DIR, "hooks", "hooks.json"))).toBe(true);
    expect(existsSync(join(MARKETPLACE_BUNDLE_DIR, "hooks", "import-router.sh"))).toBe(false);
    expect(existsSync(join(MARKETPLACE_BUNDLE_DIR, "hooks", "update-router.sh"))).toBe(true);
    for (const wrapper of SESSION_WRAPPER_SCRIPTS) {
      expect(existsSync(join(MARKETPLACE_BUNDLE_DIR, "hooks", wrapper)), `missing marketplace wrapper: ${wrapper}`).toBe(true);
    }
    expect(readFileSync(join(MARKETPLACE_BUNDLE_DIR, "hooks", POLICY_WRAPPER_SCRIPT), "utf8")).toBe(
      readFileSync(join(REPO_ROOT, "hooks", POLICY_WRAPPER_SCRIPT), "utf8"),
    );
    expect(readFileSync(join(MARKETPLACE_BUNDLE_DIR, "hooks", "hooks.json"), "utf8")).toBe(
      readFileSync(join(REPO_ROOT, "hooks", "hooks.json"), "utf8"),
    );
    // AP-26: 에이전트 프롬프트 md 는 번들에 없어요 — 훅 스크립트가 판정·적용을 소유해요.
    expect(existsSync(join(MARKETPLACE_BUNDLE_DIR, "hooks", "plugin-restart-confirm-prompt.md"))).toBe(false);
    expect(existsSync(join(MARKETPLACE_BUNDLE_DIR, "hooks", "auto-update-prompt.md"))).toBe(false);
    expect(existsSync(join(MARKETPLACE_BUNDLE_DIR, "node_modules"))).toBe(false);
    expect(existsSync(join(MARKETPLACE_BUNDLE_DIR, ".git"))).toBe(false);
  });

  test("committed marketplace runtime matches a freshly generated bundle", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "axhub-marketplace-bundle-"));
    const outDir = join(tempRoot, "bundle");
    try {
      const result = Bun.spawnSync({
        cmd: ["bun", "scripts/build-plugin-bundle.ts", "--out", outDir, "--json"],
        cwd: REPO_ROOT,
        stdout: "pipe",
        stderr: "pipe",
      });

      expect(result.exitCode, result.stderr.toString()).toBe(0);

      const generated = walk(outDir).map((file) => relative(outDir, file)).sort();
      const committed = walk(MARKETPLACE_BUNDLE_DIR).map((file) => relative(MARKETPLACE_BUNDLE_DIR, file)).sort();
      expect(committed).toEqual(generated);

      for (const file of generated) {
        const expected = readFileSync(join(outDir, file), "utf8");
        const actual = readFileSync(join(MARKETPLACE_BUNDLE_DIR, file), "utf8");
        expect(actual, `bundle drift in ${file}`).toBe(expected);
      }
    } finally {
      rmSync(tempRoot, { recursive: true, force: true });
    }
  });
});
