// Phase 22.0.4 — SB-2 baseline lock test.
// registry.json 의 entry count + safe_default 개수가 plan 의 baseline 과 일치하는지 검증.
// drift 발생 시 (새 AskUserQuestion 추가, key text 변경 등) 즉시 fail.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REGISTRY_PATH = join(
  import.meta.dir,
  "fixtures",
  "ask-defaults",
  "registry.json",
);

interface SafeDefaultEntry {
  safe_default: string;
  rationale: string;
}

type RegistryValue =
  | string
  | { safe_default?: string; rationale?: string; [key: string]: unknown };

interface Registry {
  [key: string]: RegistryValue | { [questionText: string]: SafeDefaultEntry };
}

const registry = JSON.parse(readFileSync(REGISTRY_PATH, "utf8")) as Registry;

const collectSafeDefaultPaths = (): string[] => {
  const paths: string[] = [];
  for (const [skill, value] of Object.entries(registry)) {
    if (typeof value !== "object" || value === null) continue;
    for (const [innerKey, inner] of Object.entries(value)) {
      if (
        typeof inner === "object" &&
        inner !== null &&
        "safe_default" in inner
      ) {
        paths.push(`${skill}.${innerKey}`);
      }
    }
  }
  return paths;
};

describe("Phase 23 — registry.json baseline (CLI coverage v0.2.0)", () => {
  test("19 top-level keys (2 메타 + 17 SKILL slug)", () => {
    const keys = Object.keys(registry);
    expect(keys).toHaveLength(19);
    expect(keys).toContain("_schema");
    expect(keys).toContain("_path_history");
    const skillSlugs = keys.filter((k) => !k.startsWith("_")).sort();
    expect(skillSlugs).toEqual([
      "apis",
      "apps",
      "auth",
      "clarify",
      "deploy",
      "doctor",
      "env",
      "github",
      "init",
      "logs",
      "open",
      "profile",
      "recover",
      "status",
      "update",
      "upgrade",
      "whatsnew",
    ]);
  });

  test("14 actual safe_default rationale 엔트리 (기존 9 + init/env/github/profile/deploy)", () => {
    const paths = collectSafeDefaultPaths();
    expect(paths).toHaveLength(14);

    const skills = paths.map((p) => p.split(".")[0]).sort();
    expect(skills).toEqual([
      "apis",
      "apps",
      "auth",
      "auth",
      "clarify",
      "deploy",
      "doctor",
      "env",
      "github",
      "init",
      "profile",
      "recover",
      "update",
      "upgrade",
    ]);
  });

  test("14 safe_default 값 (safe fallback 카탈로그)", () => {
    const auth = registry["auth"] as Record<string, SafeDefaultEntry>;
    expect(auth["다시 로그인할래요?"]?.safe_default).toBe("abort");
    expect(auth["로그아웃할래요?"]?.safe_default).toBe("abort");

    const recover = registry["recover"] as Record<string, SafeDefaultEntry>;
    expect(
      recover["직전 안정 커밋으로 다시 배포해요?"]?.safe_default,
    ).toBe("abort");

    const apis = registry["apis"] as Record<string, SafeDefaultEntry>;
    expect(
      apis[
        "다른 팀 API도 볼래요? 권한 있는 모든 endpoint 보여줄 수 있지만, 보통 현재 앱이 호출하는 것만 봐도 충분해요."
      ]?.safe_default,
    ).toBe("stay");

    const apps = registry["apps"] as Record<string, SafeDefaultEntry>;
    expect(apps["앱이 더 있어요. 전체 목록 볼래요?"]?.safe_default).toBe("skip");

    const deploy = registry["deploy"] as Record<string, SafeDefaultEntry>;
    expect(deploy["배포 전 저장 지점을 만들까요?"]?.safe_default).toBe(
      "명령어만 보기",
    );

    const clarify = registry["clarify"] as Record<string, SafeDefaultEntry>;
    expect(clarify["어떤 작업 원해요?"]?.safe_default).toBe("abort");

    const doctor = registry["doctor"] as Record<string, SafeDefaultEntry>;
    expect(
      doctor["여러 항목 점검 필요해요. 어디부터 고쳐요?"]?.safe_default,
    ).toBe("later");

    const update = registry["update"] as Record<string, SafeDefaultEntry>;
    expect(update["axhub CLI 업그레이드해요?"]?.safe_default).toBe("skip");

    const upgrade = registry["upgrade"] as Record<string, SafeDefaultEntry>;
    expect(
      upgrade["플러그인 업그레이드 명령 보여줄까요?"]?.safe_default,
    ).toBe("show");

    const init = registry["init"] as Record<string, SafeDefaultEntry>;
    expect(init["어떤 템플릿으로 시작할까요?"]?.safe_default).toBe("abort");

    const env = registry["env"] as Record<string, SafeDefaultEntry>;
    expect(env["어떤 환경변수 작업을 할까요?"]?.safe_default).toBe("조회만");

    const github = registry["github"] as Record<string, SafeDefaultEntry>;
    expect(github["GitHub 연동 작업을 고를까요?"]?.safe_default).toBe("목록만");

    const profile = registry["profile"] as Record<string, SafeDefaultEntry>;
    expect(profile["프로필 작업을 고를까요?"]?.safe_default).toBe(
      "현재 프로필 보기",
    );
  });

  test("read-only/no-question skills keep metadata without safe_default", () => {
    const deploy = registry["deploy"] as Record<string, RegistryValue>;
    expect(deploy["_note"]).toBeString();
    expect(deploy["default_subprocess_action"]).toBe("--dry-run");

    const logs = registry["logs"] as Record<string, RegistryValue>;
    expect(logs["_note"]).toBeString();
    expect(logs["default_source"]).toBe("build");

    const status = registry["status"] as Record<string, RegistryValue>;
    expect(status["_note"]).toBeString();
    expect(status["cold_cache_default"]).toBe("most_recent");
    expect(status["exit_65_default"]).toBe("abort");

    const open = registry["open"] as Record<string, RegistryValue>;
    expect(open["_note"]).toBeString();

    const whatsnew = registry["whatsnew"] as Record<string, RegistryValue>;
    expect(whatsnew["_note"]).toBeString();
  });

  test("모든 safe_default 엔트리에 rationale 첨부 (drift catch)", () => {
    const paths = collectSafeDefaultPaths();
    for (const path of paths) {
      const [skill, ...rest] = path.split(".");
      if (!skill) continue;
      const questionKey = rest.join(".");
      const skillObj = registry[skill] as Record<string, SafeDefaultEntry>;
      const entry = skillObj?.[questionKey];
      expect(entry?.rationale, `rationale missing for ${path}`).toBeString();
      expect((entry?.rationale ?? "").length).toBeGreaterThan(20);
    }
  });
});
