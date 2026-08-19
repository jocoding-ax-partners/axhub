/**
 * host 별 기대값 fixture.
 *
 * 테스트가 잠그는 host 결합 문자열(명령 문자열, exact 문장, 금지 문자열)을
 * host 키 아래 테이블로 모아요. 지금은 Claude lane 하나만 있고, codex 파생
 * 번들 검증(후속 유닛)은 이 파일에서 HOST_EXPECTATIONS 의 타입을
 * `Record<"claude" | "codex", HostExpectations>` 로 넓히고 codex 열 값을
 * 채우는 것만으로 확장해요 — 테스트 로직 재작성 없이 열 추가만이 목표예요.
 *
 * 역방향(금지) 3계열 — forbidden.hookBannerField(systemMessage) ·
 * forbidden.pluginUpdateCommand(claude plugin update) ·
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
  "session-windows-contract.sh",
  "session-update-router-guard.sh",
  "session-restart-confirm.sh",
  "session-feedback-contract.sh",
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
    /** 스킬·훅이 자발적으로 실행하면 안 되는 플러그인 업데이트 명령 접두 */
    pluginUpdateCommand: string;
    /** bootstrap 이 언급·정리하면 안 되는 외부 자동화 플러그인 이름 */
    externalAutomationPlugin: string;
    /** 같은 외부 자동화 플러그인의 스킬 이름 */
    externalAutomationSkill: string;
    /** onboarding ready card 가 안내하면 안 되는 MCP 등록 명령 접두 */
    mcpRegisterCommand: string;
  };
  /** 플러그인 업데이트 뒤 재시작 안내 문안 계열 */
  restartNotice: {
    applySentence: string;
    exactSayRule: string;
    exactFinalRule: string;
    neverOtherNoticeRule: string;
  };
  /** update skill(SKILL.md + references) 의 host 결합 기대 문장 */
  updateSkill: {
    autopilotSlashRef: string;
    pluginCacheGuard: string;
    exactListOnlyOutput: string;
    permissionCardExactList: string;
    compoundListProbeInlineCode: string;
    listRedirectInlineCode: string;
    listRedirectGrepInlineCode: string;
    rewriteToBareListRule: string;
    readFullListInternally: string;
    singleListForVersion: string;
    listFailureFallback: string;
    duplicateListEntries: string;
    reListAfterSuccess: string;
    noUpdateForLowerDup: string;
    noCleanupUpdateWhenLatest: string;
    neverJudgeByFirstStale: string;
    noSlashFallbackAfterList: string;
    pluginOnlyFlow: string;
    marketplaceCmdInlineCode: string;
    marketplaceRefreshFirst: string;
    scopedUpdateDirect: string;
    noReprobeAfterBoundary: string;
    listGrepShortInlineCode: string;
    locationCheckTableRow: string;
    noManualCheckStamp: string;
  };
  /** hooks 계약 표면(hooks.json + wrapper/router 스크립트)의 기대 문장 */
  hooksContract: {
    visibleListExact: string;
    noReprobeContract: string;
    neverDesktopAppTools: string;
    compoundListProbePlain: string;
    listRedirectGrepPlain: string;
  };
  /** POLICY.md 의 host 결합 기대 문장 */
  policy: {
    noPreemptBySlash: string;
    noReprobeInAppFlow: string;
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
  /** onboarding skill 의 host 결합 기대 문장 */
  onboardingSkill: {
    neverMcpRegister: string;
  };
  /** bootstrap skill(SKILL.md + references) 의 host 결합 기대 문장 */
  bootstrapSkill: {
    nativeBadgeNoRepeat: string;
    desktopMetadataFolders: string;
    noTenantFileWrite: string;
    nameConfirmBeforeFinalize: string;
    desktopWatcherBan: string;
  };
  /** 승인 fallback 사다리·headless 정의(AP-12, 4스킬 공통) 의 계약 문장 */
  approvalGate: {
    /** bootstrap·deploy·import·scaffold 4사본 공통 fallback 사다리 문장 전문 */
    approvalFallbackSentence: string;
    /** headless 정의·subprocess 가드의 고정 host 병기 토큰 */
    headlessCodexExecMention: string;
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
      pluginUpdateCommand: "claude plugin update",
      externalAutomationPlugin: "oh-my-claudecode",
      externalAutomationSkill: "autopilot",
      mcpRegisterCommand: "claude mcp",
    },
    restartNotice: {
      applySentence: "Claude Code 를 재시작하면 새 버전이 적용돼요.",
      exactSayRule: "정확히 `받았어요. Claude Code 를 재시작하면 새 버전이 적용돼요.` 라고 말해요",
      exactFinalRule: "이 마지막 문장은 정확히 `받았어요. Claude Code 를 재시작하면 새 버전이 적용돼요.` 예요",
      neverOtherNoticeRule:
        "NEVER 플러그인 업데이트 성공 뒤 `받았어요. Claude Code 를 재시작하면 새 버전이 적용돼요.` 가 아닌 재시작 안내 문장을 만들지 말아요",
    },
    updateSkill: {
      autopilotSlashRef: "/oh-my-claudecode:autopilot",
      pluginCacheGuard:
        "Claude Desktop 에서는 `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 같은 플러그인 캐시 파일을 읽지 않아요",
      exactListOnlyOutput: "정확히 `claude plugin list` 만 실행한 출력",
      permissionCardExactList: "이 권한 카드의 Desktop-visible command 는 글자 하나도 더하지 말고 정확히 `claude plugin list` 예요",
      compoundListProbeInlineCode: "`command -v claude && claude plugin list`",
      listRedirectInlineCode: "`claude plugin list 2>&1`",
      listRedirectGrepInlineCode: "`claude plugin list 2>&1 | grep ...`",
      rewriteToBareListRule:
        "실행하려는 command 가 `claude plugin list 2>&1` 또는 `command -v claude && claude plugin list` 로 떠오르면 **반드시 `claude plugin list` 로 바꿔요.**",
      readFullListInternally: "출력이 길어도 전체 `claude plugin list` 결과를 도구 응답에서 내부적으로 읽고",
      singleListForVersion: "정확히 `claude plugin list` 한 번으로 `axhub@axhub` 의 현재 버전을 내부 변수 `<PLUGIN_VERSION>` 으로만 둬요",
      listFailureFallback: "`claude plugin list` 가 실패하거나 목록에서 못 찾으면 `<PLUGIN_VERSION>` 없이 CLI 업데이트 확인만 진행해요",
      duplicateListEntries: "`claude plugin list` 에 `axhub@axhub` 가 여러 번 나오면",
      reListAfterSuccess: "성공하면 `claude plugin list` 를 한 번 더 실행해",
      noUpdateForLowerDup: "이때 낮은 중복 scope 가 있어도 `claude plugin update` 를 실행하지 않아요",
      noCleanupUpdateWhenLatest: "최고 enabled 버전이 이미 최신이면, 낮은 중복 항목을 최신화하기 위한 `claude plugin update` 를 실행하지 않아요",
      neverJudgeByFirstStale: "NEVER `claude plugin list` 에서 처음 발견한 낡은 `axhub@axhub` 항목만 보고 업데이트 여부를 판단하지 말아요",
      noSlashFallbackAfterList: "`claude plugin list` 성공 뒤에는 수동 slash-command 폴백으로 내려가지 않아요",
      pluginOnlyFlow: "`플러그인만` 요청은 `claude plugin list` → `axhub update check --plugin-version <PLUGIN_VERSION> --json`",
      marketplaceCmdInlineCode: "`claude plugin marketplace update axhub`",
      marketplaceRefreshFirst:
        "Desktop Bash tool 로 정확히 `claude plugin marketplace update axhub` 를 먼저 실행해 marketplace cache 를 최신으로 만들어요",
      scopedUpdateDirect: "Desktop Bash tool 로 `claude plugin update axhub@axhub --scope <SCOPE>` 를 직접 실행해요",
      noReprobeAfterBoundary: "`command -v axhub`, `axhub --version`, `command -v claude`, `claude plugin list` 를 다시 실행하지 않아요",
      listGrepShortInlineCode: "`claude plugin list 2>&1 | grep`",
      locationCheckTableRow: "| 플러그인 설치 위치 확인 (정확히 `claude plugin list`) | `플러그인 설치 위치 확인` |",
      noManualCheckStamp: "수동 확인 기록은 Claude Desktop 경로에서 갱신하지 않아요",
    },
    hooksContract: {
      visibleListExact: "For plugin version lookup, the visible command is exactly claude plugin list; never claude plugin list 2>&1",
      noReprobeContract: "must not run command-v, axhub --version, claude plugin list, plugin-location checks, or version probes again",
      neverDesktopAppTools:
        "Never call Claude Desktop axhub App/MCP tools such as Deployment list (axhub), App get (axhub), or Tenant recent deployments (axhub)",
      compoundListProbePlain: "command -v claude && claude plugin list",
      listRedirectGrepPlain: "claude plugin list 2>&1 | grep",
    },
    policy: {
      noPreemptBySlash: "`/axhub:clarity`, `/oh-my-claudecode:autopilot` 를 먼저 실행하지 않아요",
      noReprobeInAppFlow:
        "이 앱 상태 흐름에 들어간 뒤에는 `command -v axhub && axhub --version`, `command -v claude && claude plugin list`, `claude plugin list 2>&1 | grep` 같은 설치·버전·플러그인 probe 를 다시 실행하지 않아요",
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
    onboardingSkill: {
      neverMcpRegister: "NEVER MCP 서버 등록(`claude mcp add`)이나 MCP OAuth 인증을 안내·실행하지 말아요",
    },
    bootstrapSkill: {
      nativeBadgeNoRepeat: "Claude Desktop 이 이미 `/axhub:bootstrap` native badge 를 보여줘도 chat 본문에서 반복하지 않아요",
      desktopMetadataFolders: "Claude Desktop may create `.omc/`",
      noTenantFileWrite: "Do not write `.axhub/state/tenant.json` from Claude Desktop",
      nameConfirmBeforeFinalize: "Do not finalize the name before one user-facing confirmation in Claude Desktop",
      desktopWatcherBan: "Claude Desktop Monitor, ScheduleWakeup, or any background watcher",
    },
    approvalGate: {
      approvalFallbackSentence:
        "네이티브 선택 UI 가 있으면 그걸로 묻고, 없으면 같은 확인을 명시 텍스트 승인 1회로 받고, 둘 다 불가한 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.",
      headlessCodexExecMention: "`claude -p`·`codex exec`",
      headlessDefinitionDeploy:
        "Headless means `claude -p`·`codex exec`, CI, `$CLAUDE_NON_INTERACTIVE`, no TTY, or no user-confirmation channel at all — neither native choice UI nor explicit text approval is available.",
      headlessDefinitionImport:
        "`claude -p`·`codex exec`, CI, `$CLAUDE_NON_INTERACTIVE`, TTY 없음, 네이티브 선택 UI 와 명시 텍스트 승인이 둘 다 불가한 상태(사용자 확인 채널 자체가 없음)는 headless 예요.",
    },
  },
  // codex 열 추가 지점: 위 타입을 Record<"claude" | "codex", HostExpectations> 로
  // 넓히고, codex 파생 번들의 기대값을 같은 키 체계로 채워요.
};
