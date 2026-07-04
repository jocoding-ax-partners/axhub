import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("clarity device flow UX contract", () => {
  test("surfaces the verification URL and user code in normal chat text", () => {
    const clarity = readRepo("skills/clarity/SKILL.md");

    expect(clarity).toContain("## Device Flow 코드 표시");
    expect(clarity).toContain("일반 채팅 본문에 URL과 입력 코드를 다시 써요");
    expect(clarity).toContain("코드를 명령 출력이나 로그 읽기 결과 안에만 남기지 않아요");
    expect(clarity).toContain("device_code 같은 내부 교환용 값은 절대 쓰지 않아요");
    expect(clarity).toContain("axhub github link");
    expect(clarity).toContain("CLI 설치 확인");
    expect(clarity).toContain("`axhubed CLI 설치 확인`, `axhubing CLI 설치 확인` 같은 자동 제목이 보이면");
  });
});
