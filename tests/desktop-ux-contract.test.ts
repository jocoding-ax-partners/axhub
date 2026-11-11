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
      expect(description).toContain("setup/bootstrap/plugins/deploy/import/development/operate");
      expect(description).not.toContain("Import priority is strict");
      expect(description).not.toContain("Routing priority is strict");
      expect(description).not.toContain("현재 버전을 확인할게요");
      expect(description).not.toContain("기존 앱을 axhub에 가져올 준비를 확인할게요");
      expect(description).not.toContain(CLAUDE.surface.pluginListCommand);
    }
  });

  // 옛 clarity 의 최신·버전 양보 route guard·CLI-only 가드 문장은 2026-11 운영 스킬 개편(clarity → operate) 때 뺐어요.
  // 최신·버전 요청은 setup 이 받고, operate 는 description 의 "설치·업데이트는 setup" 경계로 넘겨요
  // (tests/operate-skill-contract.test.ts 가 새 본문을 잠가요).
});
