# Risks Register

> 27 codex finding 흡수 후 남은 risk + mitigation. severity = (Likelihood × Impact).

---

## CRITICAL (즉시 차단 필요)

### R-1. Phase A0 가 broken intermediate state 머지 위험

- **codex ENG finding E4**
- 시나리오: A0 만 머지 = 신규 6 SKILL routing 추가됐는데 SKILL 파일 없음. CI 가 manifest test 의 11-skill 화이트리스트 lock 만 = green 통과. 사용자 NL 시 "skills/init/SKILL.md not found" silent error.
- **Mitigation**: feature flag `plugin.json:features.beta_skills:false` default OFF. Phase D 에서 flip. A0 만 머지된 intermediate state 에서 신규 SKILL routing 비활성 (clarify fallback).
- **Validation**: A0 머지 후 vibe coder 가 "결제 앱 만들어줘" 시 clarify SKILL 가 catch + "v0.2.0 정식 release 까지 raw axhub init 안내".

### R-2. ConsentBinding migration 6 파일 중 일부 누락 시 security hole

- **codex ENG finding E2**
- 시나리오: TS 만 update + Rust 잊으면, shipped binary (Rust) 가 새 11 destructive action 인식 못 해서 PreToolUse gate 우회.
- **Mitigation**: Phase A0 #6 의 6 파일 명시 + cargo test + bun test 모두 PASS gate. CI 가 두 path 동시 검증.
- **Validation**: 신규 11 destructive 마다 mint-then-verify cycle test. bypass attempt test (env-prefix, $(...), bash -c).

### R-3. env set 의 secret value 가 argv leak

- **codex CEO finding F8**
- 시나리오: AskUserQuestion 으로 받은 value 가 `axhub env set KEY <VALUE> --app X` 처럼 argv 로 들어가면 ps aux / shell history / process accounting 에 평문 노출. **회사 IT 의 audit log 에 비밀 키 저장됨**.
- **Mitigation**: env set SKILL = `--from-stdin` pipe 강제. value 절대 argv X. helper telemetry redact.
- **Validation**: E2E test 가 mock-hub 의 bash trace log 에 value 평문 안 들어가는지 mechanical 검증.

### R-4. helper bootstrap sudo prompt 우회 / 자동 install 회사 IT 정책 위반

- 시나리오: 회사 IT 가 sudo / brew / volta 자체 차단. helper bootstrap 가 자동 install 시도 시 "회사 정책 위반" 알림 트리거. 또는 corporate antivirus 가 helper Rust binary 자체 격리.
- **Mitigation**: bootstrap 시 명시적 AskUserQuestion 동의 필요 (`--no-confirm` flag 안 쓰면 무조건 ask). 회사 IT 정책 차단 시 detect + 안내 ("회사 IT 가 sudo 차단. cli.jocodingax.ai 화이트리스트 요청"). cosign 서명 binary 제공 + IT-friendly install path.
- **Validation**: corporate proxy mock 환경 + sudo 차단 mock 시 graceful failure + 명확한 안내 메시지.

---

## HIGH (위험 큼, mitigation 필수)

### R-5. Rust helper parity drift (TS-only 변경 시 shipped binary 동작 안 함)

- **codex ENG finding E1**
- 시나리오: prompt-route 11→17 enum 을 TS 만 변경, Rust router (`crates/.../main.rs:258-555`) 잊음. shipped binary = Rust → routing 동작 안 함.
- **Mitigation**: Phase A0 #4 명시 (Rust router 도 동시 update). cargo test 가 동일 매트릭스 검증. CI 가 두 path 모두 PASS gate.

### R-6. nl-lexicon collision (다중 SKILL 매칭)

- **codex ENG finding E5**
- 시나리오: TS first-match wins (`prompt-route.ts:217-220`). Rust substring match. "환경" 단독 → env vs profile vs doctor 모두 매칭 가능.
- **Mitigation**: Phase A0 #8 의 5 explicit negative tests (TS+Rust). 추가 collision 발견 시 즉시 baseline 추가.

### R-7. v0.10 `--timeout` semantics 변경

- 시나리오: CLI v0.10 의 `--timeout 0` 동작 변경. helper preflight `--timeout 0` 사용 시 동작 다름.
- **Mitigation**: helper Rust 가 `--timeout 0` 안 쓰고 default 60s migration failsafe 활용. preflight 만 영향, 나머지 path = command context 통한 deadline.
- **Validation**: preflight test 가 v0.10.x CLI 와 v0.7.x CLI 모두 PASS.

### R-8. github connect AppHub 미설치 first-time vibe coder

- 시나리오: vibe coder 가 GitHub App 사전 install 안 한 상태. `github connect` exit 67 install_not_found.
- **Mitigation**: install URL 안내 + retry guide. install 끝나면 "다 됐어" 라는 NL 로 retry trigger.
- **Validation**: error-empathy-catalog 의 "install_not_found" 4-part Korean template.

### R-9. examples repo templates.json drift / fetch fail

- 시나리오: examples repo 의 templates.json 가 stale (template 추가했는데 manifest 안 update). 또는 GitHub raw.githubusercontent.com 도달 X (회사 firewall).
- **Mitigation**: helper list-templates 가 stale-while-revalidate cache (~/.cache/axhub-plugin/templates.json, TTL 1시간). fetch fail 시 ax-hub-cli builtin 5 fallback.
- **Validation**: cache hit / cache miss / fetch fail 3 test path.

### R-10. lifecycle E2E harness multi-step opt-in 의 sandbox state corruption

- 시나리오: persist session 활성 시 case 간 sandbox state leak. 한 case 의 axhub.yaml 가 다음 case 영향.
- **Mitigation**: lifecycle case 단 1개. 다른 case 와 sandbox 분리. cleanup hook 명시.

---

## MEDIUM (모니터링 필요)

### R-11. volta 가 nvm/asdf 와 충돌

- 시나리오: Mac/Linux 의 dev 가 이미 nvm 사용 중. volta 설치 시 PATH 충돌, node version 안 바뀜.
- **Mitigation**: helper bootstrap 가 nvm/asdf detect → warn + 사용자 동의 ("PATH 우선순위 volta 가 위로 갈게요"). 사용자 reject 시 수동 install 안내.

### R-12. profile add 의 임의 endpoint 등록

- **codex CEO finding F9**
- 시나리오: vibe coder 가 prompt injection 으로 악의적 endpoint (예: attacker.com) 등록 → 다음 axhub auth login 시 token 노출.
- **Mitigation**: profile add SKILL 의 endpoint allowlist gate (`*.jocodingax.ai`, `localhost`). 외 도메인 시 AskUserQuestion warn.
- **Validation**: profile add E2E 가 non-allowlist domain 시 warn + 사용자 explicit confirm 강제.

### R-13. apis call 의 write side-effect

- **codex CEO finding F10**
- 시나리오: vibe coder 가 "API 호출해" 시 apis call SKILL 가 write scope endpoint 호출 → 데이터 mutation. 사전 인지 X.
- **Mitigation**: apis call full consent gate (deploy-equivalent preview + mint + verify). 4-dim preview schema 확장 (payload + side_effect + auth_scope + idempotency).

### R-14. README + GIF maintenance drift

- **codex DX finding F5**
- 시나리오: README 의 lifecycle GIF 가 SKILL workflow 변경 후 stale. vibe coder 가 GIF 따라 했는데 실제 동작 다름.
- **Mitigation**: GIF maintenance ownership = release narrative 작성자. pre-release checklist 에 "GIF re-render 검토" 포함. README version drift = codegen 자동 sync.

### R-15. telemetry contract drift

- **codex DX finding F6**
- 시나리오: plan 가정 (`AXHUB_TELEMETRY=on` / `~/.cache/.../skill-funnel.jsonl`) vs 실제 코드 (`AXHUB_TELEMETRY === "1"` / `~/.local/state/axhub-plugin/usage.jsonl`) 불일치.
- **Mitigation**: plan 텍스트 코드 그대로 인용. 새 telemetry 신설 X, 기존 usage.jsonl 에 SKILL workflow events 추가.

### R-16. SKILL 11→18 의 nl-lexicon surface 65% 증가 → 모델 ambiguity

- 시나리오: 모델이 "axhub 어쩌고" NL 시 17 SKILL 중 어느 것 발화할지 헷갈림. clarify fallback 자주 트리거.
- **Mitigation**: Phase A0 #8 의 negative tests + ax-hub-cli SessionStart prompt routing hook + clarify SKILL = catch.
- **Validation**: corpus runner 가 vibe coder 100 utterance fixture 실행, routing 정확도 ≥90%.

---

## LOW (수용 가능)

### R-17. init→apps create chain 포기 (E9 결정)

- 사용자가 "결제 앱 만들어줘" 한 마디로 backend app 까지 생성 안 됨. 두 ask (init 후 + apps create 별도) = mental load.
- **Mitigation**: explicit ask = 안전성 우선 (E9 + DX F3 결정). chain 자동화 = v0.3.0 candidate (resolver 확장 + e2e shim 보강 후).
- **Honest tradeoff**: 약 10초 추가 + 추가 ask 1회. acceptable.

### R-18. agent install SKILL DEFER 로 plugin self-bootstrap 부재

- **codex CEO finding F5 / E9**
- 사용자가 plugin install 후 axhub manifest 자동 등록 안 됨. 수동 mcp.json 편집 또는 v0.3.0 대기.
- **Mitigation**: ax-hub-cli `cmd/agent/install.go:296` 의 atomic write 도입 PR 후 v0.3.0 에서 plugin 흡수. 그 전까지는 raw `axhub agent install --client=claude-code` 안내.

### R-19. admin SKILL = skeleton (별도 design pass 필요)

- v0.2.0 의 admin SKILL = AskUserQuestion + helper subcommand wrap skeleton 만. 실제 backend endpoint (axhub teams create / members add) = ax-hub-cli 에 없음. sibling repo PR 또는 별도 admin API 필요.
- **Mitigation**: design pass 일정 = Phase A0 완료 후 Phase B 진입 전. plan/PR 에 명시.

### R-20. cold customer 의 비정상 OS / 회사 정책

- 시나리오: 회사 vibe coder 의 ARM Windows / 회사 자체 Linux distro / 회사 corporate antivirus 차단 등 edge case 가 어떤 OS 매트릭스 에 안 잡힘.
- **Mitigation**: 가장 흔한 3 OS (Mac arm64 / Mac amd64 / Ubuntu LTS amd64 / Windows 11 amd64) 만 v0.2.0 ship. edge case 는 v0.3.0 보완.

---

## Risk Summary

| Severity | 개수 | Mitigation 완료 |
|---|---|---|
| CRITICAL | 4 | 4/4 (Phase A0/B 안에 흡수) |
| HIGH | 6 | 6/6 |
| MEDIUM | 6 | 6/6 |
| LOW | 4 | 4/4 (수용 가능) |

**0 unmitigated critical/high risk**. Implementation gate open.
