import { afterEach, describe, expect, test } from "bun:test";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const wrapper = resolve(import.meta.dir, "../hooks/policy-context.sh");
const registrations = JSON.parse(readFileSync(resolve(import.meta.dir, "../hooks/hooks.json"), "utf8")).hooks as
  Record<string, Array<{ hooks: Array<{ command: string; timeout?: number }> }>>;
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
  const hook = registrations[event]?.flatMap(group => group.hooks)
    .find(entry => entry.command.includes("hooks/policy-context.sh") && entry.command.endsWith(` ${event}`));
  if (!hook?.timeout) throw new Error(`Missing policy deadline for ${event}`);
  return Bun.spawnSync(["/bin/bash", wrapper, event], {
    cwd: project, stdin: Buffer.from(payload),
    env: { HOME: home, PATH: "/usr/bin:/bin", ...env },
    timeout: hook.timeout * 1000,
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
  test("unsupported CLI becomes a sanitized block in a bound project", () => {
    const f = fixture(); bind(f.project);
    cli(f.home, 'echo private-secret; echo private-secret >&2; exit 64');
    const r = run(f.home, f.project);
    expect(r.exitCode).toBe(2); expect(r.stdout.length).toBe(0);
    expect(r.stderr.toString()).toContain("UNSUPPORTED");
    expect(r.stderr.toString()).not.toContain("private-secret");
  });
  test("CLI failure never leaks rejected stdout or stderr", () => {
    const f = fixture(); bind(f.project);
    cli(f.home, 'echo private-secret; echo private-secret >&2; exit 2');
    const r = run(f.home, f.project);
    expect(r.exitCode).toBe(2); expect(r.stdout.length).toBe(0);
    expect(r.stderr.toString()).not.toContain("private-secret");
  });
});

const realCli = process.env.AXHUB_POLICY_TEST_BINARY;
describe.skipIf(!realCli)("policy wrapper with shipped CLI hook mode", () => {
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
  test.each(["SessionStart", "UserPromptSubmit"])("slow CLI startup preserves the %s failure protocol", event => {
    const f = fixture(); bind(f.project);
    cli(f.home, 'sleep 2.3\nexec "$REAL_POLICY_CLI" "$@"');
    const r = run(f.home, f.project, event, { REAL_POLICY_CLI: realCli! });
    expect(r.exitCode).toBe(0);
    const host = JSON.parse(r.stdout.toString());
    if (event === "UserPromptSubmit") {
      expect(host.decision).toBe("block");
      expect(host.reason).toContain("#binding-conflict");
    } else {
      expect(host.decision).toBeUndefined();
      expect(host.systemMessage || host.hookSpecificOutput?.additionalContext).toContain("#binding-conflict");
    }
  }, 15000);
});
