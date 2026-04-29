/**
 * tests/axhub-helpers.test.ts — preflight + resolve coverage (US-001).
 *
 * Validates:
 *   1. preflight: cli_too_old (axhub 0.0.5)
 *   2. preflight: cli_in_range (axhub 0.1.0)
 *   3. preflight: cli_too_new (axhub 0.2.0)
 *   4. preflight: cli_missing → exit 64, no crash
 *   5. preflight: auth missing/expired → exit 65 with cli_in_range still true
 *   6. resolve: app resolved (slug=ccrank matches one app)
 *   7. resolve: app ambiguous (slug=app matches 2+ apps → exit 64)
 *   8. resolve: app not found (slug=zzz matches none → exit 67)
 *   9. resolve: auth missing → exit 65
 *  10. extractSlugCandidate: filters Korean stop-words
 *
 * Strategy: inject a fake CommandRunner (no real axhub binary, no PATH
 * manipulation, no tempdir cleanup). Each test owns its runner so cases
 * are isolated.
 */

import { describe, expect, test } from "bun:test";
import {
  runPreflight,
  type CommandRunner,
  type SpawnResult,
} from "../src/axhub-helpers/preflight.ts";
import {
  runResolve,
  extractSlugCandidate,
  filterAppsBySlug,
  type AppRecord,
} from "../src/axhub-helpers/resolve.ts";
import {
  promptMatchesDoctorIntent,
  runPromptRoute,
} from "../src/axhub-helpers/prompt-route.ts";

// ---- runner factories ----------------------------------------------------

const ok = (stdout: string): SpawnResult => ({ exitCode: 0, stdout, stderr: "" });
const fail = (stderr = ""): SpawnResult => ({ exitCode: 1, stdout: "", stderr });

const AUTH_OK_JSON = JSON.stringify({
  user_email: "test@jocodingax.ai",
  user_id: 42,
  expires_at: "2026-04-23T11:08:04.411432Z",
  scopes: ["read", "write", "deploy:execute"],
});

const AUTH_ERR_JSON = JSON.stringify({
  code: "validation.endpoint_required",
  detail: "No axhub endpoint configured",
  recovery_hint: "Set AXHUB_ENDPOINT",
  title: "Endpoint not configured",
});

const APPS_JSON = JSON.stringify([
  { id: 6, slug: "ccrank", name: "ccrank" },
  { id: 42, slug: "paydrop", name: "paydrop" },
  { id: 50, slug: "app-3", name: "지원사업 크롤링" },
  { id: 51, slug: "app-3-staging", name: "스테이징" },
  { id: 99, slug: "test-fixture", name: "fixture" },
]);

/**
 * Build a runner that dispatches by argv[0..N]. Falls through to fail() so
 * tests notice unexpected calls instead of silently returning empty.
 */
const makeRunner = (handlers: {
  versionStdout?: string;
  versionExitCode?: number;
  authStdout?: string;
  appsStdout?: string;
  gitBranch?: string;
  gitSha?: string;
  gitMsg?: string;
}): CommandRunner => {
  return (cmd: string[]): SpawnResult => {
    // Match by joined tail so test-level argv arrays stay readable.
    const joined = cmd.slice(1).join(" ");
    if (joined === "--version" && handlers.versionStdout !== undefined) {
      return {
        exitCode: handlers.versionExitCode ?? 0,
        stdout: handlers.versionStdout,
        stderr: "",
      };
    }
    if (joined === "auth status --json" && handlers.authStdout !== undefined) {
      return ok(handlers.authStdout);
    }
    if (joined === "apps list --json" && handlers.appsStdout !== undefined) {
      return ok(handlers.appsStdout);
    }
    // git
    if (cmd[0] === "git") {
      if (joined === "branch --show-current") return ok((handlers.gitBranch ?? "main") + "\n");
      if (joined === "rev-parse HEAD") return ok((handlers.gitSha ?? "deadbeef") + "\n");
      if (joined === "log -1 --pretty=%s") return ok((handlers.gitMsg ?? "fix: bug") + "\n");
    }
    return fail(`unexpected cmd: ${cmd.join(" ")}`);
  };
};

// ---- preflight ------------------------------------------------------------

describe("runPreflight()", () => {
  test("cli too old (0.0.5) → in_range:false, cli_too_old:true, exit 64", () => {
    const runner = makeRunner({
      versionStdout: "axhub 0.0.5 (commit abc, built 2026-01-01T00:00:00Z, darwin/arm64)\n",
      authStdout: AUTH_OK_JSON,
    });
    const { output, exitCode } = runPreflight(runner);
    expect(output.cli_version).toBe("0.0.5");
    expect(output.in_range).toBe(false);
    expect(output.cli_too_old).toBe(true);
    expect(output.cli_too_new).toBe(false);
    expect(output.cli_present).toBe(true);
    expect(exitCode).toBe(64);
  });

  test("cli in range (0.1.0) + auth ok → in_range:true, exit 0", () => {
    const runner = makeRunner({
      versionStdout: "axhub 0.1.0 (commit 447222925b, built 2026-04-23T06:32:24Z, darwin/arm64)\n",
      authStdout: AUTH_OK_JSON,
    });
    const { output, exitCode } = runPreflight(runner);
    expect(output.cli_version).toBe("0.1.0");
    expect(output.in_range).toBe(true);
    expect(output.cli_too_old).toBe(false);
    expect(output.cli_too_new).toBe(false);
    expect(output.auth_ok).toBe(true);
    expect(output.scopes).toEqual(["read", "write", "deploy:execute"]);
    expect(output.user_email).toBe("test@jocodingax.ai");
    expect(exitCode).toBe(0);
  });

  test("cli too new (0.2.0) → cli_too_new:true, exit 64", () => {
    const runner = makeRunner({
      versionStdout: "axhub 0.2.0 (commit xyz, built 2026-12-01T00:00:00Z, darwin/arm64)\n",
      authStdout: AUTH_OK_JSON,
    });
    const { output, exitCode } = runPreflight(runner);
    expect(output.cli_version).toBe("0.2.0");
    expect(output.in_range).toBe(false);
    expect(output.cli_too_old).toBe(false);
    expect(output.cli_too_new).toBe(true);
    expect(exitCode).toBe(64);
  });

  test("cli within range, intermediate (0.1.5) → in_range:true", () => {
    const runner = makeRunner({
      versionStdout: "axhub 0.1.5 (commit ddd, built 2026-06-01T00:00:00Z, darwin/arm64)\n",
      authStdout: AUTH_OK_JSON,
    });
    const { output, exitCode } = runPreflight(runner);
    expect(output.cli_version).toBe("0.1.5");
    expect(output.in_range).toBe(true);
    expect(exitCode).toBe(0);
  });

  test("cli missing (spawn returns empty stdout) → exit 64, no crash", () => {
    const runner: CommandRunner = (cmd) => {
      if (cmd[1] === "--version") return { exitCode: 127, stdout: "", stderr: "command not found" };
      return fail();
    };
    const { output, exitCode } = runPreflight(runner);
    expect(output.cli_present).toBe(false);
    expect(output.cli_version).toBeNull();
    expect(output.in_range).toBe(false);
    expect(output.auth_ok).toBe(false);
    expect(output.auth_error_code).toBe("cli_unavailable");
    expect(exitCode).toBe(64);
  });

  test("cli missing (spawn throws ENOENT) → exit 64, no crash", () => {
    const runner: CommandRunner = (_cmd) => {
      throw new Error("ENOENT");
    };
    const { output, exitCode } = runPreflight(runner);
    expect(output.cli_present).toBe(false);
    expect(output.cli_version).toBeNull();
    expect(exitCode).toBe(64);
  });

  test("cli ok but auth missing → exit 65, in_range stays true", () => {
    const runner = makeRunner({
      versionStdout: "axhub 0.1.0 (commit aaa, built 2026-04-23T06:32:24Z, darwin/arm64)\n",
      authStdout: AUTH_ERR_JSON,
    });
    const { output, exitCode } = runPreflight(runner);
    expect(output.in_range).toBe(true);
    expect(output.auth_ok).toBe(false);
    expect(output.auth_error_code).toBe("validation.endpoint_required");
    expect(exitCode).toBe(65);
  });
});

// ---- prompt-route ---------------------------------------------------------

describe("runPromptRoute()", () => {
  test("환경 점검해 → UserPromptSubmit doctor context with version-skew phrase", () => {
    const output = runPromptRoute(
      JSON.stringify({ hook_event_name: "UserPromptSubmit", prompt: "환경 점검해" }),
      () => ({
        output: runPreflight(makeRunner({
          versionStdout: "axhub 0.0.5 (commit abc, built fake, darwin/arm64)\n",
          authStdout: AUTH_OK_JSON,
        })).output,
        exitCode: 64,
      }),
    );
    const context = output.hookSpecificOutput?.additionalContext ?? "";
    expect(output.hookSpecificOutput?.hookEventName).toBe("UserPromptSubmit");
    expect(context).toContain("axhub doctor");
    expect(context).toContain("버전 확인");
    expect(context).toContain("오래된 버전");
    expect(context).toContain("업그레이드");
  });

  test("non-axhub prompt → no injected context", () => {
    expect(runPromptRoute(JSON.stringify({ prompt: "오늘 날씨 알려줘" }))).toEqual({});
  });

  test("doctor intent lexicon includes Korean and English variants", () => {
    expect(promptMatchesDoctorIntent("환경 점검해주세요")).toBe(true);
    expect(promptMatchesDoctorIntent("health check axhub")).toBe(true);
    expect(promptMatchesDoctorIntent("그냥 설명해줘")).toBe(false);
  });
});

// ---- resolve --------------------------------------------------------------

describe("runResolve()", () => {
  test("app resolved (slug=paydrop matches one app) → exit 0", () => {
    const runner = makeRunner({
      authStdout: AUTH_OK_JSON,
      appsStdout: APPS_JSON,
      gitBranch: "main",
      gitSha: "a3f9c1b",
      gitMsg: "결제 페이지 버그 수정",
    });
    const { output, exitCode } = runResolve(
      ["--intent", "deploy", "--user-utterance", "paydrop 배포해줘"],
      runner,
    );
    expect(output.candidate_slug).toBe("paydrop");
    expect(output.app_id).toBe(42);
    expect(output.app_slug).toBe("paydrop");
    expect(output.matched_apps).toEqual([{ id: 42, slug: "paydrop" }]);
    expect(output.branch).toBe("main");
    expect(output.commit_sha).toBe("a3f9c1b");
    expect(output.commit_message).toBe("결제 페이지 버그 수정");
    expect(output.eta_sec).toBe(60);
    expect(output.error).toBeNull();
    expect(exitCode).toBe(0);
  });

  test("app ambiguous (slug=app matches 2+ apps) → exit 64, error=app_ambiguous", () => {
    const runner = makeRunner({
      authStdout: AUTH_OK_JSON,
      appsStdout: APPS_JSON,
    });
    const { output, exitCode } = runResolve(
      ["--intent", "deploy", "--user-utterance", "app-3 배포해줘"],
      runner,
    );
    expect(output.candidate_slug).toBe("app-3");
    expect(output.matched_apps.length).toBeGreaterThanOrEqual(2);
    expect(output.matched_apps.map((a) => a.slug).sort()).toEqual(["app-3", "app-3-staging"]);
    expect(output.app_id).toBeNull();
    expect(output.error).toBe("app_ambiguous");
    expect(exitCode).toBe(64);
  });

  test("app not found (slug=zzz) → exit 67, error=app_not_found", () => {
    const runner = makeRunner({
      authStdout: AUTH_OK_JSON,
      appsStdout: APPS_JSON,
    });
    const { output, exitCode } = runResolve(
      ["--intent", "deploy", "--user-utterance", "zzznonexistent 배포해줘"],
      runner,
    );
    expect(output.candidate_slug).toBe("zzznonexistent");
    expect(output.app_id).toBeNull();
    expect(output.error).toBe("app_not_found");
    expect(exitCode).toBe(67);
  });

  test("auth missing → exit 65", () => {
    const runner = makeRunner({
      authStdout: AUTH_ERR_JSON,
      appsStdout: APPS_JSON,
    });
    const { output, exitCode } = runResolve(
      ["--user-utterance", "paydrop 배포해"],
      runner,
    );
    expect(output.error).toBe("auth_validation.endpoint_required");
    expect(output.app_id).toBeNull();
    expect(exitCode).toBe(65);
  });

  test("no candidate slug extractable → exit 67, error=no_candidate_slug", () => {
    const runner = makeRunner({
      authStdout: AUTH_OK_JSON,
      appsStdout: APPS_JSON,
    });
    const { output, exitCode } = runResolve(
      ["--user-utterance", "배포해 그거"],
      runner,
    );
    expect(output.candidate_slug).toBeNull();
    expect(output.error).toBe("no_candidate_slug");
    expect(exitCode).toBe(67);
  });
});

// ---- extractSlugCandidate -------------------------------------------------

describe("extractSlugCandidate()", () => {
  test("paydrop 배포해줘 → paydrop", () => {
    expect(extractSlugCandidate("paydrop 배포해줘")).toBe("paydrop");
  });

  test("배포해줘 paydrop → paydrop (mid-sentence)", () => {
    expect(extractSlugCandidate("배포해줘 paydrop")).toBe("paydrop");
  });

  test("ship paydrop now → paydrop", () => {
    expect(extractSlugCandidate("ship paydrop now")).toBe("paydrop");
  });

  test("Korean-only stop words → null", () => {
    expect(extractSlugCandidate("배포해 그거")).toBeNull();
  });

  test("empty utterance → null", () => {
    expect(extractSlugCandidate("")).toBeNull();
  });

  test("punctuation stripped: 'paydrop, ship!' → paydrop", () => {
    expect(extractSlugCandidate("paydrop, ship!")).toBe("paydrop");
  });
});

// ---- filterAppsBySlug -----------------------------------------------------

describe("filterAppsBySlug()", () => {
  const apps: AppRecord[] = [
    { id: 1, slug: "paydrop" },
    { id: 2, slug: "paydrop-v2" },
    { id: 3, slug: "ccrank" },
  ];

  test("exact prefix match (paydrop) → both paydrop variants", () => {
    const hits = filterAppsBySlug(apps, "paydrop");
    expect(hits.map((a) => a.slug).sort()).toEqual(["paydrop", "paydrop-v2"]);
  });

  test("substring fallback (rank) → ccrank", () => {
    const hits = filterAppsBySlug(apps, "rank");
    expect(hits).toEqual([{ id: 3, slug: "ccrank" }]);
  });

  test("no match → empty array", () => {
    expect(filterAppsBySlug(apps, "zzz")).toEqual([]);
  });
});
