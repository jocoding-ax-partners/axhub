# Deploy User-Facing Language

deploy SKILL 이 로드하는 내부 reference 예요. 사용자에게 보이는 문구·tool 제목을 만들기 전에 이 규칙을 그대로 따라요.

Keep chat human and Korean. Do not echo raw ids, raw JSON, schema names, exit numbers, internal command names, or stderr unless `AXHUB_DEPLOY_VERBOSE=1`.

User-visible Bash/tool call titles must be Korean noun phrases only. Do not expose helper-shaped or English labels such as `manifesting`, `manifested`, `gitted`, `pushed`, `Push`, `resumed`, `bootstraped`, `deploy-prep`, `in-flight`, `dry-run`, `token-gate`, `execute`, `production`, `terminal success`, `grep pipe`, `gitignore`, `gitignoring`, `gitting`, `checking`, `Build passed`, `Working tree clean`, or `Not ignored`. Good examples: `배포 준비 확인`, `변경사항 확인`, `커밋 동기화 확인`, `원격 반영 확인`, `진행 중 배포 확인`, `배포 미리보기 확인`, `인증 상태 확인`, `배포 실행`, `배포 결과 확인`.

The same rule applies to chat prose, preview cards, and final tables. Use `운영` for the user-facing environment of a normal app and `스테이징` for a staging opt-in app (`staging_enabled=true`, AP-25), `진행 중 배포` for in-flight work, `미리보기` for dry-run, `인증 상태 확인` for token gate, `배포 실행` for execute, and `검증 성공` for terminal success. Command names may appear only when the user explicitly asks for technical evidence or when an error needs exact copy-paste recovery.

사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL 로만 써요. Markdown URL 링크 문법은 전부 금지예요. `[https://...](https://...)`, `[열기](https://...)`, `<https://...>` 처럼 URL 을 괄호나 label 로 감싸지 말고 `https://...` 그대로 보여줘요.

When a technical check fails or needs recovery, translate it before showing it. Say `원격 반영이 필요해요`, not `commit_not_found`; `재생성되는 빌드 파일은 정리했어요`, not `Not ignored` or `Working tree clean`; `원격 저장소 확인`, not `gitting` or `gitignore 확인`. Do not create English status snippets during build/lint/git cleanup.

Do not narrate approval state in English. Never write `User explicitly authorized`, `Proceeding`, `Push 성공`, or `Push failed`. Use `사용자가 배포와 원격 반영을 요청했으니 계속 진행해요`, `원격 반영 성공`, or `원격 반영 실패` instead.

When shell output is noisy, capture it in temp files or summarize after the command. Do not pipe important axhub commands through `grep`, `head`, or similar filters in a way that can change the command exit code or hide a long-running helper. If cosmetic filtering is truly needed, preserve and inspect the original command exit code first.
