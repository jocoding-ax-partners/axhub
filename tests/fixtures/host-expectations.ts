/**
 * host 별 기대값 fixture.
 *
 * 테스트가 잠그는 host 결합 문자열(명령 문자열, exact 문장, 금지 문자열)을
 * host 키 아래 테이블로 모아요. 지금은 Claude lane 하나만 있고, codex 파생
 * 번들 검증(후속 유닛)은 이 파일에서 HOST_EXPECTATIONS 의 타입을
 * `Record<"claude" | "codex", HostExpectations>` 로 넓히고 codex 열 값을
 * 채우는 것만으로 확장해요 — 테스트 로직 재작성 없이 열 추가만이 목표예요.
 *
 * 역방향(금지) 계열 — forbidden.hookBannerField(systemMessage) ·
 * forbidden.externalAutomationPlugin(oh-my-claudecode) — 은 값만 여기 두고,
 * `not.toContain` assert 자체는 각 테스트 본문에 명시적으로 남아 있어요.
 * 테이블 루프 뒤로 숨겨 사라지게 하지 않는 게 계약이에요.
 */

/**
 * hooks.json 의 SessionStart entry 순서와 1:1 인 wrapper 스크립트 파일명이에요.
 * hook-execution·smooth-behavior·plugin-bundle(2지점) 이 이 단일 목록을
 * 소비해요 — 파일명·순서 변경은 여기 한 곳에서만 해요.
 */
export const SESSION_WRAPPER_SCRIPTS = [
  "session-auto-update.sh",
  "session-restart-confirm.sh",
] as const;

export interface HostExpectations {
  /** host 표면 토큰 — 명령·env 변수·경로 조각 단위 */
  surface: {
    /** 플러그인 목록 확인의 유일한 허용 명령 */
    pluginListCommand: string;
    /** hook 스크립트가 참조하는 플러그인 루트 env 변수 이름 */
    hookPluginRootEnv: string;
    /** Windows backslash 경로를 slash 로 정규화하는 bash 치환 마커 */
    hookSlashNormalizeMarker: string;
    /** hooks.json 이 wrapper 스크립트로 위임하는 command 형태 */
    hookWrapperCommand: (script: string) => string;
  };
  /** 역방향(금지) 문자열 — assert 는 테스트 본문의 not.toContain 이 소유해요 */
  forbidden: {
    /** hook 출력에 등장하면 안 되는 사용자 배너 필드 */
    hookBannerField: string;
    /** bootstrap 이 언급·정리하면 안 되는 외부 자동화 플러그인 이름 */
    externalAutomationPlugin: string;
    /** 같은 외부 자동화 플러그인의 스킬 이름 */
    externalAutomationSkill: string;
  };
  /** README.md 의 host 결합 기대 문장 */
  readme: {
    desktopCliOnly: string;
  };
}

export const HOST_EXPECTATIONS: Record<"claude", HostExpectations> = {
  claude: {
    surface: {
      pluginListCommand: "claude plugin list",
      hookPluginRootEnv: "CLAUDE_PLUGIN_ROOT",
      hookSlashNormalizeMarker: "${CLAUDE_PLUGIN_ROOT//",
      hookWrapperCommand: (script: string): string => `bash "\${CLAUDE_PLUGIN_ROOT}/hooks/${script}"`,
    },
    forbidden: {
      hookBannerField: "systemMessage",
      externalAutomationPlugin: "oh-my-claudecode",
      externalAutomationSkill: "autopilot",
    },
    readme: {
      desktopCliOnly: "Claude Desktop 에 axhub App/MCP 도구가 같이 보여도 플러그인 스킬 흐름은 CLI-only",
    },
  },
  // codex 열 추가 지점: 위 타입을 Record<"claude" | "codex", HostExpectations> 로
  // 넓히고, codex 파생 번들의 기대값을 같은 키 체계로 채워요.
};
