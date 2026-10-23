import { describe, expect, test } from "bun:test";

import { extractChangelogSection } from "../scripts/changelog-section.ts";

// P3-35: release-tag 는 `(?=^## \[|\Z)` 로 섹션 끝을 잡았는데 JS 정규식의 `\Z` 는 리터럴 'Z' 예요.
// narrative 가 "ZIP …" 처럼 Z 로 시작하면 캡처가 빈 줄이 되어 release:tag 를 거부했고, 마지막
// 섹션(뒤에 `## [` 없음)은 Z 가 없으면 아예 매칭되지 않았어요.
const NARRATIVE_ZIP = "ZIP 아카이브로 받은 플러그인도 실행 권한을 그대로 지켜요. 설치 뒤 훅이 바로 돌아요.";

describe("release-tag CHANGELOG 섹션 추출 (P3-35)", () => {
  test("narrative 가 Z 로 시작해도 다음 섹션 직전까지 전부 잘라요", () => {
    const changelog = [
      "# Changelog",
      "",
      "## [1.33.0](https://example.com) (2026-10-23)",
      "",
      NARRATIVE_ZIP,
      "",
      "### Fixed",
      "",
      "* **plugin:** keep exec bits",
      "",
      "## [1.32.1](https://example.com) (2026-10-20)",
      "",
      "이전 릴리스예요.",
      "",
    ].join("\n");
    const section = extractChangelogSection(changelog, "1.33.0");
    expect(section).not.toBeNull();
    expect(section!).toContain(NARRATIVE_ZIP);
    expect(section!).toContain("* **plugin:** keep exec bits");
    expect(section!).not.toContain("이전 릴리스예요.");
  });

  test("뒤에 다른 섹션이 없는 마지막 섹션도 파일 끝까지 잘라요", () => {
    const changelog = ["# Changelog", "", "## [0.1.0] (2026-01-01)", "", "첫 릴리스 narrative 예요. 플러그인을 처음 공개해요.", ""].join("\n");
    expect(extractChangelogSection(changelog, "0.1.0")?.trim()).toBe("첫 릴리스 narrative 예요. 플러그인을 처음 공개해요.");
  });

  test("버전의 점은 리터럴이라 다른 버전 섹션을 잡지 않아요", () => {
    const changelog = ["## [1x33y0]", "", "가짜예요.", "", "## [1.33.0]", "", "진짜예요.", ""].join("\n");
    expect(extractChangelogSection(changelog, "1.33.0")?.trim()).toBe("진짜예요.");
  });

  test("섹션이 없으면 null 이에요", () => {
    expect(extractChangelogSection("# Changelog\n\n## [1.0.0]\n\nx\n", "2.0.0")).toBeNull();
  });
});
