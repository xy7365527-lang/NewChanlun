import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { DRAIN_ROOT, drainRequested, withAdmission, withLegacyAdmission } from "./drain-control.mts";

export const EXECUTOR_ROOT = "/Volumes/AgentStorage/deepagents-exec";
const POLICY = JSON.parse(readFileSync(new URL("./queue-policy.json", import.meta.url), "utf8"));
export const QUEUE_LABEL: string = POLICY.release_label;
export const HOLD_LABELS: string[] = POLICY.hold_labels;
const SERVICES = ["com.newchanlun.sandcastle-claimer", "com.newchanlun.sandcastle-harvest"];
const HELPER = fileURLToPath(new URL("./deepagents-pilot/pilot1742/cutover.py", import.meta.url));

export function releaseLabel(stateRoot = EXECUTOR_ROOT, drainRoot = DRAIN_ROOT): string {
  try {
    const record = JSON.parse(readFileSync(`${stateRoot}/cutover.json`, "utf8"));
    const drain = JSON.parse(readFileSync(`${drainRoot}/state.json`, "utf8"));
    if (record.v !== 1 || drain.v !== 1 || drain.mode !== "draining" ||
        !Number.isInteger(record.drain_generation) || record.drain_generation < 1 ||
        record.drain_generation !== drain.generation || record.release_label !== QUEUE_LABEL ||
        record.remote_jobs_idle !== true || record.remote_freeze_var !== "1" ||
        record.variable_source !== "user-token-readonly" ||
        !Array.isArray(record.open_claims) || record.open_claims.length !== 0 ||
        !Array.isArray(record.legacy_snapshot) ||
        !record.legacy_snapshot.every((p: { pid?: unknown; start_id?: unknown; cmd?: unknown }) =>
          Number.isInteger(p.pid) && Number(p.pid) > 0 && typeof p.start_id === "string" && p.start_id.length > 0 &&
          typeof p.cmd === "string" && p.cmd.length > 0) ||
        typeof record.frozen_at !== "string" || record.frozen_at.length === 0 ||
        !Array.isArray(record.legacy_services_unloaded) || record.legacy_services_unloaded.length !== SERVICES.length ||
        !SERVICES.every(s => record.legacy_services_unloaded.includes(s)) ||
        typeof record.workers_log_path !== "string" || !record.workers_log_path.startsWith("/") ||
        !Number.isInteger(record.workers_log_length) || record.workers_log_length < 0 ||
        typeof record.engine_head !== "string" || !/^[a-f0-9]{40}$/.test(record.engine_head)) return "sandcastle";
    return QUEUE_LABEL;
  } catch { return "sandcastle"; }
}

export async function withQueueAdmission<T>(action: () => T | Promise<T>, stateRoot = EXECUTOR_ROOT,
                                             drainRoot = DRAIN_ROOT): Promise<T> {
  return withAdmission(action, [HELPER, "--state-root", stateRoot, "--drain-root", drainRoot]);
}

/** engine 与裁决消费者共用的准入根；只影响调度，不改变工蜂或队列协议。 */
export interface GraphAdmissionRoots {
  stateRoot?: string;
  drainRoot?: string;
}

/** 轮询的快速停止判断；真正写动作仍须经 withGraphAdmission 在锁内复核。 */
export function graphAdmissionFrozen(roots: GraphAdmissionRoots = {}): boolean {
  return drainRequested(roots.drainRoot) && releaseLabel(roots.stateRoot, roots.drainRoot) !== QUEUE_LABEL;
}

/** 有效切流复用新队列的冻结锁准入；否则保留旧入口冻结语义，不降级重试。 */
export async function withGraphAdmission<T>(action: () => T | Promise<T>,
  roots: GraphAdmissionRoots = {}): Promise<T> {
  return releaseLabel(roots.stateRoot, roots.drainRoot) === QUEUE_LABEL
    ? withQueueAdmission(action, roots.stateRoot, roots.drainRoot)
    : withLegacyAdmission(action, roots.drainRoot);
}
