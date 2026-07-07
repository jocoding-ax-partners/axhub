# CLI 핸드오프: Windows 셸 간 credential/PATH parity

> 대상 repo: ax-hub-cli (`axhub` 바이너리). 이 문서는 axhub plugin repo 에서 진단한 근본원인을 CLI 팀에 넘기는 스펙·repro 예요. 코드 수정은 CLI repo 몫이에요.

## 배경
Windows 첫 온보딩에서 반복 실패가 관측됐어요 (2026-07). `axhub` 가 설치됐는데 실행 세션 PATH 에 없었고, 이어서 로그인은 성공했는데 `axhub auth status` 가 "미로그인" 으로 나와 device code 를 여러 번 소진했어요. 대부분 이미 있는 올바른 흐름(Git Bash·`repair-path`·단일 폴링 로그인)을 PowerShell 로 우회해서 생긴 문제였어요.

plugin 쪽 mitigation 은 AP-13 (Windows = Git Bash 전용, PowerShell 금지)으로 처리했어요. 하지만 셸에 따라 결과가 갈리는 최심부 원인은 CLI 에 있어요.

## 가설 (미검증)
`axhub` 가 credential/config 디렉토리를 `$HOME` 기준으로 해석하는데, Git Bash 는 `HOME` 을 세팅하고 native PowerShell 은 안 해서, Git Bash 에서 로그인한 자격증명을 PowerShell 세션이 못 찾는 것으로 보여요. **단, plugin repo 에서는 CLI 내부 해석 경로를 검증할 수 없어요** — Windows 에서 `HOME` 이 `USERPROFILE` 과 같을 때도 많아 실제 동작은 repro 로 확인해야 해요.

## Repro (Windows 필요)
1. Git Bash: `axhub auth login` → 성공 확인.
2. PowerShell(같은 사용자): `axhub auth status --json` → "미로그인" false-negative 가 재현되는지 봐요.
3. 두 셸에서 `echo $HOME`(Git Bash) vs `echo $env:HOME` / `echo $env:USERPROFILE`(PowerShell) 값을 비교해요.
4. credential/config 파일 실제 위치를 확인해요 (`~/.axhub/`? `%USERPROFILE%\.axhub`? OS credential store?).
5. 판정: 해석이 `HOME` 전용인지, `USERPROFILE` 도 보는지.

## 스펙 (repro 가 HOME 전용을 확인하면)
1. Windows 에서 credential/config 디렉토리를 `USERPROFILE`/`LOCALAPPDATA` 우선 + `$HOME` 폴백으로 해석해요. Git Bash 와 PowerShell 이 같은 store 를 봐서 `auth status` 가 셸 무관하게 동작해요.
2. `axhub plugin-support repair-path` 가 PATH 를 고친 뒤, 실행 중 세션은 그 변경을 못 읽으니 "새 터미널을 열어야 반영" 신호를 명시적으로 emit 해요 (에이전트·사용자가 같은 세션에서 헛싸움 안 하게).

## repro 결과가 "HOME 전용 아님" 이면
스펙은 폐기해요. plugin AP-13 (Git Bash 전용)만으로 충분하고 CLI 변경은 필요 없어요.

## 다음 액션
- 이 문서를 ax-hub-cli 이슈로 등록해요.
- repro 결과를 이슈에 붙여 판정을 확정해요.
