---
name: diagnosis
description: '이 스킬은 사용자가 axhub 배포 실패 원인을 명시적으로 진단해 달라고 할 때만 사용해요. 활성화 예: "배포 실패 원인 진단해줘", "왜 배포가 죽었어", "이 앱 배포 실패 진단해줘", "방금 배포 왜 실패했어", "배포 에러 원인만 알려줘", "diagnose deployment failure", "why did deploy fail", 또는 배포 실패의 원인·해결 후보를 읽기 전용으로 보고 싶다는 의도. 경계: 배포 실행·재배포·검증은 deploy, 일반 상태·로그·롤백·운영 명령은 clarity, CLI·플러그인 최신화는 update, 앱 코드 생성은 development 가 맡아요. 이 스킬은 진단 결과를 5가지 사용자 카테고리로 요약하고 재배포·롤백을 직접 실행하지 않아요.'
examples:
  - utterance: "배포 실패 원인 진단해줘"
    intent: "diagnose deployment failure cause"
  - utterance: "왜 배포가 죽었어"
    intent: "diagnose deployment failure cause"
  - utterance: "이 앱 배포 실패 진단해줘"
    intent: "diagnose app deployment failure cause"
  - utterance: "방금 배포 왜 실패했어?"
    intent: "diagnose latest deployment failure cause"
  - utterance: "diagnose deployment failure"
    intent: "diagnose deployment failure cause"
allows-dependency-execution: false
model: sonnet
---

# axhub diagnosis (배포 실패 원인 진단)

배포 실패 원인을 읽기 전용으로 진단하는 스킬이에요. 사용자가 실패 원인이나 해결 후보를 명시적으로 물을 때만 들어오고, 배포 실행·재배포·롤백은 절대 직접 하지 않아요.

## 핵심 책임

- `axhub deploy diagnose` 공개 CLI 표면을 우선 사용해 배포 실패 원인과 해결 후보를 확인해요.
- MCP `deployment_diagnosis` 는 현재 세션에서 이미 호출 가능한 도구로 노출된 경우에만 보조 경로로 써요. MCP 설치·등록·서버 실행은 하지 않아요.
- 사용자에게는 raw id, exit code, JSON, stderr, pod signal, log line 을 그대로 보여주지 않고 다섯 가지 결과 중 하나로 요약해요.
- 해결 행동이 필요하면 `deploy` 또는 `clarity` 로 이어질 자연어 다음 행동만 안내해요. 이 스킬 안에서 재배포·롤백·로그 원문 조회를 실행하지 않아요.

## 라우팅 경계

| 요청 | 담당 |
|---|---|
| 배포 실패 원인 진단, 해결 후보 요약 | diagnosis |
| 새 배포, 재배포, 배포 성공 검증 | deploy |
| 배포 상태 확인, 로그 보기, 롤백, 운영 명령 실행 | clarity |
| CLI·플러그인 업데이트 | update |
| 기존 앱 화면·페이지·대시보드·엔드포인트 코드 생성 | development |
| 첫 셋업·CLI 설치·로그인 | onboarding |

경계가 섞이면 진단은 여기서 읽기 전용으로 끝내고, 실행이 필요한 단계는 담당 스킬로 넘겨요. 예를 들어 "왜 실패했는지 보고 다시 배포해줘" 는 먼저 원인만 요약하고, 재배포는 `deploy` 의 preview-confirm 흐름으로 다시 시작해야 해요.

## 사용자 결과 카테고리

최종 메시지는 반드시 아래 다섯 가지 중 하나로 시작해요.

| 카테고리 | 의미 | 다음 행동 |
|---|---|---|
| `정상이에요` | 진단 대상이 현재 건강하거나 실패 근거가 없어요. | 필요하면 상태 확인을 `clarity` 로 이어가요. |
| `해결 후보가 있어요` | 실패 원인 후보와 안전한 해결 순서가 있어요. | 사용자가 선택하면 담당 흐름으로 넘겨요. |
| `대상을 못 찾았어요` | 앱이나 배포 대상을 특정하지 못했어요. | 앱 이름이나 배포 단서를 요청해요. |
| `로그인/권한이 필요해요` | 인증·권한 문제로 진단을 끝내지 못했어요. | 로그인이나 권한 전환은 onboarding/clarity 로 넘겨요. |
| `진단을 못 했어요` | CLI 표면도 MCP 보조 경로도 사용할 수 없어요. | CLI 업데이트나 나중 재시도를 안내해요. |

## Visibility 규칙

사용자 chat 에 절대 그대로 노출하지 않는 값:

- raw app id, deployment id, release id, trace id
- exit code, 내부 에러 코드, raw JSON, raw stderr
- pod name, signal name, container reason, stack trace, log line 원문
- MCP transport 오류 세부정보, tool schema, 내부 fallback 판정

대신 사용자에게는 원인군과 다음 행동만 말해요. 예: "환경변수 쪽이 가장 의심돼요. 먼저 설정값을 확인하고, 맞으면 같은 배포를 다시 시도하면 돼요."

## Workflow

1. **CLI 가드.** `axhub` 가 없으면 진단을 실행하지 않고 `진단을 못 했어요` 로 끝내요. 설치 안내는 onboarding 소관이에요.

   ```bash
   if ! command -v axhub >/dev/null 2>&1; then
     echo "진단을 못 했어요"
     exit 0
   fi
   ```

2. **대상 추론.** 다음 단서만 사용해 앱이나 배포 대상을 좁혀요.
   - 현재 디렉터리의 `axhub.yaml`
   - 같은 대화에서 방금 언급된 앱 이름이나 배포 문맥
   - 공개 CLI 로 안전하게 조회 가능한 최근 앱·배포 목록

   대상이 하나로 좁혀지면 계속 진행해요. 대화형에서만 앱 이름을 한 번 물을 수 있어요. 헤드리스에서는 질문하지 않고 `대상을 못 찾았어요` 로 끝내요.

3. **CLI 기능 probe.** 먼저 공개 CLI 의 진단 명령이 있는지 확인해요.

   ```bash
   axhub deploy diagnose --help
   ```

   help 가 존재하면 그 help 에 나온 인자와 플래그만 써서 실행해요. `--json` 이 지원되면 기계 파싱용으로 붙이고, 지원되지 않으면 사람이 읽을 출력만 내부적으로 분류해요. help 에 없는 플래그나 positional 인자는 만들지 않아요.

4. **CLI 진단 실행.** 대상이 앱 단위인지 배포 단위인지는 help 에 맞춰 조립해요.

   ```bash
   axhub deploy diagnose <help에 나온 대상 인자> <help가 지원하는 --json 플래그>
   ```

   명령이 성공하거나 도메인 결과를 반환하면 그것이 최종 진단이에요. 앱 없음, 권한 없음, 진단 불가, 정상, 실패 후보 있음 같은 도메인 결과는 fallback 트리거가 아니에요.

5. **MCP 보조 경로.** fallback 은 오직 아래 경우에만 허용해요.
   - `axhub deploy diagnose` 명령이 없는 경우
   - 해당 명령의 help/실행이 tool missing, unsupported, transport failure 처럼 표면 자체 문제로 실패한 경우
   - 현재 세션에 이미 callable MCP 도구 `deployment_diagnosis` 가 노출된 경우

   MCP 가 없으면 설치하거나 설정하지 말고 `진단을 못 했어요` 로 끝내요. MCP 가 있더라도 결과는 같은 다섯 카테고리로만 요약해요.

6. **결과 분류.** CLI/MCP 원자료를 아래로 접어요.
   - healthy, no failure evidence → `정상이에요`
   - failure candidates, likely cause, remediation steps → `해결 후보가 있어요`
   - app/deployment not found, ambiguous target → `대상을 못 찾았어요`
   - auth expired, forbidden, scope missing → `로그인/권한이 필요해요`
   - missing diagnosis surface, unsupported CLI, transport unavailable → `진단을 못 했어요`

7. **행동 안내.** 재배포, 롤백, 로그 원문 확인을 직접 실행하지 않아요. 필요하면 자연어 handoff 만 남겨요.
   - 재배포 후보: "다시 배포하려면 '다시 배포해줘' 라고 말하면 돼요."
   - 롤백 후보: "되돌리려면 '이전 버전으로 롤백해줘' 라고 말하면 돼요."
   - 로그 확인 후보: "상세 로그가 필요하면 '로그 보여줘' 라고 말하면 돼요."

## 실패와 fallback 구분

| 상황 | 처리 |
|---|---|
| `axhub deploy diagnose` 가 없어요 | MCP 보조 경로를 시도할 수 있어요. |
| CLI transport/tooling 이 깨졌어요 | MCP 보조 경로를 시도할 수 있어요. |
| 앱을 못 찾았어요 | fallback 하지 않고 `대상을 못 찾았어요`. |
| 로그인이 만료됐어요 | fallback 하지 않고 `로그인/권한이 필요해요`. |
| 앱이 정상이에요 | fallback 하지 않고 `정상이에요`. |
| 실패 후보가 있어요 | fallback 하지 않고 `해결 후보가 있어요`. |
| MCP 도구가 없어요 | 설치하지 않고 `진단을 못 했어요`. |

## 최종 메시지 템플릿

- `정상이에요. 지금은 실패로 볼 근거가 없어요. 더 보려면 "배포 상태 확인해줘"라고 말하면 돼요.`
- `해결 후보가 있어요. 환경변수 설정이 가장 의심돼요. 먼저 설정을 확인하고, 맞으면 다시 배포하면 돼요.`
- `대상을 못 찾았어요. 어떤 앱이나 배포를 봐야 하는지 한 단서가 더 필요해요.`
- `로그인/권한이 필요해요. axhub 권한을 확인한 뒤 다시 진단하면 돼요.`
- `진단을 못 했어요. 현재 CLI 에 진단 표면이 없고 보조 진단 도구도 연결되어 있지 않아요.`

## 금지

- 재배포, 롤백, 앱 삭제, 환경변수 변경 같은 mutation 을 실행하지 않아요.
- 실패 후보를 확정 원인처럼 말하지 않아요. evidence 가 약하면 "가장 의심돼요" 처럼 후보로 표현해요.
- raw 출력이나 내부 id 를 보여주지 않아요.
- MCP 를 설치하거나 설정하라고 내부 절차를 만들지 않아요.
- `clarity` 처럼 전체 `--json-schema` 를 탐색해 임의 명령을 찾아 실행하지 않아요. 이 스킬의 CLI 표면은 `axhub deploy diagnose` 하나예요.
