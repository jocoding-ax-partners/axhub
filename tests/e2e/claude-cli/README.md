# axhub Plugin — `claude -p` Subprocess E2E QA Harness

vibe coder 가 Claude Code 안에서 axhub 자연어를 입력했을 때의 끝-끝 흐름 (SKILL routing → preflight injection → AskUserQuestion preview gate → exit-code 한국어 분류) 을 subprocess `claude -p` 로 직접 driving 해서 회귀를 catch 해요.

diet 후 matrix 는 핵심 deploy 경로 회귀 **단일 케이스(case 19, 한국어 NL deploy)** 만 유지해요. 실제 `claude -p` 호출이라 claude API 비용이 들어, CI 에서는 nightly + 수동 dispatch 로만 돌려요 (PR-blocking 아님).

## 디렉토리

```
tests/e2e/claude-cli/
├── README.md                          (이 파일)
├── CLAUDE_FLAGS.md                    claude CLI flags freeze
├── CLAUDE_JSON_SCHEMA.md              claude --output-format json schema freeze
├── matrix.jsonl                       case spec (현재 case 19, tier T1)
├── run-matrix.sh                      orchestrator (POSIX bash)
├── lib/
│   ├── spawn.sh                       claude -p wrapper (timeout, env 격리)
│   └── assert.sh                      exit / grep / jq / file-presence helpers
├── fixtures/
│   ├── token-headed.json              valid-shape token
│   └── bin/
│       ├── axhub                      shim — sentinel touch + current CLI-surface stub
│       └── required-subcommands.txt   coverage closed-loop assertion
└── output/                            (gitignored) summary.tsv, per-case dirs
```

## 빠른 시작

```bash
# 전체 실행 (현재 단일 case 19)
bash tests/e2e/claude-cli/run-matrix.sh

# 단일 case
bash tests/e2e/claude-cli/run-matrix.sh --only 19

# tier 지정 (t1 / nightly)
bash tests/e2e/claude-cli/run-matrix.sh --tier nightly
```

실제 `claude -p` 를 띄우므로 `claude` CLI + `ANTHROPIC_API_KEY` 가 필요해요. CI 에서는 `.github/workflows/claude-cli-e2e.yml` 의 nightly job 이 돌려요.

## 안전 가드

1. **No prod side-effects** — 모든 mutate (`deploy create`, `auth login`) 는 CLI shim / 격리 fixture 만. read-only 만 실제 hub-api 핑 허용.
2. **격리 sandbox** — `env -i` + 격리 `HOME`/`XDG_CONFIG_HOME` + `fixtures/bin/axhub` shim PATH 1순위 + `/usr/local/bin` 제거. 시스템 axhub binary 호출 0건 (sentinel touch 검증).
3. **CLI wrapper fixtures** — backend/auth/read path 는 직접 HTTP 를 열지 않고 `fixtures/bin/axhub` shim 을 통해 현재 CLI surface 를 검증해요.
4. **TTL 강제** — per-case `timeout`. matrix total 상한.

## 새 case 추가

1. `matrix.jsonl` 에 row 추가 (tier / utterance / expected_exit / expected_phrases / expected_state).
2. `cases/NN-name.case.sh` 생성 (spawn 호출 + assert 호출).
3. fixture 가 새로 필요하면 `fixtures/<name>.json` 추가.
4. `bash run-matrix.sh --only NN` 단독 PASS 확인 후 PR.

## 트러블슈팅

| 증상 | 원인 | 해결 |
|------|------|------|
| case 가 timeout | preview/AskUserQuestion 가드 누락 | SKILL 본문 가드 grep 검증 |
| `command not found: axhub` | shim PATH 안 잡힘 | `chmod +x fixtures/bin/axhub` + PATH 순서 검토 |
| 실제 axhub 를 호출함 | shim PATH 안 잡힘 | `fixtures/bin/axhub` sentinel(`shim-called`)과 `AXHUB_BIN` override 검토 |

## 관련 파일

- Workflow: `.github/workflows/claude-cli-e2e.yml` (nightly T1)
