// #1456：原生 Codex 子代理管理一个外部 Sandcastle/Codex 工蜂；一次运行，不取票、不合入。
import type { AgentProvider, NoSandboxHandle } from "@ai-hero/sandcastle";
import { createHash, randomUUID } from "node:crypto";
import { execFileSync, spawn } from "node:child_process";
import {
  appendFileSync, closeSync, existsSync, lstatSync, mkdirSync, openSync,
  readFileSync, realpathSync, renameSync, unlinkSync, writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, sep } from "node:path";
import { StringDecoder } from "node:string_decoder";
import { MODEL, EFFORT, readRouterConfiguration } from "./codex-router-runtime.ts";

export { MODEL, EFFORT };
export type ChildMode = "review" | "execute";
export type ChildStatus = "running" | "completed" | "failed" | "cancelled";
/** 输出渠道等监督设施失败属于 failed，不冒充管理者主动取消。 */
export class SupervisionError extends Error {}

export interface ChildOptions {
  ticket: number;
  parentThreadId: string;
  managerAgentId: string;
  checkout: string;
  expectedHead: string;
  mode: ChildMode;
  promptFile: string;
  outputDir: string;
  /** 管理者所在 checkout；默认为启动入口的 cwd。不能同时也是工蜂 checkout。 */
  controllerCheckout?: string;
  idleTimeoutSeconds?: number;
  timeoutSeconds?: number;
}

export interface WorkerOutput {
  status: "completed" | "blocked" | "failed";
  summary: string;
  findings: string[];
  validation: string[];
}

export interface CheckoutSnapshot {
  root: string;
  branch: string;
  head: string;
  status: string;
}

export interface ChildRecord {
  schema_version: 1;
  run_id: string;
  ticket: number;
  parent_thread_id: string;
  manager_agent_id: string;
  manager_kind: "codex_native_subagent";
  worker_kind: "sandcastle_external_codex";
  native_subagent: false;
  status: ChildStatus;
  mode: ChildMode;
  model: typeof MODEL;
  effort: typeof EFFORT;
  sandbox_backend: "no-sandbox";
  codex_sandbox: "read-only" | "workspace-write";
  approval_policy: "never";
  automatic_claim: false;
  automatic_push: false;
  automatic_merge: false;
  checkout_preserved: true;
  expected_head: string;
  checkout_before: CheckoutSnapshot | null;
  checkout_after: CheckoutSnapshot | null;
  started_at: string;
  ended_at: string | null;
  process_pid: number | null;
  process_exit: { code: number | null; signal: NodeJS.Signals | null } | null;
  external_session_id: string | null;
  external_session_log: string | null;
  output_dir: string;
  stdout_log: string;
  stderr_log: string;
  events_log: string;
  prompt_sha256: string | null;
  result: WorkerOutput | null;
  failure_reason: string | null;
}

export const OUTPUT_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    status: { type: "string", enum: ["completed", "blocked", "failed"] },
    summary: { type: "string" },
    findings: { type: "array", items: { type: "string" } },
    validation: { type: "array", items: { type: "string" } },
  },
  required: ["status", "summary", "findings", "validation"],
} as const;

function shellQuote(value: string): string {
  return "'" + value.replaceAll("'", "'\\''") + "'";
}

function inside(path: string, root: string): boolean {
  const suffix = relative(root, path);
  return suffix === "" || (!isAbsolute(suffix) && suffix !== ".." && !suffix.startsWith(".." + sep));
}

function git(root: string, args: string[]): string {
  return execFileSync("git", [
    "--no-replace-objects", "-c", "core.filemode=true", "-c", "diff.ignoreSubmodules=none",
    "-C", root, ...args,
  ], {
    encoding: "utf8", stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" },
  }).trim();
}

export function snapshotCheckout(checkout: string): CheckoutSnapshot {
  const root = realpathSync(git(checkout, ["rev-parse", "--show-toplevel"]));
  // #1456：status 会相信这些 index flags，从而把真实修改藏在“干净”结果里。
  // 只检查索引标志，不重写配置/索引，也不另行重算整个工作区的内容散列。
  const hidden = git(root, ["ls-files", "-v", "-z"]).split("\0")
    .filter((entry) => entry && (entry[0] === "S" || /^[a-z]$/.test(entry[0])));
  if (hidden.length) throw new Error(`checkout 含 assume-unchanged/skip-worktree 标志（${hidden.length} 项），不能证明干净`);
  return {
    root,
    branch: git(root, ["symbolic-ref", "--quiet", "--short", "HEAD"]),
    head: git(root, ["rev-parse", "HEAD"]),
    status: git(root, ["status", "--porcelain=v1", "--untracked-files=all", "--ignore-submodules=none"]),
  };
}

export function validateOptions(options: ChildOptions): void {
  if (!Number.isSafeInteger(options.ticket) || options.ticket < 1) throw new Error("ticket 必须是正整数");
  for (const [name, value] of [["parent-thread-id", options.parentThreadId], ["manager-agent-id", options.managerAgentId]]) {
    if (typeof value !== "string" || !/^[A-Za-z0-9][A-Za-z0-9_./:-]{0,199}$/.test(value)) {
      // 原生工位的 canonical name 可由 / 开始。
      if (name !== "manager-agent-id" || typeof value !== "string" || !/^\/[A-Za-z0-9_./:-]{1,199}$/.test(value)) {
        throw new Error(`${name} 缺失或格式无效`);
      }
    }
  }
  if (!/^[a-f0-9]{40}$/.test(options.expectedHead)) throw new Error("expected-head 必须是完整的 40 位小写提交 SHA");
  if (options.mode !== "review" && options.mode !== "execute") throw new Error("mode 只能是 review 或 execute");
  for (const [name, path] of [["checkout", options.checkout], ["prompt-file", options.promptFile], ["output-dir", options.outputDir]]) {
    if (!path || !isAbsolute(path)) throw new Error(`${name} 必须是绝对路径`);
  }
  for (const seconds of [options.idleTimeoutSeconds ?? 600, options.timeoutSeconds ?? 3600]) {
    if (!Number.isFinite(seconds) || seconds <= 0 || seconds > 43200) throw new Error("超时必须在 0 到 43200 秒之间");
  }
}

export function preflightCheckout(options: ChildOptions): CheckoutSnapshot {
  validateOptions(options);
  const before = snapshotCheckout(options.checkout);
  if (before.root !== realpathSync(options.checkout)) throw new Error("checkout 必须指向独立 checkout 的根目录");
  const controller = realpathSync(options.controllerCheckout ?? process.cwd());
  if (inside(before.root, controller) || inside(controller, before.root)) {
    throw new Error("工蜂必须使用管理者工作目录之外的独立 checkout");
  }
  if (!/^codex\/[A-Za-z0-9][A-Za-z0-9_./-]*$/.test(before.branch)) {
    throw new Error("工蜂仅接受 codex/* 分支，拒绝 main、其他分支和 detached HEAD");
  }
  if (before.head !== options.expectedHead) throw new Error("checkout HEAD 与 expected-head 不同");
  if (before.status) throw new Error("工蜂 checkout 不干净，包含未提交或未跟踪文件");
  // #1456：不让项目配置重新引入 MCP、hooks、权限或凭据；只查文件存在性，不读取其内容。
  if (existsSync(join(before.root, ".codex", "config.toml"))) {
    throw new Error("入口不接受带项目 .codex/config.toml 的 checkout；需管理者先审定配置接线");
  }
  return before;
}

/** 只透传启动/认证定位所需环境，不提取认证，不加载项目 .env，不转发 API/GitHub 密钥。 */
export function workerEnvironment(source: NodeJS.ProcessEnv = process.env): Record<string, string> {
  const result: Record<string, string> = {};
  for (const key of [
    "PATH", "HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "CODEX_HOME",
    // 路由沿用宿主；不打印代理地址，也不把环境写入运行记录。
    "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY", "http_proxy", "https_proxy", "all_proxy", "no_proxy",
  ]) {
    if (source[key]) result[key] = source[key]!;
  }
  result.NO_COLOR = "1";
  return result;
}

/** SDK 0.12 的 codex() 忽略 dangerouslySkipPermissions=false；在调用边界做精确、失败即停的适配。 */
export async function boundedCodex(mode: ChildMode, schemaPath: string, env = workerEnvironment()): Promise<AgentProvider> {
  // 根 package 为 CommonJS、Sandcastle 只导出 ESM；显式动态导入，不改变全仓模块制。
  const { codex } = await import("@ai-hero/sandcastle");
  const provider = codex(MODEL, {
    effort: EFFORT,
    sessionStorage: { hostSessionsDir: join(env.CODEX_HOME ?? join(env.HOME ?? homedir(), ".codex"), "sessions") },
  });
  return {
    ...provider,
    buildPrintCommand(options) {
      if (options.resumeSession || options.forkSession) throw new Error("一次性工蜂入口不允许隐式 resume/fork");
      const built = provider.buildPrintCommand({ ...options, dangerouslySkipPermissions: false });
      const bypass = " --dangerously-bypass-approvals-and-sandbox";
      if (!built.command.startsWith("codex exec --json ") || built.command.split(bypass).length !== 2 || built.stdin !== options.prompt) {
        throw new Error("Sandcastle codex() 命令契约已变，拒绝猜测权限替换");
      }
      const sandbox = mode === "review" ? "read-only" : "workspace-write";
      const route = readRouterConfiguration(env);
      const routing = [
        'model_provider="codex-router"',
        'model_providers.codex-router.name="Codex Router"',
        `model_providers.codex-router.base_url=${JSON.stringify(route.baseUrl)}`,
        'model_providers.codex-router.wire_api="responses"',
        'model_providers.codex-router.requires_openai_auth=true',
        'model_providers.codex-router.supports_websockets=false',
        `model_catalog_json=${JSON.stringify(route.catalog)}`,
      ].map((value) => ` -c ${shellQuote(value)}`).join("");
      const command = built.command
        .replace("codex exec", "codex -a never exec")
        .replace(bypass, ` --sandbox ${sandbox} --ignore-user-config --color never`)
        + ` --output-schema ${shellQuote(schemaPath)}`
        + routing
        + ` -c 'sandbox_workspace_write.network_access=false' -c 'web_search="disabled"'`
        + ` -c 'shell_environment_policy.inherit="core"' --disable multi_agent -`;
      if (/dangerously|danger-full-access|--full-auto|on-request/.test(command)) throw new Error("检测到不允许的权限参数");
      return { command, stdin: built.stdin };
    },
  };
}

export class CodexTranscript {
  sessionId: string | null = null;
  finalText: string | null = null;
  turnCompleted = false;
  failure: string | null = null;
  constructor(private readonly provider: AgentProvider) {}

  accept(line: string): void {
    if (!line.trim()) return;
    let event: Record<string, unknown>;
    try {
      const value: unknown = JSON.parse(line);
      if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("not object");
      event = value as Record<string, unknown>;
      if (typeof event.type !== "string") throw new Error("type missing");
    } catch {
      this.failure ??= "Codex stdout 不是有效的事件 JSONL";
      return;
    }
    if (["error", "turn.failed", "turn.cancelled"].includes(event.type as string)) {
      this.failure ??= `Codex 报告 ${event.type}`;
    }
    if (event.type === "turn.completed") this.turnCompleted = true;
    const item = event.item as Record<string, unknown> | undefined;
    if (event.type === "item.completed" && item?.type === "agent_message" && typeof item.text === "string") {
      this.finalText = item.text;
    }
    for (const parsed of this.provider.parseStreamLine(line)) {
      if (parsed.type === "session_id") {
        if (!/^[a-zA-Z0-9-]{1,100}$/.test(parsed.sessionId)) this.failure ??= "Codex session_id 格式无效";
        else if (this.sessionId && this.sessionId !== parsed.sessionId) this.failure ??= "同一次运行出现多个 Codex session_id";
        else this.sessionId = parsed.sessionId;
      }
    }
  }

  output(): WorkerOutput {
    if (this.failure) throw new Error(this.failure);
    if (!this.sessionId || !this.turnCompleted || !this.finalText?.trim()) throw new Error("缺少 session_id、turn.completed 或最终结果");
    let value: unknown;
    try { value = JSON.parse(this.finalText); } catch { throw new Error("Codex 最终结果不是有效 JSON"); }
    const result = value as Partial<WorkerOutput> | null;
    if (!result || typeof result !== "object" || Array.isArray(result)
      || !["completed", "blocked", "failed"].includes(result.status ?? "")
      || typeof result.summary !== "string" || !result.summary.trim()
      || !Array.isArray(result.findings) || result.findings.some((x) => typeof x !== "string")
      || !Array.isArray(result.validation) || result.validation.some((x) => typeof x !== "string")
      || Object.keys(result).sort().join(",") !== "findings,status,summary,validation") {
      throw new Error("Codex 最终结果不符合工蜂结果契约");
    }
    return result as WorkerOutput;
  }
}

interface ProcessObservation {
  signal?: AbortSignal;
  env: Record<string, string>;
  stdoutFile: string;
  stderrFile: string;
  idleTimeoutSeconds: number;
  timeoutSeconds: number;
  onSpawn(pid: number): void;
  onExit(code: number | null, signal: NodeJS.Signals | null): void;
  /** 日志故障的定向注入；CLI 不暴露此替换。 */
  writeLog?: (fd: number, chunk: Uint8Array, stream: "stdout" | "stderr") => void;
}

/**
 * #1456：保留 noSandbox 的 host 生命周期，替换其无取消、signal→exit0 的 exec。
 * 不调用 SDK 顶层 run/createWorktree：这些入口会读 .sandcastle/.env 并管理分支/合入。
 * 只运行当前进程创建并记录的独立进程组；没有宿主全局进程扫描。
 * noSandbox/read-only 限制不等于整个宿主的读取隔离；它不是容器。
 * 入口不读取 .env/认证内容；工蜂不读凭据仍包含任务指令约束。
 */
export async function supervisedNoSandbox(checkout: string, observation: ProcessObservation): Promise<NoSandboxHandle> {
  const { noSandbox } = await import("@ai-hero/sandcastle/sandboxes/no-sandbox");
  // SDK 的 NoSandboxProvider 公共类型隐藏 create；0.12 实装导出该低层生命周期。
  const backend = noSandbox() as unknown as {
    create?: (options: { worktreePath: string; env: Record<string, string> }) => Promise<NoSandboxHandle>;
  };
  if (typeof backend.create !== "function") throw new Error("Sandcastle noSandbox 生命周期契约已变");
  const handle = await backend.create({ worktreePath: checkout, env: {} });
  return {
    ...handle,
    exec(command, options) {
      if (options?.sudo || (options?.cwd && options.cwd !== checkout)) throw new Error("工蜂不能更改执行根或使用 sudo");
      return new Promise((resolveResult, reject) => {
        if (observation.signal?.aborted) { reject(new Error("启动前已取消")); return; }
        let stdoutFd: number | undefined;
        let stderrFd: number | undefined;
        try {
          stdoutFd = openSync(observation.stdoutFile, "wx", 0o600);
          stderrFd = openSync(observation.stderrFile, "wx", 0o600);
        } catch (error) {
          if (stdoutFd !== undefined) { try { closeSync(stdoutFd); } catch { /* 没有子进程。 */ } }
          reject(error); return;
        }
        // command 只来自上面的固定 SDK 适配器，prompt 通过 stdin 传入。
        let child: ReturnType<typeof spawn>;
        try {
          child = spawn("/bin/sh", ["-c", `exec ${command}`], {
            cwd: checkout, env: observation.env, detached: true, stdio: ["pipe", "pipe", "pipe"],
          });
        } catch (error) {
          for (const fd of [stdoutFd, stderrFd]) { try { closeSync(fd); } catch { /* 没有子进程。 */ } }
          reject(error); return;
        }
        let buffer = "";
        const decoder = new StringDecoder("utf8");
        let issue: string | null = null;
        let killTimer: ReturnType<typeof setTimeout> | undefined;
        let idleTimer: ReturnType<typeof setTimeout> | undefined;
        let totalBytes = 0;
        const stop = (reason: string) => {
          issue ??= reason;
          if (!child.pid || killTimer) return;
          try { process.kill(-child.pid, "SIGTERM"); } catch (error) {
            if ((error as NodeJS.ErrnoException).code !== "ESRCH") issue = "无法停止工蜂进程组";
          }
          killTimer = setTimeout(() => {
            try { process.kill(-child.pid!, "SIGKILL"); } catch { /* 进程组已退出；仍等待 close。 */ }
          }, 1000);
        };
        const guarded = (label: string, action: () => void) => {
          try { action(); } catch (error) {
            stop(`${label}：${error instanceof Error ? error.message : "未知错误"}`);
          }
        };
        const resetIdle = () => {
          clearTimeout(idleTimer);
          idleTimer = setTimeout(() => stop("工蜂输出空闲超时"), observation.idleTimeoutSeconds * 1000);
        };
        const abort = () => stop("管理者取消工蜂");
        observation.signal?.addEventListener("abort", abort, { once: true });
        const fullTimer = setTimeout(() => stop("工蜂总运行超时"), observation.timeoutSeconds * 1000);
        resetIdle();
        child.on("spawn", () => guarded("工蜂启动通知失败", () => observation.onSpawn(child.pid!)));
        child.on("error", () => { issue ??= "无法启动 Codex 进程"; });
        child.stdin!.on("error", () => stop("Codex 提前关闭 prompt 输入"));
        child.stdout!.on("data", (chunk: Buffer) => guarded("工蜂 stdout 记录或回调失败", () => {
          if (observation.writeLog) observation.writeLog(stdoutFd!, chunk, "stdout");
          else appendFileSync(stdoutFd!, chunk);
          resetIdle();
          totalBytes += chunk.length;
          if (totalBytes > 32 * 1024 * 1024) { stop("工蜂 stdout 超过 32 MiB 界限"); return; }
          buffer += decoder.write(chunk);
          let end: number;
          while ((end = buffer.indexOf("\n")) >= 0) {
            const line = buffer.slice(0, end).replace(/\r$/, "");
            buffer = buffer.slice(end + 1);
            options?.onLine?.(line);
          }
        }));
        child.stderr!.on("data", (chunk: Buffer) => guarded("工蜂 stderr 记录失败", () => {
          if (observation.writeLog) observation.writeLog(stderrFd!, chunk, "stderr");
          else appendFileSync(stderrFd!, chunk);
          resetIdle();
        }));
        child.stdout!.on("error", () => stop("工蜂 stdout 读取失败"));
        child.stderr!.on("error", () => stop("工蜂 stderr 读取失败"));
        child.on("close", async (code, signal) => {
          clearTimeout(fullTimer); clearTimeout(idleTimer);
          observation.signal?.removeEventListener("abort", abort);
          buffer += decoder.end();
          if (buffer) { try { options?.onLine?.(buffer); } catch { issue ??= "工蜂尾行解析失败"; } }
          for (const fd of [stdoutFd!, stderrFd!]) {
            try { closeSync(fd); } catch { issue ??= "工蜂日志关闭失败"; }
          }
          // 根进程可能先结束；清理它的同组子进程后才让管理者得到最终结果。
          if (child.pid) {
            try { process.kill(-child.pid, "SIGKILL"); } catch (error) {
              if ((error as NodeJS.ErrnoException).code !== "ESRCH") issue ??= "工蜂进程组清理失败";
            }
            // 等待本次独立进程组真正消失。无法证明退出则失败，不能提前宣称完成。
            const deadline = Date.now() + 3000;
            for (;;) {
              try { process.kill(-child.pid, 0); } catch (error) {
                if ((error as NodeJS.ErrnoException).code === "ESRCH") break;
                issue ??= "无法确认工蜂进程组已退出";
                break;
              }
              if (Date.now() >= deadline) { issue ??= "工蜂进程组退出未确认"; break; }
              await new Promise((done) => setTimeout(done, 25));
            }
          }
          clearTimeout(killTimer);
          try { observation.onExit(code, signal); } catch { issue ??= "工蜂退出通知失败"; }
          if (issue) reject(new Error(issue));
          else if (code !== 0 || signal) reject(new Error(`Codex 非成功退出：code=${code}, signal=${signal}`));
          else resolveResult({ stdout: "", stderr: "", exitCode: 0 });
        });
        if (observation.signal?.aborted) abort();
        guarded("工蜂 prompt 输入失败", () => child.stdin!.end(options?.stdin ?? ""));
      });
    },
  };
}

function readPrompt(path: string): string {
  if (/(^|[._-])(env|credentials?|auth|tokens?|secrets?)([._-]|$)/i.test(basename(path))) throw new Error("prompt-file 不能指向环境或凭据文件");
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 4 * 1024 * 1024) throw new Error("prompt-file 必须是最多 4 MiB 的普通文件");
  const prompt = readFileSync(path, "utf8");
  if (!prompt.trim()) throw new Error("prompt-file 不能为空");
  return prompt;
}

function runPrompt(options: ChildOptions, task: string): string {
  return `你是 Sandcastle 启动的外部 Codex 工蜂，由原生 Codex 子代理管理。
票号：#${options.ticket}；父任务：${options.parentThreadId}；管理者：${options.managerAgentId}。
你的真实 session_id 由运行时事件登记；不要声称自己已注册为父任务的原生子代理。
模式：${options.mode}；固定基线：${options.expectedHead}。
仅完成下面的已授权任务。不能认领票、发布评论、push、merge、commit、切分支、修改 Git 配置或开启其他工蜂。
不得读取 .env、认证文件、钥匙串、任何凭据或密钥；认证由 Codex 自身沿用，禁止把凭据带入结果。
${options.mode === "review" ? "只读审阅，不写 checkout。" : "只在当前独立 codex/* checkout 修改任务范围文件并做定向验证；保留差异给管理者收包。"}
不重跑无关全量验证。若阻塞，说明实际缺项，不擅自换判据或批准后续动作。
最终仅返回符合输出 schema 的 JSON；summary、findings、validation 用简体中文，status 为 completed、blocked 或 failed。
\n管理者任务：\n${task}`;
}

export async function runCodexChild(
  options: ChildOptions,
  control: {
    signal?: AbortSignal;
    onState?: (record: ChildRecord) => void;
    env?: Record<string, string>;
    /** 靶向测试查找等待窗口；CLI 不暴露替代 session 来源。 */
    sessionFinder?: (id: string) => Promise<string | undefined>;
    writeLog?: ProcessObservation["writeLog"];
  } = {},
): Promise<ChildRecord> {
  validateOptions(options);
  const outputDir = join(realpathSync(dirname(options.outputDir)), basename(options.outputDir));
  const checkout = realpathSync(options.checkout);
  if (inside(outputDir, checkout)) throw new Error("output-dir 必须在工蜂 checkout 之外");
  // 不接管旧结果目录，不覆盖失败证据。
  mkdirSync(outputDir, { mode: 0o700 });
  const record: ChildRecord = {
    schema_version: 1, run_id: randomUUID(), ticket: options.ticket,
    parent_thread_id: options.parentThreadId, manager_agent_id: options.managerAgentId,
    manager_kind: "codex_native_subagent", worker_kind: "sandcastle_external_codex", native_subagent: false,
    status: "running", mode: options.mode, model: MODEL, effort: EFFORT,
    sandbox_backend: "no-sandbox", codex_sandbox: options.mode === "review" ? "read-only" : "workspace-write",
    approval_policy: "never", automatic_claim: false, automatic_push: false, automatic_merge: false,
    checkout_preserved: true, expected_head: options.expectedHead,
    checkout_before: null, checkout_after: null, started_at: new Date().toISOString(), ended_at: null,
    process_pid: null, process_exit: null, external_session_id: null, external_session_log: null,
    output_dir: outputDir, stdout_log: join(outputDir, "codex.stdout.jsonl"),
    stderr_log: join(outputDir, "codex.stderr.log"), events_log: join(outputDir, "events.jsonl"),
    prompt_sha256: null, result: null, failure_reason: null,
  };
  const persistState = () => {
    writeFileSync(join(outputDir, "state.json.tmp"), JSON.stringify(record, null, 2) + "\n", { mode: 0o600 });
    renameSync(join(outputDir, "state.json.tmp"), join(outputDir, "state.json"));
    appendFileSync(record.events_log, JSON.stringify(record) + "\n", { mode: 0o600 });
  };
  const publish = () => {
    persistState();
    control.onState?.({ ...record });
  };
  const fail = (error: unknown) => {
    record.status = "failed";
    const message = error instanceof Error ? error.message : "工蜂运行失败";
    record.failure_reason = record.failure_reason ? `${record.failure_reason}；${message}` : message;
  };
  const applyCancellation = () => {
    if (!control.signal?.aborted) return;
    const reason: unknown = control.signal.reason;
    record.status = reason instanceof SupervisionError ? "failed" : "cancelled";
    record.failure_reason ??= reason instanceof SupervisionError ? reason.message : "管理者取消工蜂";
  };
  const persistResult = () => {
    const file = join(outputDir, "result.json");
    writeFileSync(file + ".tmp", JSON.stringify(record, null, 2) + "\n", { mode: 0o600 });
    renameSync(file + ".tmp", file);
  };
  const lockPath = join(tmpdir(), `sandcastle-codex-child-${createHash("sha256").update(checkout).digest("hex")}.lock`);
  let locked = false;
  let handle: NoSandboxHandle | undefined;
  let transcript: CodexTranscript | undefined;
  let provider: AgentProvider | undefined;
  let sessionLookupAttempted = false;
  const findSession = async (id: string) => {
    sessionLookupAttempted = true;
    return control.sessionFinder ? control.sessionFinder(id) : (await provider!.sessionStorage!.findByIdOnHost(id)).path;
  };
  try {
    publish();
    if (control.signal?.aborted) throw new Error("启动前已取消");
    writeFileSync(lockPath, JSON.stringify({ pid: process.pid, run_id: record.run_id, output_dir: outputDir }), { flag: "wx", mode: 0o600 });
    locked = true;
    record.checkout_before = preflightCheckout(options);
    const prompt = runPrompt(options, readPrompt(options.promptFile));
    record.prompt_sha256 = createHash("sha256").update(prompt).digest("hex");
    writeFileSync(join(outputDir, "prompt.md"), prompt, { flag: "wx", mode: 0o600 });
    const schemaPath = join(outputDir, "output.schema.json");
    writeFileSync(schemaPath, JSON.stringify(OUTPUT_SCHEMA), { flag: "wx", mode: 0o600 });
    const env = workerEnvironment(control.env ?? process.env);
    provider = await boundedCodex(options.mode, schemaPath, env);
    transcript = new CodexTranscript(provider);
    const command = provider.buildPrintCommand({ prompt, dangerouslySkipPermissions: false });
    // 最后一次预检紧邻 spawn；不以最初快照代替当前状态。
    preflightCheckout(options);
    handle = await supervisedNoSandbox(checkout, {
      signal: control.signal, env, stdoutFile: record.stdout_log, stderrFile: record.stderr_log,
      writeLog: control.writeLog,
      idleTimeoutSeconds: options.idleTimeoutSeconds ?? 600, timeoutSeconds: options.timeoutSeconds ?? 3600,
      onSpawn(pid) { record.process_pid = pid; publish(); },
      onExit(code, signal) { record.process_exit = { code, signal }; },
    });
    await handle.exec(command.command, {
      stdin: command.stdin,
      onLine(line) {
        transcript!.accept(line);
        if (transcript!.sessionId && record.external_session_id !== transcript!.sessionId) {
          record.external_session_id = transcript!.sessionId;
          publish();
        }
      },
    });
    if (control.signal?.aborted) throw new Error("管理者取消工蜂");
    record.result = transcript.output();
    record.checkout_after = snapshotCheckout(checkout);
    if (record.checkout_after.head !== record.expected_head || record.checkout_after.branch !== record.checkout_before.branch) {
      throw new Error("工蜂运行期间 HEAD 或分支发生变化");
    }
    if (options.mode === "review" && record.checkout_after.status) throw new Error("只读工蜂运行后 checkout 出现修改");
    record.external_session_log = (await findSession(transcript.sessionId!)) ?? null;
    if (control.signal?.aborted) throw new Error("会话日志查找期间取消工蜂");
    if (!record.external_session_log) throw new Error("没有找到实际 Codex session 日志，不能交付完整身份映射");
    if (record.result.status !== "completed") throw new Error(`工蜂报告 ${record.result.status}`);
    record.status = "completed";
  } catch (error) {
    fail(error);
    applyCancellation();
    if (record.checkout_before && !record.checkout_after) {
      try { record.checkout_after = snapshotCheckout(checkout); } catch { /* 保留原始错误。 */ }
    }
  } finally {
    try { await handle?.close(); } catch (error) { fail(error); }
    // 失败/取消同样尽量绑定已经产生的真实 session；不读取凭据和 session 内容。
    if (record.external_session_id && !record.external_session_log && provider?.sessionStorage && !sessionLookupAttempted) {
      try { record.external_session_log = (await findSession(record.external_session_id)) ?? null; } catch { /* 不覆盖失败原因。 */ }
    }
    if (locked) { try { unlinkSync(lockPath); } catch (error) { fail(error); } }
    record.ended_at = new Date().toISOString();
    // 查找/close 的 await 之后、终态通知之前均复核取消。
    applyCancellation();
    // 监督输出失败也必须先释放锁/清理进程，再尝试交付 failed。
    // 持久化本身失败时最多补写一次失败状态，不循环重试或把未写入宣称为成功。
    const saveTerminal = () => {
      try { persistState(); persistResult(); } catch (error) {
        fail(new SupervisionError(`终态持久化失败：${error instanceof Error ? error.message : "未知错误"}`));
        applyCancellation();
        try { persistState(); } catch { /* receipt 仍有独立写入机会。 */ }
        try { persistResult(); } catch { /* 返回 failed；CLI 非零，不宣称 receipt 已存在。 */ }
      }
    };
    saveTerminal();
    const beforeNotification = JSON.stringify(record);
    // completed 只能在成功持久化后通知；通知同步失败则改写为 failed。
    try { control.onState?.({ ...record }); } catch (error) { fail(error); }
    applyCancellation();
    if (JSON.stringify(record) !== beforeNotification) saveTerminal();
  }
  return record;
}
