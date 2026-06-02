import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

const read = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("vibe gap-fill SKILL contracts", () => {
  test("apps skill covers current read inventory surfaces", () => {
    const skill = read("skills/apps/SKILL.md");

    expect(skill).toContain("### apps owned / workspace / members");
    expect(skill).toContain("axhub apps owned --json");
    expect(skill).toContain("axhub apps workspace --json");
    expect(skill).toContain('axhub apps members "$APP" --page "$PAGE" --per-page "$PER_PAGE" --json');
    expect(skill).toContain("have no pagination flags in v0.17.3");
    expect(skill).toContain(".current_team_id // empty");
    expect(skill).toContain('axhub apps list --tenant "$TEAM_ID" --json');
    expect(skill).toContain("current profile scoped");
    expect(skill).not.toContain("$AXHUB_TEAM_ID");
    expect(skill).not.toContain("team_id does not match");
  });

  test("team skill access commands are grounded by the trailing-var source audit", () => {
    const skill = read("skills/team/SKILL.md");
    const audit = read("specs/007-vibe-skill-gapfill/source-audit.md");

    for (const command of [
      'axhub access check --app "$APP_ID" --json',
      'axhub access grant --app "$APP_ID" --json',
      'axhub access revoke --app "$APP_ID" --execute --json',
      'axhub access invite --app "$APP_ID" --user "$USER_ID" --execute --json',
      'axhub access uninvite --app "$APP_ID" --user "$USER_ID" --execute --json',
    ]) {
      expect(skill).toContain(command);
    }

    expect(audit).toContain("Dynamic trailing-var command audit");
    expect(audit).toContain("access.rs");
    expect(audit).toContain("access check --app <id>");
    expect(audit).toContain("access grant --app <id>");
    expect(audit).toContain("access revoke --app <id> --execute");
    expect(audit).toContain("access invite --app <id> --user <uuid> --execute");
    expect(audit).toContain("access uninvite --app <id> --user <uuid> --execute");
  });

  test("uses current v0.17.3 grant and row mutation flags", () => {
    const skill = read("skills/tables/SKILL.md");

    expect(skill).toContain(
      'axhub tables grants issue "$TABLE" --app "$APP_ID" --principal-id "$PRINCIPAL_ID" --principal-type user --actions read,write --execute --json',
    );
    expect(skill).toContain('axhub data insert "$TABLE" --app "$APP_ID" --body "$ROW_JSON" --execute --json');
    expect(skill).toContain('axhub data insert "$TABLE" --app "$APP_ID" --batch rows.jsonl --execute --json');
    expect(skill).toContain(
      'axhub data update "$TABLE" "$ROW_ID" --app "$APP_ID" --body "$ROW_JSON" --execute --json',
    );

    expect(skill).not.toContain("--subject");
    expect(skill).not.toContain("--data-file");
  });

  test("publish skill separates consent mint from destructive publish execution", () => {
    const skill = read("skills/publish/SKILL.md");
    const bashBlocks = [...skill.matchAll(/```bash\n([\s\S]*?)```/g)].map((match) => match[1]);

    expect(skill).toContain("다음 Bash 호출에서만 destructive publish 를 실행해요");
    expect(bashBlocks.some((block) => block.includes("consent-mint"))).toBe(true);
    expect(bashBlocks.some((block) => block.includes('axhub publish --app "$APP"'))).toBe(true);
    expect(
      bashBlocks.some((block) => block.includes("consent-mint") && block.includes("axhub publish")),
    ).toBe(false);
  });
});
