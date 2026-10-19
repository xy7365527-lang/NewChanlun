// #1547：Sandcastle 工蜂的 Claude 路由——Claude Code CLI 经本机 CLIProxyLocal 统一路由（自动路由，无前缀模型名）。
// main.mts 与冒烟脚本共用这一份接线，保证冒烟验的就是生产配置。
import { claudeCode } from "@ai-hero/sandcastle";
import type { AgentProvider } from "@ai-hero/sandcastle";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export const CLIPROXY_BASE_URL = "http://127.0.0.1:8318";
export const CLIPROXY_CONFIG = join(homedir(), "Library/Application Support/CLIProxyLocal/config.yaml");

/**
 * 取 CLIProxyLocal 配置顶层 `access.api-keys` 列表首项。
 * 值只在内存里交给 agent 进程环境：不打印、不写日志、不落文件；报错信息里也不带值。
 */
export function readCliProxyAccessKey(configPath: string = CLIPROXY_CONFIG): string {
  let text: string;
  try {
    text = readFileSync(configPath, "utf8");
  } catch (e) {
    throw new Error(`[FAIL-loud #1547] 读不到 CLIProxyLocal 配置 ${configPath}：${String(e).slice(0, 200)}`);
  }
  let inAccess = false;
  let inKeys = false;
  for (const raw of text.split("\n")) {
    const line = raw.replace(/\s+#.*$/, "");
    if (!line.trim()) continue;
    const indent = line.length - line.trimStart().length;
    if (indent === 0) {
      inAccess = line.trim() === "access:";
      inKeys = false;
      continue;
    }
    if (!inAccess) continue;
    const t = line.trim();
    if (/^api-keys:\s*$/.test(t)) {
      inKeys = true;
      continue;
    }
    if (inKeys && t.startsWith("- ")) {
      const v = t.slice(2).trim().replace(/^["']|["']$/g, "");
      if (v) return v;
    } else if (inKeys) {
      inKeys = false;
    }
  }
  throw new Error(`[FAIL-loud #1547] ${configPath} 里没有 access.api-keys 首项`);
}

export function cliProxyEnv(): Record<string, string> {
  return {
    ANTHROPIC_BASE_URL: CLIPROXY_BASE_URL,
    ANTHROPIC_AUTH_TOKEN: readCliProxyAccessKey(),
  };
}

/** 工蜂 provider：Sandcastle 自带 claudeCode + 统一路由环境变量。 */
export function claudeWorker(
  model: string,
  effort: "low" | "medium" | "high" | "xhigh" | "max" = "medium",
): AgentProvider {
  return claudeCode(model, {
    effort,
    env: cliProxyEnv(),
    // noSandbox 不会替 agent 加 --dangerously-skip-permissions；AFK 工蜂须显式放开工具权限。
    permissionMode: "bypassPermissions",
  });
}
