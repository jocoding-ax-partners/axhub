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

const DOCTOR_PATTERNS = [
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
];

export const promptMatchesDoctorIntent = (prompt: string): boolean => {
  const normalized = prompt.normalize("NFKC").trim();
  if (normalized.length === 0) return false;
  return DOCTOR_PATTERNS.some((pattern) => pattern.test(normalized));
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
  preflight: PromptRoutePreflight,
): string => {
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
  return [
    "[axhub prompt routing]",
    `사용자 발화 "${prompt}"는 axhub doctor/환경 점검 의도예요.`,
    "일반 저장소 환경 체크로 답하지 말고 axhub doctor 워크플로우를 적용해요.",
    `Preflight 결과: ${preflightJson}`,
    versionSkewGuidance(preflight),
  ].join("\n");
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
  if (typeof prompt !== "string" || !promptMatchesDoctorIntent(prompt)) {
    return {};
  }

  const preflight = preflightRunner();
  return {
    hookSpecificOutput: {
      hookEventName: "UserPromptSubmit",
      additionalContext: buildPromptRouteContext(prompt, preflight),
    },
  };
};
