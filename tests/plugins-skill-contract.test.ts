import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const REFERENCE_PATH = join("skills", "plugins", "references", "publish.md");
const source = readFileSync(join(ROOT, "skills", "plugins", "SKILL.md"), "utf8");
const reference = readFileSync(join(ROOT, REFERENCE_PATH), "utf8");
const skillText = `${source}\n${reference}`;
const claudeBundle = readFileSync(join(ROOT, "plugins", "axhub", "skills", "plugins", "SKILL.md"), "utf8");
const codexBundle = readFileSync(join(ROOT, "plugins", "axhub-codex", "skills", "plugins", "SKILL.md"), "utf8");
const readme = readFileSync(join(ROOT, "README.md"), "utf8");
const codexReadme = readFileSync(join(ROOT, "codex-overrides", "README.md"), "utf8");
const changelog = readFileSync(join(ROOT, "CHANGELOG.md"), "utf8");
const policy = readFileSync(join(ROOT, "POLICY.md"), "utf8");
const codexPolicy = readFileSync(join(ROOT, "codex-overrides", "POLICY.md"), "utf8");
const agentPolicy = readFileSync(join(ROOT, "docs", "policy", "agent-policy.md"), "utf8");

// 부록 B "CLI next_action" 열 — CLI `plugin manage status --app` 가 내는 state 와
// 같은 first-match 순서예요. skill 은 이 표를 그대로 옮겨야 해요.
const APPENDIX_B: ReadonlyArray<readonly [state: string, action: string]> = [
  ["takedown", "none"],
  ["delisted", "publish_new_version"],
  ["apply_failed", "contact_support"],
  ["visibility_limited", "request_visibility_review"],
  ["servable", "install_plugin"],
  ["applying", "wait_for_publish"],
  ["ingest_failed", "fix_and_publish_new_version"],
  ["gate_blocked_secret", "fix_and_publish_new_version"],
  ["gate_blocked", "review_gate_findings"],
  ["rejected", "fix_and_publish_new_version"],
  ["closed", "submit_plugin_version_for_review"],
  ["gate_slow", "wait_for_review"],
  ["gate_running", "wait_for_review"],
  ["admin_pending", "wait_for_review"],
  ["other_version_in_review", "wait_for_review"],
  ["uploading", "wait_for_ingest"],
  ["not_submitted", "submit_plugin_version_for_review"],
];
const APPENDIX_B_ACTIONS = new Set(APPENDIX_B.map(([, action]) => action));
// 동사형 접두사를 가진 backtick 식별자는 next_action 이름으로 읽혀요. 아래는
// 같은 접두사를 쓰지만 action 이 아닌 응답 필드·상태 값이에요.
const ACTION_LIKE = /^(?:none|(?:submit|publish|fix|review|request|install|wait|contact|inspect|issue)_[a-z_]+)$/;
const NON_ACTION_FIELDS: Record<string, true> = {
  review_submitted: true,
  review_request_id: true,
  install_name: true,
  request_id: true,
};

// reference 의 ```bash 블록 안 axhub 명령을 줄 이음(`\`)을 풀어 한 줄씩 모아요.
const referenceCommands = [...reference.matchAll(/```bash\n([\s\S]*?)```/g)]
  .flatMap((match) => match[1].replace(/\\\n\s*/g, "").split("\n"))
  .map((line) => line.trim())
  .filter((line) => line.startsWith("axhub "));

describe("app-centric plugins skill contract", () => {
  test("owns list, exact download, host install, and publish while yielding app deployment", () => {
    for (const trigger of [
      "플러그인 목록 보여줘",
      "플러그인 1.2.0 내려 받아",
      "Claude에 설치해줘",
      "플러그인으로 올려",
      "플러그인 심사 상태",
    ]) {
      expect(source, trigger).toContain(trigger);
    }
    expect(source).toContain("앱 배포는 deploy");
  });

  test("uses ordinary App routes and excludes retired plugin settings/admin surfaces", () => {
    expect(source).toContain("/discovery?type=plugin");
    expect(source).toContain("/apps/<slug>/console");
    expect(source).toContain("/console/review");
    expect(skillText).not.toContain("/settings/integrations/plugin");
    expect(skillText).not.toContain("plugin admin");
  });

  test("lists paginated plugin Apps with the final nested version summary", () => {
    expect(source).toContain("axhub plugin list --page 1 --per-page 20 --json");
    expect(source).toContain("pagination.page/per_page/total");
    expect(source).toContain("`next_cursor`는 생략돼요");
    expect(source).toContain("plugin.current_servable_version");
    expect(source).toContain("summary object");
    expect(source).toContain("`owner`를 publisher로 바꿔 부르지 않아요");
    expect(source).toContain("한 번에 전량 fetch하지 않아요");
  });

  test("keeps headless read-only list and exact download available", () => {
    expect(source).toContain("headless에서도 목록·exact download는 실행해요");
    expect(source).toContain("install·publish execute만 preview에서 멈춰요");
    expect(source).not.toContain("tenant marketplace 대상인지 판정하고 headless면 멈춰요");
  });

  test("downloads one immutable semver with verified no-clobber output", () => {
    expect(source).toContain('axhub plugin download \\\n  --app "<slug-or-UUID>" \\\n  --version "<exact-semver>"');
    expect(source).toContain("`latest`·생략 version·mutable name-only download는 금지예요");
    expect(source).toContain("size·SHA-256 검증 → atomic no-clobber");
    expect(source).toContain("app_id·app_slug·install_name·version·output·size_bytes·sha256");
    expect(source).toContain("`version_id`를 만들거나 보고하지 않아요");
    expect(source).toContain("받은 code를 자동 실행하거나 unzip하지 않고");
  });

  test("installs one exact version through the official Claude or Codex plugin CLI", () => {
    expect(source).toContain('axhub plugin install \\\n  --app "<slug-or-UUID>" \\\n  --version "<exact-semver>" \\\n  --host "<claude|codex>"');
    expect(source).toContain("--execute --yes");
    expect(source).toContain("traversal·symlink·duplicate path·archive bomb");
    expect(source).toContain("AxHub 관리 local marketplace");
    expect(source).toContain("host 공식 plugin CLI");
    expect(source).toContain("status=installed");
    expect(source).toContain("restart_required=true");
    expect(source).toContain("download 요청과 install 요청을 구분");
    expect(source).toContain("네이티브 선택 UI 가 있으면 그걸로 묻고");
    expect(source).toContain("없으면 같은 확인을 명시 텍스트 승인 1회로 받고");
  });

  test("shows the served version and risk inventory before install approval without claiming verification (P2-30)", () => {
    const install = source.slice(source.indexOf("## 3. Host install"), source.indexOf("## 4. Publish input"));
    const riskSummary = install.indexOf("`plugin.risk_inventory`");
    const question = install.indexOf("user scope에 설치할까요?");
    expect(install).toContain('axhub plugin list --q "<slug>" --json');
    expect(install).toContain("`plugin.current_servable_version.version`");
    expect(riskSummary).toBeGreaterThanOrEqual(0);
    expect(riskSummary).toBeLessThan(question);
    expect(install).toContain("`검사 안 됨`");
    expect(install).toContain("`없음`이 아니에요");
    expect(skillText).not.toContain("검증된");
  });

  test("publishes, submits for 게시 심사, and treats only servable as installable (P1-2)", () => {
    expect(source).toContain('axhub plugin publish "<path>" --app "<slug-or-UUID>" --release-version "<new-semver>" --json');
    expect(source).toContain("canonical `--app <slug|UUID>`");
    expect(source).toContain("`review_submitted`/`upload_complete` + `installable=false`");
    expect(source).toContain("`axhub plugin submit`(next_action `submit_plugin_version_for_review`)");
    expect(source).toContain('axhub plugin manage status --app "<app>" --json');
    expect(source).toContain("`servable`일 때만 §3으로 설치해요");
    expect(source).toContain("UUID v4 `--idempotency-key`를 고정해 `--attest-distribution-rights --execute --submit` 한 번");
    expect(source).toContain("업로드한 뒤 게시 심사를 신청할까요?");
    for (const stale of ["review_ready", "review-ready", "`published`/`installable=true`"]) {
      expect(skillText, stale).not.toContain(stale);
    }

    const publish = referenceCommands.find((command) => command.startsWith("axhub plugin publish"));
    expect(publish).toBeDefined();
    for (const flag of ['--idempotency-key "<UUID-v4>"', "--attest-distribution-rights", "--execute", "--submit", "--json"]) {
      expect(publish, flag).toContain(flag);
    }
    const submit = referenceCommands.find((command) => command.startsWith("axhub plugin submit"));
    expect(submit).toBe(
      'axhub plugin submit --tenant "<tenant-UUID>" --app "<slug-or-UUID>" --release-version "<semver>" --execute --json',
    );
    expect(reference).toContain("`data.status=review_submitted` + `installable=false`");
    expect(reference).toContain("`data.status=upload_complete` + `installable=false`");
    expect(reference).toContain("`already_submitted=true`");
    expect(reference).toContain("publish를 다시 실행하지 말고 `next_action.command` 의 submit만 다시 실행해요");
    expect(reference).toContain("`plugin_version_not_ready`");
    expect(reference).toContain("`--wait` 는 업로드 처리만 기다려요");
    expect(reference).toContain("결과 알림은 없어요");
    expect(reference).toContain("App Console에서 관리하고 Console Review에서 승인돼");
  });

  test("maps every Appendix B state to its canonical next_action (DS1, DX16)", () => {
    const table = reference
      .split("\n")
      .filter((line) => line.startsWith("| `"))
      .map((line) => {
        const cells = line.split("|").map((cell) => cell.trim().replace(/`/g, ""));
        return [cells[1], cells[3]];
      });
    expect(table).toEqual(APPENDIX_B.map(([state, action]) => [state, action]));
    const status = referenceCommands.find((command) => command.startsWith("axhub plugin manage status"));
    expect(status).toBe(
      'axhub plugin manage status --tenant "<tenant-UUID>" --app "<slug-or-UUID>" --release-version "<semver>" --json',
    );
    expect(reference).toContain("`--watch --watch-timeout 5m`");
    expect(reference).toContain("같은 명령을 반복 호출하지 않아요");
    expect(reference).toContain("`surface=app_console` 의 `url` 을 사람에게 넘겨요");
  });

  test("names only Appendix B next_action actions", () => {
    const used = [...skillText.matchAll(/`([^`\n]+)`/g)]
      .map((match) => match[1])
      .filter((token) => ACTION_LIKE.test(token) && !NON_ACTION_FIELDS[token]);
    expect(used.length).toBeGreaterThan(0);
    for (const action of used) {
      expect(APPENDIX_B_ACTIONS.has(action), action).toBe(true);
    }
    for (const retired of ["submit_publication_review", "inspect_gate_rejection_and_publish_new_version", "issue_plugin_pat"]) {
      expect(skillText, retired).not.toContain(retired);
    }
  });

  test("uses one term for each review kind (DS4)", () => {
    expect(source).toContain("게시 심사");
    expect(reference).toContain("공개범위 변경 심사");
    expect(skillText).not.toContain("공개 신청");
  });

  test("keeps preview, rights, auth boundaries, idempotency, and artifact-secret gates", () => {
    expect(source).toContain("network=false");
    expect(source).toContain("OAuth 또는 active broad PAT");
    expect(source).toContain("Publish execute에는 OAuth나 broad PAT 대신");
    expect(source).toContain("plugins:read` + `plugins:write");
    expect(reference).toContain('--idempotency-key "<UUID-v4>"');
    expect(reference).toContain("retry는 같은 key·payload로만 해요");
    expect(source).toContain("artifact를 배포할 권리가 있음을 확인하고");
    expect(source).toContain("미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요");
    expect(source).toContain("`.env`, `*.pem`, `*.key`, `id_rsa*`");
    expect(source.match(/네이티브 선택 UI 가 있으면 그걸로 묻고/g)?.length).toBe(2);
  });

  test("recovers a missing publish PAT with a stored scoped PAT, never a printed token (P2-31, DX12)", () => {
    expect(source).toContain("exit 4·`plugin_pat_required`는 `auth login`·onboarding이 아니라 publish.md의 PAT 복구로 풀어요");
    expect(reference).toContain(
      'axhub auth pat issue --scopes plugins:read,plugins:write --tenant-id "<tenant-UUID>" --use',
    );
    expect(skillText).not.toContain("--show-token");
    for (const line of skillText.split("\n").filter((line) => line.includes("auth pat issue"))) {
      expect(line, line).toContain("--use");
    }
    const apiKeyFileLines = skillText.split("\n").filter((line) => line.includes("--api-key-file"));
    expect(apiKeyFileLines.length).toBeGreaterThan(0);
    for (const line of apiKeyFileLines) {
      expect(line, line).toContain("사람이 repo 밖에 직접 만들어 둔 PAT 파일");
    }
    expect(reference).toContain("`auth login`·onboarding으로 보내지 않아요");
    expect(reference).toContain("`axhub auth pat unset`");
  });

  test("keeps tenant handling explicit for scoped PATs (P3-31)", () => {
    expect(source).toContain("목록의 `--tenant`를 download·install에도 그대로 붙여요");
    expect(source).toContain("`tenants[].tenant_id`");
    expect(source).toContain("`plugin_tenant_uuid_required`면 UUID로 다시 실행해요");
    const mutatingOrStatus = referenceCommands.filter((command) =>
      /^axhub plugin (publish|submit|manage status)\b/.test(command),
    );
    expect(mutatingOrStatus.length).toBe(3);
    for (const command of mutatingOrStatus) {
      expect(command, command).toContain('--tenant "<tenant-UUID>"');
    }
  });

  test("connects a hosted marketplace through the credential helper without tokens in URLs (DX13)", () => {
    expect(source).toContain("host marketplace 연결 요청은 publish.md §5를 따라요");
    expect(reference).toContain('axhub --tenant "<tenant>" git setup --marketplace --json');
    expect(reference).toContain("GIT_TERMINAL_PROMPT=0 git ls-remote");
    expect(reference).toContain("`add_commands`");
    expect(reference).toContain("`marketplace_url` 에 token을 넣거나");
  });

  test("guards the 0.50.0 CLI surface by capability and names the update command (P2-29, DX11)", () => {
    const guard = source.indexOf("**CLI 가드(0.50.0+):**");
    expect(guard).toBeGreaterThanOrEqual(0);
    expect(guard).toBeLessThan(source.indexOf("## 1. 목록"));
    const guardText = source.slice(guard, source.indexOf("\n", guard));
    expect(guardText).toContain("모든 mode 전에");
    expect(guardText).toContain("`axhub plugin submit --help` 성공");
    expect(guardText).toContain("`axhub plugin publish --help`의 `--submit`");
    expect(guardText).toContain("axhub update apply --execute --yes");
    expect(guardText).toContain("0.50.0 이상이 필요해요");
    expect(guardText).toContain("update로 양보해요");
    expect(source).not.toContain("요청 mode의 public help");
  });

  test("keeps AP-17 continuation and the Windows contract", () => {
    expect(source).toContain("이후 명령은 반환된 `bin_path`");
    expect(source).toContain("`cargo run`, `target/debug/axhub`");
    expect(source).toContain("`bin_path`가 없으면 앞에서 발견한 절대경로");
    expect(source).toContain("세 경로 모두 없을 때만 onboarding");
    expect(source).toContain("plugin-support repair-path --json");
    expect(source).toContain("Windows 실행 계약 (AP-13)");
    expect(source).toContain("axhub 명령은 Git Bash 전용으로 실행해요");
  });

  test("references publish.md from SKILL.md and keeps it host-neutral", () => {
    expect(existsSync(join(ROOT, REFERENCE_PATH))).toBe(true);
    expect(source).toContain("[references/publish.md](references/publish.md)");
    for (const hostSpecific of ["claude plugin", "Claude Code", "Claude Desktop"]) {
      expect(reference, hostSpecific).not.toContain(hostSpecific);
    }
  });

  test("keeps public and policy sources on the final auth and review contract", () => {
    for (const document of [readme, codexReadme, changelog, policy, codexPolicy, agentPolicy]) {
      expect(document).toContain("App Console");
      expect(document).toContain("Console Review");
      expect(document).toContain("plugins:read");
      expect(document).not.toContain("/settings/integrations/plugin");
      expect(document).not.toContain("plugin admin");
    }
    for (const document of [readme, codexReadme, policy, codexPolicy, agentPolicy]) {
      expect(document).toContain("plugin install");
      expect(document).toContain("upload_complete");
      expect(document).toContain("review_submitted");
      expect(document).toContain("axhub plugin submit");
      expect(document).toContain("axhub plugin manage status");
      expect(document).not.toContain("review_ready");
      expect(document).not.toContain("--show-token");
    }
    expect(policy).toContain("~/.axhub/plugins/");
    expect(codexPolicy).toContain("~/.axhub/plugins/");
    expect(agentPolicy).toContain("download 요청과 install 요청을 구분");
    expect(agentPolicy).toContain("AP-21 app-backed plugin marketplace");
  });

  test("discloses every persistent and recoverable install-state path", () => {
    for (const document of [policy, codexPolicy]) {
      for (const path of [
        "<host>/.install.lock",
        ".marketplace-transaction.json",
        ".marketplace-host-mutating.json",
        ".marketplace-rollback-pending.json",
        ".marketplace-host-installed.json",
        ".marketplace-staging-",
        ".marketplace-backup-",
        "AXHUB_PLUGIN_HOME",
      ]) {
        expect(document, path).toContain(path);
      }
    }
  });

  test("generated bundles stay synced and complete under Codex 8KB", () => {
    expect(claudeBundle).toBe(source);
    for (const bundle of ["axhub", "axhub-codex"]) {
      const bundled = join(ROOT, "plugins", bundle, REFERENCE_PATH);
      expect(existsSync(bundled), bundled).toBe(true);
      expect(readFileSync(bundled, "utf8"), bundled).toBe(reference);
    }
    expect(Buffer.byteLength(codexBundle, "utf8")).toBeLessThan(8_000);
    expect(codexBundle).not.toContain("네이티브 선택 UI");
    expect(codexBundle).toContain("Claude에 설치해줘");
    expect(codexBundle).toContain("Codex에 설치해줘");
    for (const sentinel of [
      "plugin list",
      "plugin download",
      "plugin install",
      "upload_complete",
      "review_submitted",
      "axhub plugin submit --help",
      "submit_plugin_version_for_review",
      "[references/publish.md](references/publish.md)",
    ]) {
      expect(codexBundle, sentinel).toContain(sentinel);
    }
  });
});
