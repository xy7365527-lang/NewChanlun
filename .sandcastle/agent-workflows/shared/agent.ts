import * as sandcastle from "@ai-hero/sandcastle";
import { fail } from "./common.ts";

/**
 * 认证/模型常量集中配置（#1110 AC-4 第二处适配，唯一的 agent auth/model 入口）。
 *
 * 口径：
 * - Prime 是模型无关控制面，不进入 workflow。GitHub-hosted runner 上使用
 *   Sandcastle 内置 `claudeCode()`，认证走 GitHub Secret `CLAUDE_CODE_OAUTH_TOKEN`
 *   （由 Claude 官方 `claude setup-token` 生成）。
 * - 本文件不读取、不提交本机 Prime/Codex auth（本机凭据目录、订阅后端 API key、
 *   推理端点一律不出现、不读取）。
 * - 缺 secret 时 fail-loud：写 failure_reason.txt 并以非零退出，workflow 的
 *   "Mark blocked on failure" 步据此回写 `agent:blocked` 标签与失败评论。
 * - 模型常量按角色集中于此：实装用 Sonnet，评审/探索/冲突调和用 Opus。
 */
export const AGENT_MODELS = {
  /** 实装（issue implement / PR implement）用 Sonnet。 */
  implement: "claude-sonnet-4-6",
  /** 独立评审用 Opus（与实装隔离的模型档）。 */
  review: "claude-opus-4-8",
  /** 只读探索/分诊用 Opus。 */
  explore: "claude-opus-4-8",
  /** PR 修正入口（评审意见落地）用 Sonnet。 */
  "implement-pr": "claude-sonnet-4-6",
  /** merge 冲突调和用 Opus。 */
  "update-branch": "claude-opus-4-8",
} as const;

export type AgentRole = keyof typeof AGENT_MODELS;

export const claudeAgent = (role: AgentRole) => {
  // `|| fail(...)`：token 缺失或为空时 fail-loud（写 failure_reason.txt + 非零退出）。
  // tsgo（TS 7 native）不把 never 返回的函数调用当控制流终止点，`if (!x) fail()` 无法
  // 收窄 x；`|| fail(...)` 让表达式类型直接为 string，绕开该收窄差异。
  const token =
    process.env.CLAUDE_CODE_OAUTH_TOKEN?.trim() ||
    fail(
      "CLAUDE_CODE_OAUTH_TOKEN is not set. Generate it with `claude setup-token` and store it as a GitHub Secret (CLAUDE_CODE_OAUTH_TOKEN), then re-run.",
    );
  return sandcastle.claudeCode(AGENT_MODELS[role], {
    env: { CLAUDE_CODE_OAUTH_TOKEN: token },
  });
};
