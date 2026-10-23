# Scaffold placeholder 값

scaffold 는 서버 치환 없이 사용자 저장소에 먼저 push 해서, bootstrap 서버가 넣는 값과 똑같이 직접 넣어요. 값이 다르면 빌드는 멀쩡하고 로그인(`{{API_BASE}}/custom-domain-auth/start`)·데이터 연동만 조용히 죽어요.

## 작업공간

1.5단계 `axhub plugin-support tenant-resolve --json` 의 `needs_pick:true` 면 `candidates[]` 로 물어 고른 `slug` 를 써요. 아니면 `tenant` 가 보통 id 라서, `{{TENANT}}` 에는 `axhub auth status --json` `tenants[]` 에서 같은 `tenant_id` 행의 `tenant_slug` 를 넣어요. 그 행의 `tenant_kind` 가 `public` 이면 개인 작업공간, 그 밖은 회사 작업공간이에요. 둘 다 비면 작업공간을 추측하지 않고 다시 로그인해요.

## 값

| 토큰 | 회사 작업공간 | 개인 작업공간 |
|---|---|---|
| `{{APP_SLUG}}`·`{{APP_NAME}}` | 3단계 값 | 같음 |
| `{{TENANT}}` | 작업공간 slug | 같음 |
| `{{API_BASE}}` | CLI 가 쓰는 API 주소 (prod `https://api.axhub.ai`) | 같음 |
| `{{APP_SUBDOMAIN}}` | slug | 앱 생성 뒤 `subdomain` |
| `{{APP_ORIGIN}}` | `https://<slug>.<작업공간 slug>.axhub.ai` (서버 AppHost) | 앱 생성 뒤 `access_url` |

import 는 subdomain 옵션 없이 항상 slug 를 subdomain 으로 써서, subdomain 을 다르게 넣으면 실제 앱 주소와 어긋나요.

## 개인 작업공간: 앱을 만든 뒤 채우기

주소를 서버가 만들어서 5단계에서는 `{{APP_SUBDOMAIN}}`·`{{APP_ORIGIN}}` 만 남겨요. 8단계 import 가 앱을 만든 뒤 `axhub apps get <slug> --tenant <tenant> --json` 의 `subdomain`·`access_url` 로 두 토큰을 바꾸고(값이 없으면 추측하지 않고 멈춰요), `git -C "<target>" add -A` → `commit -q -m "Fill app address placeholders"` → `push -u origin main` 으로 올려요 — 이 push 가 다음 배포예요. `여기서 멈추기` 면 두 토큰이 남았다는 것을 마무리 안내에 한 줄로 남겨요.
