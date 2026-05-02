# Deferred (NOT in scope for v0.2.0)

> v0.2.0 의 명시적 NOT in scope. 각 항목 = 검토했지만 deferred 결정 근거 + 재검토 trigger 명시.

---

## D-1. `agent install` SKILL — DEFER to v0.2.5+ or v0.3.0

- **Trigger**: ax-hub-cli `cmd/agent/install.go:296` 의 atomic mcp.json write 도입 (현재 `os.WriteFile`, NOT atomic)
- **Why deferred**: codex CEO finding F5 + ENG E9. CLI 측 race window → claude-code 가 동시 실행 중일 때 mcp.json corrupt 가능. plugin trust assumption 거짓.
- **Workaround for v0.2.0 user**: raw `axhub agent install --client=claude-code` 명령 직접 실행 안내 (README 또는 doctor SKILL 끝 hint).
- **Re-evaluate**: ax-hub-cli sibling repo 가 atomic write fix PR 머지 시 plugin 측 SKILL 작성. ~2시간 effort.

## D-2. `dev` SKILL (HTTP reverse proxy) — DEFER to v0.3.0

- **Trigger**: vibe coder onboarding 요청 5건 이상 / "axhub dev 자연어로 부탁" pilot signal 5건 이상
- **Why deferred**: power-user surface. 대부분 vibe coder = local dev 시 raw `axhub dev http://localhost:3000` 직접 입력 충분. plugin SKILL 가치 marginal.
- **Workaround**: README 의 "고급 사용" 섹션에 raw 사용법 안내.
- **Re-evaluate**: pilot 5건 / quarter 검토.

## D-3. `tables` SKILL (CRUD) — DEFER to v0.3.0

- **Trigger**: 별도 design pass 완료 (drop destructive consent gate, 7 subcommand schema, references manifest)
- **Why deferred**:
  - 7 subcommand (columns / create / drop / get / list / references / rows) = 큰 surface
  - drop = 매우 destructive (`--force --confirm=<name>`)
  - rows / columns CRUD = backend mutation 무거움
  - design pass 별도 ~3-4시간 effort
- **Workaround**: raw `axhub tables ...` 명령 직접 (admin 사용자 위주).
- **Re-evaluate**: vibe coder 가 dynamic table 사용 시작 시 (현재 거의 사용 X).

## D-4. `feedback` SKILL (GH issue jump) — DEFER (가치 marginal)

- **Why deferred**: codex DX 검토 결과 SKILL infra cost > 가치. 단 1 명령 (`axhub feedback --bug` → browser open). plugin SKILL = NL trigger + AskUserQuestion + bash 호출 = overhead 큼.
- **Workaround**: vibe coder 가 GitHub issue 직접 + GH issue template 사용.
- **Re-evaluate**: 사용자 요청 5건 이상 시.

## D-5. `audit log` 조회 SKILL — DEFER to v0.3.0+

- **Trigger**: agent observability stable + admin 권한 사용자 5명 이상
- **Why deferred**:
  - `axhub agent audit list` = admin / RBAC IDOR-protected
  - vibe coder = 자기 audit log 보고 싶을 때 < 1회/월 가정
  - admin = 별도 SKILL (admin SKILL skeleton 안에 통합 또는 별도)
- **Workaround**: raw `axhub agent audit list --since 1h --json` 안내.
- **Re-evaluate**: admin SKILL 의 audit log 조회 path 와 통합 검토.

## D-6. Community channels (Slack/Discord) — SKIP (DX TODO #1)

- **Why skip**: enterprise context = 회사 내부 IT 팔로 충분. 공식 OSS community 불필요.
- **Re-evaluate**: 회사 외부 OSS 사용자 발생 시 (현재 0).

## D-7. CONTRIBUTING.md / GH issue template — SKIP (DX TODO #2)

- **Why skip**: 단일 owner 운영 (jocoding-ax-partners). 외부 contrib flow 불필요.
- **Re-evaluate**: 회사 내부 다른 직원 PR 5건 이상 시.

## D-8. /devex-review boomerang post-launch — SKIP (DX TODO #3)

- **Why skip**: vibe coder 가 /devex-review 안 쓸 가능성. 측정 자체 marginal.
- **Workaround**: opt-in telemetry (`AXHUB_TELEMETRY=1` + `~/.local/state/axhub-plugin/usage.jsonl`) 만 활성. manual /devex-review 1회 권장.

## D-9. Champion tier (<2분) sandbox/playground — DEFER to v1.0.0+

- **Trigger**: cloud-hosted axhub sandbox backend 신설
- **Why deferred**:
  - Stripe/Vercel tier 진입장벽 = "login 안 했어도 볼 수 있는 도메인" 필요
  - 별도 backend (sandbox app + sample data + isolated tenant)
  - 회사 SaaS 백엔드 작업 ~수개월
- **Workaround**: examples repo 의 template README 가 "5분 만에 본인 axhub 으로 deploy" 시연.

## D-10. 영어 docs — SKIP

- **Why skip**: axhub plugin = Korean-NL 정체성. 영어 docs = scope creep.
- **Re-evaluate**: 영어권 회사 vibe coder 도입 시.

## D-11. ax-hub-cli 자체 변경 (sibling repo)

- **NOT IN SCOPE**: sibling repo `jocoding-ax-partners/ax-hub-cli` = 별도 owner. 본 plan 의 scope 밖.
- **단 dependent items**:
  - examples repo 의 templates.json 신규 (D-12 참조, 별도 PR)
  - agent install atomic write fix (D-1 trigger)
  - admin SKILL 의 backend endpoint (axhub teams create 등) — 별도 API design 필요

## D-12. ax-hub-cli 의 init template registry update — SKIP

- **Why skip**: codex CEO finding F7 의 "examples repo source of truth" 결정 = plugin 이 직접 examples repo fetch (helper list-templates). ax-hub-cli builtin 5 update PR 불필요. 두 path 별도 evolution.
- **단 examples repo 작업 필요**: `github.com/jocoding-ax-partners/examples/templates.json` manifest 신규 추가. 11번 phase 문서 참조.

## D-13. Lifecycle E2E harness 의 multi-tenant test

- **Why deferred**: 현재 lifecycle E2E = 단일 tenant. multi-tenant (profile use 로 회사 A → B switching mid-flow) = 복잡, marginal.
- **Workaround**: profile SKILL 의 unit test 만 + manual smoke.

## D-14. Mac touchID / Linux pkexec / Windows UAC 의 통합 wrapper

- **Why deferred**: helper bootstrap 가 OS 별 native sudo prompt 호출. wrapper 추상화 = 복잡 (touchID 가 AppleScript wrapper 필요, Linux 가 polkit dependency 등).
- **v0.2.0 approach**: 각 OS 의 native paradigm 그대로 (touchID dialog, Linux sudo password, Windows UAC). 통합 wrapper = v0.3.0 evaluate.

---

## NOT IN SCOPE (재확인용 명시)

| 항목 | scope | trigger to revisit |
|---|---|---|
| `agent install` SKILL | v0.2.5+ | CLI atomic write fix |
| `dev` SKILL | v0.3.0 | 5+ pilot 요청 |
| `tables` SKILL | v0.3.0 | 별도 design pass + vibe coder 사용 시작 |
| `feedback` SKILL | indefinite | 5+ 사용자 요청 |
| `audit log` SKILL | v0.3.0+ | admin SKILL 통합 검토 |
| Community channels | indefinite | 외부 OSS 사용자 |
| CONTRIBUTING.md | indefinite | 외부 PR 5+ |
| /devex-review boomerang | manual | TTHW 측정 결과 |
| Sandbox playground | v1.0.0+ | cloud backend |
| 영어 docs | indefinite | 영어권 vibe coder |
| ax-hub-cli builtin registry update | SKIP | (대신 examples repo manifest path) |
| Multi-tenant lifecycle E2E | v0.3.0+ | 회사 외부 다중 tenant 사용 |
| OS sudo wrapper 통합 | v0.3.0+ | UX 일관성 요구 |
