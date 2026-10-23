// CHANGELOG.md 에서 `## [<version>]` 섹션 본문을 잘라요. 끝은 다음 `## [` 헤더나 파일 끝이에요 —
// JS 정규식에는 `\Z` 가 없어(리터럴 'Z' 로 읽혀요) `(?![\s\S])` 로 파일 끝을 잡아요.
export const extractChangelogSection = (changelog: string, version: string): string | null => {
  const escapedVersion = version.replace(/\./g, "\\.");
  const sectionRe = new RegExp(`^## \\[${escapedVersion}\\][^\\n]*\\n([\\s\\S]*?)(?=^## \\[|(?![\\s\\S]))`, "m");
  return changelog.match(sectionRe)?.[1] ?? null;
};
