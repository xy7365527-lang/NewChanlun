import * as sandcastle from "@ai-hero/sandcastle";
import { primeAgent } from "../../prime-agent-provider.ts";
import { fail } from "./common.ts";

/**
 * 认证/模型集中配置（#1110 AC-4 第二处适配 → #1128 显式模型 registry）。
 *
 * 路由口径（#1112 裁定 A + #1128 追加路由）：
 * - 默认无 `agent:model:*` 标签 = Claude 原装路线：
 *   Sandcastle 内置 `claudeCode()`，GitHub Secret `CLAUDE_CODE_OAUTH_TOKEN`
 *   （Claude 官方 `claude setup-token` 生成）。Sonnet 实装，Opus 评审/探索/冲突调和。
 * - 显式 `agent:model:deepseek-v4-pro` = Prime Agent `deepseek` provider /
 *   `deepseek-v4-pro` 模型，只作用于实装角色 `implement` / `implement-pr`；
 *   评审（review）、探索（explore）、merge 冲突调和（update-branch）固定 Claude，
 *   与模型标签解耦，保持独立 session/model（#1128 AC-1）。
 * - label → provider → model → Secret 名称全部是本文件代码内 allowlist。
 *   Issue/PR 上的任意字符串只被当作标签值查表，绝不参与拼接 env key 或 secret 名。
 *   未知标签、多个冲突模型标签一律 fail-loud（ModelRegistryError → failure_reason.txt
 *   → workflow "Mark blocked on failure" 回写 agent:blocked）。
 * - 模型凭据只在构造对应 agent provider 时读取，缺失在任何模型调用前失败。
 *   DeepSeek 缺失 `DEEPSEEK_API_KEY` 不会回退 Claude，也不会先启动任何模型。
 *
 * remote-child 边界（#1114 裁定 B / #1128 AC-4）：
 * - 模型凭据只进入 provider 自身的 `env`（模型子进程专用）。
 * - remote-child 的 invitation/lease 是 capability identity（#1142），走独立
 *   `REMOTE_CHILD_IDENTITY_ENV_ALLOWLIST`；二者永不合并，模型凭据不转发给
 *   remote-child admission/lease/broker。
 * - 本文件不读取、不提交本机 Prime/Codex auth；本机 key 只允许经 workflow 显式
 *   Secret 注入，禁止自动复制现有 `.sandcastle/.env` 或本机凭据目录。
 */

/** 固定角色 → Claude 默认模型（#1110 AC-4；#1128 保持默认路线不变）。 */
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

export const MODEL_LABEL_PREFIX = "agent:model:";

/** 模型凭据 Secret/env 名 allowlist（代码内唯一来源；标签字符串永不成为 key）。 */
export const MODEL_SECRET_NAMES = [
  "CLAUDE_CODE_OAUTH_TOKEN",
  "DEEPSEEK_API_KEY",
] as const;
export type ModelSecretName = (typeof MODEL_SECRET_NAMES)[number];

export const CLAUDE_CODE_SECRET: ModelSecretName = "CLAUDE_CODE_OAUTH_TOKEN";
export const DEEPSEEK_SECRET: ModelSecretName = "DEEPSEEK_API_KEY";

export interface ModelRegistryEntry {
  /** GitHub 标签全名（含前缀）。 */
  readonly label: string;
  /** Sandcastle AgentProvider 类型。 */
  readonly provider: "prime-agent";
  /** prime-agent 的 --provider 参数（固定值，不与 Secret 名/env key 拼接）。 */
  readonly primeProvider: string;
  /** prime-agent 的 --model 参数。 */
  readonly model: string;
  /** 该模型唯一允许读取的 GitHub Secret/env 名（allowlist 内）。 */
  readonly secretName: ModelSecretName;
  /** 模型标签生效的角色；评审/探索/冲突调和永远不消费模型标签。 */
  readonly appliesTo: readonly AgentRole[];
}

/**
 * 显式模型 registry。新增 hosted provider 必须：
 * 1) 在这里加一条 allowlist 条目；
 * 2) 在 workflow 里显式注入该条目声明的固定 Secret（不得用标签名派生 Secret）；
 * 3) 在 selectedAgent() 里加对应的 provider 构造分支。
 */
export const MODEL_REGISTRY = {
  "agent:model:deepseek-v4-pro": {
    label: "agent:model:deepseek-v4-pro",
    provider: "prime-agent",
    primeProvider: "deepseek",
    model: "deepseek-v4-pro",
    secretName: DEEPSEEK_SECRET,
    appliesTo: ["implement", "implement-pr"],
  },
} as const satisfies Record<string, ModelRegistryEntry>;

/**
 * remote-child invitation/lease 的独立 identity env allowlist（#1142/#1128 AC-4）。
 * 模型凭据名（MODEL_SECRET_NAMES）永不进入此列表；此列表只透传 capability identity，
 * 不读取、不转发任何模型凭据。
 */
export const REMOTE_CHILD_IDENTITY_ENV_ALLOWLIST = [
  "PRIME_REMOTE_CHILD_INVITATION",
  "PRIME_REMOTE_CHILD_LEASE",
] as const;

export class ModelRegistryError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ModelRegistryError";
  }
}

export interface ModelSelection {
  readonly label: string | null;
  readonly entry: ModelRegistryEntry | null;
}

const DEFAULT_CLAUDE_SELECTION: ModelSelection = { label: null, entry: null };

const knownModelLabels = (): string => Object.keys(MODEL_REGISTRY).join(", ");

/**
 * 纯函数：从事件标签解析模型选择。不读任何 Secret/env；未知/冲突抛
 * ModelRegistryError，由 workflow 顶层 catch → fail() 写 failure_reason.txt。
 */
export const resolveModelSelection = (
  labels: readonly string[],
): ModelSelection => {
  const modelLabels = labels
    .map((label) => label.trim())
    .filter((label) => label.startsWith(MODEL_LABEL_PREFIX));

  if (modelLabels.length === 0) {
    return DEFAULT_CLAUDE_SELECTION;
  }

  const unique = [...new Set(modelLabels)];
  if (unique.length > 1) {
    throw new ModelRegistryError(
      `Conflicting model labels: ${unique.join(", ")}. ` +
        `At most one ${MODEL_LABEL_PREFIX}* label is allowed. ` +
        `Known model labels: ${knownModelLabels()}.`,
    );
  }

  const label = unique[0] as string;
  const entry = MODEL_REGISTRY[
    label as keyof typeof MODEL_REGISTRY
  ] as ModelRegistryEntry | undefined;

  if (!entry) {
    throw new ModelRegistryError(
      `Unknown model label ${label}. ` +
        `Known model labels: ${knownModelLabels()}. ` +
        "Remove the label to use the default Claude route.",
    );
  }

  return { label, entry };
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** 从 GitHub `labeled` 事件（issue 或 pull_request）读取标签名数组。 */
export const modelLabelsFromEvent = (event: unknown): string[] => {
  if (!isRecord(event)) {
    throw new ModelRegistryError(
      "EVENT_PAYLOAD must be a JSON object (toJSON(github.event)).",
    );
  }
  const container = isRecord(event.issue) ? event.issue : event.pull_request;
  if (!isRecord(container)) {
    throw new ModelRegistryError(
      "EVENT_PAYLOAD must be a GitHub labeled event containing issue or pull_request labels.",
    );
  }
  const labels = container.labels;
  if (labels === undefined) return [];
  if (!Array.isArray(labels)) {
    throw new ModelRegistryError(
      "EVENT_PAYLOAD labels must be an array of { name: string } objects.",
    );
  }
  return labels.map((label) => {
    if (!isRecord(label) || typeof label.name !== "string" || label.name === "") {
      throw new ModelRegistryError(
        "EVENT_PAYLOAD label entries must have a non-empty string name.",
      );
    }
    return label.name;
  });
};

export const parseEventPayload = (raw: string | undefined): unknown => {
  if (!raw) {
    throw new ModelRegistryError(
      "EVENT_PAYLOAD is not set. Workflows must pass toJSON(github.event) to the agent step.",
    );
  }
  try {
    return JSON.parse(raw);
  } catch {
    throw new ModelRegistryError("EVENT_PAYLOAD is not valid JSON.");
  }
};

/** 从进程环境读取模型标签（缺少 EVENT_PAYLOAD 时按无标签处理，便于本地直跑）。 */
export const readModelLabelsFromEnv = (
  env: NodeJS.ProcessEnv = process.env,
): string[] => {
  if (!env.EVENT_PAYLOAD) return [];
  return modelLabelsFromEvent(parseEventPayload(env.EVENT_PAYLOAD));
};

/**
 * 只校验标签合法性（未知/冲突 fail-loud），不读模型凭据。
 * 供固定 Claude 角色（review/explore/update-branch）在工作流入口调用：
 * 有合法 DeepSeek 标签时这些角色仍走 Claude，且不会因缺 DeepSeek key 失败。
 */
export const assertValidModelLabels = (labels: readonly string[]): void => {
  resolveModelSelection(labels);
};

const SECRET_HELP: Readonly<Record<ModelSecretName, string>> = {
  CLAUDE_CODE_OAUTH_TOKEN:
    "Generate it with `claude setup-token` and store it as a GitHub Secret (CLAUDE_CODE_OAUTH_TOKEN), then re-run.",
  DEEPSEEK_API_KEY:
    "Store a DeepSeek API key as a GitHub Secret named exactly DEEPSEEK_API_KEY, then re-run.",
};

/**
 * 读取模型凭据。secretName 只能是本文件 allowlist 的 ModelSecretName；
 * 这里显式分支访问固定 env 名，调用侧无法把标签/票面字符串变成 env key。
 */
export const modelCredential = (
  secretName: ModelSecretName,
  env: NodeJS.ProcessEnv = process.env,
): string => {
  const value =
    secretName === DEEPSEEK_SECRET
      ? env.DEEPSEEK_API_KEY?.trim()
      : env.CLAUDE_CODE_OAUTH_TOKEN?.trim();
  if (!value) {
    throw new ModelRegistryError(
      `${secretName} is not set or is empty. ${SECRET_HELP[secretName]}`,
    );
  }
  return value;
};

/**
 * remote-child identity env：只透传 allowlist 内的 invitation/lease；
 * 模型凭据名不在 allowlist 内，因此结构上不可能被夹带（测试锁定）。
 */
export const remoteChildIdentityEnv = (
  env: NodeJS.ProcessEnv = process.env,
): Record<string, string> => {
  const identity: Record<string, string> = {};
  for (const name of REMOTE_CHILD_IDENTITY_ENV_ALLOWLIST) {
    const value = env[name]?.trim();
    if (value) identity[name] = value;
  }
  return identity;
};

export type SelectableRole = "implement" | "implement-pr";

/** 固定 Claude 角色：不消费模型标签，直接使用 claudeCode + CLAUDE_CODE_OAUTH_TOKEN。 */
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

/**
 * 按事件标签构建实装 agent（implement / implement-pr）。
 * 默认 Claude Sonnet；`agent:model:deepseek-v4-pro` → Prime Agent deepseek。
 * 未知/冲突标签与 DeepSeek 缺 key 都在返回 provider 前抛错，先于任何模型调用。
 */
export const selectedAgent = (
  role: SelectableRole,
  labels: readonly string[] = readModelLabelsFromEnv(),
  env: NodeJS.ProcessEnv = process.env,
) => {
  const selection = resolveModelSelection(labels);

  if (selection.entry) {
    const entry = selection.entry;
    if (!entry.appliesTo.includes(role)) {
      throw new ModelRegistryError(
        `${entry.label} does not apply to role ${role} (allowed: ${entry.appliesTo.join(", ")}).`,
      );
    }

    const token = modelCredential(entry.secretName, env);
    if (
      entry.provider === "prime-agent" &&
      entry.secretName === DEEPSEEK_SECRET
    ) {
      return primeAgent(entry.model, {
        provider: entry.primeProvider,
        // 只注入 deepseek 专属凭据；Claude token / remote-child identity 一律不进 provider env。
        env: { DEEPSEEK_API_KEY: token },
      });
    }
    throw new ModelRegistryError(
      `Registry entry ${entry.label} has no provider constructor branch.`,
    );
  }

  const token = modelCredential(CLAUDE_CODE_SECRET, env);
  return sandcastle.claudeCode(AGENT_MODELS[role], {
    env: { CLAUDE_CODE_OAUTH_TOKEN: token },
  });
};
