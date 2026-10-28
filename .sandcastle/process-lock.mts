/** #1742：带持有者身份的独占锁。SIGKILL 会留下 `wx` 锁，同清单重入必须能自动恢复，但只在能确认持有者已死时才接管。
 * 锁内容：pid、该进程启动时间（ps lstart，防 pid 复用）、主机名、随机 token、cwd。
 * 接管 = 先把旧锁 rename 归档（带时间戳与 uuid，留在原目录作证据），再 wx 新建；rename 在 POSIX 上原子，并发重入只有一个能搬走旧锁。
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

function create(lock: string, owner: LockOwner): void {
  // 先写完整内容再原子发布：其他进程看到的锁要么不存在，要么内容完整。
  const tmp = `${lock}.tmp-${process.pid}-${randomUUID()}`;
  const fd = openSync(tmp, "wx", 0o600);
  try { writeFileSync(fd, JSON.stringify(owner)); fsyncSync(fd); } finally { closeSync(fd); }
  try { linkSync(tmp, lock); } finally { unlinkSync(tmp); } // 目标已存在即 EEXIST
}

export interface Held { owner: LockOwner; archived?: string; release: () => void }

const sleepMs = (ms: number) => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);

/** 取锁。已存在的锁只在“同主机且持有者已死”时接管（归档旧锁后新建）。
 * 接管的“核对→搬走→新建”必须在 `${lock}.takeover` 守卫内完成：否则 B 读到旧持有者后，A 已接管并建了新锁，B 的 rename 会把 A 的活锁搬走。
 * 守卫内重读锁并确认仍是核对过的那个 token，才 rename；守卫只持有毫秒级（不含 ps 调用），被占用时短等后仍占用即拒绝。 */
export function acquireLock(lock: string, cwd: string, env: LockEnv = realEnv(), beforeTakeover?: (holder: LockOwner) => void): Held {
  const start = env.startOf(env.pid);
  if (!start) throw new Error("无法取得本进程启动时间，不能建立带身份的锁");
  const owner: LockOwner = { pid: env.pid, start, host: env.host, token: randomUUID(), cwd };
  const guardPath = `${lock}.takeover`;
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
    // 持有者已死（pid 不存在，或 pid 被复用：启动时间不同）。接管前的清理（回收孤儿容器）抛错则整体拒绝，锁与现场不动。
    beforeTakeover?.(holder);
    // 然后进入守卫接管。
    let guarded = false;
    for (let wait = 0; wait < 40 && !guarded; wait++) {
      try { create(guardPath, { ...owner, token: randomUUID() }); guarded = true; }
      catch (e) { if ((e as NodeJS.ErrnoException).code !== "EEXIST") throw e; sleepMs(50); }
    }
    if (!guarded) throw new Error(`${guardPath} 长时间被占用，另一个重入可能正在接管（若确认其已死，由主控处理该守卫文件），拒绝`);
    try {
      let again: LockOwner;
      try { again = parseOwner(readFileSync(lock, "utf8")); } catch { continue; } // 已变化：回到外层重新判定
      if (again.token !== holder.token) continue; // 已被他人接管或释放：重新判定
      const target = join(dirname(lock), `${basename(lock)}.stale-${env.now().toISOString().replace(/[:.]/g, "-")}-${randomUUID().slice(0, 8)}`);
      renameSync(lock, target);
      archived = target;
      env.afterArchive?.();
      try { create(lock, owner); }
      catch (e) { if ((e as NodeJS.ErrnoException).code === "EEXIST") continue; throw e; } // 正常取锁者抢先建了新锁：回到外层按活锁判定
      break;
    } finally { try { unlinkSync(guardPath); } catch { /* 已不存在 */ } }
  }
  return {
    owner, archived,
    // 仅当锁仍是自己的（token 一致）才删除；被他人接管后不得误删别人的锁。
    release: () => {
      try { if (parseOwner(readFileSync(lock, "utf8")).token === owner.token) unlinkSync(lock); } catch { /* 已不属于自己或不存在 */ }
    },
  };
}
