# 라우팅

axhub plugin 의 자연어 prompt 라우팅 architecture 예요. v0.3.2 부터 Claude 의 native skill matching 을 사용해요.

## 작동 방식

### 흐름

```
사용자 발화
    │
    ▼
UserPromptSubmit hook (5초 budget)
    ├─ preflight (CLI 버전, auth 상태)
    ├─ audit log JSONL append (silent)
    └─ additionalContext = preflight 결과 만
    │
    ▼
Claude 가 SKILL.md description 보고 native 매칭
    │
    ▼
PreToolUse Bash hook (HMAC consent gate)
    │
    ▼
PostToolUse Bash hook (exit-code classifier)
```

### Layer 1 — SKILL.md description (source of truth)

각 `skills/<skill>/SKILL.md` 의 frontmatter `description` field 가 trigger 어구 source of truth 예요. Claude Code 가 description 들을 자동 비교해서 가장 적합한 skill 을 invoke 해요.

예시 (`skills/deploy/SKILL.md`):

```yaml
---
name: deploy
description: '이 스킬은 사용자가 현재 브랜치를 axhub 라이브로 배포하고 싶어할 때 사용해요. 다음 표현에서 활성화: "deploy", "ship", "release", "rollout", "launch", "배포해", ...'
---
```

### Layer 2 — Preflight context injection (Rust hook)

`UserPromptSubmit` hook 가 preflight 만 수행해요:
- `axhub --version` (CLI 설치 / 버전)
- `axhub auth status` (로그인 상태)
- 현재 앱 / 프로필

이 정보가 `additionalContext` 로 Claude 에게 전달돼서 매칭 시 context-aware 결정이 가능해요 (예: 로그인 안 되어 있으면 deploy intent 도 auth 부터 안내).

### Layer 3 — HMAC consent gate (PreToolUse)

destructive 작업 (deploy / apps delete / env set) 은 `PreToolUse` Bash hook 에서 HMAC consent token 검증으로 결정론을 보장해요.

### Layer 4 — Exit-code classifier (PostToolUse)

`axhub` CLI 의 exit code 를 한국어 error guidance 로 분류해서 사용자에게 다음 step 을 안내해요.

## 라우팅 통계

`axhub-helpers routing-stats [OPTIONS]` 로 본인 환경 데이터를 조회해요.

### Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--since <DURATION>` | `7d` | `1d`, `7d`, `30d`, `all` |
| `--json` | false | machine-readable |
| `--top <N>` | 10 | top N axhub-related hash |
| `--confused` | false | clarify 가 발동된 hash + chosen_skill 만 조회 |

### 예시

```bash
axhub-helpers routing-stats
axhub-helpers routing-stats --since 30d --top 20
axhub-helpers routing-stats --json | jq .axhub_related
axhub-helpers routing-stats --confused --json | jq .confused_prompts
axhub-helpers routing-dashboard --html > /tmp/axhub-routing.html
```

## Audit log schema

JSONL line per prompt 예요:

```jsonl
{"ts":"2026-05-07T14:30:00Z","prompt_hash":"sha256:abc...","prompt_len":42,"cli_version":"0.11.0","auth_ok":true,"is_axhub_related":true,"clarify_invoked":false,"chosen_skill":null,"decision":"ignore","marker_present":false,"authed":false,"explicit_invocation":false,"axhub_keyword_present":false,"foreign_keyword_present":false}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `ts` | string | ISO 8601 UTC |
| `prompt_hash` | string | sha256 of prompt content (`sha256:` prefix + 64 hex) |
| `prompt_len` | number | 원본 prompt 길이 (bytes) |
| `cli_version` | string \| null | preflight 결과 |
| `auth_ok` | bool | preflight 결과 |
| `is_axhub_related` | bool | 단순 `prompt.contains("axhub")` boolean (measurement 분석용) |
| `clarify_invoked` | bool | clarify feedback record 여부. legacy line 은 default false |
| `chosen_skill` | string \| null | clarify 메뉴에서 사용자가 고른 최종 skill. prompt 원문은 저장하지 않아요 |
| `decision` | string \| null | 공유 routing-decision 함수의 결정타입 (`axhub`/`yield`/`ignore`/`ask`/`explicit`). legacy + clarify sentinel line 은 null |
| `marker_present` | bool \| null | `axhub.yaml` 마커가 cwd→git-root walk-up 에서 발견됐는지 (decide 입력) |
| `authed` | bool \| null | token-file `.exists()` stat 결과 (decide 입력, bootstrap 안 함) |
| `explicit_invocation` | bool \| null | prompt 가 slash invocation (`/deploy`, `/axhub:…`) 인지 (decide 입력) |
| `axhub_keyword_present` | bool \| null | literal `"axhub"` 키워드 포함 여부 (keyword-driven 신호) |
| `foreign_keyword_present` | bool \| null | foreign 타깃 키워드 (vercel/netlify/…) 포함 여부 (named-target-wins 신호) |

## Feedback loop

`skills/clarify/SKILL.md` 는 사용자가 메뉴에서 최종 skill 을 고른 뒤 `axhub-helpers audit-clarify --hash <sha256> --chosen <skill>` 을 fail-soft 로 호출해요. 이 추가 record 는 `clarify_invoked=true` 와 `chosen_skill=<skill>` 만 담아요.

이후 운영자는 다음 명령으로 feedback 을 볼 수 있어요:

```bash
axhub-helpers routing-stats --confused --json
bun run routing:tune --confused
```

`routing:tune --confused` 는 audit log 에 prompt 원문이 없다는 privacy constraint 를 유지해요. 그래서 hash + chosen_skill 을 안정적으로 출력하고, 원문 확인은 사용자 manual review 로 이어져요.

기본 `routing:tune --dry-run` 은 API key 가 있어도 offline deterministic mode 로 실행돼요. 외부 LLM suggestion 은 `--online`, `--llm`, 또는 `--apply` 를 명시했을 때만 사용해요.

## Dashboard

`axhub-helpers routing-dashboard --html` 은 지난 7일 audit 를 정적 HTML 로 렌더해요:

- total / axhub-related / auth failed / clarify invoked count
- clarify feedback 기반 per-skill feedback hits
- failing prompt hashes (`clarify_invoked=true`)
- drift trend 확인 명령 (`bun run test:routing:100`)

### 저장 위치

`$XDG_STATE_HOME/axhub-plugin/routing-audit-{YYYY-MM-DD}.jsonl` (기본 `~/.local/state/axhub-plugin/`).

권한: dir 0700, file 0600 (Unix; Windows 는 ACL equivalent).

## Privacy

### 무엇이 저장돼요

- prompt 의 sha256 hash (32-byte hex digest)
- prompt 길이 (bytes)
- preflight 결과 (CLI 버전, 인증 boolean)
- "axhub" substring 포함 여부 boolean
- clarify feedback 여부와 chosen_skill (clarify 가 발동된 경우)
- 타임스탬프

### 무엇이 저장 안 돼요

- prompt content (원문)
- 일반 prompt 의 사용자 발화 의도 / native 결정 skill
- 외부 전송 (telemetry endpoint X)
- 식별 정보 (user id, hostname, ip)

### Hash 익명화 한계

짧은 prompt 의 sha256 hash 는 reverse 가능해요:
- "deploy" (6 bytes) → hash dictionary 로 reverse
- "배포" (6 bytes UTF-8) → 동일

긴 prompt 의 hash 는 사실상 reverse 불가 but **이론적 익명화 보장 X** 예요.

privacy 우려 시:
- `AXHUB_NO_AUDIT=1` 으로 opt-out
- 또는 `axhub-helpers cleanup-audit --all` 로 전체 삭제

### 7일 자동 회전

매 `axhub-helpers routing-stats` 호출 시 silent rotate(7) trigger 가 동작해요. 7일 cap (오래된 파일 삭제). 별도 명시 cleanup 명령은 `axhub-helpers cleanup-audit` (default 7일 이상) 또는 `--all` (전체).

## 트러블슈팅

### "왜 이 skill 이 매칭됐지?"

1. `axhub-helpers routing-stats --since 1d --top 10` 으로 최근 패턴 확인해요.
2. 해당 skill 의 SKILL.md description 확인 — 매칭된 trigger 어구 검토해요.
3. 다른 prompt 로 다시 시도해요.

### "라우팅이 잘못된 것 같아"

`tests/corpus.jsonl` 의 corresponding row 확인 + GitHub issue report 부탁해요:
- 발화 + 기대 skill + 실제 매칭 skill 첨부

### "audit log 가 너무 커요"

- 7일 자동 rotation 동작 확인: `axhub-helpers routing-stats` 호출 시 자동 trigger
- 강제 cleanup: `axhub-helpers cleanup-audit` (7일 이상만 삭제) 또는 `--all` (전체)

### "AXHUB_NO_AUDIT 가 작동 안 해요"

- 환경 변수 export 확인: `echo $AXHUB_NO_AUDIT`
- shell rc 에 영구 설정: `export AXHUB_NO_AUDIT=1` 을 `.zshrc` / `.bashrc` 에 추가
- 변수 set 후 다음 prompt 부터 적용돼요

## SKILL 작성자 가이드

새 skill 추가 시:

1. `bun run skill:new <slug>` 으로 scaffold 를 생성해요.
2. `description` field 에 trigger 어구를 명시해요 (한국어 + 영어 mix 권장).
3. 다른 skill 과 phrase collision 방지 (`bun run skill:doctor` 검증).
4. 최소 5 trigger phrase.
5. corpus.jsonl 에 ~5 row 추가 (positive matching).

상세는 [`AGENTS.md`](../AGENTS.md) / [`CLAUDE.md`](../CLAUDE.md) 의 "Skill Authoring" 섹션을 참고해요.
