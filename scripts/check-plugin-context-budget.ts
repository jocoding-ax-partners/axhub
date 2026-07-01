import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { isAbsolute, join, relative, resolve } from "node:path";

export const DEFAULT_MAX_SKILL_BYTES = 35_000;
export const DEFAULT_MAX_TOTAL_BYTES = 180_000;
export const DEFAULT_MAX_ALWAYS_ON_TOKENS = 2_500;
export const DEFAULT_MAX_OTHER_ON_INVOKE_TOKENS = 8_000;
export const DEFAULT_MAX_ON_INVOKE_TOKENS_BY_COMPONENT: Record<string, number> = { deploy: 12_000, init: 12_000, onboarding: 10_000 };

export interface BudgetOptions {
  root?: string; skillsDir?: string; maxSkillBytes?: number; maxTotalBytes?: number;
  pluginDetailsText?: string; pluginDetailsPath?: string;
  maxAlwaysOnTokens?: number; maxOtherOnInvokeTokens?: number;
  maxOnInvokeTokensByComponent?: Record<string, number>;
}

export interface SkillBudget { slug: string; path: string; bytes: number; overBy: number; }
export interface ClaudePluginComponentDetails { component: string; alwaysOnTokens: number; onInvokeTokens: number; }
export interface ClaudePluginDetails { alwaysOnTokens: number; components: ClaudePluginComponentDetails[]; }
export interface ComponentTokenBudget extends ClaudePluginComponentDetails { maxOnInvokeTokens: number; overBy: number; }

export interface TokenBudgetResult {
  measured: boolean; ok: boolean; maxAlwaysOnTokens: number; maxOtherOnInvokeTokens: number;
  maxOnInvokeTokensByComponent: Record<string, number>;
  alwaysOnTokens: number; alwaysOnOverBy: number;
  components: ComponentTokenBudget[]; overBudgetComponents: ComponentTokenBudget[]; errors: string[];
}

export interface BudgetCheckResult {
  ok: boolean; root: string; skillsDir: string; maxSkillBytes: number; maxTotalBytes: number;
  totalBytes: number; totalOverBy: number;
  skills: SkillBudget[]; overBudgetSkills: SkillBudget[]; tokenBudget: TokenBudgetResult; errors: string[];
}

class BudgetInputError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BudgetInputError";
  }
}

const ALWAYS_ON_LINE = /^Always-on(?: tokens)?:\s*(~?\d[\d,]*(?:\.\d+)?k?)(?:\s+tok(?:ens)?)?/i;
const COMPONENT_TABLE_HEADER = /^Component\s+Always-on(?:\s+tokens)?\s+On-invoke(?:\s+tokens)?$/i;
const COMPONENT_ROW = /^([A-Za-z][A-Za-z0-9_-]*)\s+(\S+)\s+(\S+)$/;

const assertPositiveInteger = (name: string, value: number): void => {
  if (!Number.isInteger(value) || value <= 0) throw new BudgetInputError(`${name} must be a positive integer`);
};

const parsePositiveInteger = (name: string, raw: string): number => {
  const value = Number(raw);
  assertPositiveInteger(name, value);
  return value;
};

const displayPath = (root: string, path: string): string => {
  const rel = relative(root, path);
  return rel && !rel.startsWith("..") ? rel : path;
};

const resolveFromRoot = (root: string, path: string): string => (isAbsolute(path) ? resolve(path) : resolve(root, path));
const resolveSkillsDir = (root: string, skillsDir?: string): string => (skillsDir ? resolveFromRoot(root, skillsDir) : join(root, "skills"));

const parseTokenNumber = (raw: string): number => {
  const match = raw.replace(/,/g, "").trim().toLowerCase().match(/^~?(\d+(?:\.\d+)?)(k)?$/);
  if (!match) throw new BudgetInputError(`invalid token value: ${raw}`);

  const value = Number(match[1]);
  if (!Number.isFinite(value)) throw new BudgetInputError(`invalid token value: ${raw}`);
  return Math.round(value * (match[2] ? 1_000 : 1));
};

export const parseClaudePluginDetails = (text: string): ClaudePluginDetails => {
  let alwaysOnTokens: number | undefined;
  let inComponentTable = false;
  const components: ClaudePluginComponentDetails[] = [];

  const lines = text.split(/\r?\n/);
  for (const [lineIndex, rawLine] of lines.entries()) {
    const line = rawLine.trim();
    if (!line) {
      inComponentTable = false;
      continue;
    }

    const alwaysOnMatch = line.match(ALWAYS_ON_LINE);
    if (alwaysOnMatch) {
      alwaysOnTokens = parseTokenNumber(alwaysOnMatch[1]);
      continue;
    }

    if (COMPONENT_TABLE_HEADER.test(line)) {
      inComponentTable = true;
      continue;
    }

    if (!inComponentTable) continue;

    const componentMatch = line.match(COMPONENT_ROW);
    if (!componentMatch) throw new BudgetInputError(`invalid token component row at line ${lineIndex + 1} in component token table`);
    try {
      components.push({ component: componentMatch[1], alwaysOnTokens: parseTokenNumber(componentMatch[2]), onInvokeTokens: parseTokenNumber(componentMatch[3]) });
    } catch (error) {
      if (error instanceof BudgetInputError) throw new BudgetInputError(`invalid token component row at line ${lineIndex + 1} in component token table`);
      throw error;
    }
  }

  if (alwaysOnTokens === undefined && components.length === 0) {
    throw new BudgetInputError("token budget data not found in Claude plugin details output");
  }
  if (alwaysOnTokens !== undefined && components.length === 0) {
    throw new BudgetInputError("token component data not found in Claude plugin details output");
  }
  return { alwaysOnTokens: alwaysOnTokens ?? 0, components };
};

const createUnmeasuredTokenBudget = (maxAlwaysOnTokens: number, maxOtherOnInvokeTokens: number, maxOnInvokeTokensByComponent: Record<string, number>): TokenBudgetResult => ({
  measured: false, ok: true, maxAlwaysOnTokens, maxOtherOnInvokeTokens, maxOnInvokeTokensByComponent,
  alwaysOnTokens: 0, alwaysOnOverBy: 0, components: [], overBudgetComponents: [], errors: [],
});

const checkTokenBudget = (
  details: ClaudePluginDetails,
  maxAlwaysOnTokens: number,
  maxOtherOnInvokeTokens: number,
  maxOnInvokeTokensByComponent: Record<string, number>,
): TokenBudgetResult => {
  const alwaysOnOverBy = Math.max(0, details.alwaysOnTokens - maxAlwaysOnTokens);
  const components = details.components.map((component) => {
    const maxOnInvokeTokens = maxOnInvokeTokensByComponent[component.component] ?? maxOtherOnInvokeTokens;
    return { ...component, maxOnInvokeTokens, overBy: Math.max(0, component.onInvokeTokens - maxOnInvokeTokens) };
  });
  const overBudgetComponents = components.filter((component) => component.overBy > 0);

  return {
    ...createUnmeasuredTokenBudget(maxAlwaysOnTokens, maxOtherOnInvokeTokens, maxOnInvokeTokensByComponent),
    measured: true,
    ok: alwaysOnOverBy === 0 && overBudgetComponents.length === 0,
    alwaysOnTokens: details.alwaysOnTokens,
    alwaysOnOverBy,
    components,
    overBudgetComponents,
  };
};

const scanSkills = (root: string, skillsDir: string, maxSkillBytes: number): Pick<BudgetCheckResult, "skills" | "errors"> => {
  if (!existsSync(skillsDir)) return { skills: [], errors: [`skills directory not found: ${displayPath(root, skillsDir)}`] };
  if (!statSync(skillsDir).isDirectory()) return { skills: [], errors: [`skills path is not a directory: ${displayPath(root, skillsDir)}`] };

  const errors: string[] = [];
  const skills = readdirSync(skillsDir).sort().flatMap((slug): SkillBudget[] => {
    const skillDir = join(skillsDir, slug);
    if (!statSync(skillDir).isDirectory()) return [];

    const path = join(skillDir, "SKILL.md");
    if (!existsSync(path)) {
      errors.push(`missing ${displayPath(root, path)}`);
      return [];
    }
    if (!statSync(path).isFile()) {
      errors.push(`skill path is not a file: ${displayPath(root, path)}`);
      return [];
    }

    const bytes = Buffer.byteLength(readFileSync(path, "utf8"), "utf8");
    return [{ slug, path, bytes, overBy: Math.max(0, bytes - maxSkillBytes) }];
  });

  return { skills, errors };
};

const buildTokenBudget = (
  root: string,
  options: BudgetOptions,
  maxAlwaysOnTokens: number,
  maxOtherOnInvokeTokens: number,
  maxOnInvokeTokensByComponent: Record<string, number>,
): TokenBudgetResult => {
  const unmeasured = createUnmeasuredTokenBudget(maxAlwaysOnTokens, maxOtherOnInvokeTokens, maxOnInvokeTokensByComponent);
  const detailsText = options.pluginDetailsText ?? (options.pluginDetailsPath ? readFileSync(resolveFromRoot(root, options.pluginDetailsPath), "utf8") : undefined);
  if (detailsText === undefined) return unmeasured;

  try {
    return checkTokenBudget(parseClaudePluginDetails(detailsText), maxAlwaysOnTokens, maxOtherOnInvokeTokens, maxOnInvokeTokensByComponent);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return { ...unmeasured, measured: true, ok: false, errors: [`token budget parse error: ${message}`] };
  }
};

export const checkPluginContextBudget = (options: BudgetOptions = {}): BudgetCheckResult => {
  const root = resolve(options.root ?? process.cwd());
  const skillsDir = resolveSkillsDir(root, options.skillsDir);
  const maxSkillBytes = options.maxSkillBytes ?? DEFAULT_MAX_SKILL_BYTES;
  const maxTotalBytes = options.maxTotalBytes ?? DEFAULT_MAX_TOTAL_BYTES;
  const maxAlwaysOnTokens = options.maxAlwaysOnTokens ?? DEFAULT_MAX_ALWAYS_ON_TOKENS;
  const maxOtherOnInvokeTokens = options.maxOtherOnInvokeTokens ?? DEFAULT_MAX_OTHER_ON_INVOKE_TOKENS;
  const maxOnInvokeTokensByComponent = { ...DEFAULT_MAX_ON_INVOKE_TOKENS_BY_COMPONENT, ...(options.maxOnInvokeTokensByComponent ?? {}) };

  assertPositiveInteger("maxSkillBytes", maxSkillBytes);
  assertPositiveInteger("maxTotalBytes", maxTotalBytes);
  assertPositiveInteger("maxAlwaysOnTokens", maxAlwaysOnTokens);
  assertPositiveInteger("maxOtherOnInvokeTokens", maxOtherOnInvokeTokens);
  for (const [component, maxTokens] of Object.entries(maxOnInvokeTokensByComponent)) {
    assertPositiveInteger(`maxOnInvokeTokensByComponent.${component}`, maxTokens);
  }

  const { skills, errors } = scanSkills(root, skillsDir, maxSkillBytes);
  const totalBytes = skills.reduce((sum, skill) => sum + skill.bytes, 0);
  const totalOverBy = Math.max(0, totalBytes - maxTotalBytes);
  const overBudgetSkills = skills.filter((skill) => skill.overBy > 0);
  const tokenBudget = buildTokenBudget(root, options, maxAlwaysOnTokens, maxOtherOnInvokeTokens, maxOnInvokeTokensByComponent);

  return { ok: errors.length === 0 && overBudgetSkills.length === 0 && totalOverBy === 0 && tokenBudget.ok,
    root, skillsDir, maxSkillBytes, maxTotalBytes, totalBytes, totalOverBy, skills, overBudgetSkills, tokenBudget, errors };
};

export const formatBudgetReport = (result: BudgetCheckResult): string => {
  const lines = [
    `Plugin context budget: ${result.ok ? "PASS" : "FAIL"}`,
    `Skills root: ${displayPath(result.root, result.skillsDir)}`,
    `Skills checked: ${result.skills.length}`,
    `Per-skill limit: ${result.maxSkillBytes} bytes`,
    `Total limit: ${result.maxTotalBytes} bytes`,
    `Total: ${result.totalBytes} bytes${result.totalOverBy > 0 ? ` (over by ${result.totalOverBy})` : ""}`,
  ];

  const pushList = (title: string, items: string[]): void => { if (items.length > 0) lines.push("", title, ...items); };

  pushList("Input errors:", result.errors.map((error) => `- ${error}`));
  pushList(
    "Over-budget skills:",
    result.overBudgetSkills.map((skill) => `- ${skill.slug}: ${skill.bytes} bytes (limit ${result.maxSkillBytes}, over by ${skill.overBy})`),
  );
  if (result.totalOverBy > 0) lines.push("", `Total budget exceeded by ${result.totalOverBy} bytes.`);

  if (!result.tokenBudget.measured) {
    lines.push("", "Token budget: not measured (no Claude plugin details output provided).");
    return lines.join("\n");
  }

  const alwaysOnOverage = result.tokenBudget.alwaysOnOverBy > 0 ? `, over by ${result.tokenBudget.alwaysOnOverBy}` : "";
  lines.push("", `Always-on tokens: ${result.tokenBudget.alwaysOnTokens} (limit ${result.tokenBudget.maxAlwaysOnTokens}${alwaysOnOverage})`);
  pushList("Token budget errors:", result.tokenBudget.errors.map((error) => `- ${error}`));
  pushList(
    "Over-budget token components:",
    result.tokenBudget.overBudgetComponents.map(
      (component) =>
        `- ${component.component}: ${component.onInvokeTokens} on-invoke tokens (limit ${component.maxOnInvokeTokens}, over by ${component.overBy})`,
    ),
  );

  return lines.join("\n");
};

const readValue = (args: string[], index: number, flag: string): string => {
  const value = args[index + 1];
  if (!value) throw new BudgetInputError(`${flag} requires a value`);
  return value;
};

const parseArgs = (args: string[]): BudgetOptions => {
  const options: BudgetOptions = {};
  for (let index = 0; index < args.length; index += 1) {
    const flag = args[index];
    if (!flag) continue;
    if (flag === "--root") options.root = readValue(args, index, flag);
    else if (flag === "--skills-dir") options.skillsDir = readValue(args, index, flag);
    else if (flag === "--max-skill-bytes") options.maxSkillBytes = parsePositiveInteger("maxSkillBytes", readValue(args, index, flag));
    else if (flag === "--max-total-bytes") options.maxTotalBytes = parsePositiveInteger("maxTotalBytes", readValue(args, index, flag));
    else if (flag === "--plugin-details-output" || flag === "--plugin-details-path") options.pluginDetailsPath = readValue(args, index, flag);
    else throw new BudgetInputError(`unknown argument: ${flag}`);
    index += 1;
  }
  return options;
};

const main = (): void => {
  try {
    const result = checkPluginContextBudget(parseArgs(Bun.argv.slice(2)));
    console.log(formatBudgetReport(result));
    if (!result.ok) process.exit(1);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Plugin context budget: ERROR\n${message}`);
    process.exit(2);
  }
};

if (import.meta.main) main();
