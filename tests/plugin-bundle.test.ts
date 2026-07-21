import { describe, expect, test } from "bun:test";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, relative } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const MARKETPLACE_BUNDLE_DIR = join(REPO_ROOT, "plugins", "axhub");
const SKILLS = ["onboarding", "bootstrap", "deploy", "import", "development", "diagnosis", "clarity", "update"] as const;
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
      expect(stats.bytes).toBeLessThan(512 * 1024);

      expect(existsSync(join(outDir, ".claude-plugin", "plugin.json"))).toBe(true);
      expect(existsSync(join(outDir, "README.md"))).toBe(true);
      expect(existsSync(join(outDir, "LICENSE"))).toBe(true);
      expect(existsSync(join(outDir, "POLICY.md"))).toBe(true);
      expect(existsSync(join(outDir, "hooks", "hooks.json"))).toBe(true);
      expect(existsSync(join(outDir, "hooks", "update-router.sh"))).toBe(true);
      expect(existsSync(join(outDir, "hooks", "import-router.sh"))).toBe(false);
      expect(existsSync(join(outDir, "hooks", "clarity-router.sh"))).toBe(false);
      expect(existsSync(join(outDir, "hooks", "status-resume-router.sh"))).toBe(false);
      expect(readFileSync(join(outDir, "POLICY.md"), "utf8")).toContain("플러그인 스킬 흐름은 그 도구를 우선 사용하지 않아요");
      expect(readFileSync(join(outDir, "hooks", "hooks.json"), "utf8")).toContain("AXHUB_NO_UPDATE_ROUTER");
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
    expect(existsSync(join(MARKETPLACE_BUNDLE_DIR, "hooks", "plugin-restart-confirm-prompt.md"))).toBe(true);
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
