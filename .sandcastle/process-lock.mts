/** #1742：带持有者身份的独占锁。SIGKILL 会留下 `wx` 锁，同清单重入必须能自动恢复，但只在能确认持有者已死时才接管。
 * 锁内容：pid、该进程启动时间（ps lstart，防 pid 复用）、主机名、随机 token、cwd。
 * 接管 = 先把旧锁硬链接归档（带时间戳与 uuid，留在原目录作证据），再把新锁写入临时文件并 rename 原子替换：锁路径在整个接管过程中始终存在，
 * 任何一步被 SIGKILL 都只会留下“持有者已死的锁”，下次重入照常接管。
 * 接管本身在 `${lock}.takeover` 守卫内进行；守卫持有者被 SIGKILL 留下的守卫，由第二层 `${lock}.takeover.g2`（只做读/rename/unlink）在确认其已死后搬走。
 * 持有者存活且身份一致、锁损坏、主机不同、启动时间无法取得——任何无法判定的情况一律抛错，不碰现场。 */
import { spawnSync } from "node:child_process";
import { closeSync, fsyncSync, linkSync, openSync, readFileSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { hostname } from "node:os";
import { randomUUID } from "node:crypto";
import { basename, dirname, join } from "node:path";

export interface LockOwner { pid: number; start: string; host: string; token: string; cwd: string }
export interface LockEnv {
  pid: number; host: string; startOf: (pid: number) => string | null; now: () => Date;
  /** 仅测试：接管时旧锁归档之后、新建之前调用，用来确定性地制造交错。 */
  afterArchive?: () => void;
  /** 仅测试：等待接管守卫的次数（每次 50ms）；缺省 40。 */
  guardTries?: number;
  /** 仅测试：回收残留守卫时，g2 内、旧守卫被搬走之后调用，用来制造“g2 内 rename 之后被杀”的残留。 */
  afterGuardReclaim?: () => void;
}
export interface PsResult { status: number | null; stdout: string; stderr: string; error?: Error }
export type PsRunner = (pid: number) => PsResult;
const runPs: PsRunner = (pid) => {
  // TZ 固定为 UTC：ps 按本地时区格式化 lstart，不同 TZ 下同一 pid 会得到不同字符串，逐字比较会把活锁当成 pid 复用。
  const r = spawnSync("ps", ["-p", String(pid), "-o", "lstart="], { encoding: "utf8", env: { ...process.env, LC_ALL: "C", TZ: "UTC" } });
  return { status: r.status, stdout: r.stdout ?? "", stderr: r.stderr ?? "", error: r.error };
};
/** 进程启动时间（UTC 格式化）。只有“退出码 1 且 stdout/stderr 都为空”才表示进程不存在（返回 null）；其他任何情况（ps 缺失、被信号杀、其他退出码、
 * 退出码 0 却无输出、stderr 有内容）都是无法判定，抛错，由调用方 FAIL-loud。 */
export function psStart(pid: number, run: PsRunner = runPs): string | null {
  const r = run(pid);
  if (r.error) throw new Error(`ps 无法运行：${r.error.message}`);
  if (r.status === 1 && !r.stdout.trim() && !r.stderr.trim()) return null;
  if (r.status === 0 && r.stdout.trim() && !r.stderr.trim()) return r.stdout.trim();
  throw new Error(`ps 结果无法判定持有者存活（status=${r.status}，stderr=${r.stderr.trim().slice(0, 80)}）`);
}
export const realEnv = (): LockEnv => ({ pid: process.pid, host: hostname(), startOf: psStart, now: () => new Date() });

function parseOwner(text: string): LockOwner {
  let raw: unknown;
  try { raw = JSON.parse(text); } catch { throw new Error("锁文件无法解析"); }
  const o = raw as Partial<LockOwner>;
  if (!o || typeof o !== "object" || !Number.isSafeInteger(o.pid) || (o.pid as number) <= 0 || typeof o.start !== "string" || !o.start
    || typeof o.host !== "string" || !o.host || typeof o.token !== "string" || !o.token || typeof o.cwd !== "string") throw new Error("锁文件缺少持有者身份（旧格式或损坏）");
  return o as LockOwner;
}

function writeTmp(path: string, owner: LockOwner): string {
  // 先写完整内容再原子发布：其他进程看到的文件要么不存在，要么内容完整。
  const tmp = `${path}.tmp-${process.pid}-${randomUUID()}`;
  const fd = openSync(tmp, "wx", 0o600);
  try { writeFileSync(fd, JSON.stringify(owner)); fsyncSync(fd); } finally { closeSync(fd); }
  return tmp;
}
function create(path: string, owner: LockOwner): void {
  const tmp = writeTmp(path, owner);
  try { linkSync(tmp, path); } finally { unlinkSync(tmp); } // 目标已存在即 EEXIST
}
/** 只删自己的文件：内容里的 token 与自己一致才 unlink（被人工或他人换过就不动）。 */
function unlinkIfToken(path: string, token: string): void {
  try { if (parseOwner(readFileSync(path, "utf8")).token === token) unlinkSync(path); } catch { /* 不存在或已不属于自己 */ }
}
const stamp = (env: LockEnv) => `${env.now().toISOString().replace(/[:.]/g, "-")}-${randomUUID().slice(0, 8)}`;

/** 接管守卫的持有者被 SIGKILL 后，守卫文件会残留。只在“同主机且持有者确认已死”时，经第二层守卫 `g2Path` 把它 rename 成 `.stale-*`。
 * 返回 true＝守卫已不在原位（可立刻重试建守卫）；false＝不能处理（存活 / 无法解析 / 主机不同 / 无法判定 / g2 被占），调用方照常等待后 FAIL-loud。
 * g2 内只做读文件、rename、unlink，不调 ps/docker/now：并发的两个回收者里只有先拿到 g2 的能搬走旧守卫，
 * 后者进 g2 后重读到的要么是 ENOENT，要么是新持有者的 token（每个守卫 token 唯一），不会搬走活守卫。
 * g2 自身被 SIGKILL 的残留属于毫秒窗口内的双重故障：保持 FAIL-loud，由主控按 runbook 人工处理。 */
function reclaimDeadGuard(guardPath: string, g2Path: string, env: LockEnv, self: LockOwner): boolean {
  let dead: LockOwner;
  try { dead = parseOwner(readFileSync(guardPath, "utf8")); }
  catch (e) { return (e as NodeJS.ErrnoException).code === "ENOENT"; } // 刚被释放：立刻重试；无法解析：不碰
  if (dead.host !== env.host) return false;
  let alive: string | null;
  try { alive = env.startOf(dead.pid); } catch { return false; } // 无法判定：不碰
  if (alive !== null && alive === dead.start) return false;
  const target = `${guardPath}.stale-${stamp(env)}`; // 先算好名字：g2 内不再调外部函数
  const g2Owner: LockOwner = { ...self, token: randomUUID() };
  try { create(g2Path, g2Owner); }
  catch (e) { if ((e as NodeJS.ErrnoException).code === "EEXIST") return false; throw e; }
  try {
    let again: LockOwner;
    try { again = parseOwner(readFileSync(guardPath, "utf8")); }
    catch (e) { return (e as NodeJS.ErrnoException).code === "ENOENT"; }
    if (again.token !== dead.token) return true; // 已被换成别人的守卫（可能是活的）：不动
    renameSync(guardPath, target);
    env.afterGuardReclaim?.();
    return true;
  } finally { unlinkIfToken(g2Path, g2Owner.token); }
}

/** 持有守卫的进程顺手清理“g2 内、rename 之后被杀”留下的 g2：守卫是独占的，清理者只有一个；g2 路径存在时没人能建新的 g2，
 * 而已死的 g2 创建者不会再去 unlink，所以确认其已死后 rename 走是安全的。持有者存活（可能正在另一条回收路径的 g2 内）、无法解析、
 * 主机不同、ps 无法判定一律不动。 */
function sweepDeadG2(g2Path: string, env: LockEnv): void {
  let g: LockOwner;
  try { g = parseOwner(readFileSync(g2Path, "utf8")); } catch { return; }
  if (g.host !== env.host) return;
  let alive: string | null;
  try { alive = env.startOf(g.pid); } catch { return; }
  if (alive !== null && alive === g.start) return;
  try {
    if (parseOwner(readFileSync(g2Path, "utf8")).token !== g.token) return;
    renameSync(g2Path, `${g2Path}.stale-${stamp(env)}`);
  } catch { /* 并发变化：留给下次 */ }
}

export interface Held { owner: LockOwner; archived?: string; release: () => void }

const sleepMs = (ms: number) => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);

/** 取锁。已存在的锁只在“同主机且持有者已死”时接管（归档旧锁后新建）。
 * 接管的“核对→搬走→新建”必须在 `${lock}.takeover` 守卫内完成：否则 B 读到旧持有者后，A 已接管并建了新锁，B 的 rename 会把 A 的活锁搬走。
 * 守卫内重读锁并确认仍是核对过的那个 token，才归档并替换；守卫内含接管前清理（回收孤儿容器，可达秒级），被活着的持有者占用时短等后仍占用即拒绝；
 * 守卫持有者已死（宿主在守卫内被 SIGKILL）则由 reclaimDeadGuard 搬走残留守卫。 */
export function acquireLock(lock: string, cwd: string, env: LockEnv = realEnv(), beforeTakeover?: (holder: LockOwner) => void): Held {
  const start = env.startOf(env.pid);
  if (!start) throw new Error("无法取得本进程启动时间，不能建立带身份的锁");
  const owner: LockOwner = { pid: env.pid, start, host: env.host, token: randomUUID(), cwd };
  const guardPath = `${lock}.takeover`;
  const g2Path = `${guardPath}.g2`;
  let archived: string | undefined;
  for (let attempt = 0; ; attempt++) {
    try { create(lock, owner); break; }
    catch (e) { if ((e as NodeJS.ErrnoException).code !== "EEXIST") throw e; }
    if (attempt >= 6) throw new Error(`${lock} 接管后仍被他人占用，拒绝`);
    let holder: LockOwner;
    try { holder = parseOwner(readFileSync(lock, "utf8")); }
    catch (e) {
      if ((e as NodeJS.ErrnoException).code === "ENOENT") continue; // 刚被释放：重试
      throw new Error(`${lock} 已存在且${e instanceof Error ? e.message : String(e)}；无法判定持有者，不接管`);
    }
    if (holder.host !== env.host) throw new Error(`${lock} 由其他主机 ${holder.host} 持有，无法判定存活，不接管`);
    const alive = env.startOf(holder.pid); // 可能抛错 → 无法判定，向上抛
    if (alive !== null && alive === holder.start) throw new Error(`${lock} 的持有者 pid ${holder.pid} 仍在运行，拒绝`);
    // 持有者已死（pid 不存在，或 pid 被复用：启动时间不同）。进入守卫后重读锁，确认仍是判定已死的那一个，
    // 再在守卫内完成“接管前清理（回收孤儿容器）→ 归档 → 新建”：否则 A 判定已死之后、回收之前，B 可能已接管并启动了自己的工蜂，A 会误停 B 的活容器。
    let guarded = false;
    const guardOwner: LockOwner = { ...owner, token: randomUUID() };
    for (let waited = 0, reclaims = 0; !guarded;) {
      try { create(guardPath, guardOwner); guarded = true; }
      catch (e) {
        if ((e as NodeJS.ErrnoException).code !== "EEXIST") throw e;
        if (reclaims < 5 && reclaimDeadGuard(guardPath, g2Path, env, owner)) { reclaims++; continue; }
        if (++waited >= (env.guardTries ?? 40)) break;
        sleepMs(50);
      }
    }
    if (!guarded) throw new Error(`${guardPath} 长时间被占用，另一个重入可能正在接管。若 ${g2Path} 残留：读出其中的 pid 与 start，执行 \`TZ=UTC LC_ALL=C ps -p <pid> -o lstart=\`，仅当无输出或输出与文件里的 start 逐字不同才把该 g2 文件 rename 走（输出相同＝持有者仍在运行，什么都不要动）；不要手动处理 ${guardPath}，重入时已死的守卫会被自动归档，拒绝`);
    try {
      sweepDeadG2(g2Path, env); // 守卫在手：顺手清理 g2 内 rename 之后被杀留下的死 g2
      let again: LockOwner;
      try { again = parseOwner(readFileSync(lock, "utf8")); } catch { continue; } // 已变化：回到外层重新判定
      if (again.token !== holder.token) continue; // 已被他人接管或释放：重新判定（此时不会调用 beforeTakeover）
      beforeTakeover?.(holder); // 守卫内、归档前；抛错则整体拒绝，锁与现场不动
      const target = join(dirname(lock), `${basename(lock)}.stale-${stamp(env)}`);
      try { linkSync(lock, target); } // 归档＝硬链接：锁路径不消失，此刻被杀也只是留下“持有者已死的锁”
      catch (e) { if ((e as NodeJS.ErrnoException).code === "ENOENT") continue; throw e; } // 锁被人工移走：重新判定
      archived = target;
      env.afterArchive?.();
      const tmp = writeTmp(lock, owner);
      try { renameSync(tmp, lock); } // 原子替换：正常取锁者在此前后看到的都是一把存在的锁
      catch (e) { try { unlinkSync(tmp); } catch { /* 已不存在 */ } throw e; }
      break;
    } finally { unlinkIfToken(guardPath, guardOwner.token); } // 只删自己的守卫
  }
  return {
    owner, archived,
    // 仅当锁仍是自己的（token 一致）才删除；被他人接管后不得误删别人的锁。
    release: () => {
      try { if (parseOwner(readFileSync(lock, "utf8")).token === owner.token) unlinkSync(lock); } catch { /* 已不属于自己或不存在 */ }
    },
  };
}
