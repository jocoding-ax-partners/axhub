import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const source = readFileSync(join(ROOT, "skills", "plugins", "SKILL.md"), "utf8");
const claudeBundle = readFileSync(join(ROOT, "plugins", "axhub", "skills", "plugins", "SKILL.md"), "utf8");
const codexBundle = readFileSync(join(ROOT, "plugins", "axhub-codex", "skills", "plugins", "SKILL.md"), "utf8");
const readme = readFileSync(join(ROOT, "README.md"), "utf8");
const codexReadme = readFileSync(join(ROOT, "codex-overrides", "README.md"), "utf8");
const changelog = readFileSync(join(ROOT, "CHANGELOG.md"), "utf8");
const policy = readFileSync(join(ROOT, "POLICY.md"), "utf8");
const codexPolicy = readFileSync(join(ROOT, "codex-overrides", "POLICY.md"), "utf8");
const agentPolicy = readFileSync(join(ROOT, "docs", "policy", "agent-policy.md"), "utf8");

describe("app-centric plugins skill contract", () => {
  test("owns list, exact download, host install, and publish while yielding app deployment", () => {
    for (const trigger of [
      "플러그인 목록 보여줘",
      "플러그인 1.2.0 내려 받아",
      "Claude에 설치해줘",
      "플러그인으로 올려",
    ]) {
      expect(source, trigger).toContain(trigger);
    }
    expect(source).toContain("앱 배포는 deploy");
  });

  test("uses ordinary App routes and excludes retired plugin settings/admin surfaces", () => {
    expect(source).toContain("/discovery?category=plugin");
    expect(source).toContain("/apps/<slug>/console");
    expect(source).toContain("/console/review");
    expect(source).not.toContain("/settings/integrations/plugin");
    expect(source).not.toContain("plugin admin");
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

  test("publishes an App-backed release into the existing app review flow", () => {
    expect(source).toContain('axhub plugin publish "<path>" --app "<slug-or-UUID>" --release-version "<new-semver>" --json');
    expect(source).toContain("canonical `--app <slug|UUID>`");
    expect(source).toContain("`review_ready`/`installable=false`");
    expect(source).toContain("submit_plugin_version_for_review");
    expect(source).toContain("App Console에서 제출하고 Console Review에서 승인");
    expect(source).not.toContain("`published`/`installable=true`");
  });

  test("keeps preview, rights, auth boundaries, idempotency, and artifact-secret gates", () => {
    expect(source).toContain("network=false");
    expect(source).toContain("OAuth 또는 active broad PAT");
    expect(source).toContain("Publish execute에는 OAuth나 broad PAT 대신");
    expect(source).toContain("plugins:read` + `plugins:write");
    expect(source).toContain('--idempotency-key "<UUID-v4>"');
    expect(source).toContain("artifact를 배포할 권리가 있음을 확인하고");
    expect(source).toContain("미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요");
    expect(source).toContain("`.env`, `*.pem`, `*.key`, `id_rsa*`");
    expect(source.match(/네이티브 선택 UI 가 있으면 그걸로 묻고/g)?.length).toBe(2);
  });

  test("requires released public CLI commands and AP-17 continuation", () => {
    for (const command of ["plugin list", "plugin download", "plugin install", "plugin publish"]) {
      expect(source, command).toContain(`axhub ${command} --help`);
    }
    expect(source).toContain("이후 명령은 반환된 `bin_path`");
    expect(source).toContain("`cargo run`, `target/debug/axhub`");
    expect(source).toContain("`bin_path`가 없으면 앞에서 발견한 절대경로");
    expect(source).toContain("세 경로 모두 없을 때만 onboarding");
    expect(source).toContain("plugin-support repair-path --json");
    expect(source).toContain("Windows 실행 계약 (AP-13)");
    expect(source).toContain("axhub 명령은 Git Bash 전용으로 실행해요");
  });

  test("keeps public and policy sources on the final auth and review contract", () => {
    for (const document of [readme, codexReadme, changelog, policy, codexPolicy, agentPolicy]) {
      expect(document).toContain("review_ready");
      expect(document).toContain("App Console");
      expect(document).toContain("Console Review");
      expect(document).toContain("plugins:read");
      expect(document).not.toContain("/settings/integrations/plugin");
      expect(document).not.toContain("plugin admin");
    }
    for (const document of [readme, codexReadme, policy, codexPolicy, agentPolicy]) {
      expect(document).toContain("plugin install");
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
    expect(Buffer.byteLength(codexBundle, "utf8")).toBeLessThan(8_000);
    expect(codexBundle).not.toContain("네이티브 선택 UI");
    expect(codexBundle).toContain("Claude에 설치해줘");
    expect(codexBundle).toContain("Codex에 설치해줘");
    for (const sentinel of [
      "plugin list",
      "plugin download",
      "plugin install",
      "review_ready",
      "submit_plugin_version_for_review",
      "--idempotency-key",
    ]) {
      expect(codexBundle, sentinel).toContain(sentinel);
    }
  });
});
