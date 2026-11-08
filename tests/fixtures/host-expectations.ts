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
  /** deploy skill references 의 host 결합 기대 문장 */
  deploySkill: {
    desktopPermissionWatchFail: string;
  };
  /** clarity skill 의 host 결합 기대 문장 */
  claritySkill: {
    desktopAutolinkGuard: string;
    desktopAppToolExposure: string;
  };
  /** development skill 의 host 결합 기대 문장 */
  developmentSkill: {
    desktopCodeMode: string;
  };
  /** 승인 fallback 사다리·headless 정의(AP-12, 4스킬 공통) 의 계약 문장 */
  approvalGate: {
    /** import 승인 fallback 사다리 문장 전문 (bootstrap·scaffold 통합 뒤엔 import 만 써요) */
    approvalFallbackSentence: string;
    /** deploy Headless Contract 의 정의 문장 (3-lane 사다리 정합 재작성본) */
    headlessDefinitionDeploy: string;
    /** import Headless rule 의 정의 문장 (3-lane 사다리 정합 재작성본) */
    headlessDefinitionImport: string;
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
    deploySkill: {
      desktopPermissionWatchFail: "A Claude Desktop permission request for a long polling shell block is a failed watch UX",
    },
    claritySkill: {
      desktopAutolinkGuard: "device-flow URL 은 Claude Desktop 이 자동 링크로 바꾸지 못하도록 URL 부분만 inline code span 으로 써요",
      desktopAppToolExposure: "Claude Desktop 에 노출된 `axhub` App/MCP 도구 호출",
    },
    developmentSkill: {
      desktopCodeMode: "Claude Desktop Code 모드",
    },
    approvalGate: {
      approvalFallbackSentence:
        "네이티브 선택 UI 가 있으면 그걸로 묻고, 없으면 같은 확인을 명시 텍스트 승인 1회로 받고, 둘 다 불가한 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.",
      headlessDefinitionDeploy:
        "Headless means `claude -p`·`codex exec`, CI, `$CLAUDE_NON_INTERACTIVE`, no TTY, or no user-confirmation channel at all — neither native choice UI nor explicit text approval is available.",
      headlessDefinitionImport:
        "`claude -p`·`codex exec`, CI, `$CLAUDE_NON_INTERACTIVE`, TTY 없음, 네이티브 선택 UI 와 명시 텍스트 승인이 둘 다 불가한 상태(사용자 확인 채널 자체가 없음)는 headless 예요.",
    },
  },
  // codex 열 추가 지점: 위 타입을 Record<"claude" | "codex", HostExpectations> 로
  // 넓히고, codex 파생 번들의 기대값을 같은 키 체계로 채워요.
};
