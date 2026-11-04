import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

const CANONICAL_BIN = '"$HOME/.axhub/bin/axhub"';

const GUARD_FILES = [
  "skills/bootstrap/SKILL.md",
  "skills/clarity/SKILL.md",
  "skills/deploy/SKILL.md",
  "skills/development/SKILL.md",
  "skills/diagnosis/SKILL.md",
  "skills/import/SKILL.md",
  "skills/up/SKILL.md",
  "hooks/session-auto-update.sh",
];

// Guards that must name the canonical binary path, not just cite the contract.
const REPAIR_PATH_FILES = [
  "skills/bootstrap/SKILL.md",
  "skills/clarity/SKILL.md",
  "skills/deploy/SKILL.md",
  "skills/development/SKILL.md",
];

describe("CLI path resolution contract (AP-17)", () => {
  test("every CLI guard treats a bare axhub failure as a PATH gap, not a missing install", () => {
    for (const file of GUARD_FILES) {
      const content = readRepo(file);

      expect(content, file).toContain("bare `axhub` 실패는 미설치가 아니에요");
      expect(content, file).toContain("~/.axhub/bin-path");
      expect(content, file).toContain("repair-path");
    }
  });

  test("guards resolve the on-disk binary before handing off to setup", () => {
    for (const file of REPAIR_PATH_FILES) {
      const content = readRepo(file);

      expect(content, file).toContain(CANONICAL_BIN);
      expect(content, file).toContain("plugin-support repair-path --json");
    }
  });

  test("bootstrap CLI guard runs the repair-path leaf command before any setup handoff", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");

    expect(bootstrap).toContain(`${CANONICAL_BIN} plugin-support repair-path --json`);
    expect(bootstrap).toContain("command not found 여도 곧바로 setup 으로 돌리지 않아요");
    expect(bootstrap).not.toContain("command not found 이면 setup 안내 후 stop");
  });
});
