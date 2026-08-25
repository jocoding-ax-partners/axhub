---
name: publishing
description: 'axhub tenant marketplace plugin·skill 게시 전용 skill. "플러그인 올려줘", "plugin publish", "스킬 퍼블리시해줘", "이 SKILL.md를 마켓에 게시", "여러 스킬을 하나의 플러그인으로 올려", "새 플러그인 버전 배포"에 사용해요. 단일·multi-skill 모두 preview → 권리 확인 → 명시 승인 → publish → gate terminal 순서예요. 앱 배포는 deploy, 기존 앱 연결은 import, CLI·plugin 업데이트는 update로 양보하며 외부 registry에는 쓰지 않아요. axhub 맥락 없으면 tenant marketplace 대상인지 먼저 확인하고 첫 응답은 "플러그인 게시 입력과 CLI 기능을 확인할게요."예요.'
examples:
  - utterance: "이 SKILL.md를 우리 회사 마켓에 올려줘"
    intent: "publish one standalone skill to the tenant marketplace"
  - utterance: "이 폴더의 스킬 여러 개를 하나의 플러그인으로 퍼블리시해줘"
    intent: "publish a multi-skill plugin directory"
  - utterance: "house-tools 새 버전 올려줘"
    intent: "publish a new immutable version of an existing plugin"
  - utterance: "앱 배포해줘"
    intent: "yield to deploy"
allows-dependency-execution: false
model: sonnet
---

# Publishing — plugin·skill marketplace 게시

플러그인 게시 입력과 CLI 기능을 확인할게요.

axhub 맥락(대화·현재 연결·직전 작업)이 없으면 axhub tenant marketplace 대상인지 한 번 확인해요. headless에서는 묻지 않고 멈춰요.

> **CLI 경로(AP-17):** bare `axhub` 실패는 미설치가 아니에요. `command -v axhub` → `~/.axhub/bin-path` → `~/.axhub/bin/axhub`(.exe) 순서로 찾고, 발견한 절대경로로 `plugin-support repair-path --json`을 실행해요. 같은 session의 이후 모든 auth·publish 명령은 반환된 `bin_path`(없으면 발견한 절대경로)로 실행해요. 모두 없을 때만 onboarding으로 보내요. Windows 명령은 Git Bash 전용이에요.

> **기능 게이트:** `axhub plugin publish --help`가 성공해야 진행해요. unknown command면 update로 보내고 멈춰요. `cargo run`, `target/debug/axhub`, custom ZIP·curl·직접 API로 우회하지 않아요.

## 1. 입력과 release 단위

canonical absolute path를 쓰고 symlink면 중단해요.

- standalone은 파일명이 `SKILL.md`여야 해요.
- directory는 root에 `.claude-plugin/plugin.json`이 필요하고 manifest `name`·`version`이 정본이에요.
- 함께 versioning·승인·설치·회수할 skill은 `skills/<slug>/SKILL.md`로 한 directory에 둬요. 독립 lifecycle이면 별도 plugin으로 나눠요.

Directory의 모든 regular file이 artifact에 들어가요. Preview file 목록에 `.env`, private key(`*.pem`·`*.key`·`id_rsa*`), token·secret·credential 파일, build output이 하나라도 보이면 execute를 금지해요. 사용자가 repo 밖으로 이동하거나 정리한 뒤 새 preview를 요구하고 파일을 대신 삭제하지 않아요.

새 등록인지 기존 plugin의 새 version인지 확정해요. 기존이면 console·이전 결과의 정확한 `plugin_id`와 새 SemVer를 써요. 같은 `(name, version)`은 immutable이라 재사용하지 않아요. 제3자 재배포의 새 등록에만 `--third-party`를 써요.

## 2. Offline preview

항상 `--execute` 없이 `--json` preview부터 실행해요.

```bash
# standalone 신규
axhub plugin publish "<SKILL.md>" --name "<name>" --release-version "<semver>" --json

# standalone 신규 제3자 재배포
axhub plugin publish "<SKILL.md>" --name "<name>" --release-version "<semver>" --third-party --json

# standalone 기존 version — name/display-name/third-party 금지
axhub plugin publish "<SKILL.md>" --plugin-id "<UUID>" --release-version "<new-semver>" --json

# directory 신규
axhub plugin publish "<plugin-root>" --json

# directory 신규 제3자 재배포
axhub plugin publish "<plugin-root>" --third-party --json

# directory 기존 version
axhub plugin publish "<plugin-root>" --plugin-id "<UUID>" --json
```

`data.mode=preview`, `data.network=false`, `data.auth=false`를 확인해요. name·version·files·SHA-256·size와 `risk_inventory`(hooks·mcp_servers·bins·dynamic_context·scripts)를 보여줘요. 위험 항목은 tenant admin 승인이 필요할 수 있다고 말해요.

## 3. Tenant와 PAT

`--tenant`에는 console·이전 publish 결과에서 확인한 tenant UUID를 명시해요. publish PAT만으로 `/me`를 조회할 수 없으므로 `axhub auth status`로 tenant를 추론하지 않아요. UUID가 없으면 사용자에게 확인을 요청하고 멈춰요.

실행에는 OAuth가 아닌 전용 publish PAT file이 필요해요. 사용자가 이 PAT를 정확히 `plugins:read` + `plugins:write` 두 scope로 발급했다고 명시 확인해야 해요. `axhub auth pat whoami --json`은 scope를 보여주지 않으므로 증거로 쓰지 않고, 기존 active/broad PAT도 대신 쓰지 않아요. Repo 밖 canonical path만 `--api-key-file`에 넘기며 내용을 읽거나 token 값을 채팅·로그·URL·명령 인자에 복사하지 않아요. Server의 `insufficient_scope`가 최종 판정이며 나오면 올바른 scoped PAT를 요청하고 중단해요.

## 4. 권리 확인과 승인

이 preview 승인 하나가 axhub 진입 확인도 겸해요. 별도 질문과 이중으로 묻지 않고 승인을 조용히 건너뛰지 않아요.

네이티브 선택 UI 가 있으면 그걸로 묻고, 없으면 같은 확인을 명시 텍스트 승인 1회로 받고, 둘 다 불가한 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.

승인 질문은 `이 artifact를 배포할 권리가 있음을 확인하고, 미리보기대로 axhub tenant marketplace에 게시할까요? 진행 또는 취소 로 답해 주세요.`예요. Preview 뒤 사용자가 새로 입력한 `진행`만 권리 attestation과 execute를 함께 승인해요. 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.

## 5. Execute

승인 후 UUID v4 Idempotency-Key 하나를 생성해 session에 고정하고 preview와 같은 path·identity·version으로 실행해요.

```bash
axhub plugin publish "<path>" \
  --tenant "<tenant-UUID>" \
  --api-key-file "<publish-PAT-file>" \
  --idempotency-key "<UUID-v4>" \
  --attest-distribution-rights \
  --execute \
  --json
```

standalone 신규는 같은 `--name`·`--release-version`, standalone 기존은 `--plugin-id`·`--release-version`만 유지해요. directory 기존은 `--plugin-id`만 추가하고 manifest identity를 덮어쓰지 않아요. 신규 제3자 preview에 쓴 `--third-party`는 execute에도 그대로 유지하고 `--plugin-id`와 함께 쓰지 않아요. 동일 payload retry는 같은 Idempotency-Key만 재사용해요.

## 6. Terminal truth

- `status=ok`, `data.status=published`, `data.installable=true`: 완료예요. plugin_id·version_id·version을 보고해요.
- `status=ok`, `data.status=awaiting`, `data.installable=false`: admin 승인 대기예요. 설치 가능이라고 말하지 않아요.
- `status=error`, `error.subcode=plugin_rejected|plugin_failed`: `data`의 IDs·version·next_action을 보고하고 수정한 새 immutable version을 요구해요.
- timeout: 같은 Idempotency-Key·동일 payload로 한 번 재개해요. 다시 timeout이면 console에서 name·version을 확인하기 전 새 mutation을 만들지 않아요.

## NEVER

- NEVER preview·권리 확인·명시 승인 없이 execute/attest하지 않아요.
- NEVER PAT 값이나 suspicious artifact file을 업로드하지 않아요.
- NEVER plugin_id를 추측하거나 `published` 외 상태를 installable로 선언하지 않아요.
- NEVER 여러 loose SKILL.md를 각각 보내지 않아요. 함께 배포할 단위는 한 plugin directory로 묶어요.
