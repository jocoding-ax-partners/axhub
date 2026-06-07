import { describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const REPO_ROOT = join(import.meta.dir, "..");
const HELPER_MANIFEST = join(REPO_ROOT, "crates", "axhub-helpers", "Cargo.toml");

function makeProject(prefix = "axhub-preview-"): string {
  return mkdtempSync(join(tmpdir(), prefix));
}

function makeViteReactApp(): string {
  const dir = makeProject("axhub-vite-preview-");
  writeFileSync(
    join(dir, "package.json"),
    JSON.stringify(
      {
        private: true,
        type: "module",
        dependencies: {
          "@vitejs/plugin-react": "latest",
          vite: "latest",
          react: "latest",
          "react-dom": "latest",
        },
      },
      null,
      2,
    ) + "\n",
    "utf8",
  );
  writeFileSync(join(dir, "index.html"), "<div id=\"root\"></div>\n", "utf8");
  return dir;
}

function runHelper(cwd: string, subcommand: string) {
  return spawnSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--manifest-path",
      HELPER_MANIFEST,
      "--",
      subcommand,
      "--user-utterance",
      "이 앱 배포해줘",
    ],
    {
      cwd,
      encoding: "utf8",
      timeout: 120_000,
      env: {
        ...process.env,
        AXHUB_CLI: "/bin/false",
      },
    },
  );
}

describe("deploy-preview-summary Desktop natural-language guard", () => {
  test("local project without axhub.yaml stops at local manifest choices before remote app registration", () => {
    const app = makeProject();
    try {
      const result = runHelper(app, "deploy-preview-summary");

      expect(result.status).toBe(0);
      expect(result.stdout).toContain("axhub 매니페스트(axhub.yaml)가 없어요.");
      expect(result.stdout).toContain("React/Vite로 초기화");
      expect(result.stdout).toContain("다른 템플릿 선택");
      expect(result.stdout).toContain("취소");
      expect(result.stdout).toContain("원격 앱 등록이나 배포는 아직 시작하지 않았어요.");
      expect(result.stdout).not.toContain("처음 배포라 앱 등록");
      expect(result.stdout).not.toContain("진행할까요?");
    } finally {
      rmSync(app, { recursive: true, force: true });
    }
  });

  test("Vite/React without axhub.yaml stops at local manifest choices before remote app registration", () => {
    const app = makeViteReactApp();
    try {
      const result = runHelper(app, "deploy-preview-summary");

      expect(result.status).toBe(0);
      expect(result.stdout).toContain("axhub 매니페스트(axhub.yaml)가 없어요.");
      expect(result.stdout).toContain("React/Vite로 초기화");
      expect(result.stdout).toContain("다른 템플릿 선택");
      expect(result.stdout).toContain("취소");
      expect(result.stdout).toContain("원격 앱 등록이나 배포는 아직 시작하지 않았어요.");
      expect(result.stdout).not.toContain("처음 배포라 앱 등록");
      expect(result.stdout).not.toContain("진행할까요?");
    } finally {
      rmSync(app, { recursive: true, force: true });
    }
  });

  test("approved run also refuses to deploy Vite/React without axhub.yaml", () => {
    const app = makeViteReactApp();
    try {
      const result = runHelper(app, "deploy-approved-run");

      expect(result.status).toBe(0);
      expect(result.stdout).toContain("axhub 매니페스트(axhub.yaml)가 없어서 배포를 시작하지 않았어요.");
      expect(result.stdout).toContain("React/Vite로 초기화");
      expect(result.stdout).not.toContain("배포를 시작했어요.");
    } finally {
      rmSync(app, { recursive: true, force: true });
    }
  });
});
