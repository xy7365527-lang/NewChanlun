import type { BindMountSandboxHandle, SandboxProvider } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { closeSync, existsSync, lstatSync, openSync, readFileSync, realpathSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { join, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";
import { CodexTranscript, OUTPUT_SCHEMA, assertRoute, type ChildModel } from "./codex-child.ts";
import { codexWorker } from "./codex-route.mts";
import { assertSwarmExecutionReady, dockerCliProxyEnv, swarmSandbox } from "./swarm-route.mts";
import { codeIdentity, type Admission } from "./docker-acceptance.mts";
import { containerEnvLeaks, declaredContainerEnv, hardenMounts, type MountSpec } from "./docker-isolation.mts";
import { assertMountsExcludeAcceptance } from "./acceptance-dir.mts";
import { gitEnvironment } from "./git-environment.ts";

export interface ReviewLaunch {
  run_id: string; launch_id: string; pr_number: number; model: ChildModel;
  base_sha: string; merge_base_sha: string; head_sha: string; controller_code_head: string;
  checkout: string; prompt: string; mode: "review";
}
export const REVIEW_IMAGE = "sandcastle:newchanlun";
const inside = (root: string, path: string) => path === root || path.startsWith(root + sep);

export function readLaunch(folder: string): ReviewLaunch {
  const path = join(folder, "launch.json");
  if (!lstatSync(path).isFile() || lstatSync(path).isSymbolicLink()) throw new Error("launch intent 不是普通文件");
  const launch = JSON.parse(readFileSync(path, "utf8")) as ReviewLaunch;
  assertRoute(launch.model);
  for (const sha of [launch.base_sha, launch.merge_base_sha, launch.head_sha, launch.controller_code_head]) {
    if (!/^[a-f0-9]{40}$/.test(sha)) throw new Error("launch SHA 无效");
  }
  if (!/^[a-f0-9-]{36}$/.test(launch.run_id) || !/^[a-f0-9-]{36}$/.test(launch.launch_id)
    || !Number.isSafeInteger(launch.pr_number) || launch.pr_number <= 0 || launch.mode !== "review"
    || typeof launch.prompt !== "string" || !launch.prompt.trim()) throw new Error("launch intent 不完整");
  if (realpathSync(launch.checkout) !== launch.checkout || !lstatSync(join(launch.checkout, ".git")).isDirectory()) {
    throw new Error("只接受独立无凭据 clone，不接受 linked worktree");
  }
  if (inside(launch.checkout, folder) || inside(folder, launch.checkout)) throw new Error("源码与宿主回执目录重叠");
  if (existsSync(join(launch.checkout, ".codex", "config.toml"))) throw new Error("候选项目 Codex 配置未获准入");
  return launch;
}

export function reviewMounts(mounts: readonly MountSpec[], checkout: string): MountSpec[] {
  assertMountsExcludeAcceptance(mounts);
  if (mounts.length !== 1 || mounts[0]!.hostPath !== checkout || mounts[0]!.sandboxPath !== "/review") {
    throw new Error("review 只允许候选 clone 一个挂载，不接收宿主 home、Git、session 或状态目录挂载");
  }
  return hardenMounts([{ hostPath: checkout, sandboxPath: "/review", readonly: true }], checkout)
    .map((m) => ({ ...m, readonly: true }));
}

type CreateOptions = { worktreePath: string; hostRepoPath: string; mounts: MountSpec[]; env: Record<string, string> };
type DockerProvider = SandboxProvider & { create(options: CreateOptions): Promise<BindMountSandboxHandle> };

export function snapshot(checkout: string): { head: string; branch: string; status: string } {
  const env = { ...gitEnvironment(), GIT_CONFIG_NOSYSTEM: "1", GIT_CONFIG_GLOBAL: "/dev/null", GIT_NO_REPLACE_OBJECTS: "1" };
  const git = (...args: string[]) => execFileSync("git", ["-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false", "-C", checkout, ...args],
    { env, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  return { head: git("rev-parse", "HEAD"), branch: git("rev-parse", "--abbrev-ref", "HEAD"), status: git("status", "--porcelain", "--untracked-files=all") };
}

export function verifyReviewAdmission(admission: Admission): void {
  if (!admission.accepted) throw new Error("外部独审必须先完成当前代码 head 的 Docker 验收，不能占用旧受控行");
}

export async function probeReviewFilesystem(handle: BindMountSandboxHandle, folder: string): Promise<Record<string, string>> {
  const canary = join(folder, "nonsecret-canary.txt");
  writeFileSync(canary, "external-review non-secret canary", { flag: "wx", mode: 0o600 });
  try {
    const program = `const fs=require('node:fs');const out={};
      const probe=(k,f)=>{try{f();out[k]='ALLOWED'}catch(e){out[k]=e.code}};
      probe('source_write',()=>fs.writeFileSync('/review/.external-review-write-probe','probe'));
      probe('git_write',()=>{const fd=fs.openSync('/review/.git/config','a');fs.closeSync(fd)});
      probe('host_read',()=>fs.readFileSync(${JSON.stringify(canary)}));
      probe('state_write',()=>fs.writeFileSync(${JSON.stringify(join(folder, ".state-probe"))},'probe'));
      console.log(JSON.stringify(out));`;
    const quote = (s: string) => "'" + s.replaceAll("'", "'\\''") + "'";
    const result = await handle.exec(`node -e ${quote(program)}`);
    if (result.exitCode) throw new Error("review 文件隔离探针无法运行");
    const observation = JSON.parse(result.stdout.trim());
    if (observation.source_write !== "EROFS" || observation.git_write !== "EROFS"
      || !["ENOENT", "EACCES"].includes(observation.host_read)
      || !["ENOENT", "EACCES", "EROFS"].includes(observation.state_write)) {
      throw new Error("review 源码、Git、宿主 canary 或状态目录可被访问");
    }
    return observation;
  } finally { unlinkSync(canary); }
}

function save(folder: string, name: string, value: unknown): void {
  const tmp = join(folder, `${name}.${process.pid}.tmp`);
  writeFileSync(tmp, JSON.stringify(value, null, 2) + "\n", { mode: 0o600 });
  renameSync(tmp, join(folder, name));
}

export interface WorkerDependencies {
  ready(image: string): string;
  sandbox(image: string): DockerProvider;
  identity(): { head: string; dirty: string };
  inspect(id: string): { Id: string; Image: string; Mounts: Array<{ Source: string; Destination: string; RW: boolean }>; Config: { Env: string[] } };
  imageEnv(image: string): string[];
  timeoutMs: number;
}
const dependencies: WorkerDependencies = {
  ready: (image) => assertSwarmExecutionReady(image, { rows: [], verify: verifyReviewAdmission }),
  sandbox: (image) => swarmSandbox(image, dockerCliProxyEnv()) as DockerProvider,
  identity: () => codeIdentity(),
  inspect: (id) => JSON.parse(execFileSync("docker", ["inspect", id], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }))[0],
  imageEnv: (image) => JSON.parse(execFileSync("docker", ["image", "inspect", image, "--format", "{{json .Config.Env}}"],
    { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] })) ?? [],
  timeoutMs: 5_400_000,
};

export async function runExternalReview(folder: string, deps: WorkerDependencies = dependencies): Promise<void> {
  const launch = readLaunch(folder);
  const lock = openSync(join(folder, "supervisor.lock"), "wx", 0o600);
  closeSync(lock);
  const record: Record<string, unknown> = { ...launch, prompt: undefined, status: "failed", mode: "review",
    sandbox: "docker-read-only", process_pid: process.pid, started_at: new Date().toISOString(),
    phase: "controller", exit_code: null, turn_completed: false, session_id: null, result: null };
  let handle: BindMountSandboxHandle | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let cancelled = false;
  const cancel = () => { cancelled = true; void handle?.close(); };
  process.once("SIGTERM", cancel); process.once("SIGINT", cancel);
  try {
    const identity = deps.identity();
    if (identity.head !== launch.controller_code_head || identity.dirty) throw new Error("controller 代码已漂移或未提交");
    record.phase = "checkout";
    const before = snapshot(launch.checkout); record.before = before;
    if (before.head !== launch.head_sha || before.status || before.branch !== "HEAD") throw new Error("checkout 不干净或 head 不符");
    record.phase = "admission";
    const image = deps.ready(REVIEW_IMAGE); record.image_id = image;
    const sandbox = deps.sandbox(image);
    const mounts = reviewMounts([{ hostPath: launch.checkout, sandboxPath: "/review" }], launch.checkout);
    record.phase = "container";
    handle = await sandbox.create({ worktreePath: launch.checkout, hostRepoPath: launch.checkout,
      mounts, env: declaredContainerEnv(sandbox.env) });
    const hostname = await handle.exec("cat /etc/hostname");
    const id = hostname.stdout.trim();
    if (hostname.exitCode || !/^[a-f0-9]{12,64}$/.test(id)) throw new Error("无法确认真实容器身份");
    const inspected = deps.inspect(id);
    if (inspected.Image !== image || !inspected.Id.startsWith(id) || inspected.Mounts.length !== mounts.length
      || mounts.some((m) => !inspected.Mounts.some((actual) => actual.Source === m.hostPath
        && actual.Destination === m.sandboxPath && !actual.RW)) || containerEnvLeaks(inspected.Config.Env, deps.imageEnv(image)).length) {
      throw new Error("实际容器挂载、镜像或 env 不符合只读合同");
    }
    record.container_id = inspected.Id;
    record.phase = "filesystem";
    record.filesystem = await probeReviewFilesystem(handle, folder);
    save(folder, "supervisor.json", record);
    record.phase = "provider";
    const provider = codexWorker({ docker: true, model: launch.model, effort: "xhigh" });
    const schema = join(folder, "output-schema.json");
    writeFileSync(schema, JSON.stringify(OUTPUT_SCHEMA), { mode: 0o600 });
    await handle.copyFileIn(schema, "/tmp/external-review-schema.json");
    const built = provider.buildPrintCommand({ prompt: launch.prompt, dangerouslySkipPermissions: false });
    if (built.stdin !== launch.prompt || !built.command.includes("--disable multi_agent")) throw new Error("Codex 命令协议已漂移");
    const command = built.command + " --output-schema /tmp/external-review-schema.json -c 'features.apps=false' -c 'features.plugins=false'"
      + " -c 'web_search=\"disabled\"' -c 'shell_environment_policy.inherit=\"core\"' -";
    const transcript = new CodexTranscript(provider);
    const events = openSync(join(folder, "events.jsonl"), "wx", 0o600);
    let timedOut = false;
    timer = setTimeout(() => { timedOut = true; void handle?.close(); }, deps.timeoutMs);
    let execution;
    record.phase = "model";
    try {
      execution = await handle.exec(command, { cwd: "/review", stdin: built.stdin, onLine(line) {
        writeFileSync(events, line + "\n");
        transcript.accept(line);
      } });
    } finally { closeSync(events); }
    record.exit_code = execution.exitCode;
    record.turn_completed = transcript.turnCompleted;
    record.session_id = transcript.sessionId;
    writeFileSync(join(folder, "stderr.log"), execution.stderr, { mode: 0o600 });
    if (cancelled || timedOut || execution.exitCode !== 0) throw new Error(cancelled ? "监督已取消" : timedOut ? "独审超时" : "Codex 退出失败");
    record.phase = "transcript";
    const output = transcript.output();
    if (!provider.sessionStorage) throw new Error("模型未提供 session storage");
    record.phase = "session";
    await provider.sessionStorage.captureToHost({ hostCwd: launch.checkout, sandboxCwd: "/review",
      sessionId: transcript.sessionId!, handle });
    const log = await provider.sessionStorage.readHostSession(launch.checkout, transcript.sessionId!);
    if (!log) throw new Error("缺少真实 session 日志");
    const first = JSON.parse(log.split("\n")[0]!);
    if (first.type !== "session_meta" || first.payload?.id !== transcript.sessionId) throw new Error("session 日志身份不符");
    writeFileSync(join(folder, "session.jsonl"), log, { mode: 0o600, flag: "wx" });
    record.phase = "checkout-after";
    record.after = snapshot(launch.checkout);
    if (JSON.stringify(record.before) !== JSON.stringify(record.after)) throw new Error("只读 review 后 checkout 变化");
    record.status = "completed"; record.result = output; record.phase = "completed";
  } catch (error) {
    record.status = cancelled ? "cancelled" : "failed";
    // SDK 传输异常可能含 Docker argv/env，只持久化错误类别，不记录其原始消息。
    record.failure_reason = error instanceof Error ? error.name : "SupervisionError";
    try { record.after = snapshot(launch.checkout); } catch { record.after = null; }
  } finally {
    if (timer) clearTimeout(timer);
    process.removeListener("SIGTERM", cancel); process.removeListener("SIGINT", cancel);
    try { await handle?.close(); } catch { record.status = "failed"; record.failure_reason = "ContainerCleanupError"; }
    record.ended_at = new Date().toISOString();
    save(folder, "receipt.json", record);
  }
}

async function main(): Promise<void> {
  if (process.argv.length !== 4 || process.argv[2] !== "--output-dir") throw new Error("只接受宿主 --output-dir");
  const folder = resolve(process.argv[3]!);
  if (folder !== realpathSync(folder)) throw new Error("回执目录含符号链接");
  await runExternalReview(folder);
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch(() => { process.stderr.write("外部独审监督入口失败\n"); process.exitCode = 2; });
}
