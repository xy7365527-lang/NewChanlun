// #1547：Sandcastle 工蜂经 Claude Code CLI 与 CLIProxyLocal 自动路由。
import { claudeCode } from "@ai-hero/sandcastle";
import type { AgentProvider } from "@ai-hero/sandcastle";

// 兼容既有调用方；实际实现不依赖 Sandcastle，wayfinder 直接引用 cliproxy-env.mts。
export {
  CLIPROXY_BASE_URL,
  CLIPROXY_CONFIG,
  cliProxyEnv,
  extractFirstApiKey,
  readCliProxyAccessKey,
} from "./cliproxy-env.mts";

/**
 * 工蜂 provider：Sandcastle 自带 claudeCode。
 *
 * 路由环境变量（cliProxyEnv）必须放在沙盒后端的 env 上，不能放在 claudeCode({ env })：
 * @ai-hero/sandcastle 0.12 的 createSandbox 流程建沙盒时 agentProviderEnv 固定为 {}，
 * sandbox.run 执行 agent 命令时也不带 provider env——放这里会被静默丢弃，CLI 退回本机
 * OAuth 登录态（#1547 冒烟实测：「OAuth session expired」）。
 */
export function claudeWorker(
  model: string,
  effort: "low" | "medium" | "high" | "xhigh" | "max" = "medium",
): AgentProvider {
  return claudeCode(model, {
    effort,
    // noSandbox 不会替 agent 加 --dangerously-skip-permissions；AFK 工蜂须显式放开工具权限。
    permissionMode: "bypassPermissions",
  });
}
