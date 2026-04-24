# 첫 고객 cold-outreach 이메일 템플릿 (Korean)

타겟: SaaS 도입 회사의 IT/개발 책임자 또는 CTO. 회사 안에 비-개발 직군이 자기 앱을 직접 배포하고 싶은 상황 (vibe coder 문화).

---

## 메일 제목 후보 (3안)

1. `axhub Claude Code 플러그인 — 비-개발 직군이 직접 자기 앱 배포하는 환경, 5명 무료 파일럿 모집`
2. `회사 vibe coder가 "deploy 어떻게 해" 물어보는 거 0회로 만드는 방법`
3. `[axhub × Claude Code] 한국어 자연어로 안전하게 deploy — 첫 고객 5팀 우선 적용`

→ 3안 추천 (서비스 + 가치 + scarcity 조합).

---

## 메일 본문 (5분 안에 읽힘)

```
안녕하세요, <CONTACT_NAME> 님.

[회사명] 의 vibe coder (마케팅·기획·디자인 등 비-개발 직군이지만 LLM으로 코드 짜고 직접 배포하는 분들) 들이 늘고 있다는 점 보고 연락드렸어요.

저희가 만든 axhub Claude Code 플러그인이 정확히 이 분들을 위한 도구입니다.

**한 줄 요약**: vibe coder가 "내 paydrop 앱 방금 푸시한 거 prod 에 올려" 라고 한국어 자연어로 말하면, Claude Code 가 자동으로 안전하게 배포까지 끝내주는 플러그인이에요. ax-hub-cli (저희 SaaS) wrapping.

---

지금 [회사명] 에서 일어나고 있을 가능성이 높은 일:

- vibe coder 가 deploy 명령 어떻게 쓰는지 매번 IT/개발팀에 물어봄 → 둘 다 시간 낭비
- 잘못된 환경에 deploy → "방금 거 prod 에 올라간 게 staging 빌드였어요?" 사고
- 토큰 만료 → vibe coder는 "왜 안 돼" 만 보이고 IT는 매번 같은 안내 반복
- "이거 로그 어디서 봐요?" 가 매주 N 번

**axhub 플러그인 을 깐 vibe coder 의 워크플로우**:

1. "내 앱 방금 푸시한 거 prod 에 배포해" (한국어 그대로)
2. Claude Code 가 5가지 정보 미리보기 카드로 보여줌 (앱/환경/브랜치/커밋/예상시간)
3. "네, 진행" 한 번 더 확인
4. 한국어로 "30초 경과, 빌드 중이에요" 같은 진행 안내
5. 1-3분 후 "배포 성공" + 라이브 URL

배포 실패해도 한국어 4단계 안내 (감정 + 원인 + 해결 + 버튼) 로 다음 행동을 vibe coder 가 자력으로 알 수 있어요. IT 안 부르고도 90% 케이스는 본인이 처리합니다.

---

**왜 안전한가** (회사 IT/보안 입장):

- 모든 destructive 명령 (deploy/update/login) 은 HMAC consent token 으로 보호 — vibe coder 의도 없이 LLM 단독으로 prod 변경 불가
- 토큰은 per-user 격리 (다른 사람 노트북에 복사 X), mode 0600
- helper 바이너리 cosign 키리스 서명 (sigstore OIDC) — supply chain 검증
- 옵션 telemetry default OFF — 회사 보안 정책 우선
- 한국어 보안 메시지 (예: "보안 검증 실패. 절대 강제 진행하지 마세요. IT 보안팀에 알려주세요.")

---

**제안**: [회사명] 의 vibe coder 5명 × 1주일 무료 파일럿.

- 우리 (jocoding-ax-partners): plugin 설치 + 5명 vibe coder onboarding + 1주일 SLA 4시간 지원
- [회사명] : axhub team 1개 생성 + 5 vibe coder 토큰 발급 + 주 1회 30분 retro
- 결과물: 5/5 vibe coder 가 1주일 안에 첫 배포 성공 + 이해도 점수 ≥4.0/5.0 → GO 결정 시 정식 도입 협의

파일럿 prep kit 전체 docs 공개:
https://github.com/jocoding-ax-partners/axhub/tree/main/docs/pilot

다음 주 30분 미팅 어떠신가요? 화면 공유로 vibe coder 가 실제로 쓰는 모습 보여드릴 수 있어요.

감사합니다,
<SENDER_NAME>
jocoding-ax-partners
<EMAIL> | https://jocodingax.ai
```

---

## 제목·본문 변형 노트

- **이미 axhub 도입 회사**: "axhub 신규 기능 — Claude Code 플러그인 으로 vibe coder onboarding 5분으로 단축" 으로 hook 변경
- **개발자 1인 스타트업**: 본인이 vibe coder 인 경우 → "5명 무료 파일럿" 대신 "본인이 직접 30분 try" 안내
- **대기업 IT 부서장**: 보안 섹션을 위로 끌어올리고, "회사 보안 정책 100% 호환 (telemetry default OFF, cosign required, per-user HMAC)" 강조

## CTA 변형

- 기본 (위): 30분 미팅
- Self-serve: "https://github.com/jocoding-ax-partners/axhub 에서 본인 직접 try (소요 5분)"
- 회피형: "관심 없으시면 답장 주실 필요 없어요. 다음에 언제든."

---

## 후속 follow-up (3-day no-reply)

```
안녕하세요 <CONTACT_NAME> 님,

지난주 보낸 axhub Claude Code 플러그인 파일럿 제안 메일 보셨는지 궁금합니다. 바쁘셔서 놓치셨을 수 있어요.

핵심만 다시:
- vibe coder 5명 × 1주일 무료 파일럿
- 우리가 onboarding + SLA 4시간 지원
- 1주일 후 GO/NO-GO 같이 결정

링크: https://github.com/jocoding-ax-partners/axhub/blob/main/docs/pilot/README.md

회사 사정상 어렵다면 그냥 답장 X 라고 알려주세요. 더 spam 안 보내요.

감사합니다,
<SENDER_NAME>
```

---

## 절대 하지 말 것 (anti-pattern)

- ❌ "AI", "혁신", "차세대" 같은 buzzword 남발
- ❌ vibe coder 를 폄하하는 표현 ("코딩 못하는 사람도 이제 deploy 가능!")
- ❌ "free for first 100 customers!" 같은 인위적 scarcity
- ❌ 길이 1500자 초과
- ❌ HTML/이미지 첨부 (스팸 분류기 트리거 + 모바일 가독성 X)

대신:
- ✅ 회사가 지금 겪고 있을 specific pain point 부터 시작
- ✅ vibe coder 를 동료로 존중 ("LLM 으로 코드 짜고 직접 배포하는 분들")
- ✅ 우리가 받을 것 (파일럿 데이터) + 그쪽이 받을 것 (5명 onboarding) 명시
- ✅ "관심 없으면 답장 X" 옵션 명시 (저자세 X 단지 honest)
