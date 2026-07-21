---
name: diagnosis
description: 'diagnosis: "배포 실패 원인 진단해줘", "왜 배포가 죽었어", "방금 배포된 앱 혹시 실패 원인 같은 거 있으면 진단해줘", "재배포는 하지 말고 원인만 봐줘", "diagnose deployment failure", "diagnose failed deployment <id> for app <slug>", "failed deployment diagnosis", "why did my deploy fail"처럼 axhub 배포 실패 원인과 해결 후보를 읽기 전용으로 알고 싶을 때만 사용해요. 영어 실패 배포 진단 요청도 반드시 이 스킬로 라우팅하고, MCP app/list/read 도구만으로 답하지 않아요. 결과는 사용자 카테고리로 요약하고 재배포·롤백은 직접 실행하지 않아요. 배포 실행/검증=deploy, 상태·로그·롤백·운영 명령=clarity, 업데이트=update, 앱 코드 생성=development 로 양보해요. 이 트리거들은 axhub 맥락(현재 폴더의 axhub 연결·발화의 axhub 언급·대화의 직전 axhub 작업)이 있을 때만 유효해요. 다른 플랫폼 배포 실패 발화에는 이 스킬을 쓰지 않아요.'
examples:
  - utterance: "배포 실패 원인 진단해줘"
    intent: "diagnose deployment failure cause"
  - utterance: "이 앱 배포 실패 진단해줘"
    intent: "diagnose app deployment failure cause"
  - utterance: "diagnose deployment failure"
    intent: "diagnose deployment failure cause"
  - utterance: "Diagnose failed deployment 96728617 for app my-app"
    intent: "diagnose deployment failure cause"
  - utterance: "why did my deploy fail?"
    intent: "diagnose deployment failure cause"
  - utterance: "방금 배포된 앱 혹시 실패 원인 같은 거 있으면 진단해줘. 재배포는 하지 말고 원인만 봐줘."
    intent: "diagnose deployment failure cause"
allows-dependency-execution: false
model: sonnet
---

# axhub diagnosis (배포 실패 원인 진단)

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

배포 실패 원인을 읽기 전용으로 진단하는 스킬이에요. 사용자가 실패 원인이나 해결 후보를 명시적으로 물을 때만 들어오고, 배포 실행·재배포·롤백은 절대 직접 실행하지 않아요. `혹시 실패 원인 같은 거 있으면`, `원인만 봐줘`, `재배포는 하지 말고` 같은 조건부·소극적 표현도 실패 원인 진단 의도예요.

발화에 axhub 언급이 없고 대화에 axhub 맥락(현재 폴더의 axhub 연결·직전 axhub 작업)도 없으면 — 배포 실패가 다른 플랫폼일 수 있으면 — 진단을 시작하기 전에 어느 플랫폼 배포인지 한 번만 확인하고, axhub 가 아니면 종료해요. headless 에서는 묻지 않고 멈춰요.

진단 결과는 두 층으로 나눠요. `deploy` 에서 방금 실패한 배포 id 를 넘긴 경우에는 그 **실패한 배포 한 건의 status** 를 먼저 읽고, status 의 시작·종료 시각으로 좁힌 **그 시간대 앱 로그**를 함께 본 뒤(`deploy logs` 는 CLI 계약상 배포 한 건이 아니라 앱 단위 로그예요), 현재 라이브 롤아웃 상태를 별도로 확인해요. 사용자가 앱만 주고 실패 배포 id 가 없으면 앱의 **현재 라이브 롤아웃 상태**를 진단해요. 그래서 "현재 라이브는 정상"이어도 방금 배포가 실패했을 수 있다는 한계를 사용자에게 정직하게 전달해요.

## 핵심 책임

- **CLI 전용이에요.** MCP `deployment_diagnosis` 같은 deployment MCP 도구가 보여도 호출하지 않아요. 진단 표면은 `axhub deploy status`, `axhub deploy logs`, `axhub deploy diagnose` 만 써요.
- 영어로 "Diagnose failed deployment ..."처럼 물어도 이 스킬이에요. MCP `App list`, `App get`, `deployment_diagnosis` 같은 도구만으로 진단을 대신하지 않아요.
- 방금 실패한 배포 id 가 있으면 `axhub deploy status <deployment-id> --json` 으로 terminal failure 와 시작·종료 시각을 확인하고, 앱 식별자가 있으면 그 시각으로 계산한 시간창을 붙여 `axhub deploy logs --app <앱> --since <시작> --until <종료> --json --limit 100` 으로 그 시간대 앱 로그를 읽어요. deployment id 를 positional 로 넘겨도 CLI 는 앱 단위 로그를 돌려주므로, 시간창 없이 읽은 로그를 그 배포의 로그라고 말하지 않아요. raw 로그는 사용자에게 그대로 붙이지 않고 원인군만 요약해요.
- 빌드 실패에는 `deploy status --json` 최상위와 `deploy diagnose` 응답 service 에 `build_log_tail`(백엔드가 secret 마스킹·절단을 끝낸 빌드 로그 끝부분)이 실릴 수 있어요. 이 필드는 Visibility 예외를 따라 사용자에게 보여주고, 원인 요약의 근거로 써요. 필드가 없으면(구 CLI·기능 이전 배포·비-빌드 실패) 기존처럼 원인군 요약만 해요.
- 앱 단위 현재 상태는 공개 CLI `axhub deploy diagnose` 로 받아요. CLI 원본 출력은 redact 가 안 돼 있어서 스킬이 직접 가려요.
- CLI 표면이 없으면 `진단을 못 했어요` 로 끝내요.
- 사용자에게는 raw id, exit code, JSON, stderr, pod signal, log line 을 그대로 보여주지 않고 여섯 가지 결과 중 하나로 요약해요.
- 해결 행동이 필요하면 `deploy` 또는 `clarity` 로 이어질 자연어 다음 행동만 안내해요. 재배포·롤백·로그 원문 조회를 이 스킬 안에서 실행하지 않아요.
- 사용자에게 보이는 진행 문구와 Bash/tool call 제목은 한국어로만 써요. 실제 호출 제목은 `CLI 표면 확인`, `실패 배포 상태 확인`, `실패 배포 로그 확인`, `현재 라이브 상태 확인` 처럼 짧은 한국어 명사구로 고정해요.
- 여러 CLI 확인을 한 tool 에 묶으면 제목은 `CLI 표면 확인` 으로 써요. 실패 배포 status+logs 를 한 tool 에 묶으면 제목은 `실패 배포 상태·로그 확인` 으로 써요. 중간 요약도 `CLI 확인 완료`, `실패 배포 확인 완료`, `현재 라이브 상태 확인` 처럼 한국어로만 남겨요.
- Bash/명령 tool 을 호출할 때 description/title/summary 필드는 반드시 위 고정 문구 중 하나로 직접 채워요. 도구가 자동으로 제목을 만들도록 비워두지 말고, 실행 전에 `axhubing CLI 확인` 같은 자동 생성 제목이 보이면 같은 명령이라도 `CLI 표면 확인` 으로 제목을 고쳐서 호출해요.
- `axhubing`, `axhubed`, `diagnosing`, `checking` 처럼 제품명이나 영어 동사를 붙인 도구 제목을 쓰지 않아요. 예를 들어 `axhubed CLI 확인` 대신 항상 `CLI 표면 확인` 을 써요. 도구 제목에는 제품명 `axhub` 자체도 넣지 않아요.
- Claude Code UI 가 Bash 본문의 `axhub` 문자열을 보고 `axhubing` 같은 자동 제목을 만들 수 있어요. 진단용 Bash 본문에서는 바이너리 이름도 나눠서 만들어요: `CLI_NAME="ax""hub"`, `CLI_BIN="$(command -v "$CLI_NAME" || true)"`, `"$CLI_BIN" deploy ...`. Bash 본문 안에는 소문자 `axhub` 연속 문자열, `AXHUB_BIN` 같은 변수명, command line 첫 단어의 bare `axhub` 를 쓰지 않아요.
- 사용자에게 보이는 문장에서는 영어 진행 문장을 쓰지 않아요. `Read-only` 도 쓰지 말고 `읽기 전용` 이라고 써요. 명령 이름(`axhub deploy status`, `status/logs/diagnose`)은 필요할 때만 짧게 허용해요.
- 중간 요약과 최종 메시지에서 raw category/stage/code 이름을 그대로 쓰지 않아요. `configuration`, `auth`, `build`, `infrastructure`, `timeout`, `resolve`, `backend_unimplemented`, `commit_not_found` 같은 값은 사용자에게 숨기고, "설정 쪽", "권한 쪽", "빌드 단계", "배포 환경", "시간 초과", "배포할 버전 찾기" 같은 말로 바꿔요. 단, `reason.message` 가 한국어 안내 문구면(빌드 실패 코드들은 백엔드가 사용자용 문구로 내려줘요) 그 문구는 그대로 전달해도 돼요.
- 사용자가 배포 id 를 직접 줬어도 최종 메시지에서 id 전체나 앞부분을 다시 쓰지 않아요. `실패 배포(96728617)` 처럼 일부만 보여주는 것도 금지예요. 항상 "방금 실패한 배포" 또는 "이 실패한 배포" 라고 말해요.
- `healthy: true`, `healthy=false`, `applicable=false`, `services[]`, `reason.category` 같은 raw 필드명·불리언 표현을 사용자에게 쓰지 않아요. "현재 라이브 롤아웃은 정상이에요", "진단 대상이 아니에요" 처럼 사람 말로 바꿔요.

## 라우팅 경계

| 요청 | 담당 |
|---|---|
| 배포 실패 원인 진단, 해결 후보 요약 | diagnosis |
| 새 배포, 재배포, 배포 성공 검증 | deploy |
| 배포 상태 확인, 로그 보기, 롤백, 운영 명령 실행 | clarity |
| 설치 상태·환경 진단(doctor), 첫 셋업·로그인 | onboarding |
| CLI·플러그인 업데이트 | update |
| 기존 앱 화면·페이지·대시보드·엔드포인트 코드 생성 | development |

경계가 섞이면 진단은 여기서 읽기 전용으로 끝내고, 실행이 필요한 단계는 담당 스킬로 넘겨요. 예를 들어 "왜 실패했는지 보고 다시 배포해줘" 는 먼저 원인만 요약하고, 재배포는 `deploy` 의 preview-confirm 흐름으로 다시 시작해야 해요. "설치 상태 진단해줘"(배포가 아니라 설치·환경 점검)는 `onboarding` 으로 넘겨요.

Claude Desktop QA처럼 사용자가 한 문장 안에 "진단하고 복구까지 해줘" 를 넣어도 diagnosis 단계는 bounded 해야 해요. 실패 원인과 현재 라이브 상태를 확인한 뒤 "복구는 배포 흐름으로 이어갈게요" 라고 말하고 이 스킬의 진단 루프는 종료해요. 이후 복구 배포를 실제로 시작했다면 `deploy verify` 한 번의 terminal 결과까지만 확인하고, running 이 길어지면 "복구 배포 확인은 계속 볼게요"가 아니라 배포 skill/status 흐름으로 넘겨요. `현재 라이브 상태 재확인 반복`, `미확인 evidence 파일 내용 점검` 같은 무기한 감시 루프를 만들지 않아요.

## 사용자 결과 카테고리

최종 메시지는 반드시 아래 여섯 가지 중 하나로 시작해요.

| 카테고리 | 의미 | 다음 행동 |
|---|---|---|
| `정상이에요` | 현재 라이브 롤아웃이 건강해요. | 방금 배포 결과가 궁금하면 상태·로그를 `clarity` 로 이어가요. |
| `진단 대상이 아니에요` | 진단할 라이브 롤아웃이 없어요 (정적 앱·배포 이력 없음·롤아웃 없음). | 실패로 볼 근거도 없어요. 첫 배포는 `deploy` 로 안내해요. |
| `해결 후보가 있어요` | 라이브 롤아웃이 건강하지 않고 원인 후보·해결 순서가 있어요. | 사용자가 고르면 담당 흐름으로 넘겨요. |
| `대상을 못 찾았어요` | 앱이나 배포 대상을 특정하지 못했어요. | 앱 이름이나 단서를 요청해요. |
| `로그인/권한이 필요해요` | 인증·권한 문제로 진단을 끝내지 못했어요. | 로그인·권한 전환은 onboarding/clarity 로 넘겨요. |
| `진단을 못 했어요` | CLI 진단 표면을 사용할 수 없어요. | CLI 업데이트나 나중 재시도를 안내해요. |

`정상이에요` 인데 사용자가 "방금 실패"를 물었으면, 현재 라이브가 정상이라는 걸 말한 뒤 "방금 배포 자체 결과를 보려면 '배포 상태 확인해줘'·'로그 보여줘'라고 말하면 돼요" 로 이어요 — 라이브 진단만으로 과거 배포 실패를 단정하지 않아요. 실패 배포 id 없이 "혹시 실패 원인 있으면"처럼 물었고 현재 라이브가 정상이면 `정상이에요` 로 끝내고, 재배포나 추가 로그 조회를 실행하지 않아요.

## Visibility 규칙

사용자에게 보여주기 전에 [references/output-contract.md](references/output-contract.md) 의 Visibility 규칙을 그대로 따라요 — raw 값은 숨기고 원인군과 다음 행동만 전달해요. 유일한 예외는 같은 문서의 `build_log_tail` 규칙이에요.

## 헤드리스 판정

`claude -p`·CI·`$CLAUDE_NON_INTERACTIVE`·TTY 없음이면 헤드리스예요. 헤드리스에선 앱 이름을 되묻지 않고 단서로만 좁히고, 못 좁히면 `대상을 못 찾았어요` 로 끝내요. CLI 호출 자체는 헤드리스에서도 동작해요.

## Workflow

1. **대상 추론.** 다음 단서만 써서 앱을 좁혀요.
   - 현재 디렉터리의 `axhub.yaml`
   - 같은 대화에서 방금 언급된 앱 이름이나 배포 문맥
   - 공개 표면으로 안전하게 조회 가능한 최근 앱·배포 목록

   하나로 좁혀지면 계속 진행해요. 대화형에서만 앱 이름을 한 번 물을 수 있어요. 헤드리스에선 묻지 않아요.

2. **표면 선택 — CLI 전용.**
   - 이 세션에 `deployment_diagnosis` 같은 MCP 도구가 callable 이어도 사용하지 않아요.
   - CLI 표면을 확인해요:

     ```bash
     CLI_NAME="ax""hub"
     CLI_BIN="$(command -v "$CLI_NAME" || true)"
     if [ -z "$CLI_BIN" ]; then
       echo "진단을 못 했어요"   # 설치 안내는 onboarding 소관
       exit 0
     fi
     "$CLI_BIN" deploy status --help
     "$CLI_BIN" deploy logs --help
     "$CLI_BIN" deploy diagnose --help   # 표면 존재 확인
     ```

     이 확인을 tool 로 실행할 때 제목은 `CLI 표면 확인` 으로 써요. help 확인까지 같은 tool 에 묶어도 제목은 그대로예요.
     Bash/명령 tool description/title/summary 도 정확히 `CLI 표면 확인` 으로 설정해요. 비워두거나 `axhubing CLI 확인`, `axhubed CLI 확인`, `Checking CLI` 로 자동 생성되게 두지 않아요.
     Bash 본문도 위 예시처럼 `CLI_NAME`/`CLI_BIN` 을 써요. bare `axhub deploy ...` 로 help 를 호출하지 않고, 변수명에도 `axhub` 를 넣지 않아요.

     help 가 있으면 그 help 인자만 써서 실행해요. help 에 없는 플래그·positional 은 만들지 않아요.

     방금 실패한 배포 id 가 있으면 먼저 status 를 읽고, status 의 시작·종료 시각으로 로그 시간창을 계산해요. `--app` 은 앱 slug/id/name 을 알고 있을 때만 붙여요.

     ```bash
     "$CLI_BIN" deploy status <deployment-id> --app <앱> --json
     "$CLI_BIN" deploy logs --app <앱> --since <시작> --until <종료> --json --limit 100
     ```

     `deploy logs` 는 앱 단위 로그예요 — 시간창으로 좁혀도 요약은 "그 배포 시간대의 앱 로그"로 정직하게 말해요.

     각 tool 제목은 `실패 배포 상태 확인`, `실패 배포 로그 확인` 으로 써요. 두 명령을 같은 tool 에 묶으면 제목은 `실패 배포 상태·로그 확인` 으로 써요.
     Bash/명령 tool description/title/summary 도 같은 고정 문구로 설정해요.

     그 다음 앱 단위 현재 라이브 롤아웃 진단을 읽어요. 이 명령은 **positional 앱 인자 하나**를 받고, `--json` 은 전역 플래그예요. deployment-id 타깃은 없어요.

     ```bash
     "$CLI_BIN" --json deploy diagnose <앱>
     ```

     이 tool 제목은 `현재 라이브 상태 확인` 으로 써요.
     Bash/명령 tool description/title/summary 도 정확히 `현재 라이브 상태 확인` 으로 설정해요.

   - CLI 표면이 없으면 `진단을 못 했어요` 로 끝내요.

3. **결과 분류.** 실패 배포가 있으면 deployment status/logs 를 먼저 원인군으로 접고, 앱 진단은 현재 라이브 상태로 따로 접어요. 앱 진단 필드는 `applicable`, `healthy`, `services[].healthy`, `services[].reason{stage,code,category,message}`, `services[].build_log_tail`(있으면), `signals[]` 를 봐요. 실패 배포 status 응답에서는 최상위 `build_log_tail` 도 봐요.

   CLI 경로의 exit code 계약 (검증된 두 값을 우선으로 봐요):

   | exit | 신호 | 처리 |
   |---|---|---|
   | `0` | `/data` 반환 (healthy=false 라도) | **도메인 결과 — fallback 금지.** 아래 카테고리 매핑으로 분류해요. |
   | `7` | `error.subcode = backend_unimplemented` | 백엔드 진단 표면 없음. `진단을 못 했어요`. |
   | 비0 | auth 만료·권한 (auth subcode) | `로그인/권한이 필요해요` |
   | 비0 | 앱 못 찾음 (404 / not found) | `대상을 못 찾았어요` |

   **핵심:** 실패 후보가 있는 정상 진단도 exit 0 이에요. "실행이 비0 이면 실패"로 오해하지 말아요 — 표면 자체 문제(exit 7·명령 없음)만 진단 불가로 다뤄요. `deploy status` 의 terminal `failed` 는 진단 성공 신호예요.

4. **카테고리 매핑.** CLI 결과를 사용자 카테고리로 접어요.

   | 조건 | 카테고리 |
   |---|---|
   | `applicable=false` | `진단 대상이 아니에요` |
   | `applicable=true`, `healthy=true` | `정상이에요` |
   | `applicable=true`, `healthy=false`, service `reason` 있음 | `해결 후보가 있어요` |
   | `applicable=true`, `healthy=false`, `reason` 없음 (일시·진행 중·읽기 실패) | `해결 후보가 있어요` (일시적이거나 진행 중일 수 있다고 안내) |
   | 앱 못 찾음 | `대상을 못 찾았어요` |
   | auth 만료·권한 | `로그인/권한이 필요해요` |
   | CLI 없음/backend_unimplemented | `진단을 못 했어요` |

   `해결 후보가 있어요` 의 원인군은 service `reason.category` 를 사람 말로 옮겨요 (확정이 아니라 "의심"으로). 실제 category 값은 다섯 가지예요:
   - `auth` → "인증·권한 쪽 문제로 보여요 (레지스트리·git 접근)"
   - `configuration` → "설정 쪽이 의심돼요 (이미지 이름·환경변수·컨테이너 설정)"
   - `build` → "빌드 단계가 의심돼요" — `build_log_tail` 이나 한국어 `reason.message` 가 있으면 "네이티브 모듈 빌드 도구가 이미지에 없어요"처럼 구체적 원인으로 좁혀 말하고, tail 발췌를 코드블록으로 이어 붙여요
   - `infrastructure` → "인프라·배포 환경 쪽이 의심돼요"
   - `timeout` → "제한 시간 안에 안정화되지 않았어요 (시작 시간·readiness 확인)"
   - category 는 열린 문자열이라 모르는 값이 올 수 있어요. 그러면 그 값을 그대로 노출하지 말고 "원인을 좁히는 중이에요" 로만 안내해요.

5. **행동 안내.** 재배포·롤백·로그 원문 확인을 직접 실행하지 않아요. 필요하면 자연어 handoff 만 남겨요.
   - 재배포 후보: "다시 배포하려면 '다시 배포해줘' 라고 말하면 돼요."
   - 롤백 후보: "되돌리려면 '이전 버전으로 롤백해줘' 라고 말하면 돼요."
   - 로그·상태 후보: "상세 로그나 상태가 필요하면 '로그 보여줘'·'배포 상태 확인해줘' 라고 말하면 돼요."

## 최종 메시지 · 금지

최종 메시지는 [references/output-contract.md](references/output-contract.md) 의 여섯 가지 템플릿 중 하나로 시작하고(빌드 실패면 같은 문서의 `build_log_tail` 발췌 규칙 적용), 같은 문서의 금지 목록(재배포·롤백·mutation 실행 금지, 원인 단정 금지, raw 출력 노출 금지, `--json-schema` 임의 탐색 금지 — 표면은 `axhub deploy status`/`deploy logs`/`deploy diagnose` 뿐)을 그대로 지켜요.
