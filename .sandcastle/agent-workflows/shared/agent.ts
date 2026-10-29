import {
  DEFAULT_HOST_SESSIONS_DIR,
  primeAgent,
} from "../../prime-agent-provider.ts";

/**
 * hosted 模型 registry（#1617 现行合同）：
 * - 无 `agent:model:*` 标签的 Claude 默认已退役；ubuntu-latest 没有获批的
 *   Astra 凭据或可达 CLIProxy 端点，不能把 Claude OAuth 错用于 gpt-6.1-sol。
 * - `agent:model:deepseek-v4-pro` 只作为 implement / implement-pr / review 的
 *   显式 legacy opt-in；缺 key、未知标签或冲突标签均在模型调用前失败。
 * - 本机受管默认不经过此 hosted registry，统一由 main-model.mts 指定
 *   `gpt-6.1-sol`，经 CLIProxyLocal 自动路由。
 * - label → provider → model → Secret 仍全部来自代码 allowlist；旧默认不得
 *   成为任何失败的 fallback。
 *
 * remote-child 边界（#1114 裁定 B / #1128 AC-4）：
 * - 模型凭据只进入 provider 自身的 `env`（模型子进程专用）；provider env 由
 *   `modelProviderEnv()` 统一构造：只含该模型唯一凭据 + 经
 *   `REMOTE_CHILD_IDENTITY_ENV_ALLOWLIST` 过滤后的 remote-child identity。
 * - remote-child 的 invitation/lease 是 capability identity（#1142），走独立
 *   allowlist，且结构上不可能夹带任何 `MODEL_SECRET_NAMES`；另一模型的凭据
 *   不会由本文件进入 provider env，模型凭据也不转发给 remote-child
 *   admission/lease/broker。
 * - 本文件不读取、不提交本机 Prime/Codex auth；本机 key 只允许经 workflow 显式
 *   Secret 注入，禁止自动复制现有 `.sandcastle/.env` 或本机凭据目录。
 */

/** 角色名仅用于 legacy registry 的适用范围，不再映射 hosted 默认模型。 */
export type AgentRole = "implement" | "implement-pr" | "review" | "explore" | "update-branch";

export const MODEL_LABEL_PREFIX = "agent:model:";

/** 模型凭据 Secret/env 名 allowlist（代码内唯一来源；标签字符串永不成为 key）。 */
export const MODEL_SECRET_NAMES = [
  "DEEPSEEK_API_KEY",
] as const;
export type ModelSecretName = (typeof MODEL_SECRET_NAMES)[number];

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
  /** legacy 标签生效的角色；hosted 无标签默认已退役。 */
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
    appliesTo: ["implement", "implement-pr", "review", "update-branch"],
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

const NO_MODEL_SELECTION: ModelSelection = { label: null, entry: null };

const knownModelLabels = (): string => Object.keys(MODEL_REGISTRY).join(", ");

/**
 * 纯函数：从事件标签解析模型选择。不读任何 Secret/env；未知/冲突抛
 * ModelRegistryError，由 workflow 顶层 catch → fail() 写 failure_reason.txt。
 */
export const resolveModelSelection = (
  labels: readonly string[],
): ModelSelection => {
  // 前缀识别不区分大小写（与 GitHub label 检索语义对齐），但 registry 查表保持
  // 精确大小写：任何大小写变体都会被识别为 model label，随后 fail-loud，
  // 不会静默进入已退役的 hosted 默认路径。
  const modelLabels = labels
    .map((label) => label.trim())
    .filter((label) =>
      label.toLowerCase().startsWith(MODEL_LABEL_PREFIX.toLowerCase()),
    );

  if (modelLabels.length === 0) {
    return NO_MODEL_SELECTION;
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
        "The hosted default route is retired; use the local Astra Sandcastle queue or an explicit supported legacy label.",
    );
  }

  return { label, entry };
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const parseLabelName = (label: unknown): string => {
  if (typeof label === "string") {
    const name = label.trim();
    if (name !== "") return name;
  } else if (isRecord(label) && typeof label.name === "string") {
    const name = label.name.trim();
    if (name !== "") return name;
  }
  throw new ModelRegistryError(
    "EVENT_PAYLOAD label entries must have a non-empty string name or be { name: string } objects.",
  );
};

const labelsFromContainer = (container: Record<string, unknown>): string[] => {
  const labels = container.labels;
  if (labels === undefined) return [];
  if (!Array.isArray(labels)) {
    throw new ModelRegistryError("EVENT_PAYLOAD labels must be an array.");
  }
  return labels.map(parseLabelName);
};

/**
 * 从最小 EVENT_PAYLOAD 或 GitHub `labeled` 事件读取标签名数组。
 * P2 接线：workflow 只传 `toJSON(github.event.*.labels.*.name)`（标签名字符串数组）；
 * 完整事件与 label 对象数组仍兼容，供本地 fixtures/直接入口对拍使用。
 */
export const modelLabelsFromEvent = (payload: unknown): string[] => {
  if (Array.isArray(payload)) {
    return payload.map(parseLabelName);
  }
  if (!isRecord(payload)) {
    throw new ModelRegistryError(
      "EVENT_PAYLOAD must be a JSON array of label names " +
        "(toJSON(github.event.issue.labels.*.name) / pull_request counterpart) " +
        "or a GitHub labeled event.",
    );
  }
  const container = isRecord(payload.issue) ? payload.issue : payload.pull_request;
  if (!isRecord(container)) {
    throw new ModelRegistryError(
      "EVENT_PAYLOAD must be a GitHub labeled event containing issue or pull_request labels.",
    );
  }
  return labelsFromContainer(container);
};

export const parseEventPayload = (raw: string | undefined): unknown => {
  if (!raw) {
    throw new ModelRegistryError(
      "EVENT_PAYLOAD is not set. Workflows must pass only labels to the agent step " +
        "(toJSON(github.event.issue.labels.*.name) / pull_request counterpart).",
    );
  }
  try {
    return JSON.parse(raw);
  } catch {
    throw new ModelRegistryError("EVENT_PAYLOAD is not valid JSON.");
  }
};

/** 从进程环境读取模型标签（最小 labels EVENT_PAYLOAD）。
 * 缺少 EVENT_PAYLOAD 必须 fail-loud；仅显式 SANDCASTLE_LOCAL=1 的本地直跑
 * 允许按无标签处理；随后由 selectedAgent/claudeAgent 对退役默认明确报错。
 */
export const readModelLabelsFromEnv = (
  env: NodeJS.ProcessEnv = process.env,
): string[] => {
  if (!env.EVENT_PAYLOAD && env.SANDCASTLE_LOCAL === "1") return [];
  return modelLabelsFromEvent(parseEventPayload(env.EVENT_PAYLOAD));
};

/**
 * 只校验标签合法性（未知/冲突 fail-loud），不读模型凭据。
 * 供 explore/update-branch 在工作流入口调用；合法 legacy 标签不读取其凭据，
 * 但这两个入口的无标签默认随后仍会按退役合同失败。
 */
export const assertValidModelLabels = (labels: readonly string[]): void => {
  resolveModelSelection(labels);
};

const SECRET_HELP: Readonly<Record<ModelSecretName, string>> = {
  DEEPSEEK_API_KEY:
    "Store a DeepSeek API key as a GitHub Secret named exactly DEEPSEEK_API_KEY, then re-run.",
};

/** 读取显式 legacy 模型凭据；不允许从票面字符串派生 env key。 */
export const modelCredential = (
  secretName: ModelSecretName,
  env: NodeJS.ProcessEnv = process.env,
): string => {
  const value = secretName === DEEPSEEK_SECRET ? env.DEEPSEEK_API_KEY?.trim() : undefined;
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

/**
 * 统一构造 agent provider env（真实生产路径）：
 * 只允许「该模型唯一凭据」+ remoteChildIdentityEnv() 过滤出的 identity 进入。
 * 调用侧无法把任意进程 env / 另一模型凭据夹带进 provider env。
 */
const modelProviderEnv = (
  secretName: ModelSecretName,
  token: string,
  env: NodeJS.ProcessEnv,
): Record<string, string> => ({
  [secretName]: token,
  ...remoteChildIdentityEnv(env),
});

export type SelectableRole = "implement" | "implement-pr" | "review" | "update-branch";

export const HOSTED_DEFAULT_RETIRED =
  "Hosted agent default is retired (#1617): this runner has no approved Kimi credential or reachable CLIProxy endpoint. " +
  "Use the local gpt-6.1-sol Sandcastle queue, or explicitly opt into agent:model:deepseek-v4-pro where supported.";

/** 旧 explore 入口保留函数名，但不再启动 Claude；update-branch 由 selectedAgent 显式选路。 */
export const claudeAgent = (
  role: AgentRole,
  env: NodeJS.ProcessEnv = process.env,
): never => {
  void role;
  void env;
  throw new ModelRegistryError(HOSTED_DEFAULT_RETIRED);
};

/**
 * 按事件标签构建实装/评审 agent（implement / implement-pr / review）。
 * 无标签默认在模型调用前明确退役；`agent:model:deepseek-v4-pro` 仅作显式 legacy opt-in。
 * explore 由 claudeAgent() 拒绝旧默认；update-branch 也可显式选择 legacy。
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
      entry.primeProvider === "deepseek" &&
      entry.secretName === DEEPSEEK_SECRET
    ) {
      // Actions runner（noSandbox，无 docker 沙盒）：HOME=/home/runner，prime-agent 会话
      // 落在 runner HOME 下；provider 默认沙盒目录 /home/agent/... 不可写（#1173 实测
      // EACCES mkdir /home/agent/.prime/agent/sessions）。GITHUB_ACTIONS 环境把沙盒会话
      // 目录指回 runner HOME（noSandbox 下沙盒 == 宿主，复用宿主会话目录常量），
      // capture/resume/extraction 二段全部走同一目录。
      const sessionStorage =
        env.GITHUB_ACTIONS === "true"
          ? { sandboxSessionsDir: DEFAULT_HOST_SESSIONS_DIR }
          : undefined;
      return primeAgent(entry.model, {
        provider: entry.primeProvider,

        // 只注入 deepseek 专属凭据 + remoteChildIdentityEnv() 过滤后的 identity；
        // Claude token 与任何未列名进程 env 一律不进 provider env。
        env: modelProviderEnv(DEEPSEEK_SECRET, token, env),

        sessionStorage,
      });
    }
    throw new ModelRegistryError(
      `Registry entry ${entry.label} has no provider constructor branch.`,
    );
  }

  throw new ModelRegistryError(HOSTED_DEFAULT_RETIRED);
};
