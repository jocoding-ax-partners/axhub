# permission-manifest-probe fixture

⚠️ **DO NOT COPY `plugin.probe.example.json` TO `.claude-plugin/plugin.json`** ⚠️

이 디렉토리는 ADR-0011 §검증된 가정 #4 (Step 0.7 Option A 매니페스트 spec probe) 의
manual binary verification 용 fixture 예요. 실제 production 매니페스트가 아니에요.

## Probe 목적

`.claude-plugin/plugin.json` 의 `permissions` 필드에 wildcard / placeholder 패턴이
Claude Code 에서 인식되는지를 1 회 manual 트리거로 binary 검증해요.

- 패턴 A1 (glob wildcard): `Bash(*/axhub-helpers preflight*)` — **production 채택 금지**.
  PATH 우선순위 hijack 으로 attacker 가 `$PATH` 첫 디렉토리에 동명 binary 를 심으면
  권한 prompt 우회 가능. ADR-0011 §검증된 가정 #4 의 인식 ✓ 결과에서도 wildcard
  variant 는 production 매니페스트에 들어가지 않아요.
- 패턴 A2 (plugin root placeholder): `Bash(${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight*)` —
  spec probe 결과 인식 ✓ 시 production 매니페스트 후보. plugin root 안의 binary 만 매칭.

## 파일 이름이 `plugin.json` 이 아닌 이유

`plugin.json` 이라는 파일 이름 자체가 Claude Code plugin loader 의 자동 로드 트리거에
해당해요 (위치는 `.claude-plugin/plugin.json` 이 production path 지만, 파일 이름이
같으면 maintainer copy-paste 사고로 production 매니페스트에 wildcard 가 유입될 수
있어요). `plugin.probe.example.json` 으로 rename 해서 loader 가 자동 로드 안 하고,
fixture 의 example-only 의도를 명시했어요.

## 사용 방법

manual probe 진행 시:

1. fixture 를 minimal test plugin 디렉토리로 임시 복사 (production `.claude-plugin/` 외 경로)
2. Claude Code 에서 해당 임시 plugin 로드 후 `axhub-helpers preflight` 실행 시도
3. 권한 prompt 없이 통과하면 wildcard 인식 ✓ (A2 패턴만 안전 후보)
4. prompt 뜨면 인식 ✗ — B 단독 path 채택
5. 결과를 `docs/adr/0011-skill-preflight-permission-fallback.md` §검증된 가정 #4 에 기록

## 관련

- [ADR-0011](../../../docs/adr/0011-skill-preflight-permission-fallback.md) §검증된 가정 #4
- [docs/HOOKS.md §3.6](../../../docs/HOOKS.md) — SKILL preprocessing `!command` fail-open contract
