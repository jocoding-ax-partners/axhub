import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const source = readFileSync(join(REPO_ROOT, "skills", "publishing", "SKILL.md"), "utf8");
const claudeBundle = readFileSync(join(REPO_ROOT, "plugins", "axhub", "skills", "publishing", "SKILL.md"), "utf8");
const codexBundle = readFileSync(join(REPO_ROOT, "plugins", "axhub-codex", "skills", "publishing", "SKILL.md"), "utf8");

describe("publishing skill contract", () => {
  test("owns axhub publishing and yields unrelated deploy/update intents", () => {
    expect(source).toContain('"플러그인 올려줘"');
    expect(source).toContain('"여러 스킬을 하나의 플러그인으로 올려"');
    expect(source).toContain("앱 배포는 deploy");
    expect(source).toContain("CLI·plugin 업데이트는 update");
    expect(source).toContain("axhub 맥락(대화·현재 연결·직전 작업)이 없으면");
  });

  test("models standalone and multi-skill plugin release units", () => {
    expect(source).toContain("standalone은 파일명이 `SKILL.md`");
    expect(source).toContain(".claude-plugin/plugin.json");
    expect(source).toContain("`skills/<slug>/SKILL.md`");
    expect(source).toContain("독립 lifecycle이면 별도 plugin으로 나눠요");
  });

  test("locks exact preview argv for new and existing standalone skills", () => {
    expect(source).toContain('axhub plugin publish "<SKILL.md>" --name "<name>" --release-version "<semver>" --json');
    expect(source).toContain(
      'axhub plugin publish "<SKILL.md>" --plugin-id "<UUID>" --release-version "<new-semver>" --json',
    );
    expect(source).toContain("name/display-name/third-party 금지");
    expect(source).toContain('axhub plugin publish "<plugin-root>" --plugin-id "<UUID>" --json');
    expect(source).toContain(
      'axhub plugin publish "<SKILL.md>" --name "<name>" --release-version "<semver>" --third-party --json',
    );
    expect(source).toContain('axhub plugin publish "<plugin-root>" --third-party --json');
    expect(source).toContain("신규 제3자 preview에 쓴 `--third-party`는 execute에도 그대로 유지하고 `--plugin-id`와 함께 쓰지 않아요");
  });

  test("requires installed public CLI and offline preview", () => {
    expect(source).toContain("`axhub plugin publish --help`가 성공해야 진행해요");
    expect(source).toContain("`cargo run`, `target/debug/axhub`");
    expect(source).toContain("custom ZIP·curl·직접 API");
    expect(source).toContain("같은 session의 이후 모든 auth·publish 명령은 반환된 `bin_path`");
    expect(source).toContain("`data.mode=preview`, `data.network=false`, `data.auth=false`");
  });

  test("binds distribution-rights attestation to the explicit approval", () => {
    expect(source).toContain(
      "이 artifact를 배포할 권리가 있음을 확인하고, 미리보기대로 axhub tenant marketplace에 게시할까요? 진행 또는 취소 로 답해 주세요.",
    );
    expect(source).toContain("사용자가 새로 입력한 `진행`만 권리 attestation과 execute를 함께 승인해요");
    expect(source).toContain("미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요");
    expect(source).toContain("카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요");
    expect(source).toContain("--attest-distribution-rights \\");
  });

  test("uses explicit idempotency and least-privilege PAT without exposing values", () => {
    expect(source).toContain("`plugins:read` + `plugins:write` 두 scope");
    expect(source).toContain("scope를 보여주지 않으므로 증거로 쓰지 않고");
    expect(source).toContain("Server의 `insufficient_scope`가 최종 판정");
    expect(source).toContain('--idempotency-key "<UUID-v4>"');
    expect(source).toContain("동일 payload retry는 같은 Idempotency-Key만 재사용해요");
    expect(source).toContain("--api-key-file");
    expect(source).not.toContain("Bearer <");
    expect(source).toContain("publish PAT만으로 `/me`를 조회할 수 없으므로 `axhub auth status`로 tenant를 추론하지 않아요");
    expect(source).toContain("기존 active/broad PAT도 대신 쓰지 않아요");
  });

  test("fails closed on suspicious artifact paths", () => {
    for (const sentinel of [".env", "*.pem", "*.key", "id_rsa*", "token·secret·credential"]) {
      expect(source, sentinel).toContain(sentinel);
    }
    expect(source).toContain("하나라도 보이면 execute를 금지해요");
    expect(source).toContain("파일을 대신 삭제하지 않아요");
  });

  test("maps success and error envelopes without overstating installability", () => {
    expect(source).toContain("`status=ok`, `data.status=published`, `data.installable=true`");
    expect(source).toContain("`status=ok`, `data.status=awaiting`, `data.installable=false`");
    expect(source).toContain("`status=error`, `error.subcode=plugin_rejected|plugin_failed`");
    expect(source).toContain("`data`의 IDs·version·next_action");
    expect(source).toContain("`published` 외 상태를 installable로 선언하지 않아요");
  });

  test("generated bundles stay synced and the complete Codex skill survives its 8KB window", () => {
    expect(claudeBundle).toBe(source);
    expect(Buffer.byteLength(codexBundle, "utf8")).toBeLessThan(8_000);
    expect(codexBundle).not.toContain("네이티브 선택 UI");
    for (const sentinel of [
      "data.status=published",
      "data.status=awaiting",
      "plugin_rejected|plugin_failed",
      "installable=false",
      "새 immutable version",
      "custom ZIP·curl·직접 API",
    ]) {
      expect(codexBundle, sentinel).toContain(sentinel);
    }
  });
});
