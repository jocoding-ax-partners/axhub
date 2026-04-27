import { describe, expect, test } from "bun:test";
import { classify } from "../src/axhub-helpers/catalog.ts";

describe("classify()", () => {
  test("exit 0 → 축하해요 emotion", () => {
    const entry = classify(0, "");
    expect(entry.emotion).toContain("축하해요");
    expect(entry.cause).toBeTruthy();
    expect(entry.action).toBeTruthy();
    expect(entry.button).toBeTruthy();
  });

  test("exit 1 → 잠깐만요 + 연결 끊김", () => {
    const entry = classify(1, "");
    expect(entry.emotion).toContain("잠깐만요");
    expect(entry.cause).toContain("연결이 잠깐 끊겼어요");
    expect(entry.action).toBeTruthy();
    expect(entry.button).toBeTruthy();
  });

  test("exit 64 base → 배포는 시작 안 했어요", () => {
    const entry = classify(64, "");
    expect(entry.emotion).toContain("배포는 시작 안 했어요");
    expect(entry.cause).toContain("검증");
    expect(entry.action).toBeTruthy();
    expect(entry.button).toBeTruthy();
  });

  test("exit 64 + validation.deployment_in_progress → 진행 중인 배포 entry", () => {
    const stdout = JSON.stringify({
      error: { code: "validation.deployment_in_progress", message: "In progress" },
    });
    const entry = classify(64, stdout);
    expect(entry.emotion).toContain("다른 배포가 먼저 진행 중이에요");
    expect(entry.cause).toContain("한 번에 하나만");
    expect(entry.button).toContain("진행 중인 거 지켜보기");
  });

  test("exit 64 + validation.app_ambiguous → 같은 이름이 두 개", () => {
    const stdout = JSON.stringify({
      error: { code: "validation.app_ambiguous", message: "Ambiguous" },
    });
    const entry = classify(64, stdout);
    expect(entry.emotion).toContain("같은 이름이 두 개");
  });

  test("exit 64 + validation.app_list_truncated → 앱이 너무 많아서", () => {
    const stdout = JSON.stringify({
      error: { code: "validation.app_list_truncated", message: "Truncated" },
    });
    const entry = classify(64, stdout);
    expect(entry.emotion).toContain("앱이 너무 많아서");
  });

  test("exit 65 → 로그인이 만료됐을 뿐이에요", () => {
    const stdout = JSON.stringify({
      error: { code: "auth.expired", message: "Token expired" },
    });
    const entry = classify(65, stdout);
    expect(entry.emotion).toContain("로그인이 만료됐을 뿐이에요");
    expect(entry.cause).toContain("토큰이 만료됐어요");
    expect(entry.action).toContain("다시 로그인해줘");
    expect(entry.button).toContain("다시 로그인");
  });

  test("exit 65 with empty stdout → falls back to base 65 entry", () => {
    const entry = classify(65, "");
    expect(entry.emotion).toContain("로그인이 만료됐을 뿐이에요");
  });

  test("exit 66 base → 권한 문제", () => {
    const entry = classify(66, "");
    expect(entry.emotion).toContain("권한 문제");
    expect(entry.cause).toContain("권한 범위");
    expect(entry.button).toBeTruthy();
  });

  test("exit 66 + scope.downgrade_blocked → 안전장치가 작동했어요", () => {
    const stdout = JSON.stringify({
      error: { code: "scope.downgrade_blocked" },
    });
    const entry = classify(66, stdout);
    expect(entry.emotion).toContain("안전장치가 작동했어요");
    expect(entry.button).toContain("강제 다운그레이드");
  });

  test("exit 66 + update.cosign_verification_failed → 보안 검증에 실패", () => {
    const stdout = JSON.stringify({
      error: { code: "update.cosign_verification_failed" },
    });
    const entry = classify(66, stdout);
    expect(entry.emotion).toContain("보안 검증에 실패했어요");
    expect(entry.action).toContain("IT 보안 담당자");
  });

  test("exit 67 → 그런 이름은 못 찾았어요", () => {
    const entry = classify(67, "");
    expect(entry.emotion).toContain("그런 이름은 못 찾았어요");
  });

  test("exit 68 → 너무 많이 요청해서", () => {
    const entry = classify(68, "");
    expect(entry.emotion).toContain("너무 많이 요청해서");
    expect(entry.button).toContain("자동으로 기다리기");
  });

  test("unknown exit code → generic default entry", () => {
    const entry = classify(99, "");
    expect(entry.emotion).toBeTruthy();
    expect(entry.cause).toContain("알 수 없는 에러");
    expect(entry.action).toContain("관리자에게 물어");
    expect(entry.button).toBeUndefined();
  });

  test("malformed stdout JSON → falls back to base exit code entry", () => {
    const entry = classify(65, "not-json{{{{");
    expect(entry.emotion).toContain("로그인이 만료됐을 뿐이에요");
  });
});
