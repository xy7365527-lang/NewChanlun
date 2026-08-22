import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  AGENT_MODELS,
  CLAUDE_CODE_SECRET,
  DEEPSEEK_SECRET,
  MODEL_LABEL_PREFIX,
  MODEL_REGISTRY,
  ModelRegistryError,
  assertValidModelLabels,
  modelCredential,
  modelLabelsFromEvent,
  readModelLabelsFromEnv,
  remoteChildIdentityEnv,
  resolveModelSelection,
  selectedAgent,
} from "./agent.ts";
import type { SelectableRole } from "./agent.ts";
import { parseDiffLines } from "./diff-lines.ts";
import {
  filterInlineComments,
  filterReplies,
  reviewOutputSchema,
  type InlineComment,
} from "./review-output.ts";
import { standardSchema, asRecord, asString } from "./common.ts";

test("parseDiffLines maps added/context new-file line numbers", () => {
  const diff = [
    "diff --git a/foo.ts b/foo.ts",
    "index 0000000..1111111 100644",
    "--- a/foo.ts",
    "+++ b/foo.ts",
    "@@ -1,3 +1,4 @@",
    " line1",
    "+added",
    " line2",
    "-removed",
    " line3",
  ].join("\n");
  const lines = parseDiffLines(diff);
  assert.deepEqual([...lines.get("foo.ts") ?? []].sort((a, b) => a - b), [
    1, 2, 3, 4,
  ]);
});

test("parseDiffLines skips removed lines and handles multiple files", () => {
  const diff = [
    "+++ b/a.ts",
    "@@ -1 +1 @@",
    "+only-added",
    "+++ b/b.ts",
    "@@ -5 +5 @@",
    "-only-removed",
    " kept",
  ].join("\n");
  const lines = parseDiffLines(diff);
  assert.deepEqual([...(lines.get("a.ts") ?? [])], [1]);
  assert.deepEqual([...(lines.get("b.ts") ?? [])], [5]);
});

test("filterInlineComments drops comments outside the diff", () => {
  const diffLines = new Map<string, Set<number>>([
    ["a.ts", new Set([10, 11])],
  ]);
  const comments: InlineComment[] = [
    { path: "a.ts", line: 10, body: "ok" },
    { path: "missing.ts", line: 10, body: "file not in diff" },
    { path: "a.ts", line: 99, body: "line not in diff" },
  ];
  const kept = filterInlineComments(comments, diffLines);
  assert.deepEqual(
    kept.map((c) => c.body),
    ["ok"],
  );
});

test("filterReplies drops replies to unknown comment ids", () => {
  const valid = new Set(["id-1"]);
  const replies = [
    { commentId: "id-1", body: "kept" },
    { commentId: "id-2", body: "dropped" },
  ];
  assert.deepEqual(filterReplies(replies, valid), [
    { commentId: "id-1", body: "kept" },
  ]);
});

test("reviewOutputSchema validates shape and coerces inline comments", () => {
  const parsed = reviewOutputSchema["~standard"].validate({
    summary: "a summary",
    inlineComments: [{ path: "a.ts", line: 3, body: "b" }],
    replies: [],
  });
  assert.ok("value" in parsed);
  assert.equal(parsed.value.summary, "a summary");
  assert.equal(parsed.value.inlineComments[0].path, "a.ts");
});

test("reviewOutputSchema rejects a missing summary", () => {
  const parsed = reviewOutputSchema["~standard"].validate({
    inlineComments: [],
    replies: [],
  });
  assert.ok("issues" in parsed);
});

test("standardSchema helper returns issues on validation failure", () => {
  const schema = standardSchema((v) => asString(asRecord(v, "r").name, "name"));
  const ok = schema["~standard"].validate({ name: "x" });
  const bad = schema["~standard"].validate({});
  assert.ok("value" in ok);
  assert.ok("issues" in bad);
});



// ── #1128 显式模型 registry ────────────────────────────────────────────────

const fixture = (name: string): unknown =>
  JSON.parse(
    readFileSync(new URL(`./fixtures/${name}`, import.meta.url), "utf8"),
  ) as unknown;

test("resolveModelSelection 无模型标签默认 Claude（不读任何 secret）", () => {
  assert.deepEqual(resolveModelSelection([]), {
    label: null,
    entry: null,
  });
  assert.deepEqual(resolveModelSelection(["agent:implement"]), {
    label: null,
    entry: null,
  });
});

test("resolveModelSelection 显式 DeepSeek 标签走 registry 的 provider/model/secret", () => {
  const selection = resolveModelSelection(["agent:model:deepseek-v4-pro"]);
  assert.equal(selection.label, "agent:model:deepseek-v4-pro");
  assert.equal(selection.entry?.provider, "prime-agent");
  assert.equal(selection.entry?.primeProvider, "deepseek");
  assert.equal(selection.entry?.model, "deepseek-v4-pro");
  assert.equal(selection.entry?.secretName, DEEPSEEK_SECRET);
  assert.deepEqual([...selection.entry?.appliesTo ?? []], [
    "implement",
    "implement-pr",
    "review",
  ]);
});

test("resolveModelSelection 未知标签 fail-loud；票面字符串不成为 env/secret key", () => {
  assert.throws(
    () => resolveModelSelection(["agent:model:DEEPSEEK_API_KEY"]),
    (error: unknown) =>
      error instanceof ModelRegistryError &&
      /Unknown model label/.test(error.message) &&
      /agent:model:DEEPSEEK_API_KEY/.test(error.message),
  );
  // #1002 旧 Kimi 票面/常量不得复活：kimi 标签在新 registry 里是未知标签。
  assert.throws(
    () => resolveModelSelection(["agent:model:kimi-k3"]),
    /Unknown model label/,
  );
});

test("resolveModelSelection 多个模型标签冲突 fail-loud（含 known+unknown）", () => {
  assert.throws(
    () =>
      resolveModelSelection([
        "agent:model:deepseek-v4-pro",
        "agent:model:DEEPSEEK_API_KEY",
      ]),
    /Conflicting model labels/,
  );
  assert.throws(
    () =>
      resolveModelSelection([
        "agent:model:deepseek-v4-pro",
        "agent:model:claude-sonnet-4-6",
      ]),
    /Conflicting model labels/,
  );
  // 同一标签重复不构成冲突（GitHub labels 集合语义）。
  assert.doesNotThrow(() =>
    resolveModelSelection([
      "agent:model:deepseek-v4-pro",
      "agent:model:deepseek-v4-pro",
    ]),
  );
});

test("selectedAgent 默认 Claude：claudeCode + 仅 CLAUDE_CODE_OAUTH_TOKEN 进 provider env", () => {
  const provider = selectedAgent("implement", [], {
    CLAUDE_CODE_OAUTH_TOKEN: "claude-token",
  });
  assert.equal(provider.name, "claude-code");
  assert.deepEqual(provider.env, {
    CLAUDE_CODE_OAUTH_TOKEN: "claude-token",
  });
  assert.ok(!("DEEPSEEK_API_KEY" in provider.env));
  assert.equal(provider.buildPrintCommand({ prompt: "hi", dangerouslySkipPermissions: true }).command.includes("claude-sonnet-4-6"), true);
});

test("selectedAgent 显式 DeepSeek：Prime Agent deepseek/deepseek-v4-pro，仅 DEEPSEEK_API_KEY 进 provider env", () => {
  const provider = selectedAgent(
    "implement",
    ["agent:model:deepseek-v4-pro"],
    { DEEPSEEK_API_KEY: "deepseek-token" },
  );
  assert.equal(provider.name, "prime-agent");
  assert.deepEqual(provider.env, {
    DEEPSEEK_API_KEY: "deepseek-token",
  });
  assert.ok(!("CLAUDE_CODE_OAUTH_TOKEN" in provider.env));
  const command = provider.buildPrintCommand({
    prompt: "hi",
    dangerouslySkipPermissions: true,
  }).command;
  assert.ok(command.includes("--provider deepseek"));
  assert.ok(command.includes("--model deepseek-v4-pro"));
});

test("selectedAgent DeepSeek 缺 key 在任何模型调用前抛错，不回退 Claude", () => {
  assert.throws(
    () => selectedAgent("implement", ["agent:model:deepseek-v4-pro"], {}),
    /DEEPSEEK_API_KEY is not set or is empty/,
  );
  assert.throws(
    () =>
      selectedAgent(
        "implement",
        ["agent:model:deepseek-v4-pro"],
        { CLAUDE_CODE_OAUTH_TOKEN: "claude-token" },
      ),
    /DEEPSEEK_API_KEY is not set or is empty/,
  );
});

test("模型标签作用于 implement/implement-pr/review；explore/update-branch 拒绝消费", () => {
  for (const role of ["implement", "implement-pr", "review"] as const) {
    const provider = selectedAgent(
      role,
      ["agent:model:deepseek-v4-pro"],
      { DEEPSEEK_API_KEY: "deepseek-token" },
    );
    assert.equal(provider.name, "prime-agent");
    assert.deepEqual(provider.env, {
      DEEPSEEK_API_KEY: "deepseek-token",
    });
  }
  for (const role of ["explore", "update-branch"] as const) {
    assert.throws(
      () =>
        selectedAgent(
          role as unknown as SelectableRole,
          ["agent:model:deepseek-v4-pro"],
          { DEEPSEEK_API_KEY: "deepseek-token" },
        ),
      new RegExp(`does not apply to role ${role}`),
    );
  }
  // 这些角色仍以 Claude 模型为固定默认。
  assert.ok(AGENT_MODELS.review.startsWith("claude-opus"));
  assert.ok(AGENT_MODELS.explore.startsWith("claude-opus"));
  assert.ok(AGENT_MODELS["update-branch"].startsWith("claude-opus"));
});

test("assertValidModelLabels 只校验标签，不读 DeepSeek/Claude 凭据", () => {
  assert.doesNotThrow(() =>
    assertValidModelLabels(["agent:model:deepseek-v4-pro"]),
  );
  assert.throws(
    () => assertValidModelLabels(["agent:model:not-a-real-model"]),
    /Unknown model label/,
  );
});

test("modelCredential 只能读 allowlist 内的固定 env 名", () => {
  assert.equal(
    modelCredential(DEEPSEEK_SECRET, { DEEPSEEK_API_KEY: " sk " }),
    "sk",
  );
  assert.equal(
    modelCredential(CLAUDE_CODE_SECRET, {
      CLAUDE_CODE_OAUTH_TOKEN: " tok ",
    }),
    "tok",
  );
  assert.throws(() => modelCredential(DEEPSEEK_SECRET, {}), /DEEPSEEK_API_KEY/);
});

test("remote-child invitation/lease 与模型凭据分离：identity env 不夹带模型 secret", () => {
  const identity = remoteChildIdentityEnv({
    PRIME_REMOTE_CHILD_INVITATION: "one-time-invitation",
    PRIME_REMOTE_CHILD_LEASE: "short-lease",
    DEEPSEEK_API_KEY: "deepseek-token",
    CLAUDE_CODE_OAUTH_TOKEN: "claude-token",
  });
  assert.deepEqual(identity, {
    PRIME_REMOTE_CHILD_INVITATION: "one-time-invitation",
    PRIME_REMOTE_CHILD_LEASE: "short-lease",
  });
  assert.ok(!("DEEPSEEK_API_KEY" in identity));
  assert.ok(!("CLAUDE_CODE_OAUTH_TOKEN" in identity));
});

test("modelLabelsFromEvent 支持 issue/pull_request labeled payload 与最小 labels 数组", () => {
  assert.deepEqual(
    modelLabelsFromEvent(
      fixture("issue-labeled-default-claude.json"),
    ),
    ["wayfinder:task", "agent:implement"],
  );
  assert.deepEqual(
    modelLabelsFromEvent(
      fixture("pr-labeled-implement-pr-deepseek-v4-pro.json"),
    ),
    ["agent:implement", "agent:model:deepseek-v4-pro"],
  );
  assert.deepEqual(
    modelLabelsFromEvent(
      fixture("pr-labeled-review-deepseek-v4-pro.json"),
    ),
    ["agent:review", "agent:model:deepseek-v4-pro"],
  );
  // P2：workflow 最小 EVENT_PAYLOAD 是 labels.*.name 的字符串数组。
  assert.deepEqual(
    modelLabelsFromEvent(["agent:implement", "agent:model:deepseek-v4-pro"]),
    ["agent:implement", "agent:model:deepseek-v4-pro"],
  );
  // 兼容只传 github.event.*.labels 数组（label 对象带 name）的形态。
  assert.deepEqual(
    modelLabelsFromEvent([
      { name: "agent:implement" },
      { name: "agent:model:deepseek-v4-pro" },
    ]),
    ["agent:implement", "agent:model:deepseek-v4-pro"],
  );
  assert.throws(
    () => modelLabelsFromEvent([{ name: "" }]),
    /non-empty string name/,
  );
});

test("readModelLabelsFromEnv 解析 EVENT_PAYLOAD；缺省按无标签处理", () => {
  const raw = readFileSync(
    new URL("./fixtures/issue-labeled-deepseek-v4-pro.json", import.meta.url),
    "utf8",
  );
  assert.deepEqual(readModelLabelsFromEnv({ EVENT_PAYLOAD: raw }), [
    "wayfinder:task",
    "agent:implement",
    "agent:model:deepseek-v4-pro",
  ]);
  // P2：EVENT_PAYLOAD 只传 label name 数组（workflow 接线形态）。
  assert.deepEqual(
    readModelLabelsFromEnv({
      EVENT_PAYLOAD: JSON.stringify([
        "agent:implement",
        "agent:model:deepseek-v4-pro",
      ]),
    }),
    ["agent:implement", "agent:model:deepseek-v4-pro"],
  );
  assert.deepEqual(readModelLabelsFromEnv({}), []);
});

test("#1128 事件 fixture：默认 Claude / 显式 DeepSeek / 冲突 / 未知 / 缺 secret", () => {
  // 默认 Claude fixture
  const defaultLabels = modelLabelsFromEvent(
    fixture("issue-labeled-default-claude.json"),
  );
  assert.equal(resolveModelSelection(defaultLabels).entry, null);

  // 显式 DeepSeek fixture
  const deepseekLabels = modelLabelsFromEvent(
    fixture("issue-labeled-deepseek-v4-pro.json"),
  );
  const deepseek = resolveModelSelection(deepseekLabels);
  assert.equal(deepseek.entry?.model, "deepseek-v4-pro");
  assert.equal(deepseek.entry?.provider, "prime-agent");

  // retry：同票重加 agent:implement 后选择保持一致
  const retryLabels = modelLabelsFromEvent(
    fixture("issue-labeled-deepseek-retry.json"),
  );
  assert.equal(
    resolveModelSelection(retryLabels).entry?.model,
    "deepseek-v4-pro",
  );

  // 冲突 fixture
  assert.throws(
    () =>
      resolveModelSelection(
        modelLabelsFromEvent(fixture("issue-labeled-conflict.json")),
      ),
    /Conflicting model labels/,
  );

  // 未知 fixture
  assert.throws(
    () =>
      resolveModelSelection(
        modelLabelsFromEvent(fixture("issue-labeled-unknown.json")),
      ),
    /Unknown model label/,
  );

  // 缺 secret fixture：标签合法但环境无 DEEPSEEK_API_KEY
  assert.throws(
    () =>
      selectedAgent(
        "implement",
        modelLabelsFromEvent(
          fixture("issue-labeled-deepseek-missing-secret.json"),
        ),
        {},
      ),
    /DEEPSEEK_API_KEY is not set or is empty/,
  );
});

test("#1128 P1 敌对标签：not-* 子串默认 Claude；-legacy fail-loud 不选中 DeepSeek", () => {
  // 旧 toJSON+contains 子串路由会误命中这两个标签；TS 层必须保持精确前缀语义。
  const notSubstringLabels = modelLabelsFromEvent(
    fixture("issue-labeled-not-deepseek-substring.json"),
  );
  assert.deepEqual(resolveModelSelection(notSubstringLabels), {
    label: null,
    entry: null,
  });
  const defaultProvider = selectedAgent("implement", notSubstringLabels, {
    CLAUDE_CODE_OAUTH_TOKEN: "claude-token",
    DEEPSEEK_API_KEY: "deepseek-token",
  });
  assert.equal(defaultProvider.name, "claude-code");
  assert.ok(!("DEEPSEEK_API_KEY" in defaultProvider.env));

  const legacyLabels = modelLabelsFromEvent(
    fixture("issue-labeled-deepseek-v4-pro-legacy.json"),
  );
  assert.throws(
    () =>
      selectedAgent("implement", legacyLabels, {
        DEEPSEEK_API_KEY: "deepseek-token",
      }),
    (error: unknown) =>
      error instanceof ModelRegistryError &&
      /Unknown model label/.test(error.message) &&
      /agent:model:deepseek-v4-pro-legacy/.test(error.message),
  );

  const prNotSubstringLabels = modelLabelsFromEvent(
    fixture("pr-labeled-implement-pr-not-deepseek-substring.json"),
  );
  assert.deepEqual(resolveModelSelection(prNotSubstringLabels), {
    label: null,
    entry: null,
  });

  const prLegacyLabels = modelLabelsFromEvent(
    fixture("pr-labeled-implement-pr-deepseek-v4-pro-legacy.json"),
  );
  assert.throws(
    () =>
      selectedAgent("implement-pr", prLegacyLabels, {
        DEEPSEEK_API_KEY: "deepseek-token",
      }),
    /Unknown model label/,
  );
});

test("#1173 PR fixture：review 无模型标签走 Claude，显式 DeepSeek 标签走 DeepSeek", () => {
  const defaultReviewLabels = modelLabelsFromEvent(
    fixture("pr-labeled-review.json"),
  );
  assert.deepEqual(resolveModelSelection(defaultReviewLabels), {
    label: null,
    entry: null,
  });
  const defaultReview = selectedAgent("review", defaultReviewLabels, {
    CLAUDE_CODE_OAUTH_TOKEN: "claude-token",
  });
  assert.equal(defaultReview.name, "claude-code");
  assert.deepEqual(defaultReview.env, {
    CLAUDE_CODE_OAUTH_TOKEN: "claude-token",
  });
  assert.ok(!("DEEPSEEK_API_KEY" in defaultReview.env));
  assert.ok(
    defaultReview
      .buildPrintCommand({ prompt: "hi", dangerouslySkipPermissions: true })
      .command.includes("claude-opus-4-8"),
  );

  const deepseekReviewLabels = modelLabelsFromEvent(
    fixture("pr-labeled-review-deepseek-v4-pro.json"),
  );
  const deepseekReview = selectedAgent("review", deepseekReviewLabels, {
    DEEPSEEK_API_KEY: "deepseek-token",
  });
  assert.equal(deepseekReview.name, "prime-agent");
  assert.deepEqual(deepseekReview.env, {
    DEEPSEEK_API_KEY: "deepseek-token",
  });
  assert.ok(!("CLAUDE_CODE_OAUTH_TOKEN" in deepseekReview.env));
  const deepseekCommand = deepseekReview.buildPrintCommand({
    prompt: "hi",
    dangerouslySkipPermissions: true,
  }).command;
  assert.ok(deepseekCommand.includes("--provider deepseek"));
  assert.ok(deepseekCommand.includes("--model deepseek-v4-pro"));
  assert.throws(
    () => selectedAgent("review", deepseekReviewLabels, {}),
    /DEEPSEEK_API_KEY is not set or is empty/,
  );

  const implementPrLabels = modelLabelsFromEvent(
    fixture("pr-labeled-implement-pr-deepseek-v4-pro.json"),
  );
  const provider = selectedAgent("implement-pr", implementPrLabels, {
    DEEPSEEK_API_KEY: "deepseek-token",
  });
  assert.equal(provider.name, "prime-agent");
  assert.deepEqual(provider.env, {
    DEEPSEEK_API_KEY: "deepseek-token",
  });
});

test("#1002 历史不复活：registry 只含显式 allowlist，不含 Kimi 常量/票面路由", () => {
  assert.ok(!("agent:model:kimi-k3" in MODEL_REGISTRY));
  for (const entry of Object.values(MODEL_REGISTRY)) {
    assert.equal(entry.primeProvider, "deepseek");
  }
  assert.ok(
    Object.values(AGENT_MODELS).every((model) => model.startsWith("claude-")),
  );
  assert.equal(MODEL_LABEL_PREFIX, "agent:model:");
});
