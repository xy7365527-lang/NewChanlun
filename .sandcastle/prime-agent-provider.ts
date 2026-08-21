// prime-agent 的 sandcastle 自定义 AgentProvider（#996）。
// 无头模式：`prime-agent -p --mode json` 输出 NDJSON 事件流；prompt 走 stdin，
// 避开 Linux 128 KB argv 上限。事件映射：session→session_id、
// message_update(text_delta)→text、tool_execution_start→tool_call、
// message_end→usage、agent_end→result。
//
// #1066：接入会话捕获与按 session id 恢复（sandcastle 的 AgentSessionStorage 契约）。
// prime-agent 的会话文件落在 <sessionsDir>/<文件id>.jsonl，而流首行 session 事件的
// id 是文件内 header 的 id——daemon 流程下二者可不同（实测 0.7.2）。故捕获/恢复一律
// 按 header.id 定位，不按文件名猜：
//   - 沙盒会话目录固定 /home/agent/.prime/agent/sessions（HOME=/home/agent）；
//   - 捕获：优先取恢复路径 <dir>/<sessionId>.jsonl（恢复轮），否则扫描目录找
//     header.id 命中者（首轮 daemon 命名的文件）；
//   - 宿主存储：~/.prime/agent/sessions/<sessionId>.jsonl（同 id 命名），沙盒销毁后
//     宿主仍可 `prime-agent -r <id> -p` 直接继续；
//   - 恢复：把宿主 JSONL 的 cwd 从 hostCwd 改写回 sandboxCwd 写回沙盒，续跑命令用
//     `-r <绝对路径>`（同 id 双文件场景下避免 catalog 前缀匹配歧义）。
import type {
  AgentCommandOptions,
  AgentProvider,
  BindMountSandboxHandle,
  PrintCommand,
} from "@ai-hero/sandcastle";
import { dirname, join, posix } from "node:path";
import { access, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";

type StreamEvent = ReturnType<AgentProvider["parseStreamLine"]>[number];
type PrimeSessionStorage = NonNullable<AgentProvider["sessionStorage"]>;

export interface PrimeAgentOptions {
  /** prime-agent 的 provider 名，默认 kimi-coding。 */
  readonly provider?: string;
  /** --thinking 档位；不传用 prime-agent 默认。 */
  readonly thinking?: "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max";
  /** 注入沙盒的环境变量（与 .sandcastle/.env 合并后生效）。 */
  readonly env?: Record<string, string>;
  /** 覆盖会话目录（测试/非常规安装用）。默认：宿主 ~/.prime/agent/sessions、沙盒 /home/agent/.prime/agent/sessions。 */
  readonly sessionStorage?: {
    readonly hostSessionsDir?: string;
    readonly sandboxSessionsDir?: string;
  };
}

const DEFAULT_HOST_SESSIONS_DIR = join(homedir(), ".prime", "agent", "sessions");
const DEFAULT_SANDBOX_SESSIONS_DIR = "/home/agent/.prime/agent/sessions";

function shellQuote(s: string): string {
  return "'" + s.replace(/'/g, "'\\''") + "'";
}

/** 解析 Prime 会话 JSONL 首行 header 的最小 id/cwd（流首行与文件首行同构）。 */
export function parsePrimeSessionHeader(line: string): { id: string; cwd: string } | undefined {
  if (!line.startsWith("{")) return undefined;
  try {
    const obj = JSON.parse(line);
    if (obj?.type === "session" && typeof obj.id === "string") {
      return { id: obj.id, cwd: typeof obj.cwd === "string" ? obj.cwd : "" };
    }
  } catch {
    // 非 JSON 行忽略
  }
  return undefined;
}

/**
 * 纯函数：改写 Prime 会话 JSONL 的 cwd（仅首行 session header 携带 cwd；消息正文里的
 * 路径串不动）。与 sandcastle 的 transferPiSession 同语义。
 */
export function transferPrimeSession(jsonl: string, fromCwd: string, toCwd: string): string {
  if (jsonl === "" || fromCwd === toCwd) return jsonl;
  return jsonl.split("\n").map((line) => {
    if (line === "") return line;
    try {
      const entry = JSON.parse(line);
      if (entry?.type === "session" && entry.cwd === fromCwd) {
        entry.cwd = toCwd;
        return JSON.stringify(entry);
      }
      return line;
    } catch {
      return line;
    }
  }).join("\n");
}

async function fileExists(path: string): Promise<boolean> {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

/** 读沙盒文件（经 copyFileOut 落临时宿主文件，避免 exec stdout 截断大文件）。 */
async function readSandboxFile(handle: BindMountSandboxHandle, sandboxPath: string): Promise<string> {
  const tmp = join(tmpdir(), `prime-cap-${Date.now()}-${Math.random().toString(36).slice(2)}.jsonl`);
  await handle.copyFileOut(sandboxPath, tmp);
  try {
    return await readFile(tmp, "utf8");
  } finally {
    await rm(tmp, { force: true }).catch(() => {});
  }
}

/** 写沙盒文件（先落临时宿主文件，再 copyFileIn，避开 exec 内联大内容）。 */
async function writeSandboxFile(handle: BindMountSandboxHandle, sandboxPath: string, content: string): Promise<void> {
  const tmp = join(tmpdir(), `prime-res-${Date.now()}-${Math.random().toString(36).slice(2)}.jsonl`);
  await writeFile(tmp, content);
  try {
    await handle.exec(`mkdir -p ${shellQuote(posix.dirname(sandboxPath))}`);
    await handle.copyFileIn(tmp, sandboxPath);
  } finally {
    await rm(tmp, { force: true }).catch(() => {});
  }
}

async function readSandboxHeader(
  handle: BindMountSandboxHandle,
  sandboxPath: string,
): Promise<{ id: string; cwd: string } | undefined> {
  const res = await handle.exec(`head -n 1 ${shellQuote(sandboxPath)}`);
  if (res.exitCode !== 0) return undefined;
  return parsePrimeSessionHeader(res.stdout.split("\n")[0] ?? "");
}

/** 在沙盒会话目录中按 header.id 定位会话文件。优先恢复路径（恢复轮），否则扫描（首轮）。 */
async function findSandboxSessionFile(
  handle: BindMountSandboxHandle,
  sandboxSessionsDir: string,
  sessionId: string,
): Promise<string | undefined> {
  const target = posix.join(sandboxSessionsDir, `${sessionId}.jsonl`);
  const targetHeader = await readSandboxHeader(handle, target);
  if (targetHeader?.id === sessionId) return target;
  const listed = await handle.exec(
    `find ${shellQuote(sandboxSessionsDir)} -maxdepth 1 -type f -name '*.jsonl' -print 2>/dev/null`,
  );
  if (listed.exitCode !== 0) return undefined;
  for (const file of listed.stdout.trim().split("\n")) {
    if (!file) continue;
    const header = await readSandboxHeader(handle, file);
    if (header?.id === sessionId) return file;
  }
  return undefined;
}

export function primeAgent(model: string, options?: PrimeAgentOptions): AgentProvider {
  const provider = options?.provider ?? "kimi-coding";
  const thinkingFlag = options?.thinking ? ` --thinking ${options.thinking}` : "";
  const hostSessionsDir = options?.sessionStorage?.hostSessionsDir ?? DEFAULT_HOST_SESSIONS_DIR;
  const sandboxSessionsDir = options?.sessionStorage?.sandboxSessionsDir ?? DEFAULT_SANDBOX_SESSIONS_DIR;
  let accumulated = "";

  const sessionStorage: PrimeSessionStorage = {
    hostSessionFilePath: (_cwd, sessionId) => join(hostSessionsDir, `${sessionId}.jsonl`),
    existsOnHost: async (_cwd, sessionId) => fileExists(join(hostSessionsDir, `${sessionId}.jsonl`)),
    readHostSession: async (_cwd, sessionId) => {
      const path = join(hostSessionsDir, `${sessionId}.jsonl`);
      if (!(await fileExists(path))) return undefined;
      return readFile(path, "utf8");
    },
    findByIdOnHost: async (sessionId) => {
      const path = join(hostSessionsDir, `${sessionId}.jsonl`);
      return { path: (await fileExists(path)) ? path : undefined, searchedRoot: hostSessionsDir };
    },
    captureToHost: async ({ hostCwd, sandboxCwd, sessionId, handle }) => {
      const sandboxFile = await findSandboxSessionFile(handle, sandboxSessionsDir, sessionId);
      if (!sandboxFile) {
        throw new Error(`session ${sessionId} not found in ${sandboxSessionsDir}`);
      }
      const jsonl = await readSandboxFile(handle, sandboxFile);
      const rewritten = transferPrimeSession(jsonl, sandboxCwd, hostCwd);
      const target = join(hostSessionsDir, `${sessionId}.jsonl`);
      await mkdir(dirname(target), { recursive: true });
      await writeFile(target, rewritten);
    },
    resumeIntoSandbox: async ({ hostCwd, sandboxCwd, sessionId, handle }) => {
      const hostPath = join(hostSessionsDir, `${sessionId}.jsonl`);
      const jsonl = await readFile(hostPath, "utf8");
      const rewritten = transferPrimeSession(jsonl, hostCwd, sandboxCwd);
      await writeSandboxFile(handle, posix.join(sandboxSessionsDir, `${sessionId}.jsonl`), rewritten);
    },
  };

  return {
    name: "prime-agent",
    env: { ...options?.env },
    captureSessions: true,
    sessionStorage,
    buildPrintCommand({ prompt, resumeSession, forkSession }: AgentCommandOptions): PrintCommand {
      const sessionFlag = resumeSession
        ? ` ${forkSession ? "--fork" : "-r"} ${shellQuote(posix.join(sandboxSessionsDir, `${resumeSession}.jsonl`))}`
        : "";
      return {
        command: `prime-agent -p --mode json --provider ${provider} --model ${model}${thinkingFlag}${sessionFlag}`,
        stdin: prompt,
      };
    },
    parseStreamLine(line: string): StreamEvent[] {
      if (!line.startsWith("{")) return [];
      let obj: any;
      try { obj = JSON.parse(line); } catch { return []; }
      if (obj.type === "session" && typeof obj.id === "string") {
        // 新会话（含续跑轮）开始：清掉上一轮可能因未收尾（如被 kill 无 agent_end）残留的文本。
        accumulated = "";
        return [{ type: "session_id", sessionId: obj.id }];
      }
      const evt = obj.assistantMessageEvent;
      if (obj.type === "message_update" && evt?.type === "text_delta" && typeof evt.delta === "string") {
        accumulated += evt.delta;
        return [{ type: "text", text: evt.delta }];
      }
      if (obj.type === "tool_execution_start" && typeof obj.toolName === "string") {
        const args = typeof obj.args === "string" ? obj.args : JSON.stringify(obj.args ?? {});
        return [{ type: "tool_call", name: obj.toolName, args }];
      }
      if (obj.type === "message_end" && obj.message?.usage) {
        const u = obj.message.usage;
        if (typeof u.input === "number" && typeof u.output === "number") {
          return [{
            type: "usage",
            usage: {
              inputTokens: u.input - (u.cacheRead ?? 0),
              cacheCreationInputTokens: u.cacheWrite ?? 0,
              cacheReadInputTokens: u.cacheRead ?? 0,
              outputTokens: u.output,
            },
          }];
        }
      }
      if (obj.type === "agent_end") {
        const result = accumulated;
        accumulated = "";
        return result ? [{ type: "result", result }] : [];
      }
      return [];
    },
  };
}
