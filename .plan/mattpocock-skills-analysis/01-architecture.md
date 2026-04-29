# 01. Architecture — repo 구조 / Plugin 모델 / 의존성 정책

## 디렉토리 레이아웃

```
mattpocock/skills/
├── .claude-plugin/
│   └── plugin.json              # 12 skill 등록 (engineering 9 + productivity 3, NO misc)
├── .out-of-scope/
│   └── question-limits.md       # "200 questions" 요청 거절 메모 (#44)
├── CLAUDE.md                    # CONTEXT.md 의 skills repo 도메인 용어 (Issue tracker / Issue / Triage role)
├── CONTEXT.md                   # 이 repo 의 bucket 정책: README + plugin.json 등록 규칙
├── LICENSE                      # MIT 2026
├── README.md                    # 4 failure mode framing + skill 카탈로그
├── docs/
│   └── adr/
│       └── 0001-explicit-setup-pointer-only-for-hard-dependencies.md
├── scripts/
│   └── link-skills.sh           # ~/.claude/skills/ 에 심볼릭 링크
└── skills/
    ├── deprecated/              # 4 skill, plugin 제외
    ├── engineering/             # 9 skill, plugin 포함
    ├── misc/                    # 4 skill, plugin 제외 (CONTEXT.md 정책 위반? 아래 참고)
    ├── personal/                # 2 skill, plugin 제외
    └── productivity/            # 3 skill, plugin 포함
```

## 5개 bucket 정책 (CONTEXT.md L1-13)

```
- engineering/  — daily code work
- productivity/ — daily non-code workflow tools
- misc/         — kept around but rarely used
- personal/     — tied to my own setup, not promoted
- deprecated/   — no longer used
```

**규칙** (CONTEXT.md L9):
> Every skill in `engineering/`, `productivity/`, or `misc/` must have a reference in the top-level `README.md` and an entry in `.claude-plugin/plugin.json`. Skills in `personal/` and `deprecated/` must not appear in either.

**현실 vs 정책 — 분석가 관찰**: `plugin.json` 에는 engineering 9 + productivity 3 = **12 skill** 만 등록되어 있고, **misc 4 skill 은 빠져 있어요**. CONTEXT.md 정책 ("must have a entry in plugin.json") 과 충돌. 매트가 의도적으로 misc 를 plugin distribution 에서 빼고, repo 안에 보존만 한 것으로 보여요. 또는 정책 자체가 살짝 stale. (이건 axhub 작업자 관점에서 takeaway: bucket 정책과 plugin manifest 가 자동 검증되지 않으면 drift 가능.)

각 bucket 마다 `README.md` 가 있고, skill name 을 SKILL.md 로 링크해요. 5/5 bucket README 모두 존재.

## Plugin manifest (`.claude-plugin/plugin.json`)

```json
{
  "name": "mattpocock-skills",
  "skills": [
    "./skills/engineering/diagnose",
    "./skills/engineering/grill-with-docs",
    "./skills/engineering/triage",
    "./skills/engineering/improve-codebase-architecture",
    "./skills/engineering/setup-matt-pocock-skills",
    "./skills/engineering/tdd",
    "./skills/engineering/to-issues",
    "./skills/engineering/to-prd",
    "./skills/engineering/zoom-out",
    "./skills/productivity/caveman",
    "./skills/productivity/grill-me",
    "./skills/productivity/write-a-skill"
  ]
}
```

**관찰**:
- 단순한 manifest — name + skill path 배열만.
- 버전, author, description 같은 metadata 없음.
- skill 등록은 디렉토리 path (SKILL.md 가 아니라). plugin loader 가 디렉토리에서 SKILL.md 를 자동으로 찾는 구조.
- engineering 9 + productivity 3 = 12 (misc 4 / personal 2 / deprecated 4 = 10 제외).

**axhub 비교**: axhub `plugin.json` 은 `name / displayName / description / version / author / homepage / license / commands / skills / agents / hooks / mcpServers` 등 풍부한 metadata 가짐. mattpocock 은 minimal.

## `link-skills.sh` — 설치 메커니즘

```bash
# 핵심 로직 (scripts/link-skills.sh L26-38)
find "$REPO/skills" -name SKILL.md -not -path '*/node_modules/*' -print0 |
while IFS= read -r -d '' skill_md; do
  src="$(dirname "$skill_md")"
  name="$(basename "$src")"
  target="$DEST/$name"   # $HOME/.claude/skills/$name

  if [ -e "$target" ] && [ ! -L "$target" ]; then
    rm -rf "$target"
  fi

  ln -sfn "$src" "$target"
done
```

**동작**:
1. `$REPO/skills/**/SKILL.md` 를 모두 발견 (`personal/`, `deprecated/`, `misc/` 포함 — `plugin.json` 과 무관하게 **전부** 링크함)
2. SKILL.md 디렉토리를 `~/.claude/skills/<name>/` 에 symlink
3. 같은 name 의 plain dir 이 이미 있으면 `rm -rf` 후 symlink 로 대체

**Self-loop 감지** (L13-22):
```bash
if [ -L "$DEST" ]; then
  resolved="$(readlink -f "$DEST")"
  case "$resolved" in
    "$REPO"|"$REPO"/*)
      echo "error: $DEST is a symlink into this repo ..." >&2
      exit 1
      ;;
  esac
fi
```

`~/.claude/skills` 자체가 이 repo 안으로 들어가는 symlink 면 (e.g. dev 환경에서 실수로) bail-out — 같은 트리에 self-link 안 만들기.

**관찰**:
- bash POSIX-ish, `set -euo pipefail`. 38 lines.
- bucket 무관 모든 SKILL.md 링크 → user 가 deprecated/personal 도 사용 가능. plugin manifest 와 분리된 두 번째 진입로.
- README.md 의 quickstart 는 `npx skills@latest add mattpocock/skills` 라는 별도 도구를 가정 — 이 link-skills.sh 는 매트 본인 dev 용으로 보여요.

## ADR-0001 — Hard vs Soft dependency 분리

`docs/adr/0001-explicit-setup-pointer-only-for-hard-dependencies.md` 가 이 repo 의 핵심 설계 결정 하나를 명시해요.

**문제**: 7개 engineering skill 이 `setup-matt-pocock-skills` 가 만든 per-repo config (issue tracker / triage label / domain doc) 에 의존. 모든 SKILL.md 에 "run /setup-matt-pocock-skills if not configured" 를 붙이면 token 낭비 + cargo cult.

**해결**: 의존을 명시적으로 두 종류로 split.

### Hard dependency — 명시적 setup pointer 포함

> "… should have been provided to you — run `/setup-matt-pocock-skills` if not."

config 없으면 출력이 **fuzzy 가 아니라 잘못됨** (틀린 issue tracker 에 publish, 잘못된 label 문자열 적용).

- `to-issues`
- `to-prd`
- `triage`

세 SKILL.md 의 frontmatter description 직후 한 줄 위 정확히 같은 문구가 들어가요.

### Soft dependency — vague 한 prose 만

> "the project's domain glossary"
> "ADRs in the area you're touching"

config 없어도 작동, 출력이 덜 sharp 할 뿐. 명시적 setup pointer 없음.

- `diagnose`
- `tdd`
- `improve-codebase-architecture`
- `zoom-out`

네 SKILL 의 본문 첫 paragraph 에 vague reference 만 있어요. setup pointer 부재.

**관찰** — 이건 매트가 token 효율 + 인지 부하 둘 다 고려한 결정. ADR 자체가 짧고 (10 lines) trade-off 만 명시. 이 repo 의 ADR 미니멀 스타일과 일치.

## Domain doc 레이아웃 (`docs/agents/domain.md` 가 정의)

`setup-matt-pocock-skills` 가 setup 후 다음 두 layout 중 하나로 인식:

### Single-context (대부분 repo)

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-event-sourced-orders.md
│   └── 0002-postgres-for-write-model.md
└── src/
```

### Multi-context (모노레포)

```
/
├── CONTEXT-MAP.md            # 어느 context 가 어디 사는지
├── docs/adr/                 # system-wide 결정
└── src/
    ├── ordering/
    │   ├── CONTEXT.md
    │   └── docs/adr/         # context-specific 결정
    └── billing/
        ├── CONTEXT.md
        └── docs/adr/
```

**핵심 규칙** (`grill-with-docs/SKILL.md` L31-44, `setup-matt-pocock-skills/domain.md` L11):

> Create files lazily — only when you have something to write.
> If `CONTEXT-MAP.md` exists, read it to find contexts.
> If only a root `CONTEXT.md` exists, single context.
> If neither exists, create a root `CONTEXT.md` lazily when the first term is resolved.

> If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront.

**관찰**: 이 lazy + silent 정책이 **soft dependency** 가 graceful degrade 하는 메커니즘.

## Issue tracker 추상화

`setup-matt-pocock-skills/issue-tracker-{github,local}.md` 가 정확한 명령어/규약을 명시해요.

### GitHub
- `gh issue create --title --body` (heredoc for multi-line)
- `gh issue view <number> --comments`
- `gh issue list --state open --json ... --jq ...`
- `gh issue comment <number> --body`
- `gh issue edit <number> --add-label / --remove-label`
- `gh issue close <number> --comment`
- repo 는 `git remote -v` 로 자동 추론

### Local markdown
- `.scratch/<feature-slug>/` 한 디렉토리 = 한 feature
- `.scratch/<feature-slug>/PRD.md` 가 PRD
- `.scratch/<feature-slug>/issues/<NN>-<slug>.md` 구현 이슈, 01부터
- triage state 는 파일 상단 `Status:` 줄
- 댓글은 파일 하단 `## Comments` 아래 append

### "Other" (Jira / Linear / etc.)
- 사용자 자유 prose 로 workflow 묘사 → 그대로 `docs/agents/issue-tracker.md` 에 기록

**관찰** — 추상화가 "publish to the issue tracker" / "fetch the relevant ticket" 같은 verb 으로 통일되어 있고, 각 backend 가 그 verb 의 의미를 정의해요. skill 내부는 backend 무관하게 "issue tracker" 만 호출.

## Triage label vocabulary

`setup-matt-pocock-skills/triage-labels.md` 가 5개 canonical role → repo 의 실제 label 문자열 매핑 표.

```
| Canonical          | Real label string  | Meaning                              |
| needs-triage       | needs-triage       | Maintainer needs to evaluate         |
| needs-info         | needs-info         | Waiting on reporter                  |
| ready-for-agent    | ready-for-agent    | Fully specified, ready for AFK agent |
| ready-for-human    | ready-for-human    | Requires human implementation        |
| wontfix            | wontfix            | Will not be actioned                 |
```

기본은 1:1 mapping. 다른 vocabulary 를 쓰는 repo 면 오른쪽 column 만 수정.

**관찰**: matt 의 단어를 강요하지 않음 — 기존 GitHub label vocabulary 가 있으면 그 위에 매핑.

## Hooks / Scripts 인벤토리

| Path | 역할 |
|---|---|
| `scripts/link-skills.sh` | 모든 SKILL.md 를 `~/.claude/skills/<name>/` 에 symlink (38 lines) |
| `skills/engineering/diagnose/scripts/hitl-loop.template.sh` | Human-In-The-Loop 재현용 bash template (`step` / `capture` 헬퍼) (41 lines) |
| `skills/misc/git-guardrails-claude-code/scripts/block-dangerous-git.sh` | PreToolUse hook 본체 — STDIN JSON 에서 command 추출, 위험 패턴 매치 시 exit 2 (25 lines) |

총 3개 script 만 있음. 나머지는 markdown.

## 메타 자료 — 이 repo 자체가 본인 패턴 적용

이 repo 가 본인 skill 의 conventions 를 본인 repo 에 적용해요:

- `CONTEXT.md` (L1-26) 는 `/grill-with-docs/CONTEXT-FORMAT.md` 의 정확한 format 따름 — Language / Relationships / Flagged ambiguities. 도메인 용어: Issue tracker, Issue, Triage role.
- `docs/adr/0001-...md` 는 `/grill-with-docs/ADR-FORMAT.md` 의 minimal template 따름 — single paragraph, optional sections 안 사용.
- `.out-of-scope/question-limits.md` (18 lines) 는 `/triage/OUT-OF-SCOPE.md` format 따름 — Why out of scope / Prior requests.
- `CLAUDE.md` (13 lines) 는 `setup-matt-pocock-skills/SKILL.md` 가 실제로 만들 `## Agent skills` 블록 없이 plain explanation. 살짝 meta — 이 repo 자체는 자기 setup 을 안 돌렸음 (그럴 필요도 없고).

## Architecture 한 줄 요약

> Bucket 별 단순한 디렉토리 + 단순한 plugin manifest + symlink 설치 + ADR 로 명시한 hard/soft dependency 분리 + lazy 한 doc creation. 어떤 build step 도 없고, 어떤 빌드 산출물도 없어요. SKILL.md 그대로가 product.
