# axhub plugin 정책 문서 체계 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 세 독자(에이전트·사용자·기여자)용 정책 문서 3개를 canonical 원천으로 만들고, 정책↔SKILL.md drift 를 bun test 로 막는다.

**Architecture:** `docs/policy/agent-policy.md` 는 규칙마다 `AP-N` ID + `적용:`(대상 파일) + `invariant:`(문자 그대로 존재해야 하는 문구) 블록을 갖고, `tests/policy-parity.test.ts` 가 이 블록을 파싱해 각 invariant 가 대상 SKILL.md 에 실존하는지 검사한다. `POLICY.md`(사용자 공개)는 bundle `ROOT_FILES` 에 1줄 추가로 배포판에 포함된다. `docs/policy/dev-policy.md`(기여자)는 `docs/` 가 bundle copy 대상이 아니라서 위치만으로 배포 제외된다.

**Tech Stack:** Bun test (기존 `bun test` 에 합류), markdown, 기존 스크립트(`scripts/build-plugin-bundle.ts`, `scripts/check-toss-tone-conformance.ts`) 최소 수정.

## Global Constraints

- 모든 한글 산문은 해요체. 금지어 목록의 canonical 원천은 `scripts/check-toss-tone-conformance.ts` 의 `FORBIDDEN` 배열 — 정책 문서 안에 금지어를 문자 그대로 나열하면 lint 가 그 라인에서 터지니 나열하지 말고 스크립트를 참조로 가리킬 것.
- 새 의존성 추가 금지. Bun ≥1.1.0, 기존 devDependencies 만 사용.
- `tests/plugin-bundle.test.ts` 의 bundle 예산: files > 8, bytes < 512KB — POLICY.md(약 2KB) 추가는 여유 있음.
- 커밋 메시지: `<type>: <subject>` 한국어, 이모지·AI attribution 없음.
- spec: `docs/superpowers/specs/2026-07-03-plugin-policy-docs-design.md`. 충돌 시 spec 이 아니라 이 계획이 아니라 — 구현 중 발견한 실제 SKILL.md 문구가 이긴다 (invariant 는 반드시 실존 문구).

## File Structure

| 파일 | 역할 |
|------|------|
| `docs/policy/agent-policy.md` (신규) | 에이전트 행동 규칙 canonical 원천. AP-1~AP-9 블록. |
| `tests/policy-parity.test.ts` (신규) | agent-policy 블록 파싱 + invariant 실존 검사. |
| `POLICY.md` (신규, repo 루트) | 사용자 공개 정책. bundle 포함. |
| `scripts/build-plugin-bundle.ts` (수정, 1줄) | `ROOT_FILES` 에 `"POLICY.md"` 추가. |
| `tests/plugin-bundle.test.ts` (수정, 1줄) | bundle 산출물에 POLICY.md 존재 assert. |
| `docs/policy/dev-policy.md` (신규) | 개발·운영 규칙 canonical 원천. DP-1~DP-7. |
| `scripts/check-toss-tone-conformance.ts` (수정, 1줄) | `explicit` 목록에 정책 문서 3개 추가. |
| `CLAUDE.md`, `README.md` (수정, 포인터만) | "원천은 정책 문서" 한 줄씩. 기존 서술 삭제 없음. |

---

### Task 1: agent-policy.md + parity 가드

**Files:**
- Create: `tests/policy-parity.test.ts`
- Create: `docs/policy/agent-policy.md`

**Interfaces:**
- Consumes: `skills/*/SKILL.md` 의 실존 문구 (아래 Step 1 에서 grep 재확인)
- Produces: 규칙 블록 형식 `## AP-N 제목` + `- 규칙:` + `- 적용:` (쉼표 구분 경로) + `- invariant:` (쌍따옴표 문구, 쉼표 구분). Task 3 의 dev-policy 와 Task 4 의 포인터가 이 파일 경로를 참조.

- [ ] **Step 1: invariant 후보 문구 실존 재확인**

Run:
```bash
cd /Users/wongil/Desktop/work/jocoding/axhub
grep -c "axhub deploy verify" skills/deploy/SKILL.md
grep -c "active_release_id" skills/deploy/SKILL.md
grep -c "preview-confirm" skills/deploy/SKILL.md
grep -c "파괴적 변경은 승인" skills/clarity/SKILL.md
grep -c "절대 직접 실행하지 않아요" skills/diagnosis/SKILL.md
grep -c "읽기 전용" skills/diagnosis/SKILL.md
grep -c "read 기본, write 게이트" skills/development/SKILL.md
grep -c "plugin-support preflight" skills/init/SKILL.md skills/deploy/SKILL.md
grep -c "양보" skills/clarity/SKILL.md skills/deploy/SKILL.md skills/development/SKILL.md skills/diagnosis/SKILL.md skills/import/SKILL.md skills/update/SKILL.md
grep -c "No automatic init" skills/onboarding/SKILL.md
grep -c "공개 표면만" skills/clarity/SKILL.md
```
Expected: 전부 1 이상. 0 이 나오는 문구가 있으면 해당 SKILL.md 에서 같은 의미의 실존 문구를 찾아 아래 Step 4 문서의 invariant 를 그 문구로 교체 (규칙 의미는 유지).

- [ ] **Step 2: 실패하는 parity 테스트 작성**

`tests/policy-parity.test.ts`:

```ts
import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const POLICY_PATH = join(REPO_ROOT, "docs", "policy", "agent-policy.md");

interface PolicyRule {
  id: string;
  files: string[];
  invariants: string[];
}

const parseRules = (markdown: string): PolicyRule[] => {
  const blocks = markdown.split(/^## (?=AP-)/m).slice(1);
  return blocks.map((block) => {
    const id = block.split(/\s/, 1)[0] ?? "";
    const fileLine = block.match(/^- 적용: (.+)$/m)?.[1] ?? "";
    const invariantLine = block.match(/^- invariant: (.+)$/m)?.[1] ?? "";
    const files = fileLine
      .split(",")
      .map((part) => part.trim().replaceAll("`", ""))
      .filter((part) => part.length > 0);
    const invariants = [...invariantLine.matchAll(/"([^"]+)"/g)].map((match) => match[1]!);
    return { id, files, invariants };
  });
};

describe("agent policy parity", () => {
  test("policy document exists", () => {
    expect(existsSync(POLICY_PATH), `missing ${POLICY_PATH}`).toBe(true);
  });

  test("every rule has files and invariants, and every invariant exists in every target file", () => {
    const rules = parseRules(readFileSync(POLICY_PATH, "utf8"));
    expect(rules.length).toBeGreaterThanOrEqual(9);

    for (const rule of rules) {
      expect(rule.files.length, `${rule.id}: 적용 대상 없음`).toBeGreaterThan(0);
      expect(rule.invariants.length, `${rule.id}: invariant 없음`).toBeGreaterThan(0);

      for (const file of rule.files) {
        const target = join(REPO_ROOT, file);
        expect(existsSync(target), `${rule.id}: 적용 파일 없음 — ${file}`).toBe(true);
        const content = readFileSync(target, "utf8");
        for (const invariant of rule.invariants) {
          expect(
            content.includes(invariant),
            `${rule.id}: "${invariant}" 가 ${file} 에 없음 — 정책과 SKILL.md 가 어긋남`,
          ).toBe(true);
        }
      }
    }
  });
});
```

- [ ] **Step 3: 테스트 실패 확인 (RED)**

Run: `bun test tests/policy-parity.test.ts`
Expected: FAIL — `missing .../docs/policy/agent-policy.md`

- [ ] **Step 4: agent-policy.md 작성 (GREEN 목표)**

`docs/policy/agent-policy.md` (Step 1 에서 교체한 invariant 가 있으면 반영):

```markdown
# axhub plugin 에이전트 행동 정책

axhub plugin 스킬들이 지켜야 하는 행동 규칙의 canonical 원천이에요. SKILL.md·README·CLAUDE.md 의 서술과 충돌하면 이 문서가 이겨요. 규칙 블록의 `적용:`/`invariant:` 형식은 `tests/policy-parity.test.ts` 가 파싱해 검증해요.

- `적용:` — 규칙 복사본을 반드시 담아야 하는 파일 경로예요. 복수면 쉼표로 구분해요.
- `invariant:` — 각 적용 파일에 문자 그대로 존재해야 하는 핵심 문구예요. 쌍따옴표로 감싸고 쉼표로 구분해요.
- 규칙을 바꿀 때는 이 문서를 먼저 고치고, 적용 파일을 따라 고쳐요. parity 테스트가 어긋남을 잡아요.

## AP-1 deploy 성공 선언
- 규칙: deployment-record 배포의 성공은 bound deployment id 에 대한 `axhub deploy verify` 가 terminal success 를 반환할 때만 선언해요. deploy-create stdout·status snapshot·watch 출력·latest 재탐색으로는 선언하지 않아요. static 앱(deploy_method=static)은 별도 lane — activate 의 `active_release_id` 로 성공을 선언하고 verify 를 호출하지 않아요.
- 적용: skills/deploy/SKILL.md
- invariant: "axhub deploy verify", "active_release_id"

## AP-2 deploy preview-confirm gate
- 규칙: 실제 배포 실행 전에 preview 를 보여주고 사용자 확인을 받아요. headless 에서는 dry-run/preview 로 멈춰요.
- 적용: skills/deploy/SKILL.md
- invariant: "preview-confirm"

## AP-3 파괴적 변경 승인
- 규칙: 삭제·롤백·force/yes/execute 급 파괴적 명령은 대화형 승인 1회 뒤에만 실행해요. 조회(목록·상태·로그)는 확인 없이 실행해요. headless 에서는 파괴적 명령을 실행하지 않고 preview/summary 로 멈춰요.
- 적용: skills/clarity/SKILL.md
- invariant: "파괴적 변경은 승인"

## AP-4 diagnosis 읽기 전용
- 규칙: diagnosis 는 배포 실패 원인을 읽기 전용으로만 진단해요. 재배포·롤백·새 deploy create 를 직접 실행하지 않고, 실행이 필요한 다음 행동은 담당 스킬로 자연어 handoff 만 남겨요.
- 적용: skills/diagnosis/SKILL.md
- invariant: "읽기 전용", "절대 직접 실행하지 않아요"

## AP-5 development write 게이트
- 규칙: development 의 코드 생성은 read 를 기본으로 하고, 스키마 변경 같은 write 는 preview-confirm 승인과 존재 확인 뒤에만 실행해요. headless 에서는 아무것도 바꾸지 않아요.
- 적용: skills/development/SKILL.md
- invariant: "read 기본, write 게이트"

## AP-6 CLI preflight 게이트
- 규칙: init·deploy 는 시작 시 `axhub` 존재와 plugin-support preflight 동작을 확인해요. CLI 가 없거나 preflight 가 안 되면 멈추고 설치/업그레이드를 안내하며, 절대 우회하지 않아요. 버전 숫자를 직접 비교하지 않아요.
- 적용: skills/init/SKILL.md, skills/deploy/SKILL.md
- invariant: "plugin-support preflight"

## AP-7 skill 양보 라우팅
- 규칙: 각 스킬은 자기 경계 밖 요청을 담당 스킬로 양보해요. 경계가 섞이면 자기 몫만 끝내고 나머지는 담당 스킬로 넘겨요.
- 적용: skills/clarity/SKILL.md, skills/deploy/SKILL.md, skills/development/SKILL.md, skills/diagnosis/SKILL.md, skills/import/SKILL.md, skills/update/SKILL.md
- invariant: "양보"

## AP-8 onboarding 자동 init 금지
- 규칙: onboarding 은 빈 폴더나 manifest 없는 폴더를 발견해도 init 을 자동 실행하거나 앱을 자동 생성하지 않아요. Ready card 로 끝내요.
- 적용: skills/onboarding/SKILL.md
- invariant: "No automatic init"

## AP-9 clarity 공개 표면만
- 규칙: clarity 는 hidden `axhub plugin-support` 그룹을 탐색·실행하지 않아요. 공개 `--json-schema`/`--help` 표면만 사용해요.
- 적용: skills/clarity/SKILL.md
- invariant: "공개 표면만"
```

- [ ] **Step 5: 테스트 통과 확인 (GREEN)**

Run: `bun test tests/policy-parity.test.ts`
Expected: PASS (2 tests)

- [ ] **Step 6: drift 감지 동작 증명 (negative check)**

agent-policy.md 의 AP-1 invariant 라인 끝에 `, "__drift_check__"` 를 임시로 추가한 뒤:

Run: `bun test tests/policy-parity.test.ts`
Expected: FAIL — `AP-1: "__drift_check__" 가 skills/deploy/SKILL.md 에 없음`

임시 추가를 되돌리고 다시 실행:

Run: `bun test tests/policy-parity.test.ts`
Expected: PASS

- [ ] **Step 7: tone 확인 + 전체 테스트**

Run:
```bash
bun scripts/check-toss-tone-conformance.ts --strict --include docs/policy/agent-policy.md
bun test
```
Expected: lint 0 error / bun test 전부 PASS

- [ ] **Step 8: Commit**

```bash
git add docs/policy/agent-policy.md tests/policy-parity.test.ts
git commit -m "feat: 에이전트 행동 정책 원천 문서와 parity 가드 추가"
```

---

### Task 2: POLICY.md + bundle 포함

**Files:**
- Create: `POLICY.md`
- Modify: `scripts/build-plugin-bundle.ts:6` (`ROOT_FILES`)
- Modify: `tests/plugin-bundle.test.ts:64` 부근 (existsSync assert 1줄 추가)

**Interfaces:**
- Consumes: 없음 (독립)
- Produces: repo 루트 `POLICY.md` — Task 4 의 README 포인터가 이 경로를 링크.

- [ ] **Step 1: bundle 테스트에 실패하는 assert 추가 (RED)**

`tests/plugin-bundle.test.ts` 의 `expect(existsSync(join(outDir, "LICENSE"))).toBe(true);` 라인 바로 아래에 추가:

```ts
      expect(existsSync(join(outDir, "POLICY.md"))).toBe(true);
```

Run: `bun test tests/plugin-bundle.test.ts`
Expected: FAIL — POLICY.md 가 bundle 산출물에 없음

- [ ] **Step 2: POLICY.md 작성**

`POLICY.md`:

```markdown
# axhub plugin 정책

axhub Claude Code plugin 이 사용자 환경에서 무엇을 하고 무엇을 하지 않는지 공개하는 문서예요.

## 네트워크 접근
- 스킬과 훅은 `axhub` CLI 와 axhub MCP 서버를 통해서만 네트워크에 접근해요.
- SessionStart auto-update 훅은 24시간에 1회만 `axhub update check` 로 새 버전을 확인해요. 네트워크 호출은 훅 스크립트가 아니라 CLI 가 수행해요.

## 로컬에 기록하는 파일
- `~/.axhub/cache/.plugin-update-check` — 업데이트 확인 throttle 마커예요.
- `~/.axhub/cache/.onboarding-mcp-restart` — 온보딩 MCP 재시작 resume 마커예요 (7일 TTL).

## 자동 업데이트와 끄는 법
- CLI 업데이트는 확인 후 자동 적용될 수 있어요. 플러그인 업데이트는 적용해도 Claude Code 재시작 후에 반영돼요.
- `AXHUB_NO_AUTO_UPDATE=1` — 자동 적용 없이 안내만 해요.
- `AXHUB_NO_ONBOARDING_RESUME=1` — 온보딩 재시작 resume 안내를 꺼요.

## 파괴적 작업 승인
- 삭제·롤백·force/execute 급 변경은 항상 사용자 확인 뒤에만 실행해요. headless 환경에서는 실행하지 않고 preview 로 멈춰요.

## 데이터 범위
- axhub MCP 도구는 OAuth 로 검증된 tenant 범위 안에서만 동작하고, 기본 도구는 read-only 예요.
- 스킬은 credential 을 파일이나 로그에 남기지 않아요.

에이전트 행동 규칙의 원천은 repo 의 `docs/policy/agent-policy.md` 예요.
```

- [ ] **Step 3: ROOT_FILES 에 POLICY.md 추가 (GREEN 목표)**

`scripts/build-plugin-bundle.ts` line 6:

```ts
const ROOT_FILES = ["README.md", "LICENSE", "POLICY.md"] as const;
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `bun test tests/plugin-bundle.test.ts`
Expected: PASS

- [ ] **Step 5: tone 확인**

Run: `bun scripts/check-toss-tone-conformance.ts --strict --include POLICY.md`
Expected: 0 error

- [ ] **Step 6: Commit**

```bash
git add POLICY.md scripts/build-plugin-bundle.ts tests/plugin-bundle.test.ts
git commit -m "feat: 사용자 대상 POLICY.md 를 plugin bundle 에 포함"
```

---

### Task 3: dev-policy.md + lint:tone 커버 확장

**Files:**
- Create: `docs/policy/dev-policy.md`
- Modify: `scripts/check-toss-tone-conformance.ts:54` 부근 (`explicit` 배열)

**Interfaces:**
- Consumes: Task 1 의 `docs/policy/agent-policy.md`, Task 2 의 `POLICY.md` (explicit 목록에 함께 등록)
- Produces: `docs/policy/dev-policy.md` — Task 4 의 CLAUDE.md·README 포인터가 이 경로를 참조.

- [ ] **Step 1: dev-policy.md 작성**

주의: 금지어를 문자 그대로 나열하지 말 것 (이 파일이 lint 스캔 대상이 됨). 목록은 스크립트 참조로 가리킨다.

`docs/policy/dev-policy.md`:

```markdown
# axhub plugin 개발·운영 정책

repo 기여자를 위한 개발·운영 규칙의 canonical 원천이에요. README·CLAUDE.md 의 서술과 충돌하면 이 문서가 이겨요. 에이전트 행동 규칙은 `docs/policy/agent-policy.md`, 사용자 공개 정책은 `POLICY.md` 가 각각 원천이에요.

## DP-1 diet 체제 — skill 추가 기준
- 공개 skill 은 8개(onboarding/init/import/deploy/development/diagnosis/clarity/update)를 유지해요.
- 새 skill 은 기존 8개의 경계·양보 규칙으로 라우팅할 수 없는 사용자 의도가 반복 관측될 때만 추가해요.
- 판정·실행 로직은 plugin 안에 두지 않고 ax-hub-cli 에 둬요. 라우팅 품질은 외부 corpus 가 아니라 frontmatter `description`·`examples` 에 투자해요.

## DP-2 tone — 해요체
- 모든 한글 산문은 해요체로 써요. 금지 토큰의 canonical 목록은 `scripts/check-toss-tone-conformance.ts` 의 `FORBIDDEN` 배열이에요.
- gate: `bun run lint:tone --strict` 0 error. 스캔 대상은 `skills/*/SKILL.md` + `explicit` 목록(정책 문서 3개)이에요.

## DP-3 frontmatter 유효성
- 8개 SKILL.md 의 frontmatter 는 `tests/frontmatter.test.ts` 로 검증해요. `name`·`description` 은 라우팅 표면이라 깨지면 안 돼요.

## DP-4 release flow
- `commit-and-tag-version` 기반 3단계로 릴리즈해요:
  1. `bun run release` — bump + commit (tag 미생성)
  2. CHANGELOG 에 해요체 1-3 문장 narrative 추가 후 `git commit --amend --no-edit -a`
  3. `bun run release:tag` — tag 생성 + push

## DP-5 quality gates
- `bun run lint:tone --strict` — 한글 tone 0 error
- `bun test` — frontmatter·bundle·routing·policy parity 포함 전부 PASS
- `bun run plugin:bundle` — clean bundle 생성, 로컬 Claude Code 검증은 repo 루트가 아니라 `dist/axhub-plugin` 사용
- 대표 여정 회귀 — 첫 셋업 → 앱 생성 → 배포 → 상태 확인 경로를 문서·skill 본문·fixture 계약으로 같은 방향에 맞춰요

## DP-6 hidden 표면 계약
- skill 이 쓰는 `axhub plugin-support <cmd>` hidden 그룹은 외부 무보증이에요. 계약 parity 테스트 + preflight 게이트(agent-policy AP-6)로 CLI 와 동기화해요.
- clarity 는 hidden 표면을 쓰지 않아요 (agent-policy AP-9).

## DP-7 bundle 규칙
- bundle 포함은 `scripts/build-plugin-bundle.ts` 의 `ROOT_FILES`(`README.md`, `LICENSE`, `POLICY.md`) + `ROOT_DIRS`(`.claude-plugin/`, `skills/`)가 원천이에요.
- `docs/`, `tests/`, `scripts/` 같은 개발 산출물은 포함하지 않아요. 검증은 `bun run plugin:bundle` + `tests/plugin-bundle.test.ts` 가 해요.
```

- [ ] **Step 2: lint explicit 목록에 정책 문서 3개 추가**

`scripts/check-toss-tone-conformance.ts` 의 `const explicit: string[] = [];` 를 다음으로 교체:

```ts
  const explicit: string[] = [
    "POLICY.md",
    "docs/policy/agent-policy.md",
    "docs/policy/dev-policy.md",
  ];
```

- [ ] **Step 3: lint + 전체 테스트**

Run:
```bash
bun run lint:tone --strict
bun test
```
Expected: lint 0 error (정책 문서 3개 포함 스캔), bun test 전부 PASS

- [ ] **Step 4: Commit**

```bash
git add docs/policy/dev-policy.md scripts/check-toss-tone-conformance.ts
git commit -m "feat: 개발·운영 정책 문서 추가와 tone lint 커버 확장"
```

---

### Task 4: CLAUDE.md·README 원천 포인터

**Files:**
- Modify: `CLAUDE.md` (`# axhub plugin (diet 체제)` 섹션 도입부, line 103 부근)
- Modify: `README.md` (`## 🔒 안전과 신뢰` line 187 부근, `## 🛠️ 개발과 기여` line 196 부근)

**Interfaces:**
- Consumes: Task 1~3 의 세 문서 경로
- Produces: 없음 (포인터만)

- [ ] **Step 1: CLAUDE.md 포인터 추가**

`# axhub plugin (diet 체제)` 제목 바로 아래 첫 문단 앞에 추가:

```markdown
> **정책 원천:** 에이전트 행동 규칙은 `docs/policy/agent-policy.md`, 개발·운영 규칙은 `docs/policy/dev-policy.md`, 사용자 공개 정책은 `POLICY.md` 가 canonical 원천이에요. 이 파일의 요약과 충돌하면 정책 문서가 이겨요. drift 가드: `tests/policy-parity.test.ts`.
```

- [ ] **Step 2: README 포인터 추가**

`## 🔒 안전과 신뢰` 섹션 본문 끝에 추가:

```markdown
플러그인이 네트워크·로컬 파일·자동 업데이트에서 무엇을 하는지는 [POLICY.md](./POLICY.md) 에 공개돼 있어요.
```

`## 🛠️ 개발과 기여` 섹션 본문 끝에 추가:

```markdown
개발·운영 규칙의 원천은 [docs/policy/dev-policy.md](./docs/policy/dev-policy.md), 에이전트 행동 규칙의 원천은 [docs/policy/agent-policy.md](./docs/policy/agent-policy.md) 예요.
```

- [ ] **Step 3: 회귀 확인**

Run:
```bash
bun run lint:tone --strict
bun test
```
Expected: 전부 PASS (CLAUDE.md·README 는 lint 스캔 대상이 아니지만 해요체 유지)

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: CLAUDE.md·README 에 정책 원천 포인터 추가"
```
