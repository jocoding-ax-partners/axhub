// Windows compatibility guard — repo tooling entrypoints must be native Bun/TS,
// not POSIX-only shell commands. POSIX .sh files may remain as compatibility
// wrappers, but package scripts and CI gates should expose cross-platform lanes.

import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

import packageJson from "../package.json" with { type: "json" };

const REPO_ROOT = join(import.meta.dir, "..");
const read = (path: string) => readFileSync(join(REPO_ROOT, path), "utf8");

describe("Windows-native repo tooling entrypoints", () => {
  test("package scripts avoid POSIX-only smoke and routing commands", () => {
    const scripts = packageJson.scripts as Record<string, string>;

    for (const name of ["smoke", "smoke:full"]) {
      expect(scripts[name]).toContain("bun scripts/smoke.ts");
      expect(scripts[name]).not.toContain("./bin/axhub-helpers");
      expect(scripts[name]).not.toContain("</dev/null");
      expect(scripts[name]).not.toContain("bash tests/docs-link-audit.sh");
    }

    for (const name of ["test:routing", "test:routing:20", "test:routing:100", "test:routing:full"]) {
      expect(scripts[name]).toContain("bun tests/run-corpus.ts");
      expect(scripts[name]).not.toContain("bash tests/run-corpus.sh");
    }
  });

  test("Bun corpus runner exists and replays committed fixtures without Bash", () => {
    const runner = join(REPO_ROOT, "tests/run-corpus.ts");
    expect(existsSync(runner)).toBe(true);

    const dir = mkdtempSync(join(tmpdir(), "axhub-win-corpus-"));
    try {
      const out = join(dir, "plugin.json");
      const result = spawnSync(
        "bun",
        [runner, "--mode", "plugin", "--corpus", "tests/corpus.20.jsonl", "--out", out],
        { cwd: REPO_ROOT, encoding: "utf8", timeout: 30000 },
      );
      if (result.status !== 0) {
        process.stderr.write(`${result.stdout}\n${result.stderr}\n`);
      }
      expect(result.status).toBe(0);
      expect(readFileSync(out, "utf8")).toContain('"utterance_id"');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("routing drift gate uses Bun runner instead of spawning Bash", () => {
    const script = read("scripts/routing-drift-check.ts");
    const workflow = read(".github/workflows/routing-drift.yml");

    expect(script).toContain("tests/run-corpus.ts");
    expect(script).not.toContain('"bash"');
    expect(script).not.toContain("tests/run-corpus.sh");
    expect(workflow).toContain("bun tests/run-corpus.ts");
    expect(workflow).not.toContain("bash tests/run-corpus.sh");
  });

  test("docs link audit has a Bun entrypoint for Windows smoke:full", () => {
    const smokeScript = read("scripts/smoke.ts");

    expect(existsSync(join(REPO_ROOT, "tests/docs-link-audit.ts"))).toBe(true);
    expect(packageJson.scripts["smoke:full"]).toBe("bun scripts/smoke.ts --full");
    expect(smokeScript).toContain("tests/docs-link-audit.ts");
  });

  test("docs link audit follows dotted reference filenames", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-win-doc-links-"));
    try {
      const skillDir = join(dir, "skills", "data");
      const deployRefs = join(dir, "skills", "deploy", "references");
      const localRefs = join(skillDir, "references");
      mkdirSync(deployRefs, { recursive: true });
      mkdirSync(localRefs, { recursive: true });
      writeFileSync(join(deployRefs, "error-empathy-catalog.generated.md"), "# generated\n");
      writeFileSync(join(localRefs, "local.generated.md"), "# local\n");
      writeFileSync(
        join(skillDir, "SKILL.md"),
        [
          "Read ../deploy/references/error-empathy-catalog.generated.md",
          "Read references/local.generated.md",
          "",
        ].join("\n"),
      );

      const result = spawnSync("bun", [join(REPO_ROOT, "tests/docs-link-audit.ts")], {
        cwd: REPO_ROOT,
        encoding: "utf8",
        env: { ...process.env, PLUGIN_ROOT: dir },
        timeout: 30000,
      });

      expect(result.status).toBe(0);
      expect(result.stdout).toContain("Broken: 0");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("Windows host build writes bin/axhub-helpers.exe as the primary helper", () => {
    const buildScript = read("scripts/build-rust-helper.ts");
    const releaseCheck = read("scripts/release-check.ts");
    const scripts = packageJson.scripts as Record<string, string>;

    expect(buildScript).toContain("hostPrimaryBinaryName");
    expect(releaseCheck).toContain('hostPrimaryName');
    expect(releaseCheck).toContain('assertVersion(`bin/${hostPrimaryName()}`)');
    expect(scripts["build:windows-amd64"]).toBe("bun scripts/build-rust-helper.ts --target-alias windows-amd64");
    expect(scripts["build:windows-amd64"]).not.toContain("--name axhub-helpers");
  });

  test("Rust release target metadata is shared by build and release checks", () => {
    const buildScript = read("scripts/build-rust-helper.ts");
    const releaseCheck = read("scripts/release-check.ts");

    expect(existsSync(join(REPO_ROOT, "scripts/rust-targets.ts"))).toBe(true);
    expect(buildScript).toContain("./rust-targets.ts");
    expect(releaseCheck).toContain("./rust-targets.ts");
    expect(releaseCheck).toContain("RELEASE_ASSET_NAMES");
  });
});
