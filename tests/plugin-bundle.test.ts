import { describe, expect, test } from "bun:test";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, relative } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS = ["onboarding", "init", "deploy", "import", "development", "diagnosis", "clarity", "update"] as const;
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

      const rootManifest = JSON.parse(readFileSync(join(REPO_ROOT, ".claude-plugin", "plugin.json"), "utf8")) as { version: string };
      const bundledManifest = JSON.parse(readFileSync(join(outDir, ".claude-plugin", "plugin.json"), "utf8")) as { version: string };
      expect(bundledManifest.version).toBe(rootManifest.version);

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
});
