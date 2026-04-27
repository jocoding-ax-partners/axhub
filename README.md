# axhub — Claude Code 플러그인

> 바이브코더가 자연어로 axhub 앱을 안전하게 배포하고 관리하는 Claude Code 플러그인.

**상태**: M0 스캐폴드 (v0.1.0 개발 중). 풀 기능은 M1.5 GO/KILL gate 통과 후 ship.

---

## 한 줄 요약

axhub SaaS를 도입한 회사의 바이브코더 직원이 Claude Code에서 "내 paydrop 앱 배포해" 같은 자연어로 자기 앱을 prod에 안전하게 올리고 관리할 수 있게 하는 플러그인입니다. ax-hub-cli (v0.1.0+) wrapping.

## 무엇을 할 수 있는가 (계획 — M1+ 에서 단계적 ship)

- **자연어 deploy**: "paydrop 배포해" → AskUserQuestion preview card → exit 0 + status watch
- **자연어 status**: "방금 배포한 거 어떻게 됐어"
- **자연어 logs**: "왜 실패했어, 빌드 로그 보여줘"
- **자연어 apps/apis**: "내 앱 목록", "어떤 API 쓸 수 있어"
- **자동 복구**: exit 65 (token 만료) → 한국어 안내 + auth login flow
- **안전 가드**: HMAC consent token, 잘못된 앱/profile 자동 차단, dry-run NL trigger
- **슬래시 커맨드**: `/axhub:deploy`, `/axhub:status`, `/axhub:help` — escape hatch

## 빠른 시작 (vibe coder 용 — M1+ 에서 작동)

준비 사항:
- ax-hub-cli v0.1.0+ 설치 (`brew install jocoding-ax-partners/tap/axhub`)
- Claude Code 최신
- axhub SaaS 계정 + scope (회사 admin이 발급)

설치:

```bash
# 1. 마켓플레이스 등록
/plugin marketplace add jocoding-ax-partners/axhub

# 2. 플러그인 설치
/plugin install axhub@axhub

# 3. 첫 인증
"axhub 로그인해줘"  # 또는 /axhub:login
#  └─ 첫 CC 세션 시작 시 환경에 맞는 helper 바이너리 자동 다운로드 (v0.1.0 release)
#  └─ 자동 다운로드 비활성화: export AXHUB_SKIP_AUTODOWNLOAD=1 (수동 install.sh)

# 4. 첫 배포
"내 paydrop 앱 배포해"
```

상세 가이드: `docs/vibe-coder-quickstart.ko.md` (M0 후속에서 작성).

## 조직 관리자용 (B2B 도입 회사 IT/admin)

배포 정책, 권한 관리, 보안 설정: `docs/org-admin-rollout.ko.md` (M0 후속에서 작성).

## 문제 해결

흔한 에러 (token 만료, 동시 배포 차단, slug 모호 등) 한국어 가이드: `docs/troubleshooting.ko.md` (M0 후속에서 작성).

## Architecture (요약)

```
사용자 발화 ("paydrop 배포해")
        │
        ▼
Claude Code  →  axhub plugin
        │              ├── skills/* (NL 자동 트리거)
        │              ├── commands/* (슬래시)
        │              ├── hooks/* (PreToolUse HMAC consent)
        │              └── bin/axhub-helpers (TS/Bun: resolve + HMAC consent + classify + redact)
        │                       │
        ▼                       │
   Bash tool ────────────────────┘
        │
        ▼
   ax-hub-cli binary (v0.1.0+)
        │
        ▼
   https://hub-api.jocodingax.ai
```

**핵심 원칙**: 플러그인은 **얇은 routing/recovery layer**다. 비즈니스 로직은 모두 ax-hub-cli에 있고, 플러그인은 (1) 자연어 인텐트 → 명령어 매핑, (2) HMAC consent token으로 destructive op 보호, (3) exit code 기반 자동 복구만 담당한다. **Plugin은 MCP를 사용하지 않으며 항상 ax-hub-cli를 호출한다.**

상세 설계: `PLAN.md` (6 phases of review, 65 audit-tracked decisions).

## 라이선스

MIT — `LICENSE` 참조.
