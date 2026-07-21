import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readImportSkill = (): string =>
  readFileSync(join(REPO_ROOT, "skills", "import", "SKILL.md"), "utf8") +
  readFileSync(join(REPO_ROOT, "skills", "import", "references", "visibility-rules.md"), "utf8") +
  readFileSync(join(REPO_ROOT, "skills", "import", "references", "manifest-authoring.md"), "utf8");

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
  if (evidence === null) {
    if (value.mode === "execute" && value.error === null) return { ok: false, reason: "execute without evidence" };
    return { ok: true };
  }
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
  test("resolves the workspace before preview and never retries an unscoped mutation", () => {
    const skill = readImportSkill();
    expect(skill).toContain("2. 작업공간 확정");
    expect(skill).toContain("axhub plugin-support tenant-resolve --json");
    expect(skill).toContain("앱을 어느 작업공간에 만들까요?");
    expect(skill).toContain("사용자-facing 문구에 `tenant` 또는 `테넌트`를 쓰지 않아요");
    expect(skill).toContain("`--tenant` 없는 execute 를 먼저 호출해 오류를 낸 뒤");
    expect(skill).toContain("작업공간 선택 전에는 preview 승인이나 execute mutation을 시작하지 않아요");
  });

  test("uses standalone Desktop commands and one bounded verify invocation", () => {
    const skill = readImportSkill();
    expect(skill).toContain("현재 선택된 프로젝트가 이미 정확히 `APP_DIR` 이면 `cd ... &&` 를 붙이지 않고");
    expect(skill).toContain("한 카드에 한 개의 bare 명령만");
    expect(skill).toContain("Monitor, background task, ScheduleWakeup, output 파일 읽기");
    expect(skill).toContain("`for`/`while`/`until`, `sleep`, command substitution, pipe, `grep`, `head`, `status=`");
    expect(skill).toContain("`capabilities.import.early_return` 과 `capabilities.import.verify_wait` 가 모두 true");
    expect(skill).toContain("axhub deploy verify <deployment-id> --app <app> --wait --wait-interval 20s --wait-timeout 10m --json");
    expect(skill).toContain("내부 폴링 예산은 최대 30회 또는 10분(AP-16)");
    expect(skill).toContain("정확히 한 번 호출");
    expect(skill).toContain("같은 verify 를 반복 호출하거나 `axhub apps get`, `deploy list`, `deploy status` 같은 사후 확인을 덧붙이지 않아요");
  });

  test("preview approval carries the commit choice without a second question", () => {
    const skill = readImportSkill();
    expect(skill).toContain("## AskUserQuestion JSON 안전 규칙");
    expect(skill).toContain("question: `이 앱을 axhub에 가져와서 미리보기대로 진행할까요?`");
    expect(skill).toContain("option 1 label: `설정도 반영하고 시작`");
    expect(skill).toContain("option 2 label: `커밋 없이 시작`");
    expect(skill).toContain("option 3 label: `먼저 수정할게요`");
    expect(skill).toContain("option 4 label: `취소`");
    expect(skill).toContain("manifest 검증 뒤에는 다시 묻지 않아요");
    expect(skill).not.toContain("axhub.yaml을 커밋하고 푸시할까요?");
    expect(skill).not.toContain("커밋·push 하고 진행 (첫 배포부터 반영)");
  });

  test("local-only GitHub import includes commit choice in the single approval", () => {
    const skill = readImportSkill();
    expect(skill).toContain("local_only 앱은 아직 git remote 가 없을 수 있지만");
    expect(skill).toContain("remote 없음만으로 이 선택지를 빼지 않아요");
    expect(skill).not.toContain("git remote 가 있으면 커밋 동의");
    expect(skill).toContain("`설정도 반영하고 시작` 을 골랐으면 검증 뒤 다시 묻지 않고");
  });

  test("uses the exact import option contract and never invents private", () => {
    const skill = readImportSkill();
    expect(skill).toContain("`--mode`, `--headless`, `--approved`, `--commit-manifest`, `--verify-wait`");
    expect(skill).toContain("정확히 `--repo-private` 를 쓰고");
    expect(skill).toContain("존재하지 않는 `--private` 를 절대 붙이지 않아요");
    expect(skill).toContain("`plugin-support import --help` 호출로 이어가지 말고 멈춰요");
    expect(skill).toContain('--repo "$GITHUB_OWNER/$REPO_NAME" --repo-private --tenant "$TENANT"');
  });

  test("manifest authoring removes direct axhub.yaml ignore rule", () => {
    const skill = readImportSkill();
    expect(skill).toContain("## Manifest 보강");
    expect(skill).toContain("무시 중이면 `.gitignore` 또는 `.git/info/exclude`");
    expect(skill).toContain("axhub.yaml 을 쓰기 전에 반드시 먼저");
    expect(skill).toContain("bare `git check-ignore -q axhub.yaml`");
    expect(skill).toContain("현재 Desktop 프로젝트가 `APP_DIR` 이면 `cd`, `&&`, `;`, `echo $?`를 붙이지 않아요");
    expect(skill).toContain("`axhub.yaml` 을 직접 무시하는 줄을 제거");
    expect(skill).toContain("`git check-ignore -q axhub.yaml` 이 실패");
    expect(skill).toContain("아직 무시되면 axhub.yaml 을 쓰거나");
    expect(skill).toContain("검증 통과 직후, execute 전에");
    expect(skill).toContain("아직 무시되면 execute 를 호출하지 말고");
    expect(skill).toContain("`.gitignore` 변경도 같은 커밋에 들어가야");
    expect(skill).toContain("실패한 execute 뒤에 뒤늦게 고치는 복구 흐름으로 두지 않아요");
  });

  test("provided app directory pins all import commands to APP_DIR", () => {
    const skill = readImportSkill();
    expect(skill).toContain("## 작업 폴더 실행 계약");
    expect(skill).toContain("그 절대 경로를 `APP_DIR` 로 고정해요");
    expect(skill).toContain("모든 import 관련 `axhub`, `git`, `npm`, build, manifest 검증 명령은 반드시 `APP_DIR` 안에서 실행");
    expect(skill).toContain('명령 앞에 실제 절대 경로를 넣은 `cd "<absolute APP_DIR>" &&` 를 붙여요');
    expect(skill).toContain("권한 카드에는 `$APP_DIR` 변수를 그대로 쓰지 말고 사용자가 준 실제 경로를 따옴표로 넣어요");
    expect(skill).toContain("workspace root 에서 `axhub --json plugin-support import`");
    expect(skill).toContain("권한 카드 명령이 bare `axhub ...`, bare `git ...`, bare `npm ...` 로 시작하거나");
    expect(skill).toContain('실제 절대 경로가 들어간 `cd "<absolute APP_DIR>" &&`');
    expect(skill).toContain("아래 Workflow 명령 예시는 모두 tool cwd 가 `APP_DIR` 로 지정된 bare 명령이에요");
    expect(skill).toContain("axhub plugin-support preflight --json");
    expect(skill).toContain("axhub deploy --explain --json");
    expect(skill).toContain('axhub --json plugin-support import --mode preview --slug "$APP_SLUG" --tenant "$TENANT"');
    expect(skill).toContain("axhub --json plugin-support import --mode execute --approved --commit-manifest");
    expect(skill).not.toContain('cd "<absolute APP_DIR>" && axhub plugin-support preflight --json');
    expect(skill).not.toContain('cd "<absolute APP_DIR>" && axhub deploy --explain --json');
  });

  test("repo owner without repo name keeps the repo name equal to app slug", () => {
    const skill = readImportSkill();
    expect(skill).toContain("GitHub owner 만 명시되고 repo 이름이 따로 없으면 `$REPO_NAME` 은 반드시 `$APP_SLUG` 와 정확히 같게 둬요");
    expect(skill).toContain('`--repo "realitsyourman/uqa-exp-public-11021-0708"`');
    expect(skill).toContain('`--repo "realitsyourman/uqa-exp-public-11021"` 처럼 줄인 명령은 실행하지 말고');
  });

  test("desktop tool titles and final URLs avoid generated product verbs", () => {
    const skill = readImportSkill();
    expect(skill).toContain("도구 제목에는 제품명 `axhub` 자체도 넣지 않아요");
    expect(skill).toContain("`axhubing`, `axhubed`, `axhub import 기능 지원 확인`, `axhub 가져오기 기능 지원 확인`");
    expect(skill).toContain("`axhubed import 기능 지원 확인`");
    expect(skill).toContain("`Expressing 앱 파일 확인`");
    expect(skill).toContain("`Expressing ...`, `Expressed ...`, `FastAPIing ...`, `axhubed ...`");
    expect(skill).toContain("스택 이름을 영어 동사처럼 만든 제목");
    expect(skill).toContain("Claude Desktop 자동 제목에 맡기지 않아요");
    expect(skill).toContain("`서버 앱 준비 확인`");
    expect(skill).toContain("`가져오기 기능 확인`");
    expect(skill).toContain("최종 성공 요약에서도 URL 은 반드시 평문 절대 URL 로만 보여줘요");
    expect(skill).toContain("`배포 URL: [https://...](...)`, `[열기](https://...)` 같은 Markdown 링크 문법을 쓰지 않아요");
  });

  test("backend existing app utterances route directly to import", () => {
    const skill = readImportSkill();
    const frontmatter = skill.slice(0, skill.indexOf("---", 4));
    expect(frontmatter).toContain("Use this before bootstrap for any existing/local-folder app import or first deploy");
    expect(skill).toContain("기존 Express 서버 앱을 axhub에 올려서 실제 배포");
    expect(skill).toContain("이미 만든 앱");
    expect(skill).toContain("이 앱을 axhub에 올려");
    expect(skill).toContain("작업 폴더는 /path");
    expect(skill).toContain("Express/Fastify/Nest/FastAPI/Flask/Django/Rails/Go/Rust/Java/PHP/.NET");
    expect(skill).toContain("기존 Express 서버 앱을 axhub에 올려서 실제 배포까지 해줘");
    expect(skill).toContain("작업 폴더는 /path/to/app 이야. axhub에 올려줘");
  });

  test("preflight fixture advertises import/v1 capability", () => {
    const { result } = runShim(["plugin-support", "preflight", "--json"]);
    expect(result.exitCode).toBe(0);
    expect(parseStdout(result.stdout)).toMatchObject({
      capabilities: {
        import: {
          supported: true,
          schemas: ["import/v1"],
          commit_manifest: true,
          early_return: true,
          verify_wait: true,
        },
      },
    });
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

  test("execute envelope with null evidence and no error is rejected", () => {
    const { result } = runShim(["plugin-support", "import", "--mode", "execute", "--approved", "--json"], { AXHUB_FIXTURE_IMPORT: "execute_success" });
    const envelope = parseStdout(result.stdout) as Record<string, unknown>;
    const stripped = { ...envelope, error: null, result: { ...(envelope.result as Record<string, unknown>), evidence: null } };
    expect(validateImportEnvelope(stripped)).toMatchObject({ ok: false, reason: "execute without evidence" });
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
