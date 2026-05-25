# PAT Bootstrap 설계 (plugin spec v1.1 #4 / spec §4.1)

> Mode B(로컬·CI·외부) 환경에서 `AXHUB_PAT` 가 없을 때 **SSO 로그인 → PAT 발급 → OS keychain 저장 → env 노출**을 자동으로 엮는 흐름이에요. 이 문서는 설계 합의용이고, 구현은 후속 PR 에서 해요.

## 배경

Mode B 사용자 코드(snippet)는 `AXHUB_PAT` env 로 인증해요. 근데 처음 쓰는 vibe coder 는 PAT 가 없어서 수동으로 발급·설정해야 했어요. spec §4.1 은 이걸 자동화해요.

## 현재 CLI 자산 (조사 결과)

**이미 있는 부품 — 재사용:**

| 명령 | 동작 |
|---|---|
| `axhub auth login` | device flow (브라우저 SSO) |
| `axhub auth pat issue --name <n> --expires-in-days <d> --json` | PAT 발급 + **keychain 자동 저장** (`store_pat`, auth.rs:1428) + raw token 1회 출력 (`print_secret_once`, :1460) |
| `axhub auth pat revoke <id>` | 폐기 |
| `axhub auth pat list` / `whoami` | 조회 (PAT 없으면 exit 65) |

keychain 저장은 `store_pat(profile, id, raw_token)`, 어느 PAT 를 쓸지는 `AXHUB_PAT_ID` env 로 선택해요.

**없는 부품 — 이번에 추가할 유일한 CLI 변경:**

- keychain 의 raw token 을 `AXHUB_PAT` env 로 노출하는 명령. 현재는 `AXHUB_PAT_ID` 로 PAT 를 선택만 하고, raw token 을 `export AXHUB_PAT=...` 형태로 emit 하지 않아요.
- → **`axhub auth pat env [--shell sh|fish]`** 추가. keychain 에서 활성 PAT 의 raw token 을 읽어 eval 가능한 `export AXHUB_PAT=<raw>` 한 줄을 출력해요. (snippet Mode B 코드가 기대하는 `AXHUB_PAT` 를 채우는 다리.)

## 신호 — helper (non-interactive)

`axhub-helpers sync` 는 non-interactive 라 브라우저 SSO 를 못 띄워요. 그래서 트리거가 아니라 **신호**만 줘요:

- #144 이후 `sync` 는 PAT 가 없으면 abort 하지 않고 `catalog.json` 에 `pat: null` 을 기록해요 (PAT hard-fail 제거).
- 여기에 더해, `sync` JSON 출력에 `pat_bootstrap_needed: true` 를 추가해요 (조건: `target ∈ Mode B` && `pat == null`). ← 이번 helper 변경.

## Bootstrap 흐름 — data SKILL (interactive)

`/axhub:data` 가 Mode B + PAT 없음을 감지하면:

1. `axhub auth whoami --json` → 로그인 안 됐으면 `/axhub:auth` 로 위임 (기존 device flow 재사용).
2. AskUserQuestion 으로 발급 동의를 받아요 (D1 가드 + registry safe_default 포함).
3. `axhub auth pat issue --name claude-code-$(hostname) --expires-in-days 30 --json` → keychain 자동 저장.
4. `axhub auth pat env` 출력을 사용자에게 안내해요 (shell 에 eval 하거나 `.env` 에 기록). PAT raw 는 화면에 1회만, 로그/파일에 평문 저장 금지.
5. 이후 snippet / 사용자 코드가 `AXHUB_PAT` 로 인증해요.

## Revoke

AskUserQuestion consent 후 `axhub auth pat revoke <id>` + keychain 제거. (`/axhub:data` 또는 별도 진입점.)

## Expired 감지

`sync` 또는 live read 가 exit 65(auth) 를 반환하면 → 기존 exit-65 routing 을 재사용해서 "PAT 만료됐어요. 재발급할까요?" prompt → bootstrap 재실행.

## Out of scope

- **Mode A (SSO cookie)** — PAT 불필요. cookie 전용이라 bootstrap 안 해요.

## 변경 범위 요약

| 구성 | 변경 | PR |
|---|---|---|
| ax-hub-cli | `auth pat env` 명령 추가 (keychain raw → `export AXHUB_PAT`) | 별도 repo |
| axhub-helpers `sync.rs` | `pat_bootstrap_needed` 신호 (Mode B && pat null) | plugin |
| `data` SKILL | bootstrap step + consent + expired routing | plugin |
| `registry.json` | bootstrap consent safe_default 등록 | plugin |

## 테스트

- CLI: `auth pat env` — keychain mock, `export AXHUB_PAT=` 출력 형식 (cargo).
- helper: `sync` 가 Mode B + pat null 일 때 `pat_bootstrap_needed` emit (`data_layer_cli`, fake axhub).
- SKILL: bootstrap 흐름 (비대화형 기본값 = 발급 안 함).

## 결정됨

- **PAT 만료 기본값 = 30일 고정 default** (2026-05-26 결정). 매번 만료 기간을 묻지 않고 30일로 발급하고, 만료되면 expired 감지 → 재발급 흐름으로 처리해요. vibe coder 편의 우선, 재발급 경로가 있어서 보안 노출 창은 30일로 한정돼요.
