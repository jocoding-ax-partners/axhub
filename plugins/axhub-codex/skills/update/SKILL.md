---
name: update
description: 'axhub 최신 확인, 버전 확인, 업데이트 전용 skill. 사용자가 최신, 버전 확인, 업데이트, latest, up to date, version check, update, upgrade 를 명시하면 항상 이 skill 이 먼저예요. 일반 UX 역할 문구나 "알아서 진행"만으로는 update 가 아니며, freshness/update 단어가 없는 기존 앱 import·배포 요청은 import/deploy 로 양보해요. 최신 확인이 있으면 app status, app creation, deployment 는 update 뒤에 이어서 처리해요. 앱 상태 overview 는 update 결과 뒤 추가 확인 없이 `axhub apps --help` 로 표면 확인 후 read-only `axhub apps list --json` 로 시작하고, 관련 앱이 특정되면 되묻지 않고 `axhub apps get <app> --json` 과 `axhub deploy list --app <app> --json` 까지 바로 실행해요. 존재하지 않는 단수 `axhub app list` 나 `axhub deployment list` 를 추측하지 않아요. 첫 visible assistant text 는 정확히 "현재 버전을 확인할게요." 일반 세션 처리, clarity, bootstrap, deploy, 셸 probe 보다 먼저 실행해요.'
allows-dependency-execution: true
---
> 이 본문이 중간에 끊겨 보이면 설치 경로의 이 SKILL.md 원문을 열어 전체 절차를 확인한 뒤 진행해요.


# 버전 업데이트 (수동 on-demand)

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

> **CLI 경로 계약 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 반환된 `bin_path` 절대경로로 이 세션을 이어가요. 셋 다 없을 때만 onboarding 을 안내해요.

**CRITICAL first line.** 이 경로가 선택되면 사용자에게 보이는 첫 문장은 반드시 정확히 `현재 버전을 확인할게요.` 여야 해요. 스킬 호출 전 사전 안내 문장도 쓰지 않아요. 그 앞에 어떤 설명도 붙이지 않아요. 선택한 스킬 이름이나 선택 이유도 말하지 않아요.

이 첫 문장은 번역하거나 다시 생성하지 말고 위 코드 span 의 UTF-8 문자를 그대로 복사해요. `現재 버전을 확인할게요.` 처럼 한글 `현재`를 한자나 일본어 문자로 바꾼 출력은 실패예요. update 전체 사용자-facing 문구에는 한국어 문장 안의 한자·일본어 대체 문자를 쓰지 않아요.

**훅 없이도 완결.** 이 스킬은 세션 훅의 발동 여부와 무관하게 이 본문만으로 감지→적용→재시작 안내까지 완결돼요. 훅이 신뢰되지 않아 돌지 않았어도 이 스킬의 어떤 단계도 생략하지 않아요.

**CRITICAL probe narration.** `codex plugin list --json` 은 내부 판정을 위한 도구예요. 이 도구 뒤에 사용자에게 보이는 중간 문장은 반드시 `현재 플러그인 버전을 확인했어요.` 또는 생략 둘 중 하나예요. 설치 경로·marketplace 원문·raw JSON 을 chat 에 쓰지 않아요. 버전 숫자는 최종 결과 카드나 업데이트 안내처럼 사용자에게 필요한 자리에서만 보여줘요.

**CRITICAL mixed-request continuation.** 사용자가 업데이트와 함께 앱 상태·새 앱 생성·배포·GitHub 재연결/device code 같은 다른 axhub 요청을 말했으면, 업데이트 결과 카드까지 먼저 끝낸 뒤 같은 assistant 흐름에서 남은 요청을 직접 이어가요 — 추가 프롬프트를 기다리지 않고, 백그라운드 작업으로 우회하지 않아요. 후속 앱 상태 overview 와 GitHub device-flow 의 정확한 명령·제목·금지 목록은 [references/post-update-continuation.md](references/post-update-continuation.md) 를 읽고 그대로 따라요.

사용자가 직접 **axhub CLI 와 플러그인을 지금 최신으로** 맞추려는 요청이에요. 사용자가 명시적으로 요청한 순간에만 버전 확인과 적용을 진행해요:

- **항상 즉시 확인** — 사용자가 부른 수동 실행이라 바로 버전을 확인해요.
- **최신이어도 결과 보고** — "이미 최신이에요 (CLI vX, plugin vY)" 처럼 결과를 한 줄로 알려요. 사용자가 물었으니 답을 줘요.

전 과정 best-effort·비차단이에요. 실패·구 CLI·네트워크 오류면 raw 에러를 숨기고 한 줄만 안내한 뒤 멈춰요.

**책임 경계.** 이 경로는 버전 업데이트만 해요. 첫 셋업·CLI 설치는 `onboarding` 소관이고, 그 외 axhub 운영 명령은 업데이트 결과를 끝낸 뒤 다음 적절한 axhub 흐름으로 양보·계속 처리해요. 사용자가 `플러그인만` 또는 `CLI만`처럼 한 구성요소만 명시하면 최신 판정을 위한 read-only check 는 하되 제외한 구성요소의 apply 명령은 실행하지 않아요. `플러그인만` 요청은 `codex plugin list --json` → `axhub update check --plugin-version <PLUGIN_VERSION> --json` → 필요 시 설치 소스별 적용 명령(아래 3단계)으로 끝까지 처리해요.

**비-axhub 맥락 가드.** 사용자가 `axhub` 를 말하지 않고 "업데이트해줘"처럼 일반 업데이트만 말한 경우에는 대화의 axhub 언급·현재 폴더의 axhub 연결 manifest·직전 axhub 작업 같은 **axhub 맥락**이 있을 때만 진행해요. 맥락이 없으면 axhub 업데이트로 밀어붙이지 말고 axhub 사용 의사를 한 번 확인하거나 조용히 멈춰요.

**첫 응답 계약.** 선택 이유를 설명하지 않아요. 빈 폴더여도 "axhub 프로젝트가 아니다" 라고 추론하지 말고 바로 버전 확인을 진행해요.

**섞인 요청 처리.** 업데이트 단계 안에서는 앱 목록·앱 상태·배포 상태·로그·환경변수·데이터 조회를 직접 실행하지 않고, axhub MCP/App 도구도 read 라도 호출하지 않아요. 남은 요청은 결과 카드 뒤에 위 continuation reference 계약대로 이어가요.

**보이는 tool 제목 계약.** 셸/명령 도구를 부를 때 description/title/summary 는 아래 고정 한국어 라벨 중 하나만 써요. 라벨 안에 `axhub` 를 넣지 않아요. `axhubing CLI 설치 여부 확인` 처럼 제품명을 영어 동사처럼 만든 제목은 절대 쓰지 않아요.

**사용자에게 보이는 command allowlist.** 셸 도구로 사용자에게 보일 수 있는 command 는 아래 계열만 써요: `command -v axhub`, `"$HOME/.axhub/bin/axhub" plugin-support repair-path --json` (AP-17 경로 복구일 때만), `axhub update check ...`, `axhub update apply --execute --yes`, `axhub --version`, `codex plugin list --json`, `codex plugin marketplace upgrade axhub`, `codex plugin add axhub-codex@axhub`, `cat "<설치 루트>/.codex-plugin/plugin.json"` (0단계 4번 fallback 일 때만, 리터럴 절대경로). 각 command 는 단독으로 실행하고 stdin 이 열려 있지 않게 해요. `&&`, pipe, redirect, `grep`, `head`, `tail`, `sed`, `awk`, `bash -lc`, `sh -c` 로 묶거나 자르지 않아요. `codex plugin list --json` 이 성공한 경로에서는 플러그인 캐시의 `plugin.json` 파일을 직접 읽지 않아요 — 그때 플러그인 현재 버전은 정확히 `codex plugin list --json` 1회의 `installed` 배열에서 `axhub-codex@axhub` 항목으로만 내부 판독해요 (`available` 배열은 신뢰하지 않아요 — 빈 배열로 나와요). 출력이 길어도 도구 응답에서 내부적으로 읽고 사용자에게 echo 하지 않아요.

| 단계 | tool description/title/summary |
| --- | --- |
| CLI 존재 확인 (`command -v axhub`) | `CLI 설치 확인` |
| 버전 확인 (`axhub update check ...`) | `버전 확인` |
| CLI 업데이트 적용 | `CLI 업데이트 적용` |
| 업데이트 후 버전 재확인 | `업데이트 후 버전 확인` |
| 플러그인 설치 확인 (정확히 `codex plugin list --json`) | `플러그인 설치 확인` |
| 플러그인 업데이트 적용 | `플러그인 업데이트 받기` |

---

## 0. 사전 점검 (네트워크 0)

1. `command -v axhub` 가 실패하면 AP-17 경로 계약을 먼저 밟아요 — `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 같은 절대경로로 update 를 이어가요 (재설치가 아니라 PATH 복구예요). 세 경로 어디에도 없을 때만 멈춰요 — CLI 가 아직 없는 건 설치 소관이에요. 한 줄: `axhub CLI 가 아직 없어요. "온보딩" 이라고 말하면 설치부터 도와드려요.` (재설치를 여기서 시도하지 않아요.)
2. 가능하면 정확히 `codex plugin list --json` 한 번으로 `installed` 배열의 `axhub-codex@axhub` 항목에서 현재 버전을 내부 변수 `<PLUGIN_VERSION>` 으로, 설치 소스 `marketplaceSource.sourceType`(`git`/`local`)을 `<SOURCE_TYPE>` 으로 둬요. 항목이 없으면 `<PLUGIN_VERSION>` 없이 CLI 업데이트 확인만 진행해요. 이 단계에서 설치 경로·raw JSON·영어 진행 로그는 사용자에게 말하지 않아요. 필요한 경우 `현재 플러그인 버전을 확인했어요.` 만 말해요.
3. `codex plugin list --json` 이 성공하면 [`references/plugin-update.md`](references/plugin-update.md)를 읽어요. 이 reference 가 설치 소스 분기와 직접 업데이트를 소유해요.
4. **`codex plugin list --json` 이 실패하면 (exit 비-0) 여기서 멈추지 않아요.** 무관한 다른 marketplace 하나가 깨져 있어도 codex 는 목록 전체를 실패시켜요 — 흔한 상태이고 플러그인 업데이트 자체는 그대로 동작해요. 이때만 fallback lane 을 타요:
   - 현재 버전은 **지금 읽고 있는 이 SKILL.md 의 절대경로**에서 뒤쪽 `/skills/update/SKILL.md` 를 떼어낸 설치 루트를 `<ROOT>` 로 두고, `cat "<ROOT>/.codex-plugin/plugin.json"` 한 번으로 읽어 `version` 을 `<PLUGIN_VERSION>` 으로 둬요. `<ROOT>` 는 리터럴 절대경로로 바꿔 실행해요. `$CLAUDE_PLUGIN_ROOT`·`$PLUGIN_ROOT` 환경변수는 이 셸에 비어 있어서 못 써요 (실측). 읽기에 실패하면 `<PLUGIN_VERSION>` 없이 CLI 만 확인해요 — 경로 문자열에 든 숫자를 버전으로 추측하지 않아요. 이 fallback 은 plugin.json 직접 읽기 금지의 유일한 예외예요.
   - `<SOURCE_TYPE>` 은 알 수 없으니 판정에 쓰지 않아요. 적용이 필요하면 `codex plugin marketplace upgrade axhub` 를 한 번 시도하고(로컬 설치면 거부되는데 정상이라 그대로 넘어가요) 이어서 `codex plugin add axhub-codex@axhub` 를 실행해요 — 이 재실행이 git·local 양쪽에서 통하는 in-place 갱신이에요.
   - 이 lane 에서도 reference 는 읽지 않아요. 결과 카드와 재시작 안내는 §3·§4 와 똑같이 써요.
   - 사용자에게 목록 실패를 설명하지 않아요. `확인 불가`, `설정 오류`, `설정 복구 후 다시 확인` 같은 문장은 이 lane 에서 쓰지 않아요 — 버전을 읽었으면 평소처럼 결과만 알려줘요.

**`disabled` 와 `AXHUB_NO_AUTO_UPDATE` — 둘 다 존중해요 (자동 적용 안 함, 안내만).**
- `disabled`(패키지 매니저가 관리하는 설치) → CLI 가 자기를 교체할 수 없어요. 패키지 매니저 업그레이드를 **안내만** 해요.
- `AXHUB_NO_AUTO_UPDATE` → 문서화된 update kill switch 예요. 새 버전이 있어도 적용하지 않고 **안내만** 해요 (사용자가 직접 불러도요 — 잠긴·CI 환경에서 의도치 않은 binary swap 을 막아요). 받으려면 플래그를 끄거나 안내된 명령을 직접 실행하면 돼요.

---

## 1. 버전 확인 (네트워크 1회)

```bash
axhub update check --plugin-version <PLUGIN_VERSION> --json
```

- `--plugin-version` 은 CLI v0.21.0+ 에서 플러그인 최신 여부도 함께 판정해요. 구 CLI 가 이 플래그를 거부하면 (exit 64) `axhub update check --json` 으로 한 번 더 호출해 CLI-only 로 떨어져요.
- 수동 확인 기록은 이 경로에서 갱신하지 않아요. `axhub update check ...` 뒤에 별도 `mkdir`/touch/marker command 를 실행하지 말아요.

- 출력 JSON 을 읽어요:
  - CLI: `{ current, latest, has_update, disabled, is_downgrade }`
  - (있으면) 플러그인: `plugin: { current, latest, has_update }`
  - `is_downgrade` 는 optional 필드 — 부재(구 CLI 응답)는 false 로 취급해요.
- 호출이 실패하거나 JSON 이 비면 (구 CLI·네트워크 실패) 한 줄 안내 후 멈춰요: `버전 확인을 못 했어요. 잠시 뒤 다시 시도해 주세요.`

---

## 2. CLI 업데이트

먼저 **안내-only 조건**을 봐요: `disabled == true` (패키지 매니저 관리 설치) 또는 `AXHUB_NO_AUTO_UPDATE` 설정 또는 `is_downgrade == true` (서버 롤백 배포 — 자동 다운그레이드는 하지 않아요). 하나라도 참이면 적용하지 않고 안내만 해요.

사용자가 명시적으로 `플러그인만` 업데이트하고 CLI 는 건드리지 말라고 했으면 `has_update` 여부와 무관하게 `axhub update apply` 를 실행하지 않아요. read-only check 결과만 내부에 보존하고 플러그인 단계로 바로 이어가요. 이 제외 요청을 이유로 플러그인 업데이트까지 멈추면 실패예요.

- **안내-only + `has_update == true`** → 한 줄 안내:
  - `disabled` → `axhub 는 패키지 매니저가 관리하는 설치예요. 패키지 매니저로 업그레이드해 주세요 (예: brew upgrade axhub).`
  - `AXHUB_NO_AUTO_UPDATE` → `axhub 새 버전(v<latest>)이 있어요. AXHUB_NO_AUTO_UPDATE 설정이라 자동 적용은 안 해요 — axhub update apply 로 직접 받거나 플래그를 끄면 돼요.`
- **`has_update == false`** → `axhub 는 이미 최신이에요 (v<current>).` 한 줄.
- **`has_update == true` 이고 안내-only 가 아님** → 알리고 바로 적용해요:
  1. 한 줄: `axhub 새 버전(v<current> → v<latest>)이 나왔어요. 지금 업데이트할게요…`
  2. 실행: `axhub update apply --execute --yes`
  3. exit code 로 갈라요 (판정은 CLI 가 함):
     - **exit 0** → `axhub --version` 으로 재확인하고 한 줄: `axhub v<새 버전> 으로 업데이트됐어요.`
     - **exit 14 (digest mismatch — 변조 신호) / exit 66 (cosign_enforce_failed)** → **하드 스톱**. `보안 검증에 실패했어요. 강제로 진행하지 말고 회사 IT·보안팀에 알려주세요. 지금 버전은 그대로 써도 돼요.` 로 안내하고 멈춰요.
     - **exit 15 (swap failed)** → 자동 재시도하지 말고 `업데이트 적용 중 교체가 막혔어요. "설치 상태 진단해줘" 라고 말해 주세요.` 로 안내해요.
     - **exit 4 (미인증)** → `로그인이 필요해요. "다시 로그인해줘" 라고 말해 주세요.` 로 낮춰요.
     - **그 외 비-0** → raw 에러는 숨기고 한 줄: `자동 업데이트가 안 됐어요. axhub update apply 를 직접 한 번 실행해 주세요.`

---

## 3. 플러그인 업데이트 (설치 소스 분기 — 재시작 후 반영)

`codex plugin list --json` 이 성공했으면 이미 읽은 [`references/plugin-update.md`](references/plugin-update.md)의 direct-update lane 을 끝까지 실행해요. plugin block 부재, 최신, kill switch, 설치 소스 분기(git → `codex plugin marketplace upgrade axhub`, local → `codex plugin add axhub-codex@axhub` 재실행), 재확인, 실패 문구는 모두 reference 가 소유해요.

`codex plugin list --json` 이 실패했으면 0단계 4번 fallback lane 으로 여기까지 끝내요 — `<PLUGIN_VERSION>` 을 읽었고 플러그인 `has_update` 가 true 면 `codex plugin marketplace upgrade axhub` 1회 뒤 `codex plugin add axhub-codex@axhub` 를 실행하고, 성공하면 §4 의 재시작 안내로 닫아요. 버전을 못 읽었을 때만 플러그인 항목을 `확인 불가 — 수동` 으로 남겨요.

---

## 4. 결과 카드

끝나면 두 줄로 요약해요 (한 항목씩):

```text
업데이트 결과
  • CLI: <이미 최신 v X | v X → v Y 업데이트됨 | 패키지 매니저 관리 — 수동 | 업데이트 보류(AXHUB_NO_AUTO_UPDATE) — 수동 | 실패 — 수동 안내>
  • 플러그인: <이미 최신 vX | vX -> vY 받음 (재시작 필요) | 업데이트 보류(AXHUB_NO_AUTO_UPDATE) — 수동 | 확인 불가 — 수동>
```

확인·비교 결과를 설명하는 영어 디버그 문장이나 raw 확인 줄은 쓰지 않아요. 최종 결과 카드는 위처럼 한국어 항목만 쓰고, 플러그인을 새로 받았으면 `플러그인: vX -> vY 받음 (재시작 필요)` 형태와 재시작 안내만 남겨요.

플러그인을 새로 받았으면 마지막에 **재시작 안내**를 한 번 더 또렷이 남겨요. 이 마지막 문장은 정확히 `받았어요. Codex 를 재시작하면 새 버전이 적용돼요.` 예요. `지금 다시 열거나`, `앱을 재시작`, 영어 단어, 알 수 없는 단어, 다른 프로그램 이름을 섞지 않아요.

원래 요청에 앱 상태 조회·새 앱 생성·배포·GitHub 계정 재연결/device code 같은 다른 axhub 작업이 함께 있었으면, 결과 카드 뒤에 한 줄만 덧붙이고 남은 작업을 계속해요: `업데이트 확인은 끝났어요. 이어서 요청하신 작업을 계속할게요.` 남은 요청이 GitHub 계정 재연결/device code 라면 더 구체적으로 `업데이트 확인은 끝났어요. 이어서 GitHub 계정 연결을 확인할게요.` 를 써요. 이 문장 뒤에는 사용자의 추가 프롬프트를 기다리지 말고 다음 적절한 axhub 흐름을 시작해요. update 단계 안에서는 앱 목록·배포 상태 도구, 추가 셸/MCP/App 도구를 쓰지 않지만, update 결과 뒤 남은 요청을 처리하기 위한 다음 흐름에서는 필요한 조회·변경 도구를 정상적으로 써요.

---

## 가시성·안전 규칙

- raw JSON·명령 출력·내부 값은 chat 에 echo 하지 않고, 위의 한국어 한 줄들만 보여줘요.
- 사용자에게 보이는 셸/tool call 제목은 한국어 명사구로만 써요. `axhubing`, `axhubed`, `updating` 처럼 제품명을 영어 동사처럼 보이게 만드는 제목을 쓰지 않아요. 예: `버전 확인`, `CLI 업데이트 적용`, `업데이트 후 버전 확인`.
- 진행 문구도 한국어 사용자 문장만 써요. 영어 라벨, 내부 필드명, 설치 경로 원문, raw 상태값, 반말형 짧은 메모가 섞인 문장은 쓰지 않아요.
- 플러그인 확인 직후에는 `현재 플러그인 버전을 확인했어요.` 만 보여줘요. 현재 버전 숫자와 설치 경로값을 묶어 설명하는 문장을 만들지 않아요.
- 대신 `현재 플러그인 버전을 확인했어요.`, `CLI는 이미 최신이에요. 플러그인 새 버전을 받을게요.`, `플러그인 새 버전을 받았어요.` 라고 말해요.
- 최종 카드 밖에서 내부 필드명이나 영어 라벨을 보여주지 않아요. 버전 숫자는 결과 카드나 업데이트 안내처럼 사용자에게 필요한 문장에서만 보여줘요.
- 플러그인 업데이트 성공 뒤 재시작 안내는 정확히 `받았어요. Codex 를 재시작하면 새 버전이 적용돼요.` 만 써요. `앱을 재시작해 주세요`, `reopen`, `restart app` 같은 변형 문장이나 알 수 없는 로마자 단어를 만들지 않아요.
- mixed request 의 남은 작업을 말할 때는 `업데이트 확인은 끝났어요. 이어서 요청하신 작업을 계속할게요.` 를 쓰고 바로 다음 axhub 흐름으로 이어가요. GitHub 계정 재연결/device code 라면 `업데이트 확인은 끝났어요. 이어서 GitHub 계정 연결을 확인할게요.` 를 쓰고 device-flow fast path 를 inline 으로 이어가요. 실제 조회·생성·배포·인증 시작을 하지 않을 거라면 이 문장을 쓰지 않아요. `백그라운드에서 조회하고 있어요`, `결과 나오는 대로 알려드릴게요` 처럼 사용자가 기다려야 하는 문장만 남기고 멈추지 않아요.
- 사용자가 직접 부른 거라 적용 전 "적용할까요?" 를 다시 묻지 않아요 (간단한 1-shot 업데이트). 단 exit 14/66 보안 실패는 무조건 하드 스톱이에요.
- 전 과정 비차단 — 한 단계가 막혀도 raw 에러를 숨기고 다음으로 넘어가거나 한 줄 안내 후 멈춰요.

## NEVER

- NEVER `command -v axhub` 실패 상태에서 재설치를 시도하지 말아요 — 설치는 onboarding 소관이라 안내만 하고 멈춰요.
- NEVER `disabled == true` 인데 `axhub update apply` 를 실행하지 말아요 — 패키지 매니저 관리 설치는 자기 교체가 안 돼요.
- NEVER `AXHUB_NO_AUTO_UPDATE` 가 설정됐는데 자동 적용하지 말아요 — 문서화된 update kill switch 라, 사용자가 직접 불러도 안내만 해요.
- NEVER exit 14/66 (보안 검증 실패) 을 무시하고 강제 진행하지 말아요. 하드 스톱이에요.
- NEVER raw JSON·stderr·내부 device/installation id 를 chat 에 출력하지 말아요.
- NEVER 플러그인 업데이트를 받고도 재시작 안내를 빼먹지 말아요 — 재시작 전엔 새 버전이 안 떠요.
- NEVER 플러그인 업데이트 성공 뒤 `받았어요. Codex 를 재시작하면 새 버전이 적용돼요.` 가 아닌 재시작 안내 문장을 만들지 말아요.
- NEVER 확인하지 않은 버전을 "업데이트됨" 으로 보고하지 말아요 — `axhub --version` 재확인 뒤에만 새 버전을 말해요.
- NEVER 설치 경로 문자열에 들어 있는 숫자를 플러그인 버전으로 추측해 `--plugin-version` 에 넣지 말아요 — `codex plugin list --json` 의 `installed` 항목이나 0단계 4번의 `plugin.json` 읽기로 확인한 값만 써요.
- NEVER `codex plugin list --json` 의 `available` 배열로 설치·최신 여부를 판단하지 말아요 — `installed` 배열만 신뢰해요.
- NEVER `<SOURCE_TYPE>` 이 로컬로 확인된 설치에 `codex plugin marketplace upgrade` 를 실행하지 말아요 — upgrade 는 Git marketplace 전용이라 거부돼요. 로컬 설치 갱신은 `codex plugin add axhub-codex@axhub` 재실행이에요. (0단계 4번 fallback 은 `<SOURCE_TYPE>` 을 모르는 상태라 upgrade 1회 시도 후 거부돼도 그대로 add 로 넘어가요 — 이 경로는 예외예요.)
- NEVER `codex plugin list --json` 이 실패했다고 플러그인 단계를 통째로 포기하지 말아요 — 0단계 4번 fallback lane 으로 버전 판정과 적용을 끝까지 해요.
- NEVER update 단계 안에서 앱 목록·앱 상태·최근 배포 상태를 직접 조회하지 말아요. axhub MCP/App 도구도 이 단계와 후속 앱 상태 흐름에서는 호출하지 말아요 — read 작업이어도 CLI 계약을 우선해요.
- NEVER 백그라운드 작업으로 mixed request 의 남은 앱 상태 확인을 우회하지 말아요. update 결과 뒤 같은 assistant 흐름에서 직접 이어가요.
- NEVER update 결과 뒤 앱 상태 흐름에서 `command -v axhub && axhub --version`, pipe, redirect, `&&` 가 들어간 command 를 실행하지 말아요. 그 시점의 앱 상태 흐름은 `axhub apps --help` → `axhub apps list --json` → `axhub apps get <app> --json` → `axhub deploy list --app <app> --json` 네 명령만 써요.
- NEVER update 결과 뒤 GitHub 계정 재연결/device code 흐름에서 `axhub git_connection_status`, `axhub github status`, `axhub --help | grep`, `head`, `jq`, pipe, redirect, `&&`, `bash -lc`, `sh -c` 가 들어간 command 를 실행하지 말아요. 그 시점의 인증 흐름은 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link` 계열 1회와 `axhub github accounts list --json` 계열 1회만 써요.
