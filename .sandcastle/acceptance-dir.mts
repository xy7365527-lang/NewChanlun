/** #1742 Docker 验收状态目录：宿主固定位置，生产路径不读环境变量、不在任何 worktree 或容器挂载内。
 * receipt / used / accepted 是准入事实，只有宿主主控进程可写；容器与工蜂 worktree 触不到这里。 */
import { execFileSync } from "node:child_process";
import { lstatSync, mkdirSync, realpathSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

export const ACCEPTANCE_DIR_NAME = join(".sandcastle-host", "docker-acceptance-1742");

/** 固定位置：$HOME 下的宿主私有目录（HOME 由宿主进程决定，容器内进程改不了宿主进程的 HOME）。 */
export function acceptanceRoot(home: string = homedir()): string {
  return join(home, ACCEPTANCE_DIR_NAME);
}

const within = (root: string, path: string) => path === root || path.startsWith(root.endsWith(sep) ? root : root + sep);

/** 不存在的路径回退到最近存在祖先的真实路径再拼回。 */
export function realpathLoose(path: string): string {
  const missing: string[] = [];
  let cur = resolve(path);
  for (;;) {
    try { return join(realpathSync(cur), ...missing.reverse()); }
    catch {
      const parent = dirname(cur);
      if (parent === cur) return resolve(path);
      missing.push(cur.slice(parent.length + 1)); cur = parent;
    }
  }
}

/** 已知 checkout 根：本模块所在 checkout、cwd 所在 checkout，以及它们的全部 linked worktree。 */
export function worktreeRoots(cwd: string = process.cwd()): string[] {
  const roots = new Set<string>();
  for (const dir of [dirname(fileURLToPath(import.meta.url)), cwd]) {
    try {
      const top = execFileSync("git", ["-C", dir, "rev-parse", "--show-toplevel"], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
      roots.add(realpathSync(top));
      const list = execFileSync("git", ["-C", top, "worktree", "list", "--porcelain"], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
      for (const line of list.split("\n")) if (line.startsWith("worktree ")) roots.add(realpathLoose(line.slice(9)));
    } catch { /* 非 git 目录：没有可比较的 worktree */ }
  }
  return [...roots];
}

/** 校验并返回规范路径：目录本体非符号链接、属于当前用户、权限无组/其他位、不在任何 worktree 内、也不包含 worktree。
 * create=true 时按 0700 创建。任何不满足都抛错，不降级。 */
export function controlledDir(dir: string, o: { create?: boolean; roots?: readonly string[] } = {}): string {
  if (o.create) mkdirSync(dir, { recursive: true, mode: 0o700 });
  let st;
  try { st = lstatSync(dir); } catch { throw new Error(`验收目录不存在：${dir}`); }
  if (st.isSymbolicLink()) throw new Error("验收目录本体是符号链接，拒绝");
  if (!st.isDirectory()) throw new Error("验收路径不是目录，拒绝");
  const uid = process.getuid?.();
  if (uid !== undefined && st.uid !== uid) throw new Error(`验收目录不属于当前用户（uid ${st.uid} ≠ ${uid}），拒绝`);
  if ((st.mode & 0o077) !== 0) throw new Error(`验收目录权限过宽（${(st.mode & 0o777).toString(8)}），须为 0700，拒绝`);
  const canonical = realpathSync(dir);
  for (const root of (o.roots ?? worktreeRoots()).map((r) => realpathLoose(r))) {
    if (within(root, canonical) || within(canonical, root)) throw new Error(`验收目录与 worktree 重叠（${root}），拒绝`);
  }
  return canonical;
}

/** 容器挂载不得暴露验收目录（含其祖先目录，如整个 $HOME）。 */
export function assertMountsExcludeAcceptance(mounts: ReadonlyArray<{ hostPath: string }>, root: string = acceptanceRoot()): void {
  const acceptance = realpathLoose(root);
  for (const m of mounts) {
    const host = realpathLoose(m.hostPath);
    if (within(host, acceptance) || within(acceptance, host)) throw new Error(`容器挂载 ${m.hostPath} 会暴露 Docker 验收状态目录，拒绝`);
  }
}
