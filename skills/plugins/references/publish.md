# plugins 게시 실행·게시 심사·상태·인증 복구

SKILL.md §6 의 publish 승인(새 `진행`)을 받은 뒤에만 §1 을 실행해요. 플러그인 버전 심사는 "게시 심사", 앱 visibility 심사는 "공개범위 변경 심사"라고만 불러요. 모든 명령은 CLI 0.50.0 이상(SKILL.md CLI 가드)을 전제로 해요.

## 1. 실행

승인 후 UUID v4 Idempotency-Key 하나를 고정하고 preview와 같은 payload로 실행해요. 처음 올리는 버전은 업로드와 게시 심사 신청을 `--submit` 으로 한 번에 해요.

```bash
axhub plugin publish "<path>" \
  --tenant "<tenant-UUID>" \
  --idempotency-key "<UUID-v4>" \
  --attest-distribution-rights \
  --execute \
  --submit \
  --json
```

- 기존 App version은 preview와 같은 `--app "<slug-or-UUID>"`·`--release-version "<new-semver>"`을 붙여요. `--tenant` 는 신규·기존 App 모두 tenant UUID예요.
- retry는 같은 key·payload로만 해요. 네트워크 timeout은 같은 key로 한 번 재개하고, §3 상태 확인 전에는 새 mutation을 만들지 않아요.
- 이미 업로드한 버전(`upload_complete`)은 publish를 다시 실행하지 않고 게시 심사 신청만 해요. 사용자가 그 신청을 승인한 경우에만 실행해요(publish 승인 문구에 이미 포함돼 있으면 다시 묻지 않아요).

```bash
axhub plugin submit \
  --tenant "<tenant-UUID>" \
  --app "<slug-or-UUID>" \
  --release-version "<semver>" \
  --execute \
  --json
```

## 2. 성공 판정

- `data.status=review_submitted` + `installable=false`: 업로드와 게시 심사 신청이 끝났어요. app_id·app_slug·version_id·version·replayed·review_request_id를 보고해요. `already_submitted=true` 면 `이미 신청돼 있어요`라고 말해요. 아직 아무도 설치할 수 없고, 워크스페이스 관리자가 Console Review에서 승인해야 제공돼요. `결과 알림은 없어요 — axhub plugin manage status 로 확인하세요`를 함께 말해요.
- `data.status=upload_complete` + `installable=false`: 업로드만 끝났고 아직 아무에게도 보이지 않아요. next_action `submit_plugin_version_for_review` 의 `command`(위 submit)로 게시 심사를 신청해요.
- `--submit` 부분 실패: exit 가 0이 아니어도 `error.details.upload` 가 있고 `submitted=false` 면 업로드는 끝났어요. publish를 다시 실행하지 말고 `next_action.command` 의 submit만 다시 실행해요.
- submit 이 exit 9·`plugin_version_not_ready` 면 업로드 처리 중이에요. 같은 submit 에 `--wait` 를 붙여 한 번 다시 실행해요(`--wait` 는 업로드 처리만 기다려요). exit 10·`plugin_submit_wait_timeout` 이면 `submitted=false` 와 출력된 재실행 명령을 그대로 알려주고 멈춰요. exit 7·`plugin_ingest_failed` 면 고친 뒤 새 SemVer로 다시 게시해요.
- publish 가 exit 7·`plugin_rejected`/`plugin_failed` 면 installable=false와 next_action을 보여주고 새 immutable version을 요구해요.
- `upload_complete`·`review_submitted` 는 설치 가능이 아니에요. App Console에서 관리하고 Console Review에서 승인돼 §3 상태가 `servable` 이 되기 전에는 release 완료라고 하지 않아요.

## 3. 상태 확인

```bash
axhub plugin manage status \
  --tenant "<tenant-UUID>" \
  --app "<slug-or-UUID>" \
  --release-version "<semver>" \
  --json
```

`--release-version` 을 빼면 가장 최근에 업로드한 버전을 봐요. `data.state` 하나와 `headline`·`next_action`·`axes` 를 읽고, 사용자에게는 `headline`(아래 표의 안내)과 다음 행동만 말해요. 현재 제공 중인 다른 버전이 있으면 따로 알려요.

| 상태 `state` | 안내 | 다음 행동 `next_action.action` |
|---|---|---|
| `takedown` | 보안 문제로 회수됐어요. 새 다운로드는 막혔고, 이미 설치한 곳은 직접 제거해야 해요. | `none` |
| `delisted` | 새로 설치할 수 없어요. 이미 설치한 곳은 그대로 남아요. | `publish_new_version` |
| `apply_failed` | 승인됐지만 아직 배포에 반영되지 않았어요. axhub 운영팀이 처리하고 있어요. | `contact_support` |
| `visibility_limited` | 앱 공개범위 때문에 플러그인을 설치할 수 없어요. 앱 관리자가 공개범위 변경 심사를 신청해야 해요. | `request_visibility_review` |
| `servable` | 팀에서 설치할 수 있어요. | `install_plugin` |
| `applying` | 승인됐고 곧 설치할 수 있어요. | `wait_for_publish` |
| `ingest_failed` | 업로드 처리에 실패했어요. 고친 뒤 새 버전을 올리세요. | `fix_and_publish_new_version` |
| `gate_blocked_secret` | 패키지에서 비밀정보가 발견돼 게시 심사를 요청할 수 없어요. 지우고 새 버전을 올리고, 이미 노출된 키는 교체하세요. | `fix_and_publish_new_version` |
| `gate_blocked` | 자동 검수에서 탈락했어요. 검수 결과를 확인하고, 문제가 없다고 판단되면 콘솔에서 사유를 적어 게시 심사를 요청할 수 있어요. | `review_gate_findings` |
| `rejected` | 반려됐어요. 반려 사유와 시각을 보여주고, 고친 새 버전을 올리게 해요. | `fix_and_publish_new_version` |
| `closed` | 게시 심사 신청이 종료됐어요. 종료 사유를 보여주고, 다시 신청할 수 있어요. | `submit_plugin_version_for_review` |
| `gate_slow` | 자동 검수가 평소보다 오래 걸려요. 24시간 안에 끝나지 않으면 신청이 닫히고 다시 신청할 수 있어요. | `wait_for_review` |
| `gate_running` | 패키지를 자동으로 검사하고 있어요. 보통 몇 분 안에 끝나요. | `wait_for_review` |
| `admin_pending` | 워크스페이스 관리자가 승인하면 제공돼요. 결과 알림은 없어요. | `wait_for_review` |
| `other_version_in_review` | 같은 앱의 다른 버전이 게시 심사 중이에요. 앱당 한 번에 한 버전만 심사해요. | `wait_for_review` |
| `uploading` | 업로드한 패키지를 처리하고 있어요. | `wait_for_ingest` |
| `not_submitted` | 업로드는 끝났지만 아직 아무에게도 보이지 않아요. 게시 심사를 신청하세요. | `submit_plugin_version_for_review` |

다음 행동 규칙:

- `install_plugin`: SKILL.md §3 설치 절차(위험 요약·preview·승인)로 가요. 다른 상태에서는 설치 가능하다고 말하지 않아요.
- `submit_plugin_version_for_review`: §1 submit 이에요. 게시 심사 신청을 승인받은 뒤에만 실행해요. `rejected` 의 `alternative` 로 같은 버전을 다시 신청할 때는 사용자가 고른 경우에만 `--reason "<메모>"` 를 붙여 실행해요.
- `publish_new_version`·`fix_and_publish_new_version`: 고친 새 SemVer로 SKILL.md §5 preview부터 다시 시작해요.
- `review_gate_findings`·`request_visibility_review`: `surface=app_console` 의 `url` 을 사람에게 넘겨요. CLI로 대신 진행하지 않아요. 공개범위 변경 심사는 앱 관리자만 신청할 수 있어요.
- `contact_support`: review_request_id를 담아 `axhub feedback -m` 이나 콘솔 도움말로 문의할 수 있다고 안내해요. 사용자가 원할 때만 보내요.
- `none`: `url` 의 회수 안내 문서만 알려요.
- `wait_for_ingest`·`wait_for_publish`, 그리고 `gate_running`: 같은 명령에 `--watch --watch-timeout 5m` 을 붙여 한 번만 다시 봐요. exit 10·`plugin_status_watch_timeout` 이면 이어 볼 명령만 남기고 멈춰요. 같은 명령을 반복 호출하지 않아요.
- 사람 결정을 기다리는 `admin_pending`·`other_version_in_review`·`gate_slow`: watch 하지 않고 나중에 볼 status 명령만 알려줘요.

## 4. 인증 복구

게시 전에 PAT를 미리 발급하지 않아요. 로그인한 상태로 §1 을 실행하면 CLI가 게시용 `plugins:read` + `plugins:write` scoped PAT를 알아서 준비·갱신하고 token을 화면에 내지 않아요. publish가 exit 4·`plugin_pat_required` 로 끝나면 `error.details.next_action.action` 에 맞는 것 하나만 해요. 다른 action이면 PAT를 짐작해 발급하지 않고 CLI 안내를 그대로 보여주고 멈춰요.

- `login`: 로그인이 없어요. 사용자에게 `axhub auth login` 을 직접 실행해 브라우저에서 로그인해 달라고 하고, 끝났다고 답하면 같은 Idempotency-Key·payload로 원래 명령을 다시 실행해요. 이때는 PAT를 발급하지 않아요.
- `issue_plugin_pat`: CLI가 게시용 PAT를 자동으로 준비하지 못했어요(이전 CLI이거나 준비 실패). scoped PAT 발급은 새 자격증명을 만드는 일이라 사용자에게 한 번 확인받은 뒤 `next_action.command` 를 고치지 않고 그대로 실행해요. 보통 아래 명령이고, `--use` 는 PAT를 keychain에 저장하고 활성 자격증명으로 쓰며 token을 화면에 내지 않아요.

```bash
axhub auth pat issue --scopes plugins:read,plugins:write --tenant-id "<tenant-UUID>" --use
```

- 발급이 끝나면 같은 Idempotency-Key·payload로 원래 명령을 다시 실행해요. `next_action.command` 가 비어 있을 때만 tenant UUID를 `axhub auth status --json` 의 `tenants[].tenant_id` 에서 얻어 위 명령을 써요.
- 사람이 repo 밖에 직접 만들어 둔 PAT 파일이 있다고 말한 경우에만 `--api-key-file "<path>"` 를 붙여요. agent가 token 값을 화면에 내거나 파일로 만들지 않아요.
- 404·403이나 `plugin_tenant_uuid_required` 면 `--tenant` 를 게시할 tenant UUID(`issue_plugin_pat` 로 발급했다면 그 PAT의 tenant UUID)로 맞춰 다시 실행해요.
- `issue_plugin_pat` 로 PAT를 활성화했다면 게시를 마치고 배포 같은 다른 axhub 작업으로 돌아갈 때 `axhub auth pat unset` 으로 이 PAT를 활성 자격증명에서 내려요.

## 5. host marketplace 연결

CLI 설치 대신 host의 plugin marketplace로 연결해 달라는 요청이면 쓰는 경로예요. 30일짜리 `marketplace:read` PAT를 git credential helper에 저장하는 일이라 한 번 확인받고 실행해요.

```bash
axhub --tenant "<tenant>" git setup --marketplace --json
```

- 출력의 `check_command`(`GIT_TERMINAL_PROMPT=0 git ls-remote <marketplace_url>`)로 비대화형 연결을 확인해요. 실패하면 `remote:` 줄의 안내를 그대로 보여주고 멈춰요.
- 확인되면 `add_commands` 에서 해당 host 명령 하나를 실행하거나 안내하고, host의 plugin 목록에서 설치한 뒤 재시작하도록 안내해요.
- `marketplace_url` 에 token을 넣거나 URL을 고쳐 쓰지 않아요.
