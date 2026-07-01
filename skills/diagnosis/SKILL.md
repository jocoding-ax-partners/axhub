---
name: diagnosis
description: 'diagnosis: "배포 실패 원인 진단해줘", "왜 배포가 죽었어", "diagnose deployment failure"처럼 axhub 배포 실패 원인과 해결 후보를 읽기 전용으로 알고 싶을 때만 사용해요. 결과는 사용자 카테고리로 요약하고 재배포·롤백은 직접 실행하지 않아요. 배포 실행/검증=deploy, 상태·로그·롤백·운영 명령=clarity, 업데이트=update, 앱 코드 생성=development 로 양보해요.'
examples:
  - utterance: "배포 실패 원인 진단해줘"
    intent: "diagnose deployment failure cause"
  - utterance: "이 앱 배포 실패 진단해줘"
    intent: "diagnose app deployment failure cause"
  - utterance: "diagnose deployment failure"
    intent: "diagnose deployment failure cause"
allows-dependency-execution: false
model: sonnet
---

# axhub diagnosis (배포 실패 원인 진단)

배포 실패 원인을 읽기 전용으로 진단하는 스킬이에요. 사용자가 실패 원인이나 해결 후보를 명시적으로 물을 때만 들어오고, 배포 실행·재배포·롤백은 절대 직접 실행하지 않아요.

진단 결과는 앱의 **현재 라이브 롤아웃 상태**예요 — 특정 과거 배포 한 건의 사후 부검이 아니에요. 그래서 방금 배포가 실패했어도 옛 버전이 아직 서빙 중이거나 자동 롤백됐으면 "현재 라이브는 정상"으로 나올 수 있어요. 이 한계를 사용자에게 정직하게 전달해요.

## 핵심 책임

- **MCP `deployment_diagnosis` 를 1순위로 써요.** 이 세션에 이미 callable 한 도구로 노출돼 있으면 그걸 먼저 호출해요 — 응답이 이미 redact·용량 제한·signal untrusted 처리돼 있어 vibe coder 에게 더 안전해요. (MCP 설치·등록·서버 실행은 하지 않아요.)
- MCP 가 없으면 공개 CLI `axhub deploy diagnose` (앱 단위) 로 같은 진단을 받아요. CLI 원본 출력은 redact 가 안 돼 있어서 스킬이 직접 가려요.
- 둘 다 없으면 `진단을 못 했어요` 로 끝내요.
- 사용자에게는 raw id, exit code, JSON, stderr, pod signal, log line 을 그대로 보여주지 않고 여섯 가지 결과 중 하나로 요약해요.
- 해결 행동이 필요하면 `deploy` 또는 `clarity` 로 이어질 자연어 다음 행동만 안내해요. 재배포·롤백·로그 원문 조회를 이 스킬 안에서 실행하지 않아요.

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

## 사용자 결과 카테고리

최종 메시지는 반드시 아래 여섯 가지 중 하나로 시작해요.

| 카테고리 | 의미 | 다음 행동 |
|---|---|---|
| `정상이에요` | 현재 라이브 롤아웃이 건강해요. | 방금 배포 결과가 궁금하면 상태·로그를 `clarity` 로 이어가요. |
| `진단 대상이 아니에요` | 진단할 라이브 롤아웃이 없어요 (정적 앱·배포 이력 없음·롤아웃 없음). | 실패로 볼 근거도 없어요. 첫 배포는 `deploy` 로 안내해요. |
| `해결 후보가 있어요` | 라이브 롤아웃이 건강하지 않고 원인 후보·해결 순서가 있어요. | 사용자가 고르면 담당 흐름으로 넘겨요. |
| `대상을 못 찾았어요` | 앱이나 배포 대상을 특정하지 못했어요. | 앱 이름이나 단서를 요청해요. |
| `로그인/권한이 필요해요` | 인증·권한 문제로 진단을 끝내지 못했어요. | 로그인·권한 전환은 onboarding/clarity 로 넘겨요. |
| `진단을 못 했어요` | MCP 도구도 CLI 진단 표면도 사용할 수 없어요. | CLI 업데이트나 나중 재시도를 안내해요. |

`정상이에요` 인데 사용자가 "방금 실패"를 물었으면, 현재 라이브가 정상이라는 걸 말한 뒤 "방금 배포 자체 결과를 보려면 '배포 상태 확인해줘'·'로그 보여줘'라고 말하면 돼요" 로 이어요 — 라이브 진단만으로 과거 배포 실패를 단정하지 않아요.

## Visibility 규칙

사용자 chat 에 절대 그대로 노출하지 않는 값:

- raw app id, deployment id, release id, trace id
- exit code, 내부 에러 코드·subcode, raw JSON, raw stderr
- pod name, signal name, container reason, stack trace, log line 원문
- MCP transport 오류 세부정보, tool schema, 내부 분기 판정

MCP `deployment_diagnosis` 응답은 backend·서버에서 이미 redact·용량 제한·signal untrusted 처리가 끝나 있어요. CLI `axhub deploy diagnose` 원본은 reason·signal text 를 그대로 찍으니 (`stage`·`code`·`category`·`message`·signal `text`) 스킬이 직접 가려요. 어느 경로든 사용자에게는 원인군과 다음 행동만 말해요. 예: "환경 설정 쪽이 가장 의심돼요. 먼저 설정값을 확인하고, 맞으면 같은 배포를 다시 시도하면 돼요."

## 헤드리스 판정

`claude -p`·CI·`$CLAUDE_NON_INTERACTIVE`·TTY 없음이면 헤드리스예요. 헤드리스에선 앱 이름을 되묻지 않고 단서로만 좁히고, 못 좁히면 `대상을 못 찾았어요` 로 끝내요. MCP·CLI 호출 자체는 헤드리스에서도 동작해요.

## Workflow

1. **대상 추론.** 다음 단서만 써서 앱을 좁혀요.
   - 현재 디렉터리의 `axhub.yaml`
   - 같은 대화에서 방금 언급된 앱 이름이나 배포 문맥
   - 공개 표면으로 안전하게 조회 가능한 최근 앱·배포 목록

   하나로 좁혀지면 계속 진행해요. 대화형에서만 앱 이름을 한 번 물을 수 있어요. 헤드리스에선 묻지 않아요.

2. **표면 선택 — MCP 우선.**
   - 이 세션에 `deployment_diagnosis` 도구가 callable 이면 그걸 1순위로 호출해요. 입력은 앱 식별자(`app_id`) 하나예요. 응답은 이미 안전 처리돼 있어 그대로 분류로 넘어가요.
   - callable 하지 않으면 CLI 로 내려가요:

     ```bash
     if ! command -v axhub >/dev/null 2>&1; then
       echo "진단을 못 했어요"   # 설치 안내는 onboarding 소관
       exit 0
     fi
     axhub deploy diagnose --help   # 표면 존재 확인
     ```

     help 가 있으면 그 help 인자만 써서 실행해요. 이 명령은 **positional 앱 인자 하나**를 받고, `--json` 은 전역 플래그예요. deployment-id 타깃은 없어요 (앱 단위 현재 롤아웃 진단). help 에 없는 플래그·positional 은 만들지 않아요.

     ```bash
     axhub --json deploy diagnose <앱>
     ```

   - MCP 도 CLI 도 없으면 `진단을 못 했어요` 로 끝내요.

3. **결과 분류.** MCP 응답이든 CLI 출력이든 같은 필드로 접어요: `applicable`, `healthy`, `services[].healthy`, `services[].reason{stage,code,category,message}`, `signals[]`.

   CLI 경로의 exit code 계약 (검증된 두 값을 우선으로 봐요):

   | exit | 신호 | 처리 |
   |---|---|---|
   | `0` | `/data` 반환 (healthy=false 라도) | **도메인 결과 — fallback 금지.** 아래 카테고리 매핑으로 분류해요. |
   | `7` | `error.subcode = backend_unimplemented` | 백엔드 진단 표면 없음. MCP 도 없으면 `진단을 못 했어요`. |
   | 비0 | auth 만료·권한 (auth subcode) | `로그인/권한이 필요해요` |
   | 비0 | 앱 못 찾음 (404 / not found) | `대상을 못 찾았어요` |

   **핵심:** 실패 후보가 있는 정상 진단도 exit 0 이에요. "실행이 비0 이면 실패"로 오해하지 말아요 — 표면 자체 문제(exit 7·명령 없음)만 진단 불가로 다뤄요.

4. **카테고리 매핑.** MCP·CLI 공통이에요.

   | 조건 | 카테고리 |
   |---|---|
   | `applicable=false` | `진단 대상이 아니에요` |
   | `applicable=true`, `healthy=true` | `정상이에요` |
   | `applicable=true`, `healthy=false`, service `reason` 있음 | `해결 후보가 있어요` |
   | `applicable=true`, `healthy=false`, `reason` 없음 (일시·진행 중·읽기 실패) | `해결 후보가 있어요` (일시적이거나 진행 중일 수 있다고 안내) |
   | 앱 못 찾음 | `대상을 못 찾았어요` |
   | auth 만료·권한 | `로그인/권한이 필요해요` |
   | MCP 미연결 + CLI 없음/backend_unimplemented | `진단을 못 했어요` |

   `해결 후보가 있어요` 의 원인군은 service `reason.category` 를 사람 말로 옮겨요 (확정이 아니라 "의심"으로). 실제 category 값은 다섯 가지예요:
   - `auth` → "인증·권한 쪽 문제로 보여요 (레지스트리·git 접근)"
   - `configuration` → "설정 쪽이 의심돼요 (이미지 이름·환경변수·컨테이너 설정)"
   - `build` → "빌드 단계가 의심돼요"
   - `infrastructure` → "인프라·배포 환경 쪽이 의심돼요"
   - `timeout` → "제한 시간 안에 안정화되지 않았어요 (시작 시간·readiness 확인)"
   - category 는 열린 문자열이라 모르는 값이 올 수 있어요. 그러면 그 값을 그대로 노출하지 말고 "원인을 좁히는 중이에요" 로만 안내해요.

5. **행동 안내.** 재배포·롤백·로그 원문 확인을 직접 실행하지 않아요. 필요하면 자연어 handoff 만 남겨요.
   - 재배포 후보: "다시 배포하려면 '다시 배포해줘' 라고 말하면 돼요."
   - 롤백 후보: "되돌리려면 '이전 버전으로 롤백해줘' 라고 말하면 돼요."
   - 로그·상태 후보: "상세 로그나 상태가 필요하면 '로그 보여줘'·'배포 상태 확인해줘' 라고 말하면 돼요."

## 최종 메시지 템플릿

- `정상이에요. 지금 라이브 롤아웃은 건강해요. 방금 배포 결과가 궁금하면 "배포 상태 확인해줘"라고 말하면 돼요.`
- `진단 대상이 아니에요. 지금 진단할 라이브 롤아웃이 없어요(아직 배포 전이거나 정적 앱). 첫 배포는 "배포해줘"라고 말하면 돼요.`
- `해결 후보가 있어요. 인프라·배포 환경 쪽이 가장 의심돼요. 먼저 설정을 확인하고, 맞으면 다시 배포하면 돼요.`
- `대상을 못 찾았어요. 어떤 앱이나 배포를 봐야 하는지 단서가 한 가지 더 필요해요.`
- `로그인/권한이 필요해요. axhub 권한을 확인한 뒤 다시 진단하면 돼요.`
- `진단을 못 했어요. 지금은 연결된 진단 도구도, CLI 진단 표면도 없어요.`

## 금지

- 재배포, 롤백, 앱 삭제, 환경변수 변경 같은 mutation 을 실행하지 않아요.
- 실패 후보를 확정 원인처럼 말하지 않아요. evidence 가 약하면 "가장 의심돼요" 처럼 후보로 표현해요.
- 라이브 진단이 정상이라고 과거 배포가 성공했다고 단정하지 않아요. 현재 롤아웃 상태와 배포 한 건의 결과는 다를 수 있어요.
- raw 출력이나 내부 id·exit code 를 사용자에게 보여주지 않아요.
- MCP 를 설치하거나 설정하라고 내부 절차를 만들지 않아요.
- `clarity` 처럼 전체 `--json-schema` 트리를 탐색해 임의 명령을 찾아 실행하지 않아요. 이 스킬의 표면은 MCP `deployment_diagnosis` 와 CLI `axhub deploy diagnose` 둘뿐이에요.
