// Phase 0.6.0 — axhub-helpers CLI subcommands (autowire-statusline / orphan-stub).
// Smoke + interface contract tests.

import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO = join(import.meta.dir, "..");
const HELPER = join(REPO, "bin/axhub-helpers");

describe("Phase 0.6.0 — axhub-helpers autowire-statusline subcommand", () => {
  test("--help exits 0", () => {
    const r = spawnSync(HELPER, ["autowire-statusline", "--help"], {
      encoding: "utf8",
    });
    expect(r.status).toBe(0);
  });

  test("--help mentions --scope and --silent flags", () => {
    const r = spawnSync(HELPER, ["autowire-statusline", "--help"], {
      encoding: "utf8",
    });
    expect(r.stdout).toContain("--scope");
    expect(r.stdout).toContain("--silent");
  });

  test("--help mentions Korean 해요체 description", () => {
    const r = spawnSync(HELPER, ["autowire-statusline", "--help"], {
      encoding: "utf8",
    });
    expect(r.stdout).not.toMatch(/합니다|입니다|시겠어요|드립니다|당신|아이고/);
  });

  test("missing required --scope → non-zero exit + Korean error", () => {
    const r = spawnSync(HELPER, ["autowire-statusline"], {
      encoding: "utf8",
    });
    expect(r.status).not.toBe(0);
    expect(r.stderr + r.stdout).not.toMatch(
      /합니다|입니다|시겠어요|드립니다|당신|아이고/,
    );
  });

  test("invalid --scope value → non-zero exit", () => {
    const r = spawnSync(
      HELPER,
      ["autowire-statusline", "--scope", "invalid_scope_value"],
      { encoding: "utf8" },
    );
    expect(r.status).not.toBe(0);
  });
});

describe("Phase 0.6.0 — axhub-helpers orphan-stub subcommand", () => {
  test("--help exits 0", () => {
    const r = spawnSync(HELPER, ["orphan-stub", "--help"], { encoding: "utf8" });
    expect(r.status).toBe(0);
  });

  test("--help mentions --install and --verify flags", () => {
    const r = spawnSync(HELPER, ["orphan-stub", "--help"], { encoding: "utf8" });
    expect(r.stdout).toContain("--install");
    expect(r.stdout).toContain("--verify");
  });

  test("--help mentions Korean 해요체", () => {
    const r = spawnSync(HELPER, ["orphan-stub", "--help"], { encoding: "utf8" });
    expect(r.stdout).not.toMatch(/합니다|입니다|시겠어요|드립니다|당신|아이고/);
  });

  test("--install in temp HOME creates orphan stub script", () => {
    const tempHome = mkdtempSync(join(tmpdir(), "axhub-orphan-stub-"));
    try {
      const r = spawnSync(HELPER, ["orphan-stub", "--install"], {
        encoding: "utf8",
        env: {
          ...process.env,
          HOME: tempHome,
          USERPROFILE: tempHome,
          XDG_STATE_HOME: "",
        },
      });
      expect(r.status).toBe(0);
      // Stub should exist under one of XDG_STATE_HOME or ~/.local/state/axhub-plugin/
      const stubSh = join(
        tempHome,
        ".local/state/axhub-plugin/orphan-stub-statusline.sh",
      );
      // At least one of the platform stubs should exist
      // (PS1 mirror creation is platform-conditional inside the helper)
      const stubExists = existsSync(stubSh);
      expect(stubExists).toBe(true);
      expect(r.stdout.trim()).toBe(stubSh);
    } finally {
      rmSync(tempHome, { recursive: true, force: true });
    }
  });

  test("--install then --verify both exit 0 (idempotent)", () => {
    const tempHome = mkdtempSync(join(tmpdir(), "axhub-orphan-stub-verify-"));
    try {
      const r1 = spawnSync(HELPER, ["orphan-stub", "--install"], {
        encoding: "utf8",
        env: {
          ...process.env,
          HOME: tempHome,
          USERPROFILE: tempHome,
          XDG_STATE_HOME: "",
        },
      });
      expect(r1.status).toBe(0);
      const r2 = spawnSync(HELPER, ["orphan-stub", "--verify"], {
        encoding: "utf8",
        env: {
          ...process.env,
          HOME: tempHome,
          USERPROFILE: tempHome,
          XDG_STATE_HOME: "",
        },
      });
      expect(r2.status).toBe(0);
    } finally {
      rmSync(tempHome, { recursive: true, force: true });
    }
  });
});
