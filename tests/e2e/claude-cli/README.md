# axhub Plugin — `claude -p` Subprocess E2E QA Harness

> Phase 22 산출물. Plan: `.omc/plans/phase-22-claude-cli-e2e-harness.md`.

## 목적

vibe coder 가 Claude Code 안에서 axhub 자연어를 입력했을 때의 끝-끝 흐름 (SKILL routing → preflight injection → AskUserQuestion D1 fallback → HMAC consent gate → exit-code 한국어 분류 → statusline cache) 을 subprocess `claude -p` 로 직접 driving 해서 회귀를 catch 해요.

## 디렉토리

```
tests/e2e/claude-cli/
├── README.md                          (이 파일)
├── CLAUDE_FLAGS.md                    claude 2.1.121 flags freeze
├── CLAUDE_JSON_SCHEMA.md              claude --output-format json schema freeze
├── matrix.jsonl                       33 case spec (T1=7 / T2=12 / T3=14)
├── run-matrix.sh                      orchestrator (POSIX bash)
├── lib/
│   ├── spawn.sh                       claude -p wrapper (timeout, env 격리)
│   ├── assert.sh                      exit / grep / jq / file-presence helpers
│   ├── isolate-env.sh                 XDG_CONFIG_HOME / HOME / MCP scrub
│   └── mock-hub.sh                    Bun-based localhost HTTP mock (AXHUB_ALLOW_PROXY=1)
├── fixtures/
│   ├── token-headed.json              valid-shape token
│   ├── token-expired.json             exp < now → exit 65
│   ├── apps-list.json                 mock hub-api response
│   ├── deploy-create-success.json     mock deploy create
│   ├── deploy-status-stream.ndjson    NDJSON tick stream
│   └── bin/
│       ├── axhub                      shim — sentinel touch + ALLOW_PROXY=1 강제
│       ├── axhub-mock-impl.sh         mock-hub wrapper
│       └── required-subcommands.txt   coverage closed-loop assertion (22.0 자동 생성)
├── cases/                             NN-name.case.sh (33 active)
└── output/                            (gitignored) summary.tsv, junit.xml, baseline-{times,cost}.json, per-case dirs
```

## 빠른 시작

```bash
# T1+T2 PR-blocking (ubuntu-only, ~5min)
bun run test:plugin-e2e -- --tier pr

# T3 nightly + release-tag (ubuntu+macos, ≤10min)
bun run test:plugin-e2e -- --tier nightly

# 단일 case
bun run test:plugin-e2e -- --only 09

# Phase 22.5.5 baseline measurement (PR-blocking 시작 전 1회)
bun scripts/measure-claude-baseline.ts
```

## 안전 가드

1. **No prod side-effects** — 모든 mutate (`deploy create`, `auth login`, `update apply`) 는 `--dry-run` / mock-hub / 격리 fixture 만. read-only (`auth status --json`, `apps list --json`) 만 실제 hub-api 핑 허용.
2. **격리 sandbox** — `env -i` + 격리 `HOME`/`XDG_CONFIG_HOME` + `tests/e2e/claude-cli/fixtures/bin/axhub` shim PATH 1순위 + `/usr/local/bin` 제거. 시스템 axhub binary 호출 0건 (sentinel touch 검증).
3. **mock-hub** — `AXHUB_ALLOW_PROXY=1` baseline path 활용 (`src/axhub-helpers/list-deployments.ts:109,141`). HTTP localhost 가능. helper bin 변경 0건.
4. **TTL 강제** — per-case `timeout --kill-after=5 30s`. matrix total `<600s`.
5. **5-state cap-hit** — `--max-budget-usd 0.30` cap-hit silent truncation 회피 — `(exit==124) AND (stdout<100byte) AND NOT (stop_reason ∈ {abort, user_cancelled, end_turn})` 트리거 시 BUDGET_EXCEEDED hard fail.
6. **strict 단일 매칭** — `expected_route` OR-allow 폐기. flake budget 1-retry.

## 새 case 추가

1. `matrix.jsonl` 에 row 추가 (tier / utterance / expected_exit / expected_phrases / expected_endpoints / expected_state).
2. `cases/NN-name.case.sh` 생성 (spawn 호출 + assert 호출).
3. fixture 가 새로 필요하면 `fixtures/<name>.json` 추가.
4. AskUserQuestion fallback 새로 등록하면 `tests/fixtures/ask-defaults/registry.json` 도 update (drift catch).
5. `bash run-matrix.sh --only NN` 단독 PASS 확인 후 PR.

## 트러블슈팅

| 증상 | 원인 | 해결 |
|------|------|------|
| case 가 30s timeout | D1 TTY guard 누락 / AskFallback registry drift | SKILL 본문 D1 가드 grep + registry.json key 일치 검증 |
| BUDGET_EXCEEDED 발생 | 모델이 cap 안에서 못 끝남 | `claude --help` 의 `--max-budget-usd` 측정 + spawn.sh cap 상향 PR |
| `expected_route` 불일치 | 모델 비결정성 | 1-retry 후 fail. nl-lexicon 첫 순위 어구로 trigger 변경 |
| `command not found: axhub` | shim PATH 안 잡힘 | `chmod +x fixtures/bin/axhub` + PATH 순서 검토 |
| `[security] hub-api TLS pin` | mock-hub HTTPS 미사용 | `AXHUB_ALLOW_PROXY=1` env 누락. spawn.sh 강제 검토 |

## Tier 정의

- **T1 (PR-blocking, ubuntu, claude -p, 7 case)**: SKILL routing + preflight + AskFallback + consent + 한국어 4-part exit
- **T2 (PR-blocking, ubuntu, helper-bin golden-file, 12 case)**: classify-exit 6 카탈로그 / redact / preflight / consent / statusline / registry walk / TLS-pin baseline
- **T3 (nightly + release-tag, ubuntu+macos, 33 active)**: T1+T2 union 재실행 + 14 unique active

## 관련 파일

- Plan: `.omc/plans/phase-22-claude-cli-e2e-harness.md`
- ADR: 위 plan 의 §ADR
- Workflow: `.github/workflows/claude-cli-e2e.yml` (e2e-pr / e2e-nightly split)
