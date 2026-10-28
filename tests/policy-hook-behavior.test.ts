import { afterEach, describe, expect, test } from "bun:test";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const wrapper = resolve(import.meta.dir, "../hooks/policy-context.sh");
const roots: string[] = [];
function fixture() {
  const root = mkdtempSync(join(tmpdir(), "axhub-policy-hook-"));
  roots.push(root);
  const home = join(root, "home");
  const project = join(root, "project");
  mkdirSync(home); mkdirSync(project); mkdirSync(join(project, ".git"));
  return { root, home, project };
}
function bind(project: string) {
  mkdirSync(join(project, ".axhub/state"), { recursive: true });
  writeFileSync(join(project, ".axhub/state/developer-policy.json"), "{}");
}
function run(home: string, project: string, event = "UserPromptSubmit", env: Record<string, string> = {}, payload = JSON.stringify({ cwd: project, hook_event_name: event })) {
  return Bun.spawnSync(["/bin/bash", wrapper, event], {
    cwd: project, stdin: Buffer.from(payload),
    env: { HOME: home, PATH: "/usr/bin:/bin", ...env },
    timeout: 2500,
  });
}
// Only a process boundary fixture: strict protocol parsing belongs to the actual
// CLI adapter and its Rust tests, not a second parser implemented by the plugin.
function cli(home: string, body: string) {
  const bin = join(home, ".axhub/bin"); mkdirSync(bin, { recursive: true });
  writeFileSync(join(bin, "axhub"), `#!/bin/bash\n${body}\n`);
  chmodSync(join(bin, "axhub"), 0o755);
}
afterEach(() => { for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true }); });

describe("policy wrapper process boundary", () => {
  test("unbound projects need no CLI", () => {
    const f = fixture(); const r = run(f.home, f.project);
    expect(r.exitCode).toBe(0); expect(r.stdout.length).toBe(0);
  });
  test("old CLI exit64 skips an unbound project without leaking usage output", () => {
    const f = fixture();
    cli(f.home, 'echo private-secret; echo private-secret >&2; exit 64');
    const r = run(f.home, f.project);
    expect(r.exitCode).toBe(0);
    expect(r.stdout.length).toBe(0);
    expect(r.stderr.length).toBe(0);
  });
  test("new CLI protocol failure still blocks an unbound process cwd", () => {
    const f = fixture();
    cli(f.home, 'exit 2');
    const r = run(f.home, f.project);
    expect(r.exitCode).toBe(2);
    expect(r.stdout.length).toBe(0);
    expect(r.stderr.toString()).toContain("PROTOCOL");
  });
  test.each(["directory", "worktree file"])("does not inherit binding across a %s Git boundary", kind => {
    const f = fixture(); bind(f.project);
    const nested = join(f.project, "nested"); mkdirSync(nested);
    if (kind === "directory") mkdirSync(join(nested, ".git"));
    else writeFileSync(join(nested, ".git"), "gitdir: /not-read-by-wrapper");
    expect(run(f.home, nested).exitCode).toBe(0);
  });
  test("nearest parent binding blocks missing CLI", () => {
    const f = fixture(); bind(f.project); const child = join(f.project, "src"); mkdirSync(child);
    const r = run(f.home, child);
    expect(r.exitCode).toBe(2); expect(r.stdout.length).toBe(0);
    expect(r.stderr.toString()).toContain("UNSUPPORTED");
  });
  test("broken binding symlink is connected, never a silent unbound skip", () => {
    const f = fixture(); mkdirSync(join(f.project, ".axhub/state"), { recursive: true });
    symlinkSync("/absent-policy-binding", join(f.project, ".axhub/state/developer-policy.json"));
    expect(run(f.home, f.project).exitCode).toBe(2);
  });
  test("opt-out exact env or marker happens before CLI invocation", () => {
    const f = fixture(); bind(f.project);
    cli(f.home, 'echo private-secret >&2; exit 91');
    expect(run(f.home, f.project, "UserPromptSubmit", { AXHUB_NO_POLICY_SYNC: "1" }).exitCode).toBe(0);
    expect(run(f.home, f.project, "UserPromptSubmit", { AXHUB_NO_POLICY_SYNC: "true" }).exitCode).toBe(2);
    mkdirSync(join(f.home, ".axhub/config"));
    writeFileSync(join(f.home, ".axhub/config/no-policy-sync"), "");
    const r = run(f.home, f.project); expect(r.exitCode).toBe(0); expect(r.stderr.length).toBe(0);
  });
  test("old adapter exit64 becomes sanitized unsupported block", () => {
    const f = fixture(); bind(f.project);
    cli(f.home, 'echo private-secret; echo private-secret >&2; exit 64');
    const r = run(f.home, f.project);
    expect(r.exitCode).toBe(2); expect(r.stdout.length).toBe(0);
    expect(r.stderr.toString()).toContain("UNSUPPORTED");
    expect(r.stderr.toString()).not.toContain("private-secret");
  });
  test.each(["SessionStart", "UserPromptSubmit"])("only emits accepted adapter output for %s", event => {
    const f = fixture(); bind(f.project);
    cli(f.home, `if [ "$3" = "--adapt" ]; then
  IFS= read -r input
  [ "$input" = protocol-v1 ] || exit 2
  printf '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"%s","additionalContext":"current revision"}}\\n' "$4"
else
  printf 'protocol-v1\\n'
fi`);
    const r = run(f.home, f.project, event);
    expect(r.exitCode).toBe(0);
    expect(JSON.parse(r.stdout.toString()).hookSpecificOutput).toEqual({ hookEventName: event, additionalContext: "current revision" });
  });
  test("adapter failure never leaks rejected stdout or stderr", () => {
    const f = fixture(); bind(f.project);
    cli(f.home, 'echo private-secret; echo private-secret >&2; exit 2');
    const r = run(f.home, f.project);
    expect(r.exitCode).toBe(2); expect(r.stdout.length).toBe(0);
    expect(r.stderr.toString()).not.toContain("private-secret");
  });
});

const realCli = process.env.AXHUB_POLICY_TEST_BINARY;
describe.skipIf(!realCli)("policy wrapper with shipped CLI adapter", () => {
  function withProtocol(protocol: string, event = "UserPromptSubmit") {
    const f = fixture(); bind(f.project);
    const protocolPath = join(f.root, "protocol.json");
    writeFileSync(protocolPath, protocol);
    cli(f.home, `if [ "$3" = "--adapt" ]; then
  exec "$REAL_POLICY_CLI" plugin-support policy-context --adapt "$4"
else
  cat "$POLICY_PROTOCOL_FILE"
fi`);
    return run(f.home, f.project, event, {
      REAL_POLICY_CLI: realCli!,
      POLICY_PROTOCOL_FILE: protocolPath,
    });
  }
  test("unbound process cwd cannot bypass a bound payload cwd", () => {
    const f = fixture();
    const payloadProject = join(f.root, "bound-project");
    mkdirSync(payloadProject); mkdirSync(join(payloadProject, ".git"));
    // A deliberately invalid binding must fail closed before any network read.
    bind(payloadProject);
    cli(f.home, 'exec "$REAL_POLICY_CLI" "$@"');
    const r = run(f.home, f.project, "UserPromptSubmit", { REAL_POLICY_CLI: realCli! },
      JSON.stringify({ cwd: payloadProject, hook_event_name: "UserPromptSubmit" }));
    expect(r.exitCode).toBe(0);
    expect(JSON.parse(r.stdout.toString()).decision).toBe("block");
  });
  test("available CLI rejects malformed host input even from an unbound process cwd", () => {
    const f = fixture();
    cli(f.home, 'exec "$REAL_POLICY_CLI" "$@"');
    const r = run(f.home, f.project, "UserPromptSubmit", { REAL_POLICY_CLI: realCli! }, "malformed-host-json");
    const blocked = r.exitCode === 2 || (r.exitCode === 0 && JSON.parse(r.stdout.toString()).decision === "block");
    expect(blocked).toBe(true);
    expect(r.stderr.toString()).not.toContain("malformed-host-json");
  });
  const verified = {
    schema_version: 1, outcome: "verified", context: "Revision 7 replaces earlier policy.",
    tenant_id: "00000000-0000-4000-8000-000000000001", revision: 7, state: "published",
  };
  test.each(["SessionStart", "UserPromptSubmit"])("delivers full policy through the real adapter for %s", event => {
    const r = withProtocol(JSON.stringify(verified), event);
    expect(r.exitCode).toBe(0);
    expect(JSON.parse(r.stdout.toString()).hookSpecificOutput).toEqual({
      hookEventName: event, additionalContext: verified.context,
    });
  });
  test.each([
    "", "not JSON", JSON.stringify(verified) + "\n{}", JSON.stringify(verified) + "\nprivate-secret",
    JSON.stringify({ ...verified, schema_version: 2 }),
    JSON.stringify({ ...verified, revision: -1 }),
    JSON.stringify({ ...verified, revision: "7" }),
    JSON.stringify({ ...verified, tenant_id: "wrong" }),
    JSON.stringify({ ...verified, context: "" }),
    JSON.stringify({ ...verified, context: "x".repeat(32769) }),
    JSON.stringify({ ...verified, outcome: "unknown" }),
    JSON.stringify({ schema_version: 1, outcome: "skip", reason: "unknown" }),
    JSON.stringify({ schema_version: 1, outcome: "blocked", error: { code: "NETWORK" } }),
  ])("blocks invalid protocol without disclosing payload (%#)", protocol => {
    const r = withProtocol(protocol);
    expect(r.exitCode).toBe(2);
    expect(r.stdout.length).toBe(0);
    expect(r.stderr.toString()).not.toContain("private-secret");
  });
  test("skip produces no host output", () => {
    const r = withProtocol('{"schema_version":1,"outcome":"skip","reason":"unbound"}');
    expect(r.exitCode).toBe(0); expect(r.stdout.length).toBe(0);
  });
  test("network failure blocks the submitted prompt", () => {
    const r = withProtocol(JSON.stringify({
      schema_version: 1, outcome: "blocked",
      error: { code: "NETWORK", message: "Policy unavailable.", recovery: "Repair network then resend." },
    }));
    expect(r.exitCode).toBe(0);
    expect(JSON.parse(r.stdout.toString()).decision).toBe("block");
  });
  test("SessionStart failure informs without acting as the final freshness gate", () => {
    const r = withProtocol(JSON.stringify({
      schema_version: 1, outcome: "blocked",
      error: { code: "NETWORK", message: "Policy unavailable.", recovery: "Repair network then resend." },
    }), "SessionStart");
    expect(r.exitCode).toBe(0);
    const host = JSON.parse(r.stdout.toString());
    expect(host.decision).not.toBe("block");
    expect(host.systemMessage || host.hookSpecificOutput?.additionalContext).toBeTruthy();
  });
  test.each([
    { state: "unset", revision: 0, context: "No current organizational policy; replaces earlier context." },
    { state: "withdrawn", revision: 8, context: "Revision 8 withdraws and replaces all earlier policy." },
  ])("delivers explicit $state context instead of silently skipping", policy => {
    const r = withProtocol(JSON.stringify({ ...verified, ...policy }));
    expect(r.exitCode).toBe(0);
    expect(JSON.parse(r.stdout.toString()).hookSpecificOutput.additionalContext).toBe(policy.context);
  });
});
