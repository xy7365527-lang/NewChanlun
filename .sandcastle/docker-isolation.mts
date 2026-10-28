/** #1742 方案B：容器即沙箱。Git 元数据与受保护子路径只读挂载，门禁逐项观测真实合同。
 * 不加载 profile、不传 --security-opt/--cap-add/--privileged；内层 Codex 不再起 bwrap。 */
import { execFileSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { lstatSync, mkdtempSync, mkdirSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, isAbsolute, join, resolve, sep } from "node:path";
import { assertMountsExcludeAcceptance } from "./acceptance-dir.mts";

export interface MountSpec { hostPath: string; sandboxPath: string; readonly?: boolean }

/** worktree 内必须物理只读的子路径；不存在则不新挂（容器内可新建，但宿主封包拒收这些路径）。 */
export const PROTECTED_SUBPATHS = [".git", ".agents", ".codex"] as const;

export interface PathProbe { lstatSymlink(path: string): boolean | null; realpath(path: string): string }
/** lstat 不跟随链接：null 表示路径不存在。 */
export const fsProbe: PathProbe = {
  lstatSymlink: (path) => { try { return lstatSync(path).isSymbolicLink(); } catch { return null; } },
  realpath: (path) => realpathSync(path),
};

const within = (root: string, path: string) => path === root || path.startsWith(root.endsWith(sep) ? root : root + sep);
/** 路径不存在时回退到最近存在祖先的真实路径再拼回，用于 hooksPath 这类可能尚未创建的目标。 */
export function realpathLoose(path: string, probe: Pick<PathProbe, "realpath"> = fsProbe): string {
  const missing: string[] = [];
  let cur = resolve(path);
  for (;;) {
    try { return join(probe.realpath(cur), ...missing.reverse()); }
    catch {
      const parent = dirname(cur);
      if (parent === cur) return resolve(path);
      missing.push(cur.slice(parent.length + 1)); cur = parent;
    }
  }
}

/** 只收紧、不新增来源：worktree 以外的挂载（Sandcastle 的 git 挂载）改只读，
 * worktree 内已存在的受保护子路径用同一宿主路径再挂一次只读。
 * 受保护子路径若是符号链接（Docker -v 会跟随到链接目标）或真实路径逃出 worktree，一律拒绝，不静默跳过。 */
export function hardenMounts(
  mounts: readonly MountSpec[],
  worktreeHostPath: string,
  probe: PathProbe = fsProbe,
): MountSpec[] {
  const worktree = mounts.find((m) => m.hostPath === worktreeHostPath);
  if (!worktree) throw new Error("Docker 挂载缺少 worktree 本体，拒绝猜测只读边界");
  assertMountsExcludeAcceptance(mounts); // 验收状态目录（含其祖先）不得进入容器
  const root = probe.realpath(worktree.hostPath);
  const out: MountSpec[] = mounts.map((m) => (m === worktree ? { ...m } : { ...m, readonly: true }));
  for (const name of PROTECTED_SUBPATHS) {
    const sandboxPath = `${worktree.sandboxPath}/${name}`;
    const hostPath = join(worktree.hostPath, name);
    const link = probe.lstatSymlink(hostPath);
    if (link === true) throw new Error(`受保护路径 ${name} 是符号链接，拒绝挂载（会逃出 worktree）`);
    const present = out.find((m) => m.sandboxPath === sandboxPath);
    if (present) { present.readonly = true; continue; }
    if (link === null) continue;
    if (!within(root, probe.realpath(hostPath))) throw new Error(`受保护路径 ${name} 的真实路径逃出 worktree，拒绝挂载`);
    out.push({ hostPath, sandboxPath, readonly: true });
  }
  return out;
}

interface HasCreate<O extends { mounts: Array<{ hostPath: string; sandboxPath: string; readonly?: boolean }>; worktreePath: string }, H> {
  create: (options: O) => Promise<H>;
}

/** 容器 env 白名单（#1742，D6 前凭据暴露闭合）：只有 CLIProxy 路由键与已审的非密钥身份键能进容器。
 * Sandcastle 会把 `<hostRepoDir>/.sandcastle/.env` 里所有键并入 create 的 env（含 KIMI/DEEPSEEK/ZAI/MOONSHOT/GH_TOKEN 等），
 * 这里在唯一入口 hardenProvider 剥离；改动即改合同摘要（contractDigest）。
 * 身份键名与 orphan-container.mts 的 MERGE_RUN_ENV/MERGE_INSTANCE_ENV 逐字一致（测试锁住）。 */
export const CONTAINER_ENV_ALLOW = ["CLIPROXY_BASE_URL", "CLIPROXY_LOCAL_KEY", "SANDCASTLE_MERGE_INSTANCE", "SANDCASTLE_MERGE_RUN"] as const;
const ALLOWED = new Set<string>(CONTAINER_ENV_ALLOW);

/** 只取 provider 自己声明的白名单键的值（来源严格＝docker({ env })，.env 与运行期 env 即使带同名键也不采信）；
 * 声明了白名单之外的键属于编程错误，FAIL-loud（只报键名，不报值）。 */
export function declaredContainerEnv(declared: Record<string, string | undefined> | undefined): Record<string, string> {
  const out: Record<string, string> = {};
  const rejected: string[] = [];
  for (const [key, value] of Object.entries(declared ?? {})) {
    if (!ALLOWED.has(key)) { rejected.push(key); continue; }
    if (value) out[key] = value;
  }
  if (rejected.length) throw new Error(`[FAIL-loud #1742] 容器 env 声明了白名单之外的键：${rejected.sort().join(", ")}（允许：${CONTAINER_ENV_ALLOW.join(", ")}）`);
  return out;
}

/** 包一层 provider.create：改写 mounts（只收紧），并把 env 收敛到白名单；其余原样交给 Sandcastle 的 docker()。 */
export function hardenProvider<P extends object, O extends { mounts: Array<{ hostPath: string; sandboxPath: string; readonly?: boolean }>; worktreePath: string; env?: Record<string, string> }, H>(
  base: P,
  probe: PathProbe = fsProbe,
): P {
  const inner = (base as unknown as HasCreate<O, H>).create;
  const env = declaredContainerEnv((base as { env?: Record<string, string | undefined> }).env);
  return {
    ...base,
    create: (options: O) => inner.call(base, { ...options, env, mounts: hardenMounts(options.mounts, options.worktreePath, probe) }),
  } as P;
}

/** 凭据类变量名（含 CLIPROXY_LOCAL_KEY 之外的一切）。 */
const SECRET_NAME = /(KEY|TOKEN|SECRET|PASSWORD|PASSWD|CREDENTIAL|AUTH)/i;
/** 镜像自带 ENV 里“名字像凭据且值非空”的条目：不得存在（空占位键允许，其值只能由白名单之外的 -e 覆盖，而白名单已把它们挡在外面）。 */
export function imageEnvSecrets(imageEnv: readonly string[]): string[] {
  return imageEnv.filter((e) => { const i = e.indexOf("="); return i > 0 && e.slice(i + 1) !== "" && SECRET_NAME.test(e.slice(0, i)); }).map((e) => e.slice(0, e.indexOf("=")));
}
/** 运行中容器的有效 env（docker inspect Config.Env）审计：每个键要么来自镜像自带（值必须与镜像一致），要么在白名单内（或 Sandcastle 自设的 HOME）；
 * 其余键，或镜像自带的凭据类键被覆盖成非空值，都是泄漏。返回违规键名（不含值）。 */
export function containerEnvLeaks(containerEnv: readonly string[], imageEnv: readonly string[]): string[] {
  const parse = (list: readonly string[]) => new Map(list.map((e) => { const i = e.indexOf("="); return [i < 0 ? e : e.slice(0, i), i < 0 ? "" : e.slice(i + 1)] as const; }));
  const image = parse(imageEnv); const bad: string[] = [];
  for (const [key, value] of parse(containerEnv)) {
    if (ALLOWED.has(key) || key === "HOME") continue;
    if (!image.has(key)) { bad.push(key); continue; }
    if (image.get(key) !== value && SECRET_NAME.test(key)) bad.push(key);
  }
  return bad.sort();
}

export interface IsolationObservation {
  apparmor: string; seccomp: string; capEff: string; capBnd: string;
  roWrite: string; rwWrite: string; roChildInRw: string; roFileAppend: string; renameMountpoint: string; hardlinkAcross: string; symlinkIntoRo: string;
}

const EROFS = /read-only file system/i;
/** Docker 默认 capability 集合（chown dac_override fowner fsetid kill setgid setuid setpcap net_bind_service net_raw
 * sys_chroot mknod audit_write setfcap）。bounding set 只能是它的子集：cap-add/privileged 会越界，cap-drop 只会缩小。 */
export const DEFAULT_CAP_MASK = 0xa80425fbn;
/** Docker 内层 Codex 沙箱模式；改动即改合同摘要。 */
export const DOCKER_INNER_SANDBOX = "danger-full-access";

/** 观测值全部满足才 Ready；缺一项即返回原因列表（不缓存、不推断）。 */
export function violations(o: IsolationObservation): string[] {
  const bad: string[] = [];
  if (!/^docker-default \(enforce\)/.test(o.apparmor)) bad.push(`AppArmor=${o.apparmor}`);
  if (o.seccomp !== "2") bad.push(`Seccomp=${o.seccomp}`);
  if (!/^0+$/.test(o.capEff)) bad.push(`CapEff=${o.capEff}`);
  if (!/^[0-9a-f]+$/i.test(o.capBnd) || (BigInt("0x" + o.capBnd) & ~DEFAULT_CAP_MASK) !== 0n) bad.push(`CapBnd=${o.capBnd} 超出 Docker 默认集合`);
  if (!EROFS.test(o.roWrite)) bad.push(`只读挂载写入未返回 EROFS: ${o.roWrite}`);
  if (o.rwWrite !== "ok") bad.push(`读写挂载不可写: ${o.rwWrite}`);
  if (!EROFS.test(o.roChildInRw)) bad.push(`读写挂载内的只读子挂载可写: ${o.roChildInRw}`);
  if (!EROFS.test(o.roFileAppend)) bad.push(`只读文件挂载可写: ${o.roFileAppend}`);
  if (o.renameMountpoint === "ok") bad.push("只读子挂载点可被 rename");
  if (o.hardlinkAcross === "ok") bad.push("可从只读挂载硬链接到读写挂载");
  if (!EROFS.test(o.symlinkIntoRo)) bad.push(`经符号链接写只读挂载未返回 EROFS: ${o.symlinkIntoRo}`);
  return bad;
}

export const PROBE_SCRIPT = String.raw`
r() { out=$("$@" 2>&1) && echo ok || echo "$out"; }
echo "apparmor=$(cat /proc/self/attr/current 2>&1 | tr -d '\0')"
echo "seccomp=$(sed -n 's/^Seccomp:[[:space:]]*//p' /proc/self/status)"
echo "capEff=$(sed -n 's/^CapEff:[[:space:]]*//p' /proc/self/status)"
echo "capBnd=$(sed -n 's/^CapBnd:[[:space:]]*//p' /proc/self/status)"
echo "roWrite=$(r touch /probe/ro/x | tr '\n' ' ')"
echo "rwWrite=$(r touch /probe/rw/x | tr '\n' ' ')"
echo "roChildInRw=$(r touch /probe/rw/child/x | tr '\n' ' ')"
echo "roFileAppend=$(r sh -c 'echo x >> /probe/rw/gitfile' | tr '\n' ' ')"
echo "renameMountpoint=$(r mv /probe/rw/child /probe/rw/child2 | tr '\n' ' ')"
echo "hardlinkAcross=$(r ln /probe/ro/seed /probe/rw/hl | tr '\n' ' ')"
ln -s /probe/ro /probe/rw/lnk 2>/dev/null
echo "symlinkIntoRo=$(r touch /probe/rw/lnk/x | tr '\n' ' ')"
`;

export type DockerCli = (args: string[]) => string;
const dockerCli: DockerCli = (args) => execFileSync("docker", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], timeout: 60_000, killSignal: "SIGKILL" });

export function parseObservation(logs: string): IsolationObservation {
  const kv = new Map<string, string>();
  for (const line of logs.split("\n")) { const i = line.indexOf("="); if (i > 0) kv.set(line.slice(0, i), line.slice(i + 1).trim()); }
  const get = (k: string) => kv.get(k) ?? "";
  return {
    apparmor: get("apparmor"), seccomp: get("seccomp"), capEff: get("capEff"), capBnd: get("capBnd"),
    roWrite: get("roWrite"), rwWrite: get("rwWrite"), roChildInRw: get("roChildInRw"), roFileAppend: get("roFileAppend"),
    renameMountpoint: get("renameMountpoint"), hardlinkAcross: get("hardlinkAcross"), symlinkIntoRo: get("symlinkIntoRo"),
  };
}

/** 用与 Sandcastle 相同的启动形态（--user 宿主 uid:gid，无 cap/security 参数）起一次临时探针容器；
 * 无网络、无凭据、只挂临时目录；容器按名删除。 */
export function probeDockerIsolation(imageName: string, cli: DockerCli = dockerCli): IsolationObservation {
  // Colima 只共享 $HOME：探针目录必须在其下，否则容器看到的是 VM 内空目录，观测无意义。
  const base = join(homedir(), ".cache", "sandcastle-isolation-probe");
  mkdirSync(base, { recursive: true });
  const root = mkdtempSync(join(base, "run-"));
  const name = `sandcastle-probe-${randomUUID()}`;
  try {
    const ro = join(root, "ro"), rw = join(root, "rw"), child = join(root, "child"), gitfile = join(root, "gitfile");
    for (const d of [ro, rw, child]) mkdirSync(d);
    writeFileSync(join(ro, "seed"), "seed");
    writeFileSync(gitfile, "gitdir: x\n");
    // 子挂载点需存在于 rw 之下：宿主侧预建，容器内才能叠加挂载。
    mkdirSync(join(rw, "child"));
    writeFileSync(join(rw, "gitfile"), "");
    const uid = process.getuid?.() ?? 1000, gid = process.getgid?.() ?? 1000;
    const logs = cli(["run", "--rm", "--name", name, "--network", "none", "--user", `${uid}:${gid}`,
      "-v", `${ro}:/probe/ro:ro`, "-v", `${rw}:/probe/rw`,
      "-v", `${child}:/probe/rw/child:ro`, "-v", `${gitfile}:/probe/rw/gitfile:ro`,
      "--entrypoint", "sh", imageName, "-c", PROBE_SCRIPT]);
    return parseObservation(logs);
  } catch (error) {
    // 超时/中断时 --rm 不一定生效：只按本次名字回收。
    if ((error as { killed?: boolean; signal?: string }).killed || (error as { signal?: string }).signal) { try { cli(["rm", "-f", name]); } catch { /* 无残留 */ } }
    throw error;
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

/** #1617：门禁——合同在这个 daemon 上逐项成立才放行；否则 FAIL-loud，不认领、保留 checkpoint/MERGE_HEAD。 */
export function assertContainerIsolationContract(imageName: string, probe: (image: string) => IsolationObservation = probeDockerIsolation): IsolationObservation {
  let observation: IsolationObservation;
  try { observation = probe(imageName); }
  catch (error) { throw new Error(`[FAIL-loud #1617] Docker 隔离合同探针无法运行；不认领、不扩大容器权限。${String(error).slice(0, 240)}`); }
  const bad = violations(observation);
  if (bad.length) throw new Error(`[FAIL-loud #1617] Docker 隔离合同不成立：${bad.join("；")}。不认领、不扩大容器权限。`);
  return observation;
}

/** 宿主封包前：仓内可控文件不得触发宿主配置的命令型 filter/diff/merge 驱动（仅 lfs 白名单），
 * hooksPath 必须指向 worktree 之外。配置只来自宿主 config 与（只读的）.git/config。 */
export function assertHostPackagingSafe(
  cwd: string,
  git: (args: string[]) => string = (args) => execFileSync("git", ["-C", cwd, ...args], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024, stdio: ["ignore", "pipe", "pipe"] }),
  allowedDrivers: readonly string[] = ["lfs"],
  probe: Pick<PathProbe, "realpath"> = fsProbe,
): void {
  let config = "";
  try { config = git(["config", "--show-origin", "--get-regexp", "^(filter|diff|merge)\\..*\\.(clean|smudge|process|textconv|driver|command)$|^core\\.(hookspath|fsmonitor)$"]); }
  catch { config = ""; }
  const drivers = new Set<string>();
  for (const line of config.split("\n").filter(Boolean)) {
    const m = line.match(/^\S+\s+(\S+)\s/);
    const key = (m?.[1] ?? "").toLowerCase();
    const driver = key.match(/^(?:filter|diff|merge)\.(.+)\.(?:clean|smudge|process|textconv|driver|command)$/);
    if (driver && !allowedDrivers.includes(driver[1]!)) drivers.add(driver[1]!);
    if (key === "core.fsmonitor") throw new Error("宿主配置了 core.fsmonitor，拒绝封包");
    if (key === "core.hookspath") {
      const value = line.split(/\s+/).slice(2).join(" ");
      if (!isAbsolute(value)) throw new Error("core.hooksPath 不是绝对路径，拒绝封包");
      // realpath 跟随符号链接：目录经链接回到 worktree 内同样拒绝。
      const root = realpathLoose(cwd, probe), target = realpathLoose(value, probe);
      // worktree 自己的 .git 在容器内只读，其下的 hooks 目录可信。
      if (within(root, target) && !within(join(root, ".git"), target)) throw new Error("core.hooksPath（含符号链接解析后）位于 worktree 内，拒绝封包");
    }
  }
  if (!drivers.size) return;
  const files = git(["ls-files", "-z"]).split("\0").filter(Boolean);
  if (!files.length) return;
  const attrs = execFileSync("git", ["-C", cwd, "check-attr", "-z", "--stdin", "filter", "diff", "merge"], { input: files.join("\0") + "\0", encoding: "utf8", maxBuffer: 256 * 1024 * 1024 }).split("\0");
  for (let i = 0; i + 2 < attrs.length; i += 3) {
    if (drivers.has(attrs[i + 2]!)) throw new Error(`仓内文件 ${attrs[i]} 引用宿主命令型驱动 ${attrs[i + 2]}，拒绝封包`);
  }
}
