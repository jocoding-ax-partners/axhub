import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { DEFAULT_MAX_TOTAL_BYTES } from "../scripts/check-plugin-context-budget.ts";

const ROOT = join(import.meta.dir, "..");
const SKILL_ROOTS = ["skills", "plugins/axhub/skills", "plugins/axhub-codex/skills"] as const;
const DIET = [
  "setup",
  "bootstrap",
  "plugins",
  "deploy",
  "import",
  "development",
  "clarity",
] as const;

const readSkill = (root: (typeof SKILL_ROOTS)[number], skill: string): string =>
  readFileSync(join(ROOT, root, skill, "SKILL.md"), "utf8");

const sliceSection = (source: string, start: string, end: string): string => {
  const from = source.indexOf(start);
  const to = source.indexOf(end, from + start.length);
  expect(from, `missing section start: ${start}`).toBeGreaterThanOrEqual(0);
  expect(to, `missing section end: ${end}`).toBeGreaterThan(from);
  return source.slice(from, to);
};

// bootstrap(옛 scaffold 통합판)은 서버 판정 하나로 Axhub 저장소·내 GitHub·회사 GitHub 조직을 모두 맡아요.
// 판정이 없는 구 서버용 `--git-backend` 분기·init-resume 이어하기는 없앴어요.
describe("spec 236 self-hosted git skill contracts", () => {
  test("T144 positive: bootstrap reads the company decision before any GitHub command", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const decision = bootstrap.indexOf("axhub apps git-backend --tenant <회사> --json");
      expect(decision, root).toBeGreaterThanOrEqual(0);
      expect(decision, root).toBeLessThan(bootstrap.indexOf("axhub github link --json"));
      expect(decision, root).toBeLessThan(bootstrap.indexOf("axhub github accounts list --json"));
      const githubOnly = sliceSection(bootstrap, "### 4. GitHub 일 때만", "### 5. 이름");
      expect(githubOnly, root).toContain("Axhub 저장소면 건너뛰어요. GitHub 연결·앱 설치 이야기는 꺼내지 않아요.");
    }
  });

  test("bootstrap carries the selected workspace through setup, clone, and follow-up", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const clone = sliceSection(bootstrap, "### 8. 코드 받기", "### 9. 배포 확인");
      expect(clone, root).toContain("axhub --tenant <회사> git setup --json");
      expect(clone, root).toContain("axhub --tenant <회사> repo clone <slug> --json");
      expect(clone, root).toContain("받은 폴더를 이후 작업 폴더로 써요");
    }
  });

  test("new storage decision is capability-gated and old CLIs are told to update", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const deploy = readSkill(root, "deploy");

      expect(bootstrap, root).toContain("`capabilities.self_hosted_git.git_policy_decision` 이 true 가 아니면 CLI 를 업데이트해 달라고 하고 멈춰요");
      // 2026-11 deploy 개편: 저장 위치별 길은 CLI `deploy push` 가 골라서, deploy 는 그 기능 하나로 가드해요.
      expect(deploy, root).toContain("`capabilities.deploy.push` 가 true 가 아니면 CLI 를 업데이트해 달라고 하고 멈춰요");
    }
  });

  test("T144 negative: GitHub bootstrap keeps the account, owner, and app-install path", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      expect(bootstrap, root).toContain("axhub github accounts list --json");
      expect(bootstrap, root).toContain("`install_url`");
      expect(bootstrap, root).toContain("`--github-owner <계정>`");
      expect(bootstrap, root).toContain("\"어느 GitHub 계정에 둘까요?\"");
    }
  });

  // 2026-11 deploy 개편(옛 up 을 합친 판): selfhosted push·GitHub·static upload·폴더 업로드 lane 을 스킬이 나누지 않고
  // CLI `deploy push` 가 골라요. 옛 lane 별 계약(T145 clone/credential/push·#455 clone 결과·#832 주소·static lane·
  // up 위임·selfhosted 대사)은 그 본문과 함께 뺐어요. 남은 스킬 몫만 잠가요.
  test("deploy leaves the storage path to the CLI and only clones into an empty folder", () => {
    for (const root of SKILL_ROOTS) {
      const deploy = readSkill(root, "deploy");
      expect(deploy, root).toContain("올리는 길(Axhub 저장소·GitHub·정적 사이트·폴더 업로드)은 CLI 가 골라요");
      expect(deploy, root).toContain("앱 이름만 말했고 폴더가 비어 있으면 `axhub repo clone <앱> . --json` 으로 받아요");
      expect(deploy, root).toContain("이 폴더가 앱 저장소에 안 붙어 있다면 import 로 보내고 멈춰요");
      expect(deploy, root).not.toContain("git_backend");
      expect(deploy, root).not.toContain("axhub apps get");
      expect(deploy, root).not.toContain("git push");
      expect(deploy, root).not.toContain("axhub up");
    }
  });

  test("policy pins CLI-only path selection and the 7-skill diet", () => {
    const agentPolicy = readFileSync(join(ROOT, "docs", "policy", "agent-policy.md"), "utf8");
    const devPolicy = readFileSync(join(ROOT, "docs", "policy", "dev-policy.md"), "utf8");
    expect(agentPolicy).toContain("## AP-23 올리는 길은 CLI 가 골라요 (deploy push)");
    expect(agentPolicy).toContain("Gitea API·C1 HTTP·remote URL");
    expect(devPolicy).toContain("공개 skill 은 7개");
    // DP-5 의 합산 예산은 게이트 상수와 같은 값이어야 해요 — 상수만 올리고 정책을 두면 기준 문서가 어긋나요.
    expect(devPolicy).toContain(`SKILL.md 7개 합산 ${DEFAULT_MAX_TOTAL_BYTES.toLocaleString("en-US")}B`);
  });

  test("source and both generated surfaces keep the 7-skill diet", () => {
    const expected = [...DIET].sort();
    for (const root of SKILL_ROOTS) {
      const actual = readdirSync(join(ROOT, root), { withFileTypes: true })
        .filter((entry) => entry.isDirectory())
        .map((entry) => entry.name)
        .sort();
      expect(actual, root).toEqual(expected);
    }
  });
});

describe("spec 236 2차: import follows the code storage location", () => {
  test("bootstrap owns GitHub repositories itself and never yields to scaffold", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      expect(bootstrap, root).not.toContain("scaffold");
      expect(bootstrap, root).toContain("회사 정책에 맞춰 코드를 Axhub 저장소나 GitHub(내 계정·회사 조직)에 둬요");
    }
  });

  // 2026-11 import 개편: 저장 위치는 bootstrap 3·4단계와 같은 정책으로 정하고 고른 key 를 CLI 에 넘겨요.
  // 옛 `--git-backend` 분기·`capabilities.import.selfhosted` 가드·`코드 저장 위치:` 줄은 의도적으로 뺐어요.
  test("import follows bootstrap's storage policy and hands the pick to the CLI", () => {
    for (const root of SKILL_ROOTS) {
      const importSkill = readSkill(root, "import");
      expect(importSkill, root).toContain("`capabilities.self_hosted_git.git_policy_decision`");
      expect(importSkill, root).toContain("앱 서비스 권한·저장 위치·GitHub 연결은 bootstrap 스킬 3·4단계와 똑같이 해요");
      expect(importSkill, root).toContain("--git-choice <key>");
      expect(importSkill, root).toContain("`--github-owner <계정>`");
      expect(importSkill, root).not.toContain("--git-backend");
      expect(importSkill, root).toContain("Axhub 저장소로 올리는 실행은 처음부터 권한 상승으로 해요");
    }
  });
});

// self-hosted 작업공간의 static 앱은 업로드 release 가 아니라 앱 저장소 push → webhook deployment 예요.
// 저장소에는 seed 커밋만 있어 업로드로 배포하던 로컬 소스를 clone 으로 버리면 seed 를 다시 배포해요.
// spec 706: 서버가 회사 모드로 정한 저장 위치(`git_backend.decision`)를 플러그인은 따르기만 해요.
// 판정이 없는 구 서버 규칙(추천값·`remembered`·GitHub/Axhub self-hosted 질문)은 그 뒤에 남아요.
describe("spec 706: skills follow the server code storage decision", () => {
  test("bootstrap follows use/ask/link_github and passes the pick with --git-choice", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const gate = sliceSection(bootstrap, "### 3. 회사 정책", "### 4. GitHub 일 때만");
      expect(bootstrap, root).toContain("capabilities.self_hosted_git.git_policy_decision");
      expect(gate, root).toContain("`git_backend.decision`");
      expect(gate, root).toContain("`use`: 묻지 않고 그 선택지를 써요.");
      expect(gate, root).toContain("\"코드를 어디에 저장할까요?\"");
      expect(gate, root).toContain("`options` 순서 그대로");
      expect(gate, root).toContain("`link_github`: 아래 GitHub 계정 연결부터 하고 다시 조회해요.");
      expect(gate, root).toContain("없으면 이 회사는 그 위치를 쓸 수 없다고 알리고 멈춰요");
      expect(gate, root).toContain("`app_platform.enabled` 가 false 면");
      expect(bootstrap, root).toContain("axhub apps bootstrap --git-choice <key>");
      expect(bootstrap, root).not.toContain("--git-backend");
    }
  });

  test("no skill tells a selfhosted-only or unlinked person that a GitHub account is required", () => {
    for (const root of SKILL_ROOTS) {
      for (const name of ["bootstrap", "import"]) {
        expect(readSkill(root, name), `${root}/${name}`).not.toContain("GitHub 계정이 필요해요");
      }
    }
  });
});
