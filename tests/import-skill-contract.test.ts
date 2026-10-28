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
  "selfhosted_repo_push",
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
  // self-hosted 정적 앱은 저장소 push → webhook deployment 라 deployment 증거예요. 업로드 static 만 release 증거예요.
  const selfhostedPush = (value.required_mutations as unknown[]).includes("selfhosted_repo_push");
  const deploymentEvidence = value.deploy_method !== "static" || selfhostedPush;
  if (evidence.kind === "deployment") {
    if (!deploymentEvidence) return { ok: false, reason: "evidence method mismatch" };
    return isNonEmptyString(evidence.deployment_id) && evidence.verification_status === "success" && isNonEmptyString(evidence.public_url)
      ? { ok: true }
      : { ok: false, reason: "bad deployment evidence" };
  }
  if (evidence.kind === "static_release") {
    if (deploymentEvidence) return { ok: false, reason: "evidence method mismatch" };
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
    expect(skill).toMatch(/사용자에게는 tenant\/테넌트 대신 `작업공간`이라고만/);
    expect(skill).toMatch(/`--tenant` 없이 execute 후.*`axhub tenants list`.*복구 금지/);
    expect(skill).toMatch(/선택 전 preview 승인·execute 금지/);
  });

  test("uses standalone Desktop commands and one bounded verify invocation", () => {
    const skill = readImportSkill();
    expect(skill).toContain("현재 선택된 프로젝트가 이미 정확히 `APP_DIR` 이면 `cd ... &&` 를 붙이지 않고");
    expect(skill).toContain("한 카드에 한 개의 bare 명령만");
    expect(skill).toContain("Monitor, background task, ScheduleWakeup, output 파일 읽기");
    expect(skill).toContain("`for`/`while`/`until`, `sleep`, command substitution, pipe, `grep`, `head`, `status=`");
    expect(skill).toContain("`capabilities.import.early_return` 과 `capabilities.import.verify_wait` 가 모두 true");
    expect(skill).toContain("axhub deploy verify <deployment-id> --app <app> --wait --wait-interval 20s --wait-timeout 9m --json");
    expect(skill).toContain("내부 폴링 예산은 최대 30회 또는 10분(AP-16)");
    expect(skill).toContain("정확히 한 번 호출");
    expect(skill).toContain("같은 verify 를 반복 호출하거나 `axhub apps get`, `deploy list`, `deploy status` 같은 사후 확인을 덧붙이지 않아요");
  });

  test("preview approval carries the commit choice without a second question", () => {
    const skill = readImportSkill();
    expect(skill).toContain("## AskUserQuestion JSON 안전 규칙");
    expect(skill).toContain("question: `지금 이대로 axhub에 올려서 인터넷 주소를 만들까요?`");
    expect(skill).toContain("option 1 label: `올리기 (추천)`");
    expect(skill).toContain("option 2 label: `설정은 나중에 올리기`");
    expect(skill).toContain("option 3 label: `먼저 고칠게요`");
    expect(skill).toContain("option 4 label: `그만두기`");
    expect(skill).toContain("manifest 검증 뒤에는 다시 묻지 않아요");
    expect(skill).not.toContain("axhub.yaml을 커밋하고 푸시할까요?");
    expect(skill).not.toContain("커밋·push 하고 진행 (첫 배포부터 반영)");
  });

  test("first-deploy copy speaks in outcomes, not files or commits", () => {
    const skill = readImportSkill();
    expect(skill).toContain("사용자 지침에 다른 보고 양식(특이사항·수정·결과 등)이 있어도 이 요약은 아래 형태로 쓰고");
    expect(skill).toContain("`첫 배포가 끝났어요. 주소: https://...`");
    expect(skill).toContain("통합 승인 질문 바로 앞 요약도 결과로만 써요");
    expect(skill).not.toContain("axhub.yaml 을 프로젝트 파일 근거로 자세히 작성할 예정이라고");
    expect(skill).not.toContain("운영 URL: https://");
  });

  test("static auto-detection asks the deploy method before the single approval", () => {
    const skill = readFileSync(join(REPO_ROOT, "skills", "import", "SKILL.md"), "utf8");
    const methodCard = skill.indexOf("5-1. 배포 방식 카드");
    const approval = skill.indexOf("6. 대화형 통합 승인 1회");
    expect(methodCard).toBeGreaterThan(-1);
    expect(methodCard).toBeLessThan(approval);

    const section = skill.slice(methodCard, approval);
    expect(section).toContain("`deploy_method` 가 `static` 이고 `required_mutations` 에 `app_create`");
    expect(section).toContain("`어떤 방식으로 배포할까요?`");
    expect(section).toContain("`정적 사이트 (추천)`(`HTML·빌드 결과만 올려요`)");
    expect(section).toContain("`서버 앱`(`서버 코드까지 실행해요`)");
    // 서버 앱 주소는 작업공간 종류(회사 `{sub}.{tenant}.axhub.ai`·개인 `{sub}.axhub.app`)로
    // 갈리고 preview 는 그 종류를 알려주지 않아요. `axhub.dev` 는 스테이징이라 어느 쪽도 아니에요.
    expect(section).toContain("주소 도메인은 적지 않아요");
    expect(section).not.toMatch(/axhub\.(dev|ai|app|page)/);
    expect(section).toContain("`detected_state.manifest_hints.start_cmd` 가 있을 때만 `--deploy-method docker`");
    expect(section).toContain("headless 는 묻지 않아요");
  });

  test("local-only GitHub import includes commit choice in the single approval", () => {
    const skill = readImportSkill();
    expect(skill).toContain("local_only 앱은 아직 git remote 가 없을 수 있지만");
    expect(skill).toContain("remote 없음만으로 이 선택지를 빼지 않아요");
    expect(skill).not.toContain("git remote 가 있으면 커밋 동의");
    expect(skill).toContain("`올리기` 를 골랐으면 검증 뒤 다시 묻지 않고");
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
          selfhosted: true,
          selfhosted_static: true,
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
  test("self-hosted static execute succeeds with webhook deployment evidence, not a static release", () => {
    const { result } = runShim(["plugin-support", "import", "--mode", "execute", "--approved", "--json"], { AXHUB_FIXTURE_IMPORT: "selfhosted_static_success" });
    expect(result.exitCode).toBe(0);
    const envelope = parseStdout(result.stdout);
    expect(validateImportEnvelope(envelope)).toMatchObject({ ok: true });
    expect(envelope).toMatchObject({
      deploy_method: "static",
      required_mutations: ["manifest_create", "app_create", "selfhosted_repo_push", "first_deploy"],
      result: { evidence: { kind: "deployment", deployment_id: "dep-fixture", verification_status: "success" } },
    });

    const staticRelease = parseStdout(runShim(["plugin-support", "import", "--mode", "execute", "--approved", "--json"], { AXHUB_FIXTURE_IMPORT: "selfhosted_static_with_static_evidence" }).result.stdout);
    expect(validateImportEnvelope(staticRelease)).toMatchObject({ ok: false, reason: "evidence method mismatch" });
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

// spec 681 FR-P03·FR-P09 (AP-30): 첫 올리기는 승인 전에 점검하고 확인 1회에 모아요.
describe("첫 올리기 점검 (AP-30)", () => {
  const skill = readFileSync(join(REPO_ROOT, "skills", "import", "SKILL.md"), "utf8");
  const development = readFileSync(join(REPO_ROOT, "skills", "development", "SKILL.md"), "utf8");

  test("점검은 미리보기 뒤·통합 승인 앞에 와요", () => {
    const check = skill.indexOf("5-2. 첫 올리기 점검 (AP-30)");
    const approval = skill.indexOf("6. 대화형 통합 승인 1회");
    expect(check).toBeGreaterThan(skill.indexOf("5. Preview 카드 렌더링"));
    expect(approval).toBeGreaterThan(check);
    const block = skill.slice(check, approval);
    expect(block).toContain("capabilities.import.env_from_dotenv");
    expect(block).toContain("axhub deploy check --json");
    expect(block).toContain("capabilities.readiness.context_url");
    expect(block).toContain("승인 카드 한 화면");
    expect(block).toContain("값은 대화로 받지 않고");
    expect(block).toContain("--env-from-dotenv .env --env-only <이름,…>");
    expect(block).toContain(".axhub/deploy-answers.json");
    expect(block).toContain("이렇게 맞춰서 올렸어요.");
  });

  test("development 는 코드를 쓰기 전에 같은 기준 문서로 6가지를 피해요", () => {
    const rule = development.indexOf("7.5. **AXHub 배포 기준 (AP-30).**");
    expect(rule).toBeGreaterThan(-1);
    expect(development.indexOf("9. **코드 생성")).toBeGreaterThan(rule);
    expect(development).toContain("SQLite·파일 대신 AXHub DB");
  });
});

// #830·#832: CLI 가 낸 typed 실패는 원인별로 복구가 달라요 — 패키지 매니저를 못 띄운 건 설치 뒤
// 같은 옵션 재실행(빌드 스크립트 수정·AP-19 리포트 아님), 주소 없는 기존 앱은 주소를 확인받고 승인 뒤
// apps update 후 같은 import 재실행. #757: manifest name 은 앱 slug 라야 deploy-prep 이 같은 앱을 찾아요.
describe("import typed failure recovery (#830·#832·#757)", () => {
  const skill = readFileSync(join(REPO_ROOT, "skills", "import", "SKILL.md"), "utf8");
  const row = (name: string): string => skill.split("\n").find((line) => line.startsWith(`| \`${name}\` |`)) ?? "";

  test("static: 패키지 매니저를 못 띄운 건 설치 뒤 같은 옵션으로 다시 실행해요", () => {
    const staticRow = row("static");
    expect(staticRow).toContain("static.package_manager_missing");
    expect(staticRow).toContain("static.package_manager_spawn_failed");
    // recovery_action 은 `npm install -g pnpm` 같은 전역 설치라 사용자 시스템을 바꿔요 — 동의 뒤에만 해요.
    expect(staticRow).toContain("recovery_action 의 설치는 사용자 동의를 받은 뒤에 하고(headless 는 안내만 하고 멈춰요) 같은 옵션으로 다시 실행");
    expect(staticRow).toContain("AP-19 리포트 대상이 아니에요");
    // 빌드 스크립트·산출물 경로 수정 안내는 그 밖의 retryable=false 에만 남아요.
    expect(staticRow.indexOf("static.package_manager_missing")).toBeLessThan(staticRow.indexOf("그 밖의 retryable=false"));
  });

  test("app: 주소 없는 기존 앱은 주소를 확인받고 승인 뒤 apps update 후 같은 import 를 다시 실행해요", () => {
    const appRow = row("app");
    expect(appRow).toContain("app.subdomain_not_configured");
    expect(appRow).toContain("첫 운영 배포 뒤에는 바꿀 수 없으니");
    expect(appRow).toContain("axhub apps update <slug> --subdomain <주소>");
    expect(appRow).toContain("같은 import 를 다시 실행");
    expect(appRow).toContain("axhub deploy list --app <slug>");
    // slug 는 고른 작업공간 안에서 해석해야 해요. `--tenant` 가 없으면 활성 작업공간에서 다시 찾아
    // 못 찾거나 같은 slug 의 다른 앱 주소를 바꿀 수 있어요 — CLI recovery_action 처럼 같은 `--tenant` 를 붙여요.
    expect(appRow).toContain("axhub apps update <slug> --subdomain <주소> --tenant <t>");
    expect(appRow).toContain("axhub deploy list --app <slug> --tenant <t>");
  });

  test("manifest name 은 표시 이름이 아니라 앱 slug 예요 (#757)", () => {
    const authoring = readFileSync(join(REPO_ROOT, "skills", "import", "references", "manifest-authoring.md"), "utf8");
    expect(authoring).not.toContain("`name`: 표시 이름(package.json name 또는 폴더 이름)");
    expect(authoring).toContain("`name`: 이 import 의 앱 slug");
    expect(authoring).toContain("`metadata.name`");
  });
});
