#!/usr/bin/env bun
// Release Step 2/2 — narrative 검증 후 tag 생성 + push.
// Step 1 (`bun run release`) 는 .versionrc.json `skip.tag=true` 로 bump+commit
// 만 생성해요. 사람이 CHANGELOG narrative 추가 + `git commit --amend -a` 한 후
// 본 스크립트로 tag 를 amended HEAD 에 생성해요. 옛 flow 의 "tag 가 amend
// 전 commit 가리켜서 release.yml 이 narrative 빈 채로 fire" 버그 회귀 방지용.

import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

const sh = (cmd: string, opts: { allowFail?: boolean } = {}): string => {
  try {
    return execSync(cmd, { encoding: "utf8", cwd: REPO_ROOT }).trim();
  } catch (err) {
    if (opts.allowFail) return "";
    throw err;
  }
};

const pkg = JSON.parse(readFileSync(join(REPO_ROOT, "package.json"), "utf8")) as { version: string };
const version = pkg.version;
const tag = `v${version}`;

const existing = sh(`git rev-parse --verify --quiet refs/tags/${tag}`, { allowFail: true });
if (existing) {
  console.error(`[release:tag] ${tag} 이미 존재해요 (${existing}).`);
  console.error("재배포면 'git tag -d' + remote 삭제 + 본 스크립트 재실행 필요해요.");
  process.exit(1);
}

const changelog = readFileSync(join(REPO_ROOT, "CHANGELOG.md"), "utf8");
const escapedVersion = version.replace(/\./g, "\\.");
const sectionRe = new RegExp(`^## \\[${escapedVersion}\\][^\\n]*\\n([\\s\\S]*?)(?=^## \\[|\\Z)`, "m");
const match = changelog.match(sectionRe);
if (!match) {
  console.error(`[release:tag] CHANGELOG.md 에 '## [${version}]' 섹션 없어요.`);
  process.exit(1);
}

const body = match[1].trim();
const bodyWithoutAutoSections = body
  .split("\n")
  .filter((line) => !/^### (Added|Fixed|Changed|Docs|Performance)/.test(line) && !/^\* /.test(line))
  .join("\n")
  .trim();

if (bodyWithoutAutoSections.length < 50) {
  console.error(`[release:tag] CHANGELOG ${tag} 섹션에 narrative paragraph 가 없거나 너무 짧아요 (${bodyWithoutAutoSections.length} chars).`);
  console.error("절차: ## [신버전] 아래 해요체 paragraph 추가 → 'git commit --amend --no-edit -a' → 재실행.");
  process.exit(1);
}

const status = sh("git status --porcelain");
if (status) {
  console.error("[release:tag] working tree 가 clean 하지 않아요. amend 했는지 확인해주세요.");
  console.error(status);
  process.exit(1);
}

// U8 가드 — 릴리즈 amend 가 미검토 재생성물을 흡수하는 표면 차단이에요.
// 태그 직전 HEAD diff(첫 부모 기준)는 bump 대상(.versionrc.json bumpFiles 전체)과
// CHANGELOG.md 밖 파일을 포함하면 안 돼요.
const versionrc = JSON.parse(readFileSync(join(REPO_ROOT, ".versionrc.json"), "utf8")) as {
  bumpFiles: Array<{ filename: string }>;
};
const allowedReleaseFiles = new Set([
  ...versionrc.bumpFiles.map((entry) => entry.filename),
  "CHANGELOG.md",
]);
const headDiffFiles = sh("git diff --name-only HEAD^1 HEAD")
  .split("\n")
  .filter((line) => line.length > 0);
const outsideBumpScope = headDiffFiles.filter((file) => !allowedReleaseFiles.has(file));
if (outsideBumpScope.length > 0) {
  console.error("[release:tag] release commit 에 bump 대상 밖 파일이 섞여 있어요:");
  for (const file of outsideBumpScope) console.error(`  - ${file}`);
  console.error("bump 대상(.versionrc.json bumpFiles)과 CHANGELOG.md 만 release commit 에 담고 재실행해주세요.");
  process.exit(1);
}

const headSha = sh("git rev-parse HEAD");
console.log(`[release:tag] ${tag} → ${headSha.slice(0, 12)} 에 tag 생성해요.`);
sh(`git tag -a ${tag} -m "chore(release): ${version}"`);

console.log(`[release:tag] git push origin main + ${tag} 실행해요.`);
sh("git push origin main");
sh(`git push origin ${tag}`);

console.log(`[release:tag] 완료. release.yml 이 ${tag} push 로 fire 돼서 release body + Slack narrative 가 정상이에요.`);
