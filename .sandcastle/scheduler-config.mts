import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { z } from "zod";

const positive = z.number().int().positive().safe();
const effort = z.enum(["low", "medium", "high", "xhigh"]);
export const schedulerValuesSchema = z.object({
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
  dockerNamespaceTimeoutMs: positive,
  frontierQueryLimit: positive,
  backlogQueryLimit: positive,
  mainModel: z.string().regex(/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,99}$/),
  workerEffort: effort,
  childEffort: effort,
  mainMaxIterations: positive,
  mainMaxTickets: positive,
  implementerIdleSeconds: positive,
  worktreeMaxAttempts: positive,
  worktreeRetryDelayMs: positive.min(5000),
  swarmMaxIterations: positive,
  swarmMaxParallel: positive,
  childIdleSeconds: positive.max(43200),
  childTimeoutSeconds: positive.max(43200),
  decisionMaps: z.array(positive).nonempty(),
  decisionMaxConcurrent: positive,
  decisionLoopMs: positive,
  decisionMaxRounds: positive,
  decisionMaxAttempts: positive.min(5),
  decisionTimeoutMs: positive,
  decisionBackoffBaseMs: positive,
  decisionBackoffCapMs: positive,
  claimPollSeconds: z.number().positive().finite(),
  claimRollbackMs: positive,
  wayfinderStaleMs: positive,
  claimStaleMs: positive,
  diskMinGiB: z.number().positive().finite(),
}).strict();
export type SchedulerValues = z.infer<typeof schedulerValuesSchema>;
export type SchedulerKey = keyof SchedulerValues;
const keys = Object.keys(schedulerValuesSchema.shape) as SchedulerKey[];
const recordSchema = z.object({
  schemaVersion: z.literal("sandcastle-scheduler/1"),
  revision: positive,
  ticket: positive,
  values: schedulerValuesSchema,
  sources: z.record(z.string(), z.string().min(1)),
}).strict().superRefine((record, ctx) => {
  for (const key of keys) {
    if (!record.sources[key]) ctx.addIssue({ code: "custom", path: ["sources", key], message: "缺少参数来源" });
  }
});

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
  const parsed = recordSchema.safeParse(raw);
  if (!parsed.success) {
    throw new Error(`[配置 #1606] 记录字段非法: ${parsed.error.issues.map((i) => i.path.join(".")).join(", ")}`);
  }
  const record = parsed.data;
  const effective: Record<string, unknown> = { ...record.values };
  const origins: Record<string, string> = {};
  for (const key of keys) {
    const name = schedulerEnvName(key);
    const value = env[name];
    origins[key] = `record:${record.revision} ${record.sources[key]}`;
    if (value === undefined) continue;
    if (key === "mainModel" || key === "workerEffort" || key === "childEffort") effective[key] = value;
    else if (key === "decisionMaps") {
      try { effective[key] = JSON.parse(value); }
      catch { throw new Error(`[配置 #1606] 环境字段 ${name} 不是JSON数组`); }
    } else effective[key] = value.trim() === "" ? NaN : Number(value);
    origins[key] = `env:${name}`;
  }
  const values = schedulerValuesSchema.safeParse(effective);
  if (!values.success) {
    throw new Error(`[配置 #1606] 有效字段非法: ${values.error.issues.map((i) => i.path.join(".")).join(", ")}`);
  }
  if (values.data.decisionBackoffBaseMs > values.data.decisionBackoffCapMs) {
    throw new Error("[配置 #1606] decisionBackoffBaseMs 超过 decisionBackoffCapMs");
  }
  Object.freeze(values.data.decisionMaps);
  return { ...record, values: Object.freeze(values.data), origins: Object.freeze(origins) };
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
