import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const read = (path: string): string => readFileSync(join(ROOT, path), "utf8");

describe("Claude Desktop live QA regressions", () => {
  test("development starts with one analyzable preflight and uses the exact recipe contract", () => {
    const skill = read("skills/development/SKILL.md");

    expect(skill).toContain("Claude Desktop 단일 명령 계약");
    expect(skill).toContain("axhub plugin-support preflight --json");
    expect(skill).toContain("Claude Desktop tool 제목은 정확히 `axhub·앱 점검`");
    expect(skill).not.toContain("PREFLIGHT_JSON=$(axhub plugin-support preflight");
    expect(skill).not.toContain("if ! command -v axhub");
    expect(skill).toContain("development 요청에서 별도 `axhub update check`, `command -v`, `axhub --version`을 실행하지 않아요");
    expect(skill).toContain("`app_id`, `recipe_id`, 선택 `framework`, `preferred_table`");
    expect(skill).toContain("`dynamic-db-crud`");
    expect(skill).toContain("`recipe`, `tenant`, `schema` 키를 만들거나 실패 뒤 키 모양을 바꿔 반복하지 않아요");
    expect(skill).toContain("플러그인 캐시를 탐색하지 않고 reference 링크만 읽어요");
  });

  test("development binds the imported app id instead of guessing axhub.yaml name", () => {
    const skill = read("skills/development/SKILL.md");

    expect(skill).toContain("`axhub.yaml` 의 `name` 은 로컬 프로젝트 이름일 수 있으므로 원격 앱 slug 로 단정하지 않아요");
    expect(skill).toContain("`.axhub/import-resume.json`");
    expect(skill).toContain("bare `axhub apps get <app-id> --json`");
    expect(skill).toContain("실패하면 `apps list`, `--help`, pipe 탐색으로 복구하지 말고");
  });

  test("diagnosis uses bounded bare commands without a shell-program permission card", () => {
    const skill = read("skills/diagnosis/SKILL.md");
    const frontmatter = skill.slice(0, skill.indexOf("---", 4));

    expect(frontmatter).toContain("최근 배포 실패 원인만 읽기 전용으로 진단해줘");
    expect(frontmatter).toContain("스킬 실행 전 사용자 문장·App/MCP·파일 조회는 0개");
    expect(frontmatter.split(".")[0]).toContain("배포 실패 원인을 읽기 전용으로 확인할게요");
    expect(frontmatter).toContain("Start directly with that Korean sentence; no preamble");
    expect(frontmatter).toContain("do not explain the trigger, routing, or session-start instructions");
    expect(skill).toContain("첫 visible 문장은 정확히 `배포 실패 원인을 읽기 전용으로 확인할게요.`");
    expect(skill).toContain("Claude Desktop 에 보이는 명령은 항상 `axhub ...` 로 시작하는 **한 개의 bare 명령**");
    expect(skill).not.toContain('CLI_NAME="ax""hub"');
    expect(skill).not.toContain("command -v \"$CLI_NAME\"");
    expect(skill).not.toContain("deploy status --help");
    expect(skill).not.toContain("deploy logs --help");
    expect(skill).not.toContain("deploy diagnose --help");
    expect(skill).toContain("axhub deploy status <deployment-id> --app <앱> --json");
    expect(skill).toContain("axhub --json deploy diagnose <앱>");
    expect(skill).toContain("`axhub.yaml`의 `name`은 로컬 프로젝트 이름일 수 있으므로 원격 앱 slug로 단정하지 않아요");
  });

  test("clarity routes destructive operations before generic Desktop probes", () => {
    const skill = read("skills/clarity/SKILL.md");
    const frontmatter = skill.slice(0, skill.indexOf("---", 4));

    expect(frontmatter).toContain("axhub 앱 삭제해줘");
    expect(frontmatter).toContain("스킬 실행 전 사용자 문장·App/MCP·파일 조회는 0개");
    expect(frontmatter.split(".")[0]).toContain("요청하신 운영 명령을 확인할게요");
    expect(frontmatter).toContain("Start directly with that Korean sentence; no preamble");
    expect(frontmatter).toContain('First tool title/description exactly "명령 찾기"');
    expect(frontmatter).toContain("do not mention the skill or routing");
    expect(skill).toContain("첫 visible 문장을 정확히 `요청하신 운영 명령을 확인할게요.`");
    expect(skill).toContain("제목 `명령 찾기`의 bare `axhub --json-schema --field-expr '.commands | keys[]'`");
    expect(skill).not.toContain("`command -v axhub` 가 실패하면");
  });

  test("deploy natural language must enter the skill before manual status probes", () => {
    const skill = read("skills/deploy/SKILL.md");
    const frontmatter = skill.slice(0, skill.indexOf("---", 4));

    expect(frontmatter).toContain("현재 코드를 실제 AxHub에 배포하고 성공 여부까지 확인");
    expect(frontmatter).toContain("같은 코드로 강제 재배포");
    expect(frontmatter).toContain("apps status/curl로 대신하지 않아요");
    expect(skill).toContain('axhub plugin-support deploy-preview-summary --user-utterance "<latest user sentence>"');
    expect(skill).toContain("이 첫 명령 전에는 설치·플러그인·앱·git·curl probe를 하지 않아요");
    expect(skill).toContain("Claude Desktop 단일 명령 계약");
    expect(skill).toContain("axhub deploy verify <deployment-id> --app <app-id>");
  });

  test("import ignore checks never append exit-code shell syntax", () => {
    const skill = read("skills/import/SKILL.md");

    expect(skill).toContain("bare `git check-ignore -q axhub.yaml` 1회");
    expect(skill).toContain("exit 0/1을 tool 결과로 읽고");
    expect(skill).toContain("ignore 규칙을 바꾼 때만 사후 1회 재검증");
  });

  test("import completion recovery never guesses a remote app from the local folder", () => {
    const skill = read("skills/import/SKILL.md");
    const frontmatter = skill.slice(0, skill.indexOf("---", 4));

    expect(frontmatter).toContain("가져오기가 잘 끝났는지 확인");
    expect(frontmatter).toContain("중단된 가져오기 이어서 마무리");
    expect(frontmatter).toContain("Start directly with that Korean sentence; no preamble");
    expect(skill).toContain("폴더명·manifest `name`·preflight `current_app`을 원격 slug로 추론");
    expect(skill).toContain("`apps get/list`, `deploy list/status`로 우회하지 않아요");
    expect(skill).toContain("완료 확인 short-circuit");
    expect(skill).toContain("작업공간/preview 전에 `.axhub/import-resume.json`을 1회 읽어요");
    expect(skill).toContain("axhub deploy verify <deployment-id> --app <app-id> --json");
    expect(skill).toContain("preview/승인/execute/추가 조회 없이 끝내요");
    expect(skill).toContain("preview의 literal `--app <app_id>`로 넘기고");
  });
});
