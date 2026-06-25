import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

type JsonObject = Record<string, unknown>;

const runShim = (args: string[], env: Record<string, string> = {}) => {
  const caseDir = mkdtempSync(join(tmpdir(), "axhub-import-shim-"));
  const result = Bun.spawnSync({
    cmd: [join(REPO_ROOT, "tests/e2e/claude-cli/fixtures/bin/axhub"), ...args],
    env: {
      ...process.env,
      SHIM_CASE_DIR: caseDir,
      ...env,
    },
  });
  return { result, caseDir };
};

const parseStdout = (stdout: Uint8Array): unknown => JSON.parse(Buffer.from(stdout).toString("utf8"));

const REQUIRED_MUTATIONS = new Set([
  "manifest_create",
  "manifest_migrate",
  "manifest_repair",
  "app_create",
  "app_select",
  "github_repo_create",
  "github_connect",
  "first_deploy",
  "static_release",
]);
const TYPED_FAILURES = new Set(["auth", "version", "manifest", "git", "repo", "app", "static", "deploy", "rate_limit", "transport"]);
const OWNERS = new Set(["plugin", "cli", "backend"]);
const PHASES = new Set(["preflight", "detect", "preview", "approval", "manifest", "app", "repo", "git", "deploy", "verify", "static", "finalize"]);
const DEPLOY_METHODS = new Set(["docker", "compose", "static"]);
const STARTING_STATES = new Set(["local_github_no_axhub_app", "local_only", "existing_axhub_app_repair"]);

const isRecord = (value: unknown): value is JsonObject => typeof value === "object" && value !== null && !Array.isArray(value);
const isNonEmptyString = (value: unknown): value is string => typeof value === "string" && value.length > 0;

const validateImportEnvelope = (value: unknown): { ok: boolean; reason?: string } => {
  if (!isRecord(value)) return { ok: false, reason: "not object" };
  if (value.schema_version !== "import/v1") return { ok: false, reason: "unknown schema" };
  if (value.mode !== "preview" && value.mode !== "execute") return { ok: false, reason: "bad mode" };
  if (typeof value.headless !== "boolean") return { ok: false, reason: "bad headless" };
  if (!isNonEmptyString(value.correlation_id)) return { ok: false, reason: "missing correlation" };
  if (!DEPLOY_METHODS.has(String(value.deploy_method))) return { ok: false, reason: "unknown deploy method" };
  if (!Array.isArray(value.required_mutations) || value.required_mutations.some((m) => !REQUIRED_MUTATIONS.has(String(m)))) {
    return { ok: false, reason: "unknown mutation" };
  }
  if (!isRecord(value.detected_state) || !isRecord(value.preview) || !isRecord(value.approval) || !isRecord(value.result)) {
    return { ok: false, reason: "missing object" };
  }
  if (!STARTING_STATES.has(String(value.detected_state.starting_state))) return { ok: false, reason: "unknown starting state" };
  if (!isNonEmptyString(value.preview.title)) return { ok: false, reason: "bad preview title" };
  if (!Array.isArray(value.preview.summary) || !Array.isArray(value.preview.safety_notes)) return { ok: false, reason: "bad preview copy" };
  if (!Array.isArray(value.preview.mutations) || value.preview.mutations.some((m) => !REQUIRED_MUTATIONS.has(String(m)))) {
    return { ok: false, reason: "bad preview mutation" };
  }
  if (typeof value.approval.required !== "boolean" || typeof value.approval.approved !== "boolean" || typeof value.approval.interactive_only !== "boolean") {
    return { ok: false, reason: "bad approval" };
  }
  if (value.error !== null) {
    if (!isRecord(value.error)) return { ok: false, reason: "bad error" };
    if (!TYPED_FAILURES.has(String(value.error.typed_failure))) return { ok: false, reason: "unknown typed failure" };
    if (!OWNERS.has(String(value.error.owner))) return { ok: false, reason: "unknown owner" };
    if (!PHASES.has(String(value.error.phase))) return { ok: false, reason: "unknown phase" };
    if (typeof value.error.retryable !== "boolean" || typeof value.error.mutation_performed !== "boolean") return { ok: false, reason: "bad error flags" };
    if (!isNonEmptyString(value.error.recovery_action) || !isNonEmptyString(value.error.message_ko)) return { ok: false, reason: "bad recovery copy" };
  }
  const evidence = value.result.evidence;
  if (value.mode === "preview" && evidence !== null) return { ok: false, reason: "preview evidence" };
  if (evidence !== null && value.error !== null) return { ok: false, reason: "evidence with error" };
  if (evidence === null) return { ok: true };
  if (!isRecord(evidence)) return { ok: false, reason: "bad evidence" };
  if (evidence.kind === "deployment") {
    if (value.deploy_method === "static") return { ok: false, reason: "evidence method mismatch" };
    return isNonEmptyString(evidence.deployment_id) && evidence.verification_status === "success" && isNonEmptyString(evidence.public_url)
      ? { ok: true }
      : { ok: false, reason: "bad deployment evidence" };
  }
  if (evidence.kind === "static_release") {
    if (value.deploy_method !== "static") return { ok: false, reason: "evidence method mismatch" };
    return isNonEmptyString(evidence.active_release_id) && evidence.verified === true && isNonEmptyString(evidence.public_url)
      ? { ok: true }
      : { ok: false, reason: "bad static evidence" };
  }
  return { ok: false, reason: "unknown evidence" };
};

describe("import skill contract", () => {
  test("preflight fixture advertises import/v1 capability", () => {
    const { result } = runShim(["plugin-support", "preflight", "--json"]);
    expect(result.exitCode).toBe(0);
    expect(parseStdout(result.stdout)).toMatchObject({ capabilities: { import: { supported: true, schemas: ["import/v1"] } } });
  });

  test("preview envelope is valid and uses one CLI import invocation", () => {
    const { result, caseDir } = runShim(["plugin-support", "import", "--mode", "preview", "--json"]);
    expect(result.exitCode).toBe(0);
    const envelope = parseStdout(result.stdout);
    expect(validateImportEnvelope(envelope)).toMatchObject({ ok: true });
    expect(envelope).toMatchObject({ schema_version: "import/v1", mode: "preview", error: null, result: { evidence: null } });
    const argvLog = readFileSync(join(caseDir, "axhub-argv.log"), "utf8").trim().split("\n");
    expect(argvLog.filter((line) => line.includes("plugin-support import"))).toHaveLength(1);
  });

  test("headless execute request returns preview semantics without approval", () => {
    const { result } = runShim(["plugin-support", "import", "--mode", "execute", "--headless", "--approved", "--json"]);
    expect(result.exitCode).toBe(0);
    const envelope = parseStdout(result.stdout);
    expect(validateImportEnvelope(envelope)).toMatchObject({ ok: true });
    expect(envelope).toMatchObject({ mode: "preview", headless: true, approval: { approved: false }, result: { evidence: null } });
  });

  test("execute success has method-specific static evidence", () => {
    const { result } = runShim(["plugin-support", "import", "--mode", "execute", "--approved", "--json"], { AXHUB_FIXTURE_IMPORT: "execute_success" });
    expect(result.exitCode).toBe(0);
    const envelope = parseStdout(result.stdout);
    expect(validateImportEnvelope(envelope)).toMatchObject({ ok: true });
    expect(envelope).toMatchObject({ result: { evidence: { kind: "static_release", active_release_id: "rel-fixture", verified: true, public_url: "https://paydrop.axhub.dev" } } });
  });
  test("execute path uses one import invocation after approval", () => {
    const { result, caseDir } = runShim(["plugin-support", "import", "--mode", "execute", "--approved", "--json"], { AXHUB_FIXTURE_IMPORT: "execute_success" });
    expect(result.exitCode).toBe(0);
    expect(validateImportEnvelope(parseStdout(result.stdout))).toMatchObject({ ok: true });
    const argvLog = readFileSync(join(caseDir, "axhub-argv.log"), "utf8").trim().split("\n");
    expect(argvLog.filter((line) => line.includes("plugin-support import"))).toHaveLength(1);
  });

  test("execute without approval returns a non-mutating approval failure", () => {
    const { result, caseDir } = runShim(["plugin-support", "import", "--mode", "execute", "--json"]);
    expect(result.exitCode).toBe(0);
    const envelope = parseStdout(result.stdout);
    expect(validateImportEnvelope(envelope)).toMatchObject({ ok: true });
    expect(envelope).toMatchObject({ result: { evidence: null }, error: { owner: "plugin", phase: "approval", mutation_performed: false } });
    const argvLog = readFileSync(join(caseDir, "axhub-argv.log"), "utf8").trim().split("\n");
    expect(argvLog.filter((line) => line.includes("plugin-support import"))).toHaveLength(1);
  });


  test("malformed, unknown schema, and unknown enum are rejected by contract validator", () => {
    const malformed = runShim(["plugin-support", "import", "--mode", "preview", "--json"], { AXHUB_FIXTURE_IMPORT: "malformed" }).result;
    expect(() => parseStdout(malformed.stdout)).toThrow();

    const unknownSchema = parseStdout(runShim(["plugin-support", "import", "--mode", "preview", "--json"], { AXHUB_FIXTURE_IMPORT: "unknown_schema" }).result.stdout);
    expect(validateImportEnvelope(unknownSchema)).toMatchObject({ ok: false, reason: "unknown schema" });

    const unknownEnum = parseStdout(runShim(["plugin-support", "import", "--mode", "preview", "--json"], { AXHUB_FIXTURE_IMPORT: "unknown_enum" }).result.stdout);
    expect(validateImportEnvelope(unknownEnum)).toMatchObject({ ok: false, reason: "unknown deploy method" });
  });
  test("preview evidence and deploy-method evidence mismatches are rejected", () => {
    const previewEvidence = parseStdout(runShim(["plugin-support", "import", "--mode", "preview", "--json"], { AXHUB_FIXTURE_IMPORT: "preview_with_evidence" }).result.stdout);
    expect(validateImportEnvelope(previewEvidence)).toMatchObject({ ok: false, reason: "preview evidence" });

    const staticDeploymentEvidence = parseStdout(runShim(["plugin-support", "import", "--mode", "execute", "--approved", "--json"], { AXHUB_FIXTURE_IMPORT: "static_with_deployment_evidence" }).result.stdout);
    expect(validateImportEnvelope(staticDeploymentEvidence)).toMatchObject({ ok: false, reason: "evidence method mismatch" });

    const dockerStaticEvidence = parseStdout(runShim(["plugin-support", "import", "--mode", "execute", "--approved", "--json"], { AXHUB_FIXTURE_IMPORT: "docker_with_static_evidence" }).result.stdout);
    expect(validateImportEnvelope(dockerStaticEvidence)).toMatchObject({ ok: false, reason: "evidence method mismatch" });
  });

  test("static verified false is rejected as strict static evidence failure", () => {
    const envelope = parseStdout(runShim(["plugin-support", "import", "--mode", "execute", "--approved", "--json"], { AXHUB_FIXTURE_IMPORT: "static_unverified" }).result.stdout);
    expect(validateImportEnvelope(envelope)).toMatchObject({ ok: false, reason: "bad static evidence" });
  });

  test("static missing URL maps to typed Korean recovery without success evidence", () => {
    const { result } = runShim(["plugin-support", "import", "--mode", "execute", "--approved", "--json"], { AXHUB_FIXTURE_IMPORT: "static_missing_url" });
    expect(result.exitCode).toBe(0);
    const envelope = parseStdout(result.stdout);
    expect(validateImportEnvelope(envelope)).toMatchObject({ ok: true });
    expect(envelope).toMatchObject({ result: { evidence: null }, error: { typed_failure: "static", owner: "backend", phase: "static", message_ko: "정적 사이트 확인 증거가 부족해요. 공개 URL과 활성 릴리스 확인 뒤 다시 시도해요." } });
  });
});
