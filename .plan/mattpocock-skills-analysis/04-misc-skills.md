# 04. Misc Skills — 4개 상세 분석

가끔 쓰는 도구. 각각 매우 specific 한 task.

> **참고** — `01-architecture.md` 에서 관찰한 대로 misc 4개는 `plugin.json` 에 등록되지 않아요. CONTEXT.md 정책 (`engineering/`, `productivity/`, `misc/` 모두 `plugin.json` 필수) 과 충돌하는데, link-skills.sh 는 모든 SKILL 을 symlink 하므로 dev 환경 사용 가능. 단, plugin distribution 으로 배포되는 건 12개 (engineering 9 + productivity 3) 만.

---

## 1. `git-guardrails-claude-code` — 위험한 git 명령 차단 hook

**파일**: `skills/misc/git-guardrails-claude-code/SKILL.md` (95) + `scripts/block-dangerous-git.sh` (25)

**Frontmatter**:
```yaml
name: git-guardrails-claude-code
description: Set up Claude Code hooks to block dangerous git commands
  (push, reset --hard, clean, branch -D, etc.) before they execute.
  Use when user wants to prevent destructive git operations,
  add git safety hooks, or block git push/reset in Claude Code.
```

### 차단 대상

| Pattern | 의미 |
|---|---|
| `git push` (모든 variant 포함 `--force`) | remote 영향 |
| `git reset --hard` | working tree + index 파괴 |
| `git clean -f` / `git clean -fd` | untracked file 삭제 |
| `git branch -D` | 강제 branch 삭제 |
| `git checkout .` / `git restore .` | working tree 변경 폐기 |

차단 시 Claude 는 "user has prevented you from doing this" 메시지 받음.

### Setup 5 step

#### 1. Ask scope
프로젝트 only (`.claude/settings.json`) vs 모든 프로젝트 (`~/.claude/settings.json`)?

#### 2. Copy hook script
번들된 `scripts/block-dangerous-git.sh` 를 target 위치 복사:
- Project: `.claude/hooks/block-dangerous-git.sh`
- Global: `~/.claude/hooks/block-dangerous-git.sh`

`chmod +x`.

#### 3. Add hook to settings

Project (`.claude/settings.json`):
```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/block-dangerous-git.sh"
          }
        ]
      }
    ]
  }
}
```

Global (`~/.claude/settings.json`) — 같은 구조, path 만 `~/.claude/hooks/...`.

기존 settings 있으면 `hooks.PreToolUse` 배열에 **merge** — overwrite 금지.

#### 4. Ask customization
사용자에게 패턴 add/remove 묻기. 복사된 script 편집.

#### 5. Verify
```bash
echo '{"tool_input":{"command":"git push origin main"}}' | <path-to-script>
```
exit 2 + BLOCKED message 확인.

### `block-dangerous-git.sh` 본체

```bash
#!/bin/bash

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command')

DANGEROUS_PATTERNS=(
  "git push"
  "git reset --hard"
  "git clean -fd"
  "git clean -f"
  "git branch -D"
  "git checkout \."
  "git restore \."
  "push --force"
  "reset --hard"
)

for pattern in "${DANGEROUS_PATTERNS[@]}"; do
  if echo "$COMMAND" | grep -qE "$pattern"; then
    echo "BLOCKED: '$COMMAND' matches dangerous pattern '$pattern'. The user has prevented you from doing this." >&2
    exit 2
  fi
done

exit 0
```

**메커니즘**:
- STDIN 의 JSON 에서 `.tool_input.command` 추출 (jq 의존).
- DANGEROUS_PATTERNS 배열 순회, `grep -qE` 매치.
- 매치 시 stderr 메시지 + exit 2 (Claude Code 의 "block" 시그널).
- 매치 없으면 exit 0 (proceed).

**관찰**:
- 단순함 — 25 lines bash. 정규식 매치 only.
- patterns 가 중복 (`"git push"` + `"push --force"`, `"git reset --hard"` + `"reset --hard"`) — defensive: command 가 `sudo git push` 같이 prefix 가질 수 있어요.
- `git checkout \.` / `git restore \.` 의 `\.` 는 literal dot match — wildcard X. 특정 destructive form 만.
- `jq` 의존 — Claude Code env 에 보통 있음.
- merge 권장 — 다른 hook 보존.

---

## 2. `migrate-to-shoehorn` — Test 의 `as` 를 shoehorn 으로

**파일**: `skills/misc/migrate-to-shoehorn/SKILL.md` (118 lines)

**Frontmatter**:
```yaml
name: migrate-to-shoehorn
description: Migrate test files from `as` type assertions to @total-typescript/shoehorn.
  Use when user mentions shoehorn, wants to replace `as` in tests, or needs partial test data.
```

### Why shoehorn?

`shoehorn` (Matt 본인 라이브러리 — `@total-typescript/shoehorn`) 가 partial data 전달 + TypeScript happy 둘 다 가능. `as` 대체.

> **Test code only.** Never use shoehorn in production code.

`as` 의 문제:
- "쓰지 마" 라고 train 됨
- 수동으로 target type 명시
- Double-as (`as unknown as Type`) — 의도적 wrong data

### Install
```bash
npm i @total-typescript/shoehorn
```

### Migration patterns

#### 1. Large object + few needed properties

Before:
```ts
type Request = {
  body: { id: string };
  headers: Record<string, string>;
  cookies: Record<string, string>;
  // ...20 more properties
};

it("gets user by id", () => {
  getUser({
    body: { id: "123" },
    headers: {},
    cookies: {},
    // ...fake all 20 properties
  });
});
```

After:
```ts
import { fromPartial } from "@total-typescript/shoehorn";

it("gets user by id", () => {
  getUser(fromPartial({ body: { id: "123" } }));
});
```

#### 2. `as Type` → `fromPartial()`

Before: `getUser({ body: { id: "123" } } as Request);`
After: `getUser(fromPartial({ body: { id: "123" } }));`

#### 3. `as unknown as Type` → `fromAny()`

Before: `getUser({ body: { id: 123 } } as unknown as Request); // wrong type on purpose`
After: `getUser(fromAny({ body: { id: 123 } }));`

### When to use each

| Function | Use case |
|---|---|
| `fromPartial()` | partial data + type-check 통과 |
| `fromAny()` | 의도적 wrong data (autocomplete 유지) |
| `fromExact()` | full object 강제 (나중 fromPartial 로 swap 위해) |

### Workflow

1. **Gather requirements** 인터뷰:
   - `as` 가 문제인 test file?
   - large object 인데 일부 property 만?
   - error testing 위한 의도적 wrong data?

2. **Install + migrate**:
   - [ ] Install
   - [ ] `as` 찾기: `grep -r " as [A-Z]" --include="*.test.ts" --include="*.spec.ts"`
   - [ ] `as Type` → `fromPartial()`
   - [ ] `as unknown as Type` → `fromAny()`
   - [ ] import 추가
   - [ ] type check verify

**관찰**:
- 매트 본인 라이브러리 — biased recommendation, 정직히 명시.
- "Test code only" 강조 — production 사용 금지 명확.
- grep regex `" as [A-Z]"` — capital letter 시작 type 만. lowercase ("as string", "as const") 는 valid use 라 skip.
- `fromExact` 는 migration target 아니고 transition tool.

---

## 3. `scaffold-exercises` — Exercise dir 구조 생성 + lint 통과

**파일**: `skills/misc/scaffold-exercises/SKILL.md` (106 lines)

**Frontmatter**:
```yaml
name: scaffold-exercises
description: Create exercise directory structures with sections, problems, solutions,
  and explainers that pass linting.
  Use when user wants to scaffold exercises, create exercise stubs, or set up a new course section.
```

매트의 코스 (Total TypeScript / aihero.dev) 콘텐츠 구조 — `pnpm ai-hero-cli internal lint` 통과시키는 디렉토리 scaffold.

### Naming

- **Section**: `XX-section-name/` (e.g. `01-retrieval-skill-building`)
- **Exercise**: `XX.YY-exercise-name/` (e.g. `01.03-retrieval-with-bm25`)
- 모두 dash-case lowercase.

### Variants

각 exercise 는 다음 중 하나 이상 subfolder:
- `problem/` — student workspace + TODO
- `solution/` — reference impl
- `explainer/` — conceptual material, no TODO

stubbing 시 default `explainer/` (plan 이 다른 거 명시 안 하면).

### Required files

각 subfolder 에 `readme.md`:
- 비어있지 X (한 title 줄도 OK)
- broken link X

stub 시 minimal:
```md
# Exercise Title

Description here
```

코드 있으면 `main.ts` (>1 line). stub 은 readme-only OK.

### Workflow

1. plan parse — section / exercise / variant 추출
2. `mkdir -p` 각 path
3. stub readme — variant 마다
4. `pnpm ai-hero-cli internal lint`
5. error 고치고 반복

### Lint rules summary

linter 검사:
- 각 exercise 가 subfolder 가짐
- `problem/` / `explainer/` / `explainer.1/` 중 하나 존재
- primary subfolder `readme.md` non-empty
- `.gitkeep` 금지
- `speaker-notes.md` 금지
- broken link 금지
- `pnpm run exercise` 명령 readme 에 금지
- `main.ts` 필수 (readme-only 아니면)

### Move/rename

`git mv` 사용 (history 보존). prefix 갱신. lint 재실행.

```bash
git mv exercises/01-retrieval/01.03-embeddings exercises/01-retrieval/01.04-embeddings
```

### Stubbing 예시

plan:
```
Section 05: Memory Skill Building
- 05.01 Introduction to Memory
- 05.02 Short-term Memory (explainer + problem + solution)
- 05.03 Long-term Memory
```

생성:
```bash
mkdir -p exercises/05-memory-skill-building/05.01-introduction-to-memory/explainer
mkdir -p exercises/05-memory-skill-building/05.02-short-term-memory/{explainer,problem,solution}
mkdir -p exercises/05-memory-skill-building/05.03-long-term-memory/explainer
```

readme stub:
```
exercises/05-memory-skill-building/05.01-introduction-to-memory/explainer/readme.md -> "# Introduction to Memory"
...
```

**관찰**:
- 매우 도메인 specific — aihero.dev 코스 프로젝트용. 다른 사용자 직접 적용성 낮음.
- linter (`pnpm ai-hero-cli`) 가 source-of-truth — skill 은 lint 통과 보장.
- `git mv` 강조 — history 보존 (refactor / rename 시 일반 원칙).

---

## 4. `setup-pre-commit` — Husky + lint-staged + Prettier + typecheck/test

**파일**: `skills/misc/setup-pre-commit/SKILL.md` (91 lines)

**Frontmatter**:
```yaml
name: setup-pre-commit
description: Set up Husky pre-commit hooks with lint-staged (Prettier),
  type checking, and tests in the current repo.
  Use when user wants to add pre-commit hooks, set up Husky, configure lint-staged,
  or add commit-time formatting/typechecking/testing.
```

### What gets set up

- **Husky** pre-commit hook
- **lint-staged** + Prettier on staged files
- **Prettier** config (없으면)
- pre-commit 안에 **typecheck** + **test** script

### Steps 8

#### 1. Detect package manager
`package-lock.json` (npm) / `pnpm-lock.yaml` (pnpm) / `yarn.lock` (yarn) / `bun.lockb` (bun). 불명 시 npm default.

#### 2. Install
```
husky lint-staged prettier
```
devDependency.

#### 3. Init Husky
```bash
npx husky init
```
`.husky/` dir + `prepare: "husky"` script in package.json.

#### 4. `.husky/pre-commit`
```
npx lint-staged
npm run typecheck
npm run test
```

> Husky v9+ 부터 shebang 불필요.

`npm` 을 detected manager 로 변경. `typecheck` / `test` script 없으면 그 줄 omit + 사용자에게 알림.

#### 5. `.lintstagedrc`
```json
{
  "*": "prettier --ignore-unknown --write"
}
```

`--ignore-unknown` — Prettier 가 parse 못하는 파일 (이미지 등) skip.

#### 6. `.prettierrc` (없으면)
```json
{
  "useTabs": false,
  "tabWidth": 2,
  "printWidth": 80,
  "singleQuote": false,
  "trailingComma": "es5",
  "semi": true,
  "arrowParens": "always"
}
```

기존 Prettier config 있으면 만들지 않음.

#### 7. Verify
- [ ] `.husky/pre-commit` exist + executable
- [ ] `.lintstagedrc` exist
- [ ] `prepare` script `"husky"`
- [ ] Prettier config exist
- [ ] `npx lint-staged` 작동

#### 8. Commit
> Stage all changed/created files, commit message: `Add pre-commit hooks (husky + lint-staged + prettier)`
>
> 새 hook 통과 = smoke test.

### Notes
- Husky v9+ shebang 안 함.
- `prettier --ignore-unknown` 으로 binary skip.
- pre-commit = lint-staged (빠름, staged-only) → typecheck → test.

**관찰**:
- 매우 deterministic skill — 순서대로 명령 실행.
- package manager auto-detect — 가장 흔한 4종 cover.
- 자가 일관성: 마지막에 commit 으로 본인 hook smoke-test.
- script 부재 시 graceful — typecheck/test 없으면 그 줄 빼고 사용자에게 알림.

---

## Misc 한 줄 요약

| Skill | 역할 | LOC | Script | plugin.json? |
|---|---|---|---|---|
| git-guardrails-claude-code | git destructive 차단 hook | 95 | bash 25L | ✗ |
| migrate-to-shoehorn | test `as` → shoehorn | 118 | - | ✗ |
| scaffold-exercises | exercise dir scaffold | 106 | - | ✗ |
| setup-pre-commit | Husky 셋업 | 91 | - | ✗ |

4개 모두 매우 specific task 에 매우 정확한 step. plugin.json 미등록은 정책 drift (CONTEXT.md L9 위반) — 의도된 것일 수도.
