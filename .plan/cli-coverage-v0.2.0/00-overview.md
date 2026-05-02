# axhub Plugin v0.2.0 — CLI Coverage + Zero-Install Bootstrap

> 작성일: 2026-05-03 · 대상: `axhub plugin` v0.2.0 release · 기반 contract: `ax-hub-cli` v0.10.2 · 현재 plugin: v0.1.26
>
> Source 검토: `~/.gstack/projects/jocoding-ax-partners-axhub/ceo-plans/2026-05-02-cli-coverage-gap.md` (CEO+ENG+DX 3-pass review)

---

## 한 줄

vibe coder 가 Claude Code 안에서 자연어로 lifecycle (`init → connect → env → deploy → open`) 끝까지 raw CLI 안 만나고 가요. cold customer 도 `init` 한 번이면 모든 dep 자동 install (node, ax-hub-cli, framework deps, template).

## Vision

```
T+0    회사 받은 새 노트북 (Mac/Linux/Windows)
T+0:30 Claude Code download (Anthropic 책임)
T+1:00 plugin marketplace install
T+1:30 vibe coder NL: "결제 앱 만들어줘"
       → init SKILL stack 선택 → helper bootstrap 자동 install
T+5~7  template fetch + npm install + apps create + github connect ask
T+10   deploy → open 브라우저 자동
```

raw CLI / docs / sales call 없어요. Korean NL 그대로.

## 변경 요약

| 영역 | 변경 |
|---|---|
| **Plugin SKILL** | 11 → 18 (7 신규 / 5 polish) |
| **helper Rust subcommand** | 11 → 15 (4 신규) |
| **examples repo** | templates.json manifest 신규 (sibling PR) |
| **TTHW (cold)** | 15-30분 → ~7분 30초 (Competitive tier) |
| **TTHW (warm)** | — → ~3분 |
| **CLI 호환 범위** | v0.1.0~v0.2.0 → v0.1.0~v0.11.0 |
| **Total effort** | ~33시간 (CC+gstack) / ~5주 (human team) |

## 신규 SKILL 7

| Skill | 책임 | multi-step | needs-preflight |
|---|---|---|---|
| `init` | zero-install + template scaffold + dep install (all-in-one) | true | false (auth 무관) |
| `env` | 환경변수 list/get/set/delete (set = `--from-stdin` pipe only) | true | true |
| `github` | repo connect/disconnect/repos | true | true |
| `open` | 배포 URL 브라우저 열기 | false | false |
| `whatsnew` | release highlights NL 안내 | false | false |
| `profile` | endpoint multi-tenant switching (add 는 allowlist gate) | true | false |
| `admin` | backend onboarding wizard (team/member/token/sandbox) | true | true |

## 기존 SKILL 5 polish

| Skill | 추가 surface |
|---|---|
| `apps` | +create / delete (`--dry-run` first) / update (dynamic field ask) / get / open delegation |
| `apis` | +schema / test + call (full consent gate, deploy-equivalent) |
| `deploy` | +cancel (consent gate) / +list (pagination) |
| `doctor` | +audit subdiagnostic |
| `update` | cosign WARN→ENFORCE 사전 안내 |

## 신규 helper Rust subcommand 4

| subcommand | 책임 |
|---|---|
| `bootstrap` | node detect/install (volta wrap), git skip, ax-hub-cli detect/install, sudo handling, corporate proxy detect, progress JSON stream |
| `fetch-template <slug>` | examples repo tarball download + extract (codeload.github.com, git X) |
| `install-deps` | template root manifest detect (package.json/requirements.txt/go.mod) + dispatch (npm/pip/go mod) |
| `list-templates` | examples repo `templates.json` manifest fetch (AskUserQuestion list 반환) |

## Phase 흐름

```
Phase A0 (foundation, ~10시간)
  ├─ helper version gate (preflight MAX 0.2.0 → 0.11.0, TS+Rust)
  ├─ prompt-route 11→17 enum (TS+Rust router parity)
  ├─ ConsentBinding generic context (6 파일, in-flight token backwards compat)
  ├─ benchmark prompt-route 50ms gate
  ├─ nl-lexicon negative tests
  ├─ registry baseline 13→20 (feature flag default OFF)
  ├─ examples repo templates.json manifest PR
  └─ 4 helper bootstrap subcommand × 5 binary cross-platform
        ↓
Phase B (신규 SKILL, ~6시간)
  ├─ init / whatsnew / open / profile / admin / env / github
        ↓
Phase B-test (E2E harness 확장, ~5시간)
  ├─ run-matrix.sh multi-step opt-in
  ├─ spawn.sh session-persistence opt-in
  ├─ lifecycle E2E case
  └─ ADR free-form preview policy
        ↓
Phase C (polish, ~1.5시간)
  └─ apps / apis / deploy / doctor / update
        ↓
Phase D (release, ~1시간)
  ├─ feature flag flip (beta_skills default ON)
  ├─ README + lifecycle GIF + 5분 만에 시작 섹션
  ├─ commit-and-tag-version (자동 minor bump)
  └─ git push origin main --tags
```

## 문서 인덱스

- `00-overview.md` — 본 문서
- `01-phase-a0-foundation.md` — Phase A0 의 helper foundation (preflight / route / consent / registry / baselines / lexicon / catalog / benchmark)
- `02-phase-a0-bootstrap.md` — Phase A0 의 helper bootstrap subcommand (4 신규 + cross-platform)
- `03-phase-b-skills.md` — 7 신규 SKILL 상세 (init / env / github / open / whatsnew / profile / admin)
- `04-phase-b-test-harness.md` — E2E harness multi-step + lifecycle E2E
- `05-phase-c-polish.md` — 5 기존 SKILL polish
- `06-phase-d-release.md` — README + GIF + flag flip + release chain
- `07-risks.md` — risks register + mitigation
- `08-success-criteria.md` — test count, scorecard, demo gate
- `09-deferred.md` — NOT in scope (agent install / dev / tables / feedback / audit log / community / CONTRIBUTING / devex boomerang)
- `10-decisions-log.md` — CEO + ENG + DX codex 27 finding 결정 history
- `11-examples-repo-manifest.md` — sibling PR (jocoding-ax-partners/examples)

## 통합 review 결과

| Review | 실행 횟수 | Status | 핵심 발견 수 |
|---|---|---|---|
| CEO codex | 1 | issues_open | 12 (8 auto + 4 user) |
| ENG codex | 1 | issues_found | 8 (5 auto + 3 user) |
| DX codex | 1 | issues_found | 7 (3 auto + 3 user + 1 noop) |
| **Total** | **3 outside voice pass** | — | **27 finding 흡수** |

VERDICT: CEO + ENG + DX cleared. Implementation gate open.
