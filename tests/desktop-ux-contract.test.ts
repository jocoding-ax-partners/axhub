import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { HOST_EXPECTATIONS } from "./fixtures/host-expectations";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");
// host 결합 기대값은 Claude lane fixture 를 참조해요 — codex 열은 후속 유닛이
// tests/fixtures/host-expectations.ts 에 추가해요.
const CLAUDE = HOST_EXPECTATIONS.claude;

describe("Desktop UX contract", () => {
  test("plugin manifests stay concise — routing contracts live in skills and hooks", () => {
    const manifest = JSON.parse(readRepo(".claude-plugin/plugin.json")) as { description: string };
    const marketplace = JSON.parse(readRepo(".claude-plugin/marketplace.json")) as { plugins: Array<{ description: string }> };
    const descriptions = [manifest.description, marketplace.plugins[0]?.description ?? ""];

    for (const description of descriptions) {
      // Discover/Installed UI 에 그대로 노출되는 필드라 간결하게 유지해요.
      // import 라우팅 계약 본문은 skills/*/SKILL.md 와 hooks/ 가
      // 소유해요 — 아래 다른 테스트들이 그 표면을 잠가요.
      expect(description.length).toBeLessThanOrEqual(320);
      expect(description).toContain("ax-hub-cli");
      expect(description).toContain("setup/bootstrap/scaffold/plugins/deploy/up/import/development/diagnosis/clarity");
      expect(description).not.toContain("Import priority is strict");
      expect(description).not.toContain("Routing priority is strict");
      expect(description).not.toContain("현재 버전을 확인할게요");
      expect(description).not.toContain("기존 앱을 axhub에 가져올 준비를 확인할게요");
      expect(description).not.toContain(CLAUDE.surface.pluginListCommand);
    }
  });

  // 최신·버전 요청은 clarity 가 아니라 setup 이 받아요. clarity 는 그 경계와 CLI-only 가드를 들고 있어요.
  test("clarity yields freshness requests and keeps its CLI-only route guard", () => {
    const clarity = readRepo("skills/clarity/SKILL.md") + readRepo("skills/clarity/references/execution-guardrails.md");
    const clarityFrontmatter = clarity.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";

    expect(clarity).toContain("axhub CLI 운영 명령 브리지");
    expect(clarityFrontmatter).not.toContain("disable-model-invocation");
    expect(clarity).toContain("frontmatter description 라우팅으로 자연어에서도 이 스킬이 직접 받아요");
    expect(clarity).toContain("## Do not invoke / route guard");
    expect(clarity).toContain("첫 visible assistant text 를 정확히 `현재 버전을 확인할게요.`");
    expect(clarity).toContain("이미 `/axhub:clarity` 배지가 뜬 뒤 이 문서를 읽었다면");
    expect(clarity).toContain("`Using /axhub:clarity...` 같은 문장을 쓰거나 명령을 실행하지 말고");
    expect(clarity).toContain("내 앱들이 지금 어떤 상태인지도 알아서 봐줘");
    expect(clarity).toContain("`command -v axhub && axhub --version`");
    expect(clarity).toContain("`Checking axhub CLI installation and version`");
    expect(clarity).toContain("`MCP tools to check axhub status and your apps`");
    expect(clarityFrontmatter).toContain("app status overview");
    expect(clarityFrontmatter).toContain("app creation");
    expect(clarityFrontmatter).toContain("mixed freshness+status+create prompts");
    expect(clarityFrontmatter).toContain("최신/버전/update/latest/freshness checks");
    expect(clarityFrontmatter).toContain("GitHub 계정 재연결/device code");
    expect(clarity).not.toContain("내 앱들이 지금 어떤 상태인지 모르겠어");
    expect(clarityFrontmatter).not.toContain("내 앱들이 지금 어떤 상태인지도 알아서 봐줘");
    expect(clarity).not.toContain("operational-lookups.md");
    expect(clarity).not.toContain("show a read-only axhub overview");
    expect(clarity).not.toContain("is production healthy?");
  });
});
