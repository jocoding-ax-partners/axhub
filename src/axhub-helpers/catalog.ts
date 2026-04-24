/**
 * catalog.ts — Error Empathy Catalog (4-part Korean templates)
 *
 * Source: skills/deploy/references/error-empathy-catalog.md
 * Embedded as JSON — no markdown parsing at runtime.
 */

export interface ErrorEntry {
  emotion: string;  // 감정 (Emotion) — KR
  cause: string;    // 원인 (Cause) — KR
  action: string;   // 해결 (Action) — KR
  button?: string;  // 버튼 (Buttons) — KR, AskUserQuestion options label string
}

// Keys: exit code as string, or "${exitCode}:${errorCode}" for sub-classified entries.
export const CATALOG: Record<string, ErrorEntry> = {
  "0": {
    emotion: "축하해요! 배포 성공입니다.",
    cause: "앱이 정상 반영됐어요. 빌드가 성공적으로 끝났습니다.",
    action:
      '라이브 URL을 한 번 확인해보시겠어요? 다음에 또 배포하실 때는 "방금 거 상태" 또는 "방금 거 로그" 라고 말씀하시면 바로 보여드려요.',
    button: "라이브 확인 / 로그 보기 / 닫기",
  },

  "1": {
    emotion: "잠깐만요. 일시적인 통신 문제예요. 당신 앱은 안전합니다.",
    cause:
      "axhub 서버까지 연결이 잠깐 끊겼어요. 네트워크가 느리거나 서버가 잠시 응답을 못한 경우예요. 배포 자체는 시작도 안 됐으니 걱정하지 마세요.",
    action:
      '한 번 더 시도해보시겠어요? "다시 시도해줘" 라고 말씀하시면 한 번만 자동 재시도해요. 배포 명령은 자동 재시도하지 않아요 (중복 배포 방지).',
    button: "다시 시도 / 잠시 후 다시 / 도와주세요",
  },

  "2": {
    emotion: "정상이에요. 배포가 아직 진행 중일 뿐입니다.",
    cause: "배포가 현재 진행 중인 단계예요. 평균 몇 분 정도 걸립니다.",
    action:
      '"계속 지켜봐줘" 라고 말씀하시면 끝날 때까지 자동으로 알려드려요. 다른 일 하시다가 끝나면 알림 드릴게요.',
    button: "계속 지켜보기 / 지금 그만 보기 / 로그도 같이 보기",
  },

  "64": {
    emotion: "잠깐만요. 배포는 시작 안 됐어요. 당신 앱은 안전합니다.",
    cause:
      "입력값에 문제가 있어서 배포 요청이 막혔어요. axhub가 받기 전에 검증에서 멈췄다는 뜻이에요.",
    action:
      '무엇을 배포하려 하셨는지 다시 한 번 풀어서 말씀해주세요. 예: "paydrop 메인 브랜치 최신 커밋 배포해" 처럼 구체적으로요.',
    button: "다시 풀어 말하기 / 도와주세요 / 취소",
  },

  "64:validation.deployment_in_progress": {
    emotion: "당신 앱은 안전합니다. 다른 배포가 먼저 진행 중이에요.",
    cause:
      "다른 배포가 아직 끝나지 않았어요. 같은 앱은 한 번에 한 배포만 진행됩니다 (서로 덮어쓰지 못하게 막아주는 안전장치예요).",
    action:
      '새로 배포하지 마시고 진행 중인 그 배포를 함께 지켜볼까요? "그거 끝날 때까지 지켜봐줘" 라고 말씀해주시면 됩니다. 절대 다시 시도하지 않습니다 — 끝나면 자연스럽게 다음 배포가 가능해요.',
    button: "진행 중인 거 지켜보기 / 5분 후 다시 알려줘 / 지금 취소",
  },

  "64:validation.app_ambiguous": {
    emotion: "잠깐만요. 같은 이름이 두 개라서 헷갈렸어요.",
    cause: "그 이름의 앱이 여러 개 있어요. 어떤 거 말씀하신 건지 골라주세요.",
    action: "아래 후보 중 하나를 골라주세요. 다음부터는 정확한 ID로 기억해둘게요.",
    button: "후보 앱 선택 / 더 많은 후보 보기 / 취소",
  },

  "64:validation.app_list_truncated": {
    emotion: "잠깐만요. 회사에 앱이 너무 많아서 다 못 가져왔어요.",
    cause: "앱이 100개를 넘어서 목록이 잘렸어요. 이름만으로는 정확히 어떤 앱인지 못 찾아요.",
    action:
      '앱의 ID 숫자를 직접 알려주실 수 있나요? 예: "id 42 배포해" 또는 "app-3 배포해" 처럼요. ID는 apps list 결과에 표시돼요.',
    button: "앱 검색하기 / 앱 ID 직접 입력 / 도와주세요",
  },

  "65": {
    emotion: "잠깐만요. 로그인이 만료됐을 뿐이에요. 당신 앱은 그대로예요.",
    cause:
      "axhub 로그인 토큰이 만료됐어요. 보안을 위해 일정 시간이 지나면 다시 로그인해야 해요. 평소 회사 메일·은행 사이트랑 똑같아요.",
    action:
      '"다시 로그인해줘" 라고 말씀하시면 브라우저로 안내드릴게요. (브라우저가 안 열리는 환경 — 예: GitHub Codespaces — 이시면 별도 안내드려요.)',
    button: "다시 로그인 / 토큰 파일로 로그인 (헤드리스) / 도와주세요",
  },

  "66": {
    emotion: "잠깐만요. 권한 문제예요. 당신 앱은 안전합니다.",
    cause:
      "지금 토큰의 권한 범위로는 이 작업을 할 수 없어요. 회사 정책상 사람 (보통 토큰 발급해주신 분 — IT 담당자나 PM) 이 권한을 더 부여해야 해요.",
    action:
      '토큰을 발급해준 분께 이 메시지 그대로 보내주세요: "axhub 토큰에 필요한 scope 추가 필요합니다." 그 분이 처리해주시면 다시 로그인하시면 됩니다.',
    button: "담당자에게 메시지 복사 / 현재 권한 확인 / 도와주세요",
  },

  "66:scope.downgrade_blocked": {
    emotion: "잠깐만요. 안전장치가 작동했어요.",
    cause:
      "더 낮은 환경으로의 다운그레이드 시도가 감지됐어요. 예를 들어 production에 있는 앱을 staging 빌드로 덮으려 했을 때 안전을 위해 막아드려요.",
    action:
      '정말로 다운그레이드가 필요하시면 명시적으로 "강제로 다운그레이드해" 라고 말씀해주세요. 그게 아니라면 의도하신 환경 (보통 production) 의 빌드를 다시 확인해주세요.',
    button: "환경 다시 확인 / 강제 다운그레이드 (위험) / 취소",
  },

  "66:update.cosign_verification_failed": {
    emotion: "잠깐만요. 보안 검증에 실패했어요. 절대 진행하지 않아요.",
    cause:
      "다운로드받은 axhub 업데이트 파일이 정품인지 검증하는 cosign 절차에서 실패했어요. 파일이 변조됐거나 네트워크 중간에 누군가 끼어든 가능성이 있어요. 보안상 업데이트를 차단했습니다.",
    action:
      "절대 강제로 진행하지 마세요. 회사 IT 보안 담당자에게 즉시 알려주세요. 그동안 axhub는 현재 버전으로 계속 사용하실 수 있어요.",
    button: "IT 보안팀에 알리기 / 업데이트 취소 / 현재 버전 유지",
  },

  "67": {
    emotion: "잠깐만요. 그런 이름은 못 찾았어요.",
    cause:
      "그 이름의 앱/배포/API 가 회사 axhub에 등록되어 있지 않아요. 오타이거나, 다른 회사 계정의 앱일 수도 있어요.",
    action:
      "가장 비슷한 후보를 보여드릴게요. 아래 중 하나를 선택하시거나 다시 입력해주세요.",
    button: "가장 유사한 거로 / 앱 목록 보기 / 다시 입력",
  },

  "68": {
    emotion: "잠깐만요. 너무 많이 요청해서 서버가 잠시 쉬자고 해요. 당신 앱은 안전합니다.",
    cause:
      "짧은 시간 안에 axhub 호출이 많이 누적돼서 잠깐 멈춰야 해요. 보통 다른 사람이랑 같은 토큰을 공유하거나, 자동화 스크립트가 너무 빨리 돌 때 생겨요.",
    action:
      "잠시 기다려주세요. 자동으로 다시 시도할게요. 그동안 커피 한 잔 어떠세요?",
    button: "자동으로 기다리기 / 지금 취소 / 도와주세요",
  },
};

const DEFAULT_ENTRY: ErrorEntry = {
  emotion: "이건 흔한 일이에요.",
  cause: "알 수 없는 에러가 발생했어요.",
  action: "관리자에게 문의해주세요.",
};

/**
 * Classify an axhub exit code into a 4-part Korean error entry.
 *
 * Tries `${exit_code}:${error_code}` first (sub-classified), then falls
 * back to `${exit_code}`, then falls back to DEFAULT_ENTRY.
 *
 * @param exit_code - numeric exit code from the CLI
 * @param stdout    - raw stdout string (may contain JSON with error.code)
 */
export function classify(exit_code: number, stdout: string): ErrorEntry {
  // Try to parse error.code from JSON stdout
  let errorCode: string | undefined;
  try {
    const parsed = JSON.parse(stdout) as unknown;
    if (
      parsed !== null &&
      typeof parsed === "object" &&
      "error" in parsed &&
      parsed.error !== null &&
      typeof parsed.error === "object" &&
      "code" in parsed.error &&
      typeof (parsed.error as { code: unknown }).code === "string"
    ) {
      errorCode = (parsed.error as { code: string }).code;
    }
  } catch {
    // stdout is not JSON or has no error.code — fine, use base exit code only
  }

  // Sub-classified lookup first
  if (errorCode !== undefined) {
    const subKey = `${exit_code}:${errorCode}`;
    const subEntry = CATALOG[subKey];
    if (subEntry !== undefined) {
      return subEntry;
    }
  }

  // Base exit code lookup
  const baseEntry = CATALOG[String(exit_code)];
  if (baseEntry !== undefined) {
    return baseEntry;
  }

  return DEFAULT_ENTRY;
}
