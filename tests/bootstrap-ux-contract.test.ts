import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { HOST_EXPECTATIONS } from "./fixtures/host-expectations";

// bootstrap 은 옛 scaffold 를 합쳐 서버 판정(`axhub apps git-backend` 의 decision) 하나로
// Axhub 저장소·내 GitHub·회사 GitHub 조직을 모두 맡아요. 본문은 reference 없이 단일 SKILL.md 예요.
// 옛 판(Fast Start·12단계 GitHub 차단 우회·init-resume 이어하기·표시 규칙 목록·references/)을
// 잠그던 계약은 2026-11 통합 때 의도적으로 뺐어요.

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");
const CLAUDE = HOST_EXPECTATIONS.claude;
const BOOTSTRAP = readRepo("skills/bootstrap/SKILL.md");
const CODEX_BOOTSTRAP = readRepo("plugins/axhub-codex/skills/bootstrap/SKILL.md");
const FRONTMATTER = BOOTSTRAP.slice(0, BOOTSTRAP.indexOf("\n---\n", 4));

const sectionOf = (start: string, end: string): string => {
  const from = BOOTSTRAP.indexOf(start);
  const to = BOOTSTRAP.indexOf(end, from + start.length);
  expect(from, `missing section start: ${start}`).toBeGreaterThanOrEqual(0);
  expect(to, `missing section end: ${end}`).toBeGreaterThan(from);
  return BOOTSTRAP.slice(from, to);
};

// 순서 계약: 앞 문자열이 뒤 문자열보다 먼저 나와야 해요.
const expectOrder = (...needles: string[]): void => {
  const offsets = needles.map((needle) => {
    const offset = BOOTSTRAP.indexOf(needle);
    expect(offset, `missing: ${needle}`).toBeGreaterThanOrEqual(0);
    return offset;
  });
  for (let i = 1; i < offsets.length; i += 1) {
    expect(offsets[i - 1], `${needles[i - 1]} → ${needles[i]}`).toBeLessThan(offsets[i]);
  }
};

describe("bootstrap contract (bootstrap·scaffold 통합판)", () => {
  test("stays one compact SKILL.md that fits the codex 8,000B window", () => {
    // codex 는 SKILL.md 를 파일 앞에서부터 8,000B 만 읽어요 — 통째로 들어가야 절차가 잘리지 않아요.
    expect(Buffer.byteLength(CODEX_BOOTSTRAP, "utf8")).toBeLessThanOrEqual(8_000);
    expect(BOOTSTRAP).not.toContain("references/");
    expect(BOOTSTRAP).not.toContain("scaffold");
  });

  test("frontmatter routes new apps here and existing code to import", () => {
    expect(FRONTMATTER).toContain("새 axhub 앱을 템플릿으로 만들고 첫 배포까지 해요");
    expect(FRONTMATTER).toContain("\"새 앱 만들어줘\"");
    expect(FRONTMATTER).toContain("\"내 깃허브에 새 앱 만들어줘\"");
    expect(FRONTMATTER).toContain("\"회사 org 에 저장소 만들고 시작해줘\"");
    expect(FRONTMATTER).toContain("이미 코드가 있는 폴더는 import 로 가요");
    expect(FRONTMATTER).toContain("intent: \"yield to import\"");
  });

  test("checks the CLI, workspace, and company policy before anything else", () => {
    expectOrder(
      "axhub plugin-support preflight --json",
      "axhub plugin-support tenant-resolve --json",
      "axhub tenants features --tenant <회사> --json",
      "axhub apps git-backend --tenant <회사> --json",
      "앱 이름을 무엇으로 할까요?",
    );
    expect(BOOTSTRAP).toContain("안 되면 setup 스킬로 보내고 멈춰요");
    expect(BOOTSTRAP).toContain("\"새 앱을 어느 작업공간에 만들까요?\"");
    expect(BOOTSTRAP).toContain("이 회사에서 앱 서비스 권한이 없으니 회사 관리자에게 요청해 달라고 하고 멈춰요");
    expect(BOOTSTRAP).toContain("정책이 허락하지 않는 건 묻지도 하지도 않아요");
  });

  test("GitHub link shows the device code in chat and waits within the tool limit", () => {
    const github = sectionOf("### 4. GitHub 일 때만", "### 5. 이름");
    expectOrder("axhub github link --json", "axhub github link --wait-approval 90s --json");
    expect(github).toContain("`https://github.com/login/device 에서 코드 <코드> 를 입력해 주세요.`");
    expect(github).toContain("명령 시간 제한 120초 이상");
    expect(github).toContain("코드가 만료됐으면 `--fresh` 로 새로 받아요");
  });

  test("confirms the name, checks the address, and uses the single template", () => {
    const name = sectionOf("### 5. 이름", "### 6. 확인");
    expect(name).toContain("\"앱 이름을 무엇으로 할까요?\"");
    expect(name).toContain("slug 는 이름의 kebab-case, 주소와 저장소 이름은 slug 그대로예요");
    expect(name).toContain("axhub apps check-availability --tenant <회사> --slug <slug> --subdomain <slug> --json");
    expectOrder("axhub apps check-availability", "--dry-run --json");
    expect(BOOTSTRAP).toContain("--template nextjs-axhub");
    expect(BOOTSTRAP).toContain("--repo-name <slug> --subdomain <slug>");
    expect(BOOTSTRAP).not.toContain("어떤 템플릿으로 시작할까요?");
  });

  test("requires the preview confirmation before a single --execute", () => {
    const confirm = sectionOf("### 6. 확인", "### 7. 만들기");
    expect(confirm).toContain("--dry-run --json");
    expect(confirm).toContain("\"지금 만들고 배포까지 진행할까요?\"(진행 / 취소)");
    expect(confirm).toContain("\"바로 해줘\"라는 말은 승인이 아니에요");
    expect(confirm).toContain("카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요");
    expectOrder("지금 만들고 배포까지 진행할까요?", "`--execute` 를 붙여");
    expect(BOOTSTRAP).toContain("`--execute` 는 한 번만 해요");
    expect(BOOTSTRAP).toContain("다시 실행하지 말고 같은 상태 명령으로 이어가요");
    expect(BOOTSTRAP).toContain("물어야 할 게 생긴 순간 아무것도 만들지 않고 선택지만 보여주고 멈춰요");
  });

  test("waits with CLI-side watches and treats a timeout as still running", () => {
    expect(BOOTSTRAP).toContain(
      "axhub apps bootstrap-status <bootstrap_id> --tenant <회사> --watch --watch-timeout 9m --json",
    );
    expect(BOOTSTRAP).toContain("axhub deploy verify <deployment_id> --app <slug> --wait --json");
    expect(BOOTSTRAP).toContain("시간 안에 안 끝난 건(exit 6·10) 실패가 아니에요");
    expect(BOOTSTRAP).toContain("배포가 실패하면 diagnosis 스킬로 원인을 봐요");
  });

  test("publishes only when asked, ends with the result table, and hides internals", () => {
    expect(BOOTSTRAP).toContain("axhub publish --app <slug> --visibility public --execute --json");
    expect(BOOTSTRAP).not.toContain("--visibility public --dry-run");
    expect(BOOTSTRAP).toContain("결과만 표로 보여주고 끝내요");
    expect(BOOTSTRAP).toContain("확인하지 못한 항목은 됐다고 표시하지 않아요");
    expect(BOOTSTRAP).toContain("다음에 할 일은 추천하지 않아요");
    expect(BOOTSTRAP).toContain("사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요");
    expect(BOOTSTRAP).not.toContain(CLAUDE.forbidden.externalAutomationSkill);
    expect(BOOTSTRAP).not.toContain(CLAUDE.forbidden.externalAutomationPlugin);
  });

  test("blocks typoed Korean prompt copy observed in desktop bootstrap QA", () => {
    for (const typo of ["앵 이름", "앱 이륨", "앱 이맄", "작업구역", "곹치지", "컴셉", "가깜운", "멈췤요", "낹니다"]) {
      expect(BOOTSTRAP, typo).not.toContain(typo);
    }
  });
});
