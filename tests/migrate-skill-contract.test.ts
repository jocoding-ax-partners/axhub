import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

const read = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

interface CorpusRow {
  id: string;
  utterance: string;
  expected_skill: string | null;
  expected_cmd_pattern: string | null;
}

interface BaselineRow {
  utterance_id?: string;
  fired_skill?: string | null;
  actual_tool_calls?: Array<{
    cmd?: string;
    exit_code?: number;
  }>;
}

const parseJsonl = (path: string) =>
  read(path)
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line) as CorpusRow);

const requiredUtterances = [
  "기존 앱 올려줘",
  "migrate this repo",
  "import existing app",
  "이미 만든 앱 배포해줘",
];

describe("migrate SKILL contract", () => {
  test("keeps migration behind CLI/helper boundaries instead of raw backend endpoints", () => {
    const skill = read("skills/migrate/SKILL.md");
    expect(skill).toContain("CLI boundary contract");
    expect(skill).toContain('"$HELPER" migrate-plan --dir "${AXHUB_MIGRATE_DIR:-.}" --json');
    expect(skill).toContain('axhub apps detect --repo "$OWNER_REPO" --ref "$REF" --path "$APP_PATH" --json');
    expect(skill).toContain('axhub apps detect --owner "$OWNER" --repo-name "$REPO" --ref "$REF" --path "$APP_PATH" --json');
    expect(skill).toContain("remote detect 는 현재 CLI 로만 써요");
    expect(skill).toContain("exit `64` 는 local path");
    expect(skill).toContain("axhub apps create --from-file axhub.yaml --json");
    expect(skill).toContain('axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json');
    expect(skill).toContain('axhub deploy create --app "$APP_ID" --commit "$COMMIT_SHA" --execute --json');
    expect(skill).toContain("raw backend endpoint 를 curl 하지");
    expect(skill).not.toContain("/api/v1/apps/detect");
  });

  test("documents the production detect matrix added after live QA", () => {
    const skill = read("skills/migrate/SKILL.md");
    for (const expected of [
      "docker-compose.yml",
      "docker-compose.yaml",
      "compose.yml",
      "compose.yaml",
      "Next.js, Nuxt, SvelteKit, Remix",
      "FastAPI, Django, Flask",
      "Gin, Fiber, Echo, Chi",
      "Sinatra, Ruby on Rails",
      "Maven, Gradle",
    ]) {
      expect(skill).toContain(expected);
    }
  });

  test("keeps Windows helper resolution executable with or without bash", () => {
    const skill = read("skills/migrate/SKILL.md");
    expect(skill).toContain("Git Bash/MSYS bash");
    expect(skill).toContain('if [ ! -x "$HELPER" ] && [ -x "${HELPER}.exe" ]; then HELPER="${HELPER}.exe"; fi');
    expect(skill).toContain("command -v axhub-helpers.exe");
    expect(skill).toContain("bash 가 없고 PowerShell 만 있으면 PowerShell snippet 을 써요");
    expect(skill).toContain('Join-Path $PluginRoot "bin/axhub-helpers.exe"');
    expect(skill).toContain("Get-Command axhub-helpers.exe");
    expect(skill).toContain("& $Helper migrate-plan --dir $MigrateDir --json");
    expect(skill).toContain('"$HELPER" migrate-plan --dir "${AXHUB_MIGRATE_DIR:-.}" --app-path "$APP_PATH" --json');
    expect(skill).toContain("& $Helper migrate-plan --dir $MigrateDir --app-path $env:APP_PATH --json");
  });

  test("corpus.100 routes core existing-app natural language to migrate", () => {
    const rows = parseJsonl("tests/corpus.100.jsonl");
    for (const utterance of requiredUtterances) {
      const row = rows.find((candidate) => candidate.utterance === utterance);
      if (!row) {
        throw new Error(`missing migrate corpus row: ${utterance}`);
      }
      expect(row.expected_skill).toBe("migrate");
      expect(row.expected_cmd_pattern).toContain("axhub-helpers migrate-plan");
    }
  });

  test("committed routing baselines preserve migrate decisions for core utterances", () => {
    const rows = parseJsonl("tests/corpus.100.jsonl").filter((row) =>
      requiredUtterances.includes(row.utterance),
    );
    const ids = rows.map((row) => row.id);
    for (const path of [
      "tests/baseline-results.docs-only.100.json",
      "tests/baseline-results.claude-native.100.json",
    ]) {
      const baseline = JSON.parse(read(path)) as BaselineRow[];
      for (const id of ids) {
        const entry = baseline.find((row) => row.utterance_id === id);
        if (!entry) {
          throw new Error(`missing migrate baseline row: ${path}:${id}`);
        }
        expect(entry.fired_skill).toBe("migrate");
        const expected = rows.find((row) => row.id === id)?.expected_cmd_pattern;
        expect(expected).toBeTruthy();
        const calls = entry.actual_tool_calls ?? [];
        expect(
          calls.some(
            (call) =>
              typeof call.cmd === "string" &&
              new RegExp(expected!.replaceAll(".*", ".*")).test(call.cmd) &&
              call.exit_code === 0,
          ),
        ).toBe(true);
      }
    }
  });

  test("all axhub.yaml examples in migrate SKILL parse as YAML", () => {
    const skill = read("skills/migrate/SKILL.md");
    const yamlBlocks = [...skill.matchAll(/```yaml\n([\s\S]*?)```/g)].map((match) => match[1]);
    const yaml = (Bun as unknown as { YAML: { parse: (input: string) => unknown } }).YAML;
    expect(yamlBlocks.length).toBeGreaterThan(0);
    for (const block of yamlBlocks) {
      expect(() => yaml.parse(block)).not.toThrow();
    }
  });
});
