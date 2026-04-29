import { runPreflight, type PreflightOutput } from "./preflight.ts";

export interface PromptRouteInput {
  prompt?: string;
}

export interface PromptRouteOutput {
  hookSpecificOutput?: {
    hookEventName: "UserPromptSubmit";
    additionalContext: string;
  };
}

export interface PromptRoutePreflight {
  output: PreflightOutput;
  exitCode: number;
}

export type PromptRouteIntent =
  | "apis"
  | "apps"
  | "auth"
  | "clarify"
  | "deploy"
  | "doctor"
  | "logs"
  | "recover"
  | "status"
  | "update"
  | "upgrade";

export interface PromptRouteDefinition {
  intent: PromptRouteIntent;
  skill: string;
  label: string;
  needsPreflight: boolean;
  patterns: RegExp[];
  guidance: string;
}

const ROUTES: PromptRouteDefinition[] = [
  {
    intent: "doctor",
    skill: "doctor",
    label: "doctor/환경 점검",
    needsPreflight: true,
    patterns: [
      /\/axhub:doctor\b/i,
      /\bdoctor\b/i,
      /\bdiagnose\b/i,
      /\bhealth\s+check\b/i,
      /\bsanity\s+check\b/i,
      /\bsetup\s+check\b/i,
      /\benv\s+check\b/i,
      /진단/,
      /닥터/,
      /환경\s*점검/,
      /헬스\s*체크/,
      /설치\s*상태/,
      /셋업.*(끝|완료|됐)/,
      /잘\s*깔렸/,
      /axhub.*점검/,
    ],
    guidance: "일반 저장소 환경 체크로 답하지 말고 axhub doctor 워크플로우를 적용해요.",
  },
  {
    intent: "apis",
    skill: "apis",
    label: "API 카탈로그 조회",
    needsPreflight: true,
    patterns: [
      /\bapis?\b/i,
      /\bapi\s+catalog\b/i,
      /\bavailable\s+(apis?|endpoints?)\b/i,
      /\bendpoints?\b/i,
      /api\s*(뭐|목록|리스트|카탈로그|보여|쓸|사용|호출|엔드포인트)/i,
      /(쓸 수 있는|사용 가능한|호출 가능한).*api/i,
      /엔드포인트/,
    ],
    guidance: "읽기 전용 API 카탈로그 요청이에요. 기본 scope 는 현재 앱으로 유지하고 skills/apis/SKILL.md 흐름을 따라요.",
  },
  {
    intent: "apps",
    skill: "apps",
    label: "앱 목록 조회",
    needsPreflight: true,
    patterns: [
      /\bapps\b/i,
      /\bmy\s+apps\b/i,
      /\bapp\s+(list|catalog)\b/i,
      /(내|제|우리|회사)?\s*앱\s*(목록|리스트|보여|봐|뭐|어떤|슬러그|id)/i,
      /(등록된|운영 중인)\s*앱/i,
    ],
    guidance: "읽기 전용 앱 목록 요청이에요. 일반 repo 탐색 대신 skills/apps/SKILL.md 흐름으로 팀 scope 안에서만 보여줘요.",
  },
  {
    intent: "auth",
    skill: "auth",
    label: "로그인/토큰/identity",
    needsPreflight: false,
    patterns: [
      /\bauth\b/i,
      /\blog\s*in\b/i,
      /\blog\s*out\b/i,
      /\bsign\s*(in|out)\b/i,
      /\bwho\s*am\s*i\b/i,
      /\bwhoami\b/i,
      /로그인|로그아웃|다시\s*로그인|토큰|인증|권한|scope|누구로|누구야|계정/,
    ],
    guidance: "axhub identity 요청이에요. 저장소 사용자 확인으로 답하지 말고 skills/auth/SKILL.md 흐름으로 로그인 상태와 토큰 상태를 확인해요.",
  },
  {
    intent: "logs",
    skill: "logs",
    label: "배포 로그 조회",
    needsPreflight: false,
    patterns: [
      /\blogs?\b/i,
      /\btail\b/i,
      /\bconsole\b/i,
      /\bwhy\s+(did|is)\b/i,
      /로그|빌드\s*로그|런타임\s*로그|왜\s*(실패|안돼|깨졌|죽었)|에러|콘솔|출력/,
    ],
    guidance: "axhub 배포 로그 요청이에요. 기본값은 빌드 로그이고 skills/logs/SKILL.md 흐름으로 deployment 를 해석해요.",
  },
  {
    intent: "status",
    skill: "status",
    label: "배포 상태 조회",
    needsPreflight: false,
    patterns: [
      /\bstatus\b/i,
      /\bwatch\b/i,
      /\bfollow\b/i,
      /\bprogress\b/i,
      /\bis\s+it\s+done\b/i,
      /배포\s*상태|진행\s*(상황|중)|어떻게\s*됐|다\s*됐|끝났|어디까지|어디쯤|올라갔|떴어|라이브\s*됐|반영\s*됐|빌드\s*됐|상태\s*봐/,
    ],
    guidance: "axhub 배포 진행 상태 요청이에요. skills/status/SKILL.md 흐름으로 최근 배포나 지정 배포를 추적해요.",
  },
  {
    intent: "recover",
    skill: "recover",
    label: "복구/rollback",
    needsPreflight: true,
    patterns: [
      /\broll\s*back\b/i,
      /\brollback\b/i,
      /\brevert\b/i,
      /\bundo\b/i,
      /\brestore\b/i,
      /\bhot\s*fix\b/i,
      /되돌|롤백|이전\s*버전|직전\s*버전|배포\s*취소|복구|안정\s*버전|마지막\s*정상/,
    ],
    guidance: "axhub 복구 요청이에요. 실제 rollback 이 아니라 직전 안정 commit 재배포 방식의 skills/recover/SKILL.md 흐름을 적용해요.",
  },
  {
    intent: "upgrade",
    skill: "upgrade",
    label: "플러그인 업그레이드",
    needsPreflight: false,
    patterns: [
      /\bplugin\s+(self-)?upgrade\b/i,
      /\bplugin\s+update\b/i,
      /(플러그인|plugin).*(업데이트|업그레이드|새\s*버전|버전|호환)/i,
    ],
    guidance: "axhub Claude Code 플러그인 업그레이드 요청이에요. CLI 업데이트와 구분해서 skills/upgrade/SKILL.md 흐름을 따라요.",
  },
  {
    intent: "update",
    skill: "update",
    label: "CLI 업데이트",
    needsPreflight: false,
    patterns: [
      /\bupdate\b/i,
      /\bupgrade\b/i,
      /\bversion\b/i,
      /\blatest\b/i,
      /\bnew\s+release\b/i,
      /새\s*버전|최신|버전\s*확인|업데이트|업그레이드|brew\s*upgrade/,
    ],
    guidance: "axhub CLI 버전 관리 요청이에요. plugin release 작업이 아니라 skills/update/SKILL.md 흐름으로 CLI 업데이트를 확인해요.",
  },
  {
    intent: "deploy",
    skill: "deploy",
    label: "라이브 배포",
    needsPreflight: true,
    patterns: [
      /\bdeploy\b/i,
      /\bship\b/i,
      /\brollout\b/i,
      /\blaunch\b/i,
      /\brelease\b/i,
      /배포|올려|올리자|쏘자|내보내자|띄워|프로덕션|공개해|demo가\s*필요/,
    ],
    guidance: "axhub 앱 라이브 배포 요청이에요. 저장소 release workflow, `bun run release`, git tag 작업으로 해석하지 말고 skills/deploy/SKILL.md 의 axhub deploy 안전 가드 흐름을 적용해요.",
  },
  {
    intent: "clarify",
    skill: "clarify",
    label: "모호한 axhub 요청",
    needsPreflight: false,
    patterns: [
      /\bhelp\s+me\s+with\s+axhub\b/i,
      /\baxhub\b/i,
      /axhub\s*(좀|도와줘|어떻게|관련|뭐)/,
    ],
    guidance: "명확한 목적지가 없는 axhub 요청이에요. 조용히 추측하지 말고 skills/clarify/SKILL.md 흐름으로 선택지를 좁혀요.",
  },
];

const DOCTOR_ROUTE = ROUTES[0] as PromptRouteDefinition;

const normalizePrompt = (prompt: string): string => prompt.normalize("NFKC").trim();

export const detectPromptRouteIntent = (prompt: string): PromptRouteDefinition | null => {
  const normalized = prompt.normalize("NFKC").trim();
  if (normalized.length === 0) return null;
  return ROUTES.find((route) => route.patterns.some((pattern) => pattern.test(normalized))) ?? null;
};

export const promptMatchesDoctorIntent = (prompt: string): boolean => {
  return detectPromptRouteIntent(prompt)?.intent === "doctor";
};

const versionSkewGuidance = (preflight: PromptRoutePreflight): string => {
  const version = preflight.output.cli_version ?? "unknown";
  if (preflight.output.cli_too_old) {
    return `axhub 버전 확인 결과, axhub가 너무 오래된 버전이에요 (${version}). 'axhub 업그레이드해줘'라고 말씀해주세요.`;
  }
  if (preflight.output.cli_too_new) {
    return `axhub 버전 확인 결과, 검증 범위보다 새 버전이에요 (${version}). 플러그인 업데이트 확인이 필요해요.`;
  }
  if (!preflight.output.cli_present) {
    return "axhub 설치 확인 결과, CLI를 찾지 못했어요. axhub 설치 후 다시 점검해주세요.";
  }
  return `axhub 버전 확인 결과, CLI ${version} 상태를 확인했어요.`;
};

export const buildPromptRouteContext = (
  prompt: string,
  preflight: PromptRoutePreflight | null,
  route: PromptRouteDefinition = DOCTOR_ROUTE,
): string => {
  const lines = [
    "[axhub prompt routing]",
    `사용자 발화 "${prompt}"는 axhub ${route.label} 의도예요.`,
    `반드시 skills/${route.skill}/SKILL.md 워크플로우를 우선 적용해요.`,
    route.guidance,
  ];
  if (preflight) {
    const preflightJson = JSON.stringify({
      cli_version: preflight.output.cli_version,
      cli_present: preflight.output.cli_present,
      in_range: preflight.output.in_range,
      cli_too_old: preflight.output.cli_too_old,
      cli_too_new: preflight.output.cli_too_new,
      auth_ok: preflight.output.auth_ok,
      auth_error_code: preflight.output.auth_error_code,
      exit_code: preflight.exitCode,
    });
    lines.push(`Preflight 결과: ${preflightJson}`);
    lines.push(versionSkewGuidance(preflight));
  }
  return lines.join("\n");
};

export const runPromptRoute = (
  rawInput: string,
  preflightRunner: () => PromptRoutePreflight = runPreflight,
): PromptRouteOutput => {
  let parsed: PromptRouteInput | null = null;
  try {
    parsed = rawInput.trim().length === 0 ? null : JSON.parse(rawInput) as PromptRouteInput;
  } catch {
    return {};
  }
  const prompt = parsed?.prompt;
  if (typeof prompt !== "string") {
    return {};
  }
  const normalized = normalizePrompt(prompt);
  const route = detectPromptRouteIntent(normalized);
  if (!route) return {};

  const preflight = route.needsPreflight ? preflightRunner() : null;
  return {
    hookSpecificOutput: {
      hookEventName: "UserPromptSubmit",
      additionalContext: buildPromptRouteContext(normalized, preflight, route),
    },
  };
};
