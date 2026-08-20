# 对抗评审模式（interrogate 移植）

本文件是 `code-review` 可选对抗评审的完整流程，移植自 pstack `interrogate`，并按 Prime
RLM 异步子代理语义改写。SKILL.md 的「升级判定」决定何时进入本模式；本文件描述进入之后
如何执行。

原则：对抗信号来自**模型多样性**，不是分配人设。每个 reviewer 拿到相同的意图、diff 与
rubric；不同模型在盲点、先验、推理模式上各不相同。跨模型一致是高置信信号，单模型发现
值得读但置信较低。交付物是**综合裁决**，**绝不自动应用改动**。

## 步骤 1：确定评审范围

- 用户指向具体文件或 diff → 用它。
- 特性分支 → `git diff main...HEAD`（或对应基线）拿完整变更集。
- 用户消息引用近期工作 → 收集相关文件。

把 diff（或文件内容）连同 reviewer 理解代码所需的周边上下文打包。

## 步骤 2：写明意图

拉 reviewer 之前，先写一段话说明这段代码想达成什么。来源：用户消息、提交信息、PR/票
描述、代码本身。reviewer 只挑战「实现是否把意图做好」，不挑战意图本身。意图不确定就先
问用户。

## 步骤 3：模型多样性前置门与独立 reviewer（Prime 语义）

### 3.1 前置门（自动升级与显式请求一视同仁）

1. **先发现再选择**：必须先执行 `await rlm.find_models(limit=8)`，不可凭记忆硬编码 slug，
   更不可运行真实模型探针来猜凭据状态。
2. **准入下限**：先按 selector 精确去重；重复 selector 只能算一个。去重后必须能选出
   至少 3 个不同、可启动的 selector，并覆盖至少 2 个 vendor/family 桶。桶表示基础模型的
   **所有者 vendor/family**：直连 provider 就是所有者时以 provider 为准，该 provider 下
   的产品线、版本与推理档全部算同一桶（例如三个 OpenAI selector 仍只有一个桶）；只有
   Prime Inference 一类聚合 provider 才按其承载的基础模型所有者 family 分桶。因此聚合
   provider 下 ZAI 与 Alibaba 可算两桶，而同 family 的不同版本/推理档不能。
3. **不足即 unavailable**：selector 少于 3 或多样性少于 2 桶时，不进入对抗模式，不得
   用三个同厂/同家族模型冒充通过。回到 `SKILL.md` 的标准 Standards/Spec 双轴，并在报告
   开头显式写 `Adversarial status: unavailable`、缺口、已发现数量/桶数和稍后重试条件。
4. **显式请求不豁免**：用户明确要求 adversarial 时仍执行同一门；门不通过必须报告缺口，
   不能静默给出看似对抗评审的绿色结果。

### 3.2 拉起与启动失败降级

为三个已选 selector 分别执行 `await rlm('task', name='adversarial-reviewer-<A/B/C>',
model=<selector>)`。任务文本是填好的 reviewer prompt（模板见
[ADVERSARIAL-REVIEWER-PROMPT.md](ADVERSARIAL-REVIEWER-PROMPT.md)）。

- 任一 selector 启动失败，或启动成功者不再满足 3 个 selector / 2 个 vendor/family 桶，
  立即停止对抗汇总。**不得从同厂/同家族补位**。
- 回到标准 Standards/Spec 双轴，在报告开头显式写
  `Adversarial status: degraded`、失败 selector/阶段和稍后重试条件。
- admission 只返回 `rlm_child_id/name/session_dir/model` 句柄，**不是 reviewer 结果**，不能
  用 admission 成功数生成 findings 或 agreement map。

### 3.3 独立性与真实结果回流

每个 reviewer 的 prompt 只含三样——(1) 意图、(2) diff/文件、(3) rubric 与
code-quality lens。**不要把其他 reviewer 的结论转给它**。三个 reviewer 独立拉起。

结果优先经 `agent_message` 回复；桥不可用时，任务可约定 child 在自己的 `session_dir`
写结果文件，根代理读取该文件，或读取 child 最终 JSONL 中的最终产出。无论哪条通道，根
代理都必须实际读取并解析真实 child 产物。若任一 reviewer 在约定等待/重试后仍无真实产物，
按 `Adversarial status: degraded` 回退标准双轴；报告必须点名具体 reviewer label/selector、
尝试过的回流通道、缺失或不可读的证据，以及稍后重试条件。不得把 admission handle 当作
结果继续汇总。

**填充 reviewer prompt**：读
[ADVERSARIAL-REVIEWER-PROMPT.md](ADVERSARIAL-REVIEWER-PROMPT.md) 模板，填入：
1. 步骤 2 的意图；
2. diff 或文件内容；
3. [ADVERSARIAL-RUBRIC.md](ADVERSARIAL-RUBRIC.md) 的评审 rubric；
4. [ADVERSARIAL-CODE-QUALITY.md](ADVERSARIAL-CODE-QUALITY.md) 的代码质量 lens。

同一份填好的模板发给所有 reviewer，每个模型都套用同一 code-quality lens。每个 reviewer
按模板产出结构化 findings。

## 步骤 4：汇总 + agreement map

结果回流后，建立统一图景：

1. **解析所有 finding**。
2. **识别共识**：2 个及以上模型独立提出的问题是最高置信信号。
3. **识别单模型发现**：仍值得读，但按单模型权重对待。
4. **去重**：不同模型可能用不同话描述同一问题，合并并注明哪些模型提出。
5. **记录分歧**：一个模型 flag、另一个明确说相反，这是裁决的有用上下文。

**agreement map 只汇总重复与分歧，不替代主控的技术判断**（见步骤 5）。

## 步骤 5：主控裁决（lead judgment）

你是主控 reviewer，一个务实的高级工程师，不是中立的聚合器。读
[ADVERSARIAL-LEAD-JUDGMENT.md](ADVERSARIAL-LEAD-JUDGMENT.md) 全文。reviewer 只看到代码库
切片；你拥有全上下文（目标、约束、时间线、已权衡过的取舍），要大胆使用。

每条 finding 归入四桶之一：

- **Act on（接受）**：按实际目标看影响正确性、安全或可维护性的真问题，会挡住真实 PR。
- **Consider（考虑）**：合理，但不确定现在处理是否值得代价，值得用户关注。
- **Noted（记录）**：技术上成立但不可执行：依赖上下文、过早优化、当前阶段低影响。
- **Dismissed（驳回）**：错误、吹毛求疵或缺上下文，简要说明原因。

每条附：哪些模型提出、桶归属、一句归类理由。

## 输出格式

### Intent
> [步骤 2 的意图段落]

### Reviewers
- Reviewer [label]：[模型名]，[N] findings（每个 reviewer 一行）

### Act On
[应处理的问题。每条：描述、哪些模型提出、为什么重要。]

### Consider
[值得思考。每条：描述、哪些模型提出、涉及的权衡。]

### Noted
[成立但低优先级，简短列表。]

### Dismissed
[被驳回的发现 + 简要理由。让用户看到过滤了什么、为什么，以便用户不同意时推翻你的判断。]

### Agreement Map
[模型在哪里一致、在哪里分歧，一致/分歧的模式说明什么。]

## 来源与本地改写

- **上游仓库**：`cursor/plugins`（GitHub，`pstack/` 目录）
- **固定 SHA**：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`（不自动跟随上游 `main`）
- **原始 Skill**：`pstack/skills/interrogate/SKILL.md`（SHA-256
  `a009220dfe6869c8f7980a94fb1c9c7763a081a0b32fb6d0afb65b8ff1868146`）及其
  `references/{reviewer-prompt,rubric,code-quality-review,lead-judgment}.md`（SHA-256
  分别 `a397cc61102add709803d917fb23d726920525bf23e7c06dc4ed0b5cbeb00e54` /
  `a67bf02426f88714634ff481d667db821d4b2cea7b335bcd165fe3126e427fb5` /
  `2462f1347b99b412b04fcb0577b96d8744c801f792f2651746f9409fd4465dc9` /
  `d2cea6cc308758201c6b8b82baf780947645f1ab752707ddf97fab374bf473f9`）。
- **许可证**：MIT（Copyright (c) 2026 Lauren Tan），原文见
  `docs/agents/pstack-lite/LICENSE.MIT`（SHA-256
  `bc957ca6bee02792566a1a028d105e02e247c6e77cf057061674273da77b200e`）。

### 借用片段

- 步骤 2（写明意图）、步骤 4（共识/单模型/去重/分歧）、步骤 5（四桶归类）、输出格式——
  借用上游 SKILL.md 对应段落，翻译为简体中文并去掉 Cursor Task 语义。
- `ADVERSARIAL-REVIEWER-PROMPT.md` 借用上游 `references/reviewer-prompt.md` 的 finding
  结构（severity/finding/evidence/suggestion）与「好 finding / 避免什么」清单。
- `ADVERSARIAL-RUBRIC.md` 借用上游 `references/rubric.md` 的六个 lens。
- `ADVERSARIAL-CODE-QUALITY.md` 借用上游 `references/code-quality-review.md` 的维度清单
  与 approval bar。
- `ADVERSARIAL-LEAD-JUDGMENT.md` 借用上游 `references/lead-judgment.md` 的过滤原则与
  verdict calibration。

### 本地改写

- **Cursor Task → Prime RLM**：上游「单条消息多 Task 调用、subagent_type generalPurpose、
  readonly true」改为 `await rlm('task', …)` 独立拉起 + `agent_message`/文件异步回流；
  admission 只返回句柄不是结果。
- **模型 slug → find_models**：上游 `~/.cursor/rules/pstack-models.mdc` 配置与
  `claude-fable-5-thinking-max` 等 Cursor slug 改为先用 `await rlm.find_models()` 发现候选，
  再执行 3 selector / 2 vendor-family 的准入门和启动失败降级；发现结果不冒充启动成功。
- **disable-model-invocation**：上游 interrogate 是 `disable-model-invocation: true`
  （只能显式调用）；合并后作为 code-review 的可选模式，跟随 code-review 的模型可见契约，
  不设该字段。
- **升级判定并入 code-review**：上游独立 Skill 的触发短语并入 code-review description；
  自动升级由「高风险/有争议/非小型 diff」判定门驱动，低风险小 diff 保持标准双轴评审。
- 本仓不创建 `pstack-interrogate` Skill。
