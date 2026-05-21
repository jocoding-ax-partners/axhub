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
  test("33 top-level keys (2 메타 + 29 SKILL slug + quality_gate + consent channel)", () => {
    const keys = Object.keys(registry);
    expect(keys).toHaveLength(33);
    expect(keys).toContain("_schema");
    expect(keys).toContain("_path_history");
    const channels = keys.filter((k) => !k.startsWith("_")).sort();
    expect(channels).toEqual([
      "apps",
      "auth",
      "axhub-debug",
      "axhub-diagnose",
      "axhub-plan",
      "axhub-review",
      "axhub-ship",
      "axhub-tdd",
      "clarify",
      "consent-megaskill",
      "deploy",
      "doctor",
      "enable-statusline",
      "env",
      "github",
      "init",
      "install-cli",
      "karpathy-guidelines",
      "logs",
      "open",
      "profile",
      "quality_gate",
      "recover",
      "routing-stats",
      "status",
      "trace",
      "update",
      "upgrade",
      "using-axhub-quality",
      "verify",
      "whatsnew",
    ]);
  });

  test("42 actual safe_default rationale 엔트리 including Phase 26 quality + Plan v6 diagnose + v0.9.3 auth PAT revoke", () => {
    const paths = collectSafeDefaultPaths();
    expect(paths).toHaveLength(42);

    const skills = paths.map((p) => p.split(".")[0]).sort();
    expect(skills).toEqual([
      "apps",
      "apps",
      "auth",
      "auth",
      "auth",
      "axhub-debug",
      "axhub-diagnose",
      "axhub-plan",
      "axhub-review",
      "axhub-review",
      "axhub-ship",
      "axhub-tdd",
      "clarify",
      "consent-megaskill",
      "deploy",
      "deploy",
      "deploy",
      "deploy",
      "deploy",
      "deploy",
      "deploy",
      "doctor",
      "doctor",
      "enable-statusline",
      "env",
      "github",
      "github",
      "github",
      "github",
      "github",
      "init",
      "init",
      "init",
      "install-cli",
      "profile",
      "quality_gate",
      "recover",
      "routing-stats",
      "trace",
      "update",
      "upgrade",
      "verify",
    ]);
  });

  test("safe_default 값 카탈로그 (safe fallback)", () => {
    const auth = registry["auth"] as Record<string, SafeDefaultEntry>;
    expect(auth["다시 로그인할래요?"]?.safe_default).toBe("abort");
    expect(auth["로그아웃할래요?"]?.safe_default).toBe("abort");
    expect(auth["PAT <id> 를 폐기할까요?"]?.safe_default).toBe("abort");

    const recover = registry["recover"] as Record<string, SafeDefaultEntry>;
    expect(
      recover["직전에 잘 됐던 버전으로 다시 올릴까요?"]?.safe_default,
    ).toBe("abort");

    const apps = registry["apps"] as Record<string, SafeDefaultEntry>;
    expect(apps["앱이 더 있어요. 전체 목록 볼래요?"]?.safe_default).toBe("skip");
    expect(apps["앱을 삭제할까요?"]?.safe_default).toBe("abort");

    const deploy = registry["deploy"] as Record<string, SafeDefaultEntry>;
    expect(deploy["배포 전 저장 지점을 만들까요?"]?.safe_default).toBe(
      "취소",
    );
    expect(deploy["진행할까요?"]?.safe_default).toBe("미리보기만");
    expect(
      deploy["axhub CLI 가 더 최신 버전인데 계속할까요?"]?.safe_default,
    ).toBe("계속해요");
    expect(
      deploy["품질 게이트가 막았어요. 그래도 진행할까요?"]?.safe_default,
    ).toBe("취소");

    const qualityGate = registry["quality_gate"] as Record<
      string,
      SafeDefaultEntry
    >;
    expect(qualityGate["abort_or_proceed"]?.safe_default).toBe("abort");

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
    expect(github["GitHub 연동 작업을 고를까요?"]?.safe_default).toBe("list_only");
    expect(github["GitHub repo 를 만들까요?"]?.safe_default).toBe("abort");
    expect(github["git remote 를 추가할까요?"]?.safe_default).toBe("abort");
    expect(github["첫 push 를 실행할까요?"]?.safe_default).toBe("abort");
    expect(github["axhub 앱에 repo 를 연결할까요?"]?.safe_default).toBe("abort");

    const profile = registry["profile"] as Record<string, SafeDefaultEntry>;
    expect(profile["프로필 작업을 고를까요?"]?.safe_default).toBe(
      "현재 프로필 보기",
    );

    const routingStats = registry["routing-stats"] as Record<string, SafeDefaultEntry>;
    expect(routingStats["다음에 뭘 볼까요?"]?.safe_default).toBe("끝");
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
