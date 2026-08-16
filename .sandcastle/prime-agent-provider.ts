// prime-agent 的 sandcastle 自定义 AgentProvider（#996）。
// 无头模式：`prime-agent -p --mode json` 输出 NDJSON 事件流；prompt 走 stdin，
// 避开 Linux 128 KB argv 上限。事件映射：session→session_id、
// message_update(text_delta)→text、tool_execution_start→tool_call、
// message_end→usage、agent_end→result。
import type {
  AgentCommandOptions,
  AgentProvider,
  PrintCommand,
} from "@ai-hero/sandcastle";

// ParsedStreamEvent 未从包导出，从接口反推。
type StreamEvent = ReturnType<AgentProvider["parseStreamLine"]>[number];

export interface PrimeAgentOptions {
  /** prime-agent 的 provider 名，默认 kimi-coding。 */
  readonly provider?: string;
  /** --thinking 档位；不传用 prime-agent 默认。 */
  readonly thinking?: "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max";
  /** 注入沙盒的环境变量（与 .sandcastle/.env 合并后生效）。 */
  readonly env?: Record<string, string>;
}

export function primeAgent(model: string, options?: PrimeAgentOptions): AgentProvider {
  const provider = options?.provider ?? "kimi-coding";
  const thinkingFlag = options?.thinking ? ` --thinking ${options.thinking}` : "";
  let accumulated = "";
  return {
    name: "prime-agent",
    env: { ...options?.env },
    captureSessions: false,
    buildPrintCommand({ prompt }: AgentCommandOptions): PrintCommand {
      return {
        command: `prime-agent -p --mode json --provider ${provider} --model ${model}${thinkingFlag}`,
        stdin: prompt,
      };
    },
    parseStreamLine(line: string): StreamEvent[] {
      if (!line.startsWith("{")) return [];
      let obj: any;
      try { obj = JSON.parse(line); } catch { return []; }
      if (obj.type === "session" && typeof obj.id === "string") {
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