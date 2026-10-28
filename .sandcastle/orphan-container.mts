/** #1742：冲突工蜂运行期间宿主 node 被 SIGKILL，工蜂容器仍活着并以读写方式挂着归并工位。第 15 行要求零人工收敛，
 * 所以锁的持有者确认已死后，由宿主自动回收“自己创建的”孤儿容器，然后才接管锁。仅限生命周期管理：不改 daemon/安全策略/权限。
 *
 * 认定方式：Sandcastle 的 docker() 没有 label 选项，所以用冲突工蜂容器的身份 env（SANDCASTLE_MERGE_RUN / SANDCASTLE_MERGE_INSTANCE，
 * 不含密钥）+ 名字前缀 sandcastle- + 镜像 + 工位挂载 Source 的 realpath 一起判定。全部满足才回收；
 * 任何一条无法确定、匹配多于一个、或身份对得上但挂载来源不同，一律 FAIL-loud，不碰现场与锁。 */
import { execFileSync } from "node:child_process";
import { mkdirSync, realpathSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const MERGE_RUN_ENV = "SANDCASTLE_MERGE_RUN";
export const MERGE_INSTANCE_ENV = "SANDCASTLE_MERGE_INSTANCE";
export const WORKSPACE_DEST = "/home/agent/workspace";

export type DockerCli = (args: string[]) => string;
export const realDocker: DockerCli = (args) => execFileSync("docker", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], timeout: 60_000 });

interface Inspected {
  Id?: string; Name?: string; Created?: string; State?: unknown;
  Config?: { Image?: string; Env?: string[] };
  Mounts?: Array<{ Type?: string; Source?: string; Destination?: string; RW?: boolean }>;
}
export interface ReclaimOptions {
  cwd: string; image: string; runId: string; instance: string; evidenceDir: string;
  docker?: DockerCli; now?: () => Date;
}
export interface Reclaimed { id: string; name: string; archive: string }

const fail = (why: string): never => { throw new Error(`[FAIL-loud #1742] 锁持有者已死，但孤儿容器无法安全回收：${why}。不碰现场与锁；请主控核对后处理`); };
const envMap = (env: string[] | undefined) => new Map((env ?? []).map((e) => { const i = e.indexOf("="); return i < 0 ? [e, ""] as const : [e.slice(0, i), e.slice(i + 1)] as const; }));
const canon = (p: string) => { try { return realpathSync(p); } catch { return resolve(p); } };

/** 归档用脱敏视图：只留 Id/Name/Image/Created/State/Mounts 与 env 的键名，永不写 env 值（CLIPROXY_LOCAL_KEY 等）。 */
export function redactedInspect(c: Inspected): Record<string, unknown> {
  return { Id: c.Id, Name: c.Name, Image: c.Config?.Image, Created: c.Created, State: c.State, Mounts: c.Mounts, EnvKeys: [...envMap(c.Config?.Env).keys()].sort() };
}

/** 找出并回收本归并 run 实例遗留的孤儿容器。返回已回收的容器（无孤儿返回 []）。调用前提：锁持有者已确认死亡。 */
export function reclaimOrphanContainers(o: ReclaimOptions): Reclaimed[] {
  const docker = o.docker ?? realDocker;
  const cwd = canon(o.cwd);
  let candidates: Inspected[];
  try {
    const ids = docker(["ps", "-q", "--filter", "name=^sandcastle-"]).split("\n").map((l) => l.trim()).filter(Boolean);
    if (!ids.length) return [];
    const parsed = JSON.parse(docker(["inspect", ...ids])) as unknown;
    if (!Array.isArray(parsed) || parsed.length !== ids.length) throw new Error("inspect 结果与 ps 不符");
    candidates = parsed as Inspected[];
  } catch (e) { return fail(`docker 不可用或输出无法判定（${e instanceof Error ? e.message.slice(0, 100) : String(e)}）`); }

  const ours: Inspected[] = [];
  for (const c of candidates) {
    const name = (c.Name ?? "").replace(/^\//, "");
    const env = envMap(c.Config?.Env);
    const mountsWorktree = (c.Mounts ?? []).some((m) => typeof m.Source === "string" && canon(m.Source) === cwd);
    const claimsIdentity = env.get(MERGE_RUN_ENV) === o.runId && env.get(MERGE_INSTANCE_ENV) === o.instance;
    if (!mountsWorktree && !claimsIdentity) continue; // 与本工位无关：不碰
    if (!name.startsWith("sandcastle-") || !c.Id) fail(`容器 ${name || "?"} 挂着本工位但不是 Sandcastle 容器`);
    if (!claimsIdentity) fail(`容器 ${name} 挂着本工位，但身份 env 与本归并 run/实例不符`);
    if (c.Config?.Image !== o.image) fail(`容器 ${name} 镜像 ${c.Config?.Image} 不是 ${o.image}`);
    const ws = (c.Mounts ?? []).filter((m) => m.Destination === WORKSPACE_DEST);
    if (ws.length !== 1 || typeof ws[0]!.Source !== "string" || canon(ws[0]!.Source) !== cwd) fail(`容器 ${name} 的工位挂载来源与本归并工位不同`);
    ours.push(c);
  }
  if (ours.length > 1) fail(`匹配到 ${ours.length} 个容器（预期至多 1 个）：${ours.map((c) => c.Name).join("、")}`);
  if (!ours.length) return [];

  const c = ours[0]!; const id = c.Id!; const name = (c.Name ?? "").replace(/^\//, "");
  // 先归档脱敏的 inspect，再停、再删；归档失败则不动容器。
  const stamp = (o.now?.() ?? new Date()).toISOString().replace(/[:.]/g, "-");
  const archive = join(o.evidenceDir, `orphan-container-${id.slice(0, 12)}-${stamp}.json`);
  try { mkdirSync(o.evidenceDir, { recursive: true }); writeFileSync(archive, JSON.stringify(redactedInspect(c), null, 2) + "\n", { flag: "wx", mode: 0o600 }); }
  catch (e) { return fail(`无法归档容器 inspect（${e instanceof Error ? e.message.slice(0, 100) : String(e)}）`); }
  for (const verb of ["stop", "rm"] as const) {
    try { docker([verb, id]); }
    catch (e) {
      // 并发重入可能已回收：容器已不存在才算成功。
      let still = true;
      try { still = docker(["ps", "-aq", "--filter", `id=${id}`]).trim().length > 0; } catch { /* 无法判定 → 视为仍在 */ }
      if (still) return fail(`docker ${verb} ${name} 失败（${e instanceof Error ? e.message.slice(0, 100) : String(e)}）`);
    }
  }
  return [{ id, name, archive }];
}
