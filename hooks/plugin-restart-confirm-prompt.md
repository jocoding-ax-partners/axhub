# axhub 플러그인 재시작 확인 (내부 — 훅 트리거)

SessionStart 훅이 `~/.axhub/cache/.plugin-update-restart` marker(7일 TTL)를 감지하면 이 지침을 불러요. auto-update 가 플러그인 새 버전을 받은 뒤 재시작으로 실제 적용됐는지 확인하는 마무리 단계예요. 사용자가 직접 부르는 skill 이 아니에요.

```
marker 상태머신:
  없음 ──plugin 적용(auto-update §3)──▶ 활성(내용 = 받은 버전)
  활성 ──재시작 후 확인 성공──▶ 삭제
  활성 ──7일 경과(재시작 안 함)──▶ 휴면(훅 발동 중지, 파일 잔존)
  휴면 ──다음 플러그인 적용──▶ 활성(덮어씀)
```

**핵심 원칙:** best-effort·비차단이에요. 확인은 세션당 정확히 한 줄 + 명령 1회로 끝내고, 실패하면 조용히 건너뛰고 사용자의 실제 요청을 절대 막지 않아요.

---

## 확인 절차

1. marker 내용을 읽어 기대 버전 `<EXPECTED>` 로 둬요 (예: `1.10.28`). 읽기 실패면 조용히 멈춰요.
2. `command -v claude` 가 실패하면 조용히 멈춰요 — marker 는 그대로 두면 TTL 로 휴면해요.
3. `claude plugin list` 를 1회 실행해 `axhub@axhub` 의 **enabled 항목 중 가장 높은 semver** 를 읽어요 (update SKILL 의 중복 설치 판정 알고리즘과 동일). 파싱이 안 되면 조용히 멈춰요.
   - 참고: 이 확인은 설치-기준이에요 — 멀티 scope 중복 설치에서 다른 scope 의 버전을 볼 수 있고, 현재 세션이 그 버전을 로드했는지까지 증명하진 않아요. 기존 update SKILL 확인과 동일한 한계예요.
4. 판정:
   - **최고 enabled semver ≥ `<EXPECTED>`** → 한 줄 안내 후 marker 삭제:
     > `플러그인 v<확인된 버전> 적용을 확인했어요.`
     ```bash
     rm -f "$HOME/.axhub/cache/.plugin-update-restart"
     ```
   - **아직 낮음** (재시작 미반영) → marker 는 유지하고 한 줄만:
     > `플러그인 새 버전이 아직 반영 전이에요. Claude Code 를 재시작하면 적용돼요.`
   - 판정 불가 → 아무 말 없이 멈춰요.

---

## 가시성·안전 규칙

- raw 목록·scope 원문·내부 값은 chat 에 echo 하지 않아요. 위의 한국어 한 줄만 보여줘요.
- 재시작 전까지 이 확인은 세션당 한 줄 + `claude plugin list` 1회가 상한이에요 — 반복 재촉·watcher·sleep 금지.
- 이 흐름은 확인 전용이에요 — 여기서 `claude plugin update` 나 `axhub update apply` 를 실행하지 않아요.
- 끄기: `AXHUB_NO_AUTO_UPDATE=1` 또는 marker 파일 `~/.axhub/config/no-auto-update` (auto-update 계열 공용 kill switch — 훅은 프로세스 env 와 marker 파일만 봐요).
