# Contract: `axhub.yaml` 스키마 (정준 manifest)

> 사용자가 repo 에 commit 하는 배포 계약. backend `domain.AppManifest` 와 1:1. `apphub.yaml` 도 dual-read(전환기).

## 필드

| 경로 | 타입 | 기본 | 설명 |
|---|---|---|---|
| `version` | string | (없음=legacy) | 스키마 게이트 (`axhub/v1`). 미인식 값 → 경고만 |
| `name` | string | — | 앱 이름 |
| `runtime.port` | int(1-65535) | 감지값 | listen 포트 |
| `runtime.health_path` | string(`/…`) | — | health check 경로 |
| `build.strategy` | enum | `auto` | `auto`=빌드마다 재감지 / `pinned`=명령 고정 |
| `build.framework` | string | "" | override/hint (보통 자동감지) |
| `build.install`/`build`/`start` | string | "" | `pinned` 일 때 고정 명령 |
| `build.dockerfile` | string | "" | 있으면 우선 (ladder) |
| `build.deploy_method` | enum | `docker` | `docker` / `compose` |
| `build.compose_file` | string | "" | `deploy_method=compose` 일 때만 |
| `env.required[]` | EnvVar[] | [] | 필수 env (이름+scope) |
| `env.optional[]` | EnvVar[] | [] | 선택 env |
| `ci.commands[]` | string[] | [] | (w5 기존) 최대 10 |
| `ci.timeout` | int(1-600) | 120 | (w5 기존) 초 |

### EnvVar
| 필드 | 타입 | 기본 | 설명 |
|---|---|---|---|
| `name` | string(non-empty, 개행금지) | — | env 이름 (값 아님) |
| `scope` | enum | `runtime` | `build`(빌드 ARG/ENV) / `runtime`(컨테이너) / `both` |

## 빌드/배포 우선순위 ladder (FR-008)
① `axhub.yaml` 의 명시 override(`strategy=pinned` 명령, `build.dockerfile`, `deploy_method=compose`) → ② repo `Dockerfile` → ③ docker-compose → ④ Railpack 자동감지(`strategy=auto`)

## 불변식
- `version` 미설정 + unknown 필드 → 기존 parser `KnownFields(true)` 가 거부(단, 신규 필드는 모두 known 으로 등록).
- 값(secret)은 절대 이 파일에 안 들어가요 — 이름·scope 만.
- `strategy=auto` 면 `install/build/start` 는 무시(재감지). 단 `runtime.port`, `env`, `build.dockerfile`, `deploy_method=compose` 같은 명시 override 는 유지해요. `pinned` 이면 명령을 고정해요.

## 예시 (BYO-data Next.js)
```yaml
version: axhub/v1
name: paydrop
runtime: { port: 3000, health_path: /api/health }
build: { strategy: auto }
env:
  required:
    - { name: DATABASE_URL, scope: runtime }
    - { name: NEXT_PUBLIC_API_URL, scope: build }
```
