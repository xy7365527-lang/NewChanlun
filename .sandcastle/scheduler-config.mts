import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
// #1617 的 wayfinder 可在无 node_modules 环境导入，因此配置边界只用 Node 内建模块。
const positive = (v: unknown): v is number => typeof v === "number" && Number.isSafeInteger(v) && v > 0;
const positiveFinite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v) && v > 0;
type Effort = "low" | "medium" | "high" | "xhigh";
const effort = (v: unknown): v is Effort => typeof v === "string" && ["low", "medium", "high", "xhigh"].includes(v);
const modelName = (v: unknown): v is string => typeof v === "string" && /^[a-zA-Z0-9][a-zA-Z0-9._-]{0,99}$/.test(v);
const fields = {
  childKillGraceMs: positive,
  childExitPollMs: positive,
  childExitWaitMs: positive,
  healthProcessWaitMs: positive,
  healthProcessPollSeconds: positive,
  healthHttpTimeoutMs: positive,
  adrLockMaxWaits: positive,
  adrLockPollMs: positive,
  dockerModelHttpTimeoutMs: positive,
  dockerModelProcessTimeoutMs: positive,
  frontierQueryLimit: positive,
  backlogQueryLimit: positive,
  mainModel: modelName,
  opusModel: (v: unknown): v is string => modelName(v) && v.startsWith("claude-"),
  workerEffort: effort,
  childEffort: effort,
  mainMaxIterations: positive,
  mainMaxTickets: positive,
  implementerIdleSeconds: positive,
  worktreeMaxAttempts: positive,
  worktreeRetryDelayMs: (v: unknown): v is number => positive(v) && v >= 5000,
  swarmMaxIterations: positive,
  swarmMaxParallel: positive,
  childIdleSeconds: (v: unknown): v is number => positive(v) && v <= 43200,
  childTimeoutSeconds: (v: unknown): v is number => positive(v) && v <= 43200,
  decisionMaps: (v: unknown): v is number[] => Array.isArray(v) && v.length > 0 && v.every(positive),
  decisionMaxConcurrent: positive,
  decisionLoopMs: positive,
  decisionMaxRounds: positive,
  decisionMaxAttempts: (v: unknown): v is number => positive(v) && v >= 5,
  decisionTimeoutMs: positive,
  decisionBackoffBaseMs: positive,
  decisionBackoffCapMs: positive,
  claimPollSeconds: positiveFinite,
  claimRollbackMs: positive,
  wayfinderStaleMs: positive,
  claimStaleMs: positive,
  diskMinGiB: positiveFinite,
};
type Checked<T> = T extends (value: unknown) => value is infer V ? V : never;
export type SchedulerValues = { [K in keyof typeof fields]: Checked<(typeof fields)[K]> };
export type SchedulerKey = keyof SchedulerValues;
const keys = Object.keys(fields) as SchedulerKey[];
function object(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function invalid(scope: string, paths: string[]): never {
  throw new Error(`[配置 #1606] ${scope}字段非法: ${paths.join(", ")}`);
}
function readValues(raw: unknown, scope: string): SchedulerValues {
  if (!object(raw)) invalid(scope, ["values"]);
  const bad = keys.filter(key => !Object.hasOwn(raw, key) || !fields[key](raw[key])).map(key => `values.${key}`);
  bad.push(...Object.keys(raw).filter(key => !Object.hasOwn(fields, key)).map(key => `values.${key}`));
  if (bad.length) invalid(scope, bad);
  return { ...raw, decisionMaps: [...raw.decisionMaps as number[]] } as SchedulerValues;
}
function readRecord(raw: unknown) {
  if (!object(raw)) invalid("记录", ["record"]);
  const bad = Object.keys(raw).filter(key => !["schemaVersion", "revision", "ticket", "values", "sources"].includes(key));
  if (raw.schemaVersion !== "sandcastle-scheduler/1") bad.push("schemaVersion");
  if (!positive(raw.revision)) bad.push("revision");
  if (!positive(raw.ticket)) bad.push("ticket");
  if (!object(raw.sources)) bad.push("sources");
  else {
    for (const key of keys) if (!Object.hasOwn(raw.sources, key)) bad.push(`sources.${key}`);
    for (const [key, value] of Object.entries(raw.sources)) {
      if (typeof value !== "string" || value.length === 0) bad.push(`sources.${key}`);
    }
  }
  if (bad.length) invalid("记录", bad);
  return { schemaVersion: "sandcastle-scheduler/1" as const, revision: raw.revision as number,
    ticket: raw.ticket as number, values: readValues(raw.values, "记录"), sources: raw.sources as Record<string, string> };
}

export const configPath = fileURLToPath(new URL("./scheduler-config.json", import.meta.url));
export function schedulerEnvName(key: SchedulerKey): string {
  if (key === "claimPollSeconds") return "CLAIM_LOOP_POLL_SECONDS";
  if (key === "diskMinGiB") return "HEALTH_DISK_MIN_GIB";
  return `SANDCASTLE_${key.replace(/[A-Z]/g, (s) => `_${s}`).toUpperCase()}`;
}

export function readSchedulerConfig(path = configPath, env: NodeJS.ProcessEnv = process.env) {
  let raw: unknown;
  try { raw = JSON.parse(readFileSync(path, "utf8")); }
  catch { throw new Error(`[配置 #1606] 无法读取JSON记录 ${path}`); }
  const record = readRecord(raw);
  const effective: Record<string, unknown> = { ...record.values };
  const origins: Record<string, string> = {};
  for (const key of keys) {
    const name = schedulerEnvName(key);
    const value = env[name];
    origins[key] = `record:${record.revision} ${record.sources[key]}`;
    if (value === undefined) continue;
    if (key === "mainModel" || key === "opusModel" || key === "workerEffort" || key === "childEffort") effective[key] = value;
    else if (key === "decisionMaps") {
      try { effective[key] = JSON.parse(value); }
      catch { throw new Error(`[配置 #1606] 环境字段 ${name} 不是JSON数组`); }
    } else effective[key] = value.trim() === "" ? NaN : Number(value);
    origins[key] = `env:${name}`;
  }
  const values = readValues(effective, "有效");
  if (values.decisionBackoffBaseMs > values.decisionBackoffCapMs) {
    throw new Error("[配置 #1606] decisionBackoffBaseMs 超过 decisionBackoffCapMs");
  }
  Object.freeze(values.decisionMaps);
  return { ...record, values: Object.freeze(values), origins: Object.freeze(origins) };
}

let loaded: ReturnType<typeof readSchedulerConfig> | undefined;
const logged = new Set<string>();
export function useScheduler(component: string, fields: readonly SchedulerKey[]): SchedulerValues {
  const config = loaded ??= readSchedulerConfig(process.env.SANDCASTLE_CONFIG_FILE ?? configPath);
  if (!logged.has(component)) {
    logged.add(component);
    console.error(`[配置 #1606] ${component} ${config.schemaVersion} revision=${config.revision} ticket=#${config.ticket}`);
    for (const key of fields) console.error(`[配置 #1606] ${component}.${key}=${JSON.stringify(config.values[key])} source=${config.origins[key]}`);
  }
  return config.values;
}
