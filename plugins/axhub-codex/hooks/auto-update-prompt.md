# axhub 자동 업데이트 (내부 — 훅 트리거, codex)

SessionStart 훅이 24시간에 한 번 이 지침을 부르면, axhub CLI 와 플러그인의 새 버전을 확인하고 적용해요. 사용자가 직접 부르는 skill 이 아니에요.

**핵심 원칙:** best-effort·비차단이에요. 실패·구 CLI·네트워크 오류면 조용히 건너뛰고, 사용자의 실제 요청을 절대 막지 않아요. 업데이트 안내는 짧게 한 줄로 끝내고 원래 작업을 이어가요.

---

## 0. 사전 점검 (네트워크 0)

1. `command -v axhub` 가 실패하면 AP-17 경로 계약을 한 번 밟아요 — bare `axhub` 실패는 미설치가 아니에요. `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 이후 명령을 그 절대경로로 이어가요. 세 경로 어디에도 없으면 즉시 멈춰요 — CLI 가 없는 건 onboarding 소관이에요.
2. `codex plugin list --json` 1회의 `installed` 배열에서 `axhub-codex@axhub` 항목의 `version` 을 `<PLUGIN_VERSION>` 으로, `marketplaceSource.sourceType` 을 `<SOURCE_TYPE>` 으로 둬요 (못 읽으면 plugin 확인은 생략하고 CLI 만 봐요). `available` 배열은 신뢰하지 않아요.
3. 환경변수 `AXHUB_NO_AUTO_UPDATE` 가 설정돼 있으면 **안내만** 모드예요 — 아래 자동 적용(2단계 apply)을 건너뛰고, 새 버전이 있을 때 한 줄 안내만 해요.
   - 이 분기가 필요한 이유(env-divergence): 훅 bash 는 호스트 프로세스 env 만 봐요 — shell profile 을 소싱하지 않아요. profile 에만 `export AXHUB_NO_AUTO_UPDATE=1` 을 둔 사용자가 GUI 로 실행하면 훅은 변수를 못 보고 발동하지만, 이 지침을 실행하는 에이전트의 셸은 profile 을 소싱하므로 여기서 잡아요. 이 분기는 그 경우의 유일한 kill switch 예요.

---

## 1. 버전 확인 (네트워크 1회)

1. 실행해요:

   ```bash
   axhub update check --plugin-version <PLUGIN_VERSION> --json
   ```

   구 CLI (v0.21.0 미만) 가 `--plugin-version` 을 거부하면 (exit 64 usage error) 여기서 조용히 멈추지 않아요 — 업데이트가 가장 필요한 구 CLI 일수록 이 fallback 이 있어야 hook 이 최신으로 끌어올릴 수 있어요. update 스킬과 같은 계약으로 플래그 없이 한 번 더 호출해 CLI-only 로 떨어져요 (플러그인 비교만 생략돼요):

   ```bash
   axhub update check --json
   ```

2. 결과와 무관하게 재확인 주기 캐시를 바로 갱신해요 (24h throttle 의 기준점):

   ```bash
   mkdir -p "$HOME/.axhub/cache" && : > "$HOME/.axhub/cache/.plugin-update-check-codex"
   ```

3. 출력 JSON 을 읽어요:
   - CLI: `{ current, latest, has_update, disabled, is_downgrade }`
   - (있으면) 플러그인: `plugin: { current, latest, has_update }`
   - `is_downgrade` 는 optional 필드예요 — **부재(구 CLI 응답)는 false 로 취급**해요. `is_downgrade == true` 면 서버가 낮은 버전을 latest 로 내려보내는 롤백 상황이라 자동 적용하지 않고 안내만 해요 (변조 방어는 이 필드가 아니라 apply 의 cosign 검증이 담당해요).
4. fallback 까지 실패하거나 JSON 이 비면 (네트워크 실패 등) 조용히 멈춰요 — 작업을 막지 않아요.

---

## 2. CLI 업데이트

분기로 처리해요:

- **`disabled == true`** (패키지 매니저가 관리하는 설치) **또는 `AXHUB_NO_AUTO_UPDATE` 설정** **또는 `is_downgrade == true`** (서버 롤백 배포 — 부재는 false) → 자동 적용하지 않아요. `has_update` 면 한 줄만 안내해요:
  > `axhub 새 버전(<latest>)이 있어요. axhub update apply 로 받을 수 있어요.`
- **`has_update == false`** → 아무것도 보여주지 않고 조용히 통과해요.
- **`has_update == true` 이고 적용 가능** → 사용자에게 해요체로 알리고 바로 적용해요 (auto):
  1. 안내 한 줄: `axhub 새 버전(<current> → <latest>)이 나왔어요. 지금 업데이트할게요…`
  2. 실행: `axhub update apply --execute --yes`
  3. 끝나면 `axhub --version` 으로 재확인하고 한 줄: `axhub <새 버전> 으로 업데이트됐어요.`
  4. 적용이 실패하면 (권한·네트워크 등) raw 에러는 숨기고 한 줄만 안내한 뒤 비차단으로 계속해요:
     > `자동 업데이트가 안 됐어요. axhub update apply 를 직접 한 번 실행해 주세요.`
  5. 단, 보안 검증 실패 (exit 14 digest mismatch / exit 66 cosign_enforce_failed) 는 재실행을 권하지 않아요 — update 스킬과 같은 하드 스톱으로 안내하고 멈춰요:
     > `보안 검증에 실패했어요. 강제로 진행하지 말고 회사 IT·보안팀에 알려주세요. 지금 버전은 그대로 써도 돼요.`

---

## 3. 플러그인 업데이트 (설치 소스 분기 — 자동 적용, 재시작 후 반영)

- `plugin` 블록이 없거나 **`plugin.has_update == false`** → 생략해요.
- **`command -v codex` 실패** (codex CLI 없음) → 한 줄 안내만: `axhub 플러그인 새 버전(<plugin.latest>)이 있어요. Codex 에서 업데이트해 주세요.`
- **`AXHUB_NO_AUTO_UPDATE` 설정** → 적용하지 않고 한 줄 안내만: `axhub 플러그인 새 버전(<plugin.latest>)이 있어요. 안내된 명령으로 받을 수 있어요.`
- **`plugin.has_update == true` 이고 적용 가능** → 자동 적용해요:
  1. 안내 한 줄: `axhub 플러그인 새 버전(<plugin.current> → <plugin.latest>)이 나왔어요. 지금 받을게요…`
  2. `<SOURCE_TYPE>` 으로 적용 명령을 갈라요:
     - **`git`** → `codex plugin marketplace upgrade axhub`
     - **`local`** → `codex plugin add axhub-codex@axhub` 재실행 (로컬 marketplace 는 upgrade 대상이 아니라 재add 가 갱신 경로예요)
  3. 성공하면 재시작 확인 marker 를 기록해요 — 다음 세션의 restart-confirm 훅이 실제 적용을 확인하고 닫아요 (절차는 `plugin-restart-confirm-prompt.md` 소유):

     ```bash
     mkdir -p "$HOME/.axhub/cache" && printf '%s' '<plugin.latest>' > "$HOME/.axhub/cache/.plugin-update-restart-codex"
     ```

     marker 내용은 `받은 버전` 이에요 — restart-confirm 이 설치 항목을 직접 확인해요.

  4. **재시작 안내(필수 — plugin 업데이트는 재시작해야 적용돼요):** `받았어요. Codex 를 재시작하면 새 버전이 적용돼요.`
  5. 실패하면 raw 에러는 숨기고 한 줄만 안내한 뒤 비차단으로 계속해요 (marker 는 기록하지 않아요) — `git` 이면 `플러그인 자동 업데이트가 안 됐어요. codex plugin marketplace upgrade axhub 를 직접 실행해 주세요.`, `local` 이면 `플러그인 자동 업데이트가 안 됐어요. codex plugin add axhub-codex@axhub 를 직접 실행해 주세요.`

---

## 가시성·안전 규칙

- raw JSON·명령 출력·내부 값은 chat 에 echo 하지 않고, 위의 한국어 한 줄들만 보여줘요.
- 전 과정 비차단이에요 — 업데이트가 사용자의 실제 요청보다 우선하지 않아요. 사용자가 이미 다른 일을 시키는 중이면 안내만 한 줄 남기고 원래 작업을 이어가요.
- 이 흐름은 SessionStart 당 최대 1회(24시간 throttle)만 돌아요. 완전히 끄려면 `AXHUB_NO_AUTO_UPDATE=1` 을 설정하면 돼요.
