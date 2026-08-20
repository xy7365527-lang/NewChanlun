---
name: pstack-arena
description: 对确有多个真实候选的高杠杆任务扇出 N 个独立候选（只读候选走 RLM、写代码候选走 Sandcastle 隔离），由统一评委按同一 rubric 选基底，再把落选者的优点 graft 进基底并验证。触发条件：存在至少两个真实候选、单次尝试会过早锁死形态、且没有更窄并行 Skill 覆盖。显式入口 /skill:pstack-arena；短语「arena this」「扔进竞技场」也触发。不触发：接口/类型/模块边界设计（归 design-an-interface）、trivial 改动、单一方案、Wayfinder 图规划。
license: MIT
metadata:
  upstream-repo: cursor/plugins
  upstream-sha: fd6dd6f7276956a532bb78a748a8d2818b6eb5f4
  upstream-path: pstack/skills/arena/SKILL.md
  license-file: LICENSE.MIT
---

# pstack-arena（多候选竞技择优）

对同一任务扇出 N 个并行候选，逐个通读后选最强者为基底，把落选者最有价值的部分 graft 进基底，最后验证合成结果。

## 触发前提（三条同时满足才用本 Skill，否则跳过）

1. 存在至少 2 个真实候选（不同方案 / 不同设计方向），不是只有一个方案。
2. 单次尝试会过早锁死形态——一次写死就难以回头的非平凡产物。
3. 没有更窄的并行 Skill 已覆盖：
   - 接口 / 类型 / 模块边界的设计候选归 `design-an-interface`（深模块设计归 `codebase-design`），本 Skill 不重复扇出；
   - 本 Skill 不包裹 Wayfinder——不开图、不动 map / ticket / frontier / 票型路由，只干当前票内的叶子能力。

## 角色分工（Prime 默认递归深度 = 1，只有一层扇出）

- **根代理 = 唯一编排者**：持有全部编排状态（rubric、输出路径、子代理句柄、评分、synthesis note），并亲自执行 Pick / Graft / Verify 三段。
- **候选与评委 = 根代理的直接子代理**（互为兄弟，同一层）。受递归深度限制，它们**不能再派孙代理**；需要再扇出的活由根代理拆好再派。

## Phase A：Frame（先立契约再扇出）

N 个候选会收到同一份任务 prompt，prompt 就是契约，扇出前必须写对。

1. 明确每个候选要产出的 artifact。
2. 派生 rubric：先写「本任务的成功长什么样」，再转成 3–6 条具体可打分的判据。具体 = 「新增 `--dry-run` 旗标，跳过写操作」；模糊 = 「代码正确」。rubric 是 Phase D 选基底的工具；候选只看到任务，看不到 rubric。
3. 挑 runner 模型：候选默认继承根代理模型；要显式换模型用 `await rlm.find_models(...)` 取准确 selector（不存在的模型会 spawn 失败）。生成型工作可用同一模型跑 N 次；判断敏感型工作用不同模型族各跑一次。
4. 分配输出路径：每个候选写到**自己独立的位置**。N 个候选写同一路径 = 共享可变状态，判违规。

## Phase B：Fan out（RLM 异步 admission）

一次扇出全部 N 个候选：

- 只读候选（设计 / 分析 / 研究 / 文档）：`await rlm('sub-task', name='arena-<slug>-cand-<n>')`。**admission 立即返回句柄（rlm_child_id / name / session_dir / model），返回值不是候选答案**；候选完成后经 `await agent_message.send(..., receiver_role='parent')` 回传，或把 artifact + rationale 写到自己的输出路径、由根代理读回。
- 每个候选必须交付 **artifact + 简短 rationale**（强制）：rationale 写它考虑过哪些替代方案、拒绝了什么。没有 rationale，根代理分不清候选的结构是「有原则的选择」还是「碰巧」，Phase E 的 graft 就不可靠。
- **候选彼此不可见**：各自独立会话；根代理不把兄弟名 / 别的候选输出路径发给候选；候选只回根代理、不互发消息。
- 候选产出失败 → 用 N-1 继续，并在 synthesis note 记 drop-out。

根代理把全部句柄存进 IPython 变量；**在分开的调用里逐个 admission，然后结束本回合等候选回流**（不要阻塞等待 `rlm()` 的返回值）。

## Phase C：Cross-judge（统一评委，同一 rubric）

全部候选完成后，起**一个只读评委**子代理：

- 评委模型优先选与根代理不同模型族（`rlm.find_models(...)` 取 selector）。
- 评委看到**同一份 rubric** + 按路径标签标注的各候选，逐条判据打分，并推荐一个基底 + 理由。
- 评委与候选**不是并行**：候选还在写时起评委，评委只会看到残缺/空输出并误报 drop-out。评委可以与根代理自己的通读（Phase D）并行。

## Phase D：Pick a base（根代理亲自选）

- 选之前**逐个候选从头读到尾**——只看表面会选中「长得最眼熟」的那个。
- 拿 rubric 逐条判据打分，不凭整体感觉。与评委结论比对：一致 = 确认；不一致 = 有一方偏了或 rubric 有歧义，两边 rationale 都读后再定。
- 基底选「未来维护者最不容易破坏不变量、最容易扩展」的那个；两个感觉打平时，偏好更干净的边界 / 更小的表面积。
- 把选择和理由写进简短 synthesis note（与基底 artifact 放一起），**含评委 verdict**。

## Phase E：Graft（根代理亲自移植）

- 把每个落选候选再走一遍，挑出值得搬进基底的东西。信号通常是每个候选 1–2 个点，不是大部分。
- **手工折叠，不机械粘贴**；结果必须在同一个心智模型下自洽。
- 记录 graft 了什么、来自哪个候选、拒绝了什么、为什么。拒绝记录是整份 record 里信号最高的部分。
- N 个候选收敛到同一形态 = 强一致信号：在 record 里注明收敛，直接交付共识形态，不需要 graft。N 个候选严重发散 = Phase A 规格不足：重写任务重跑，而不是取平均。

## Phase F：Verify（合成结果照常过验证）

合成 artifact 必须和其他任何产出一样经受验证（prove-it-works），竞技场不给你免检。

验证暴露了竞技场没抓到的问题 → 要么 Phase A 错了（重新 Frame 重跑），要么某个候选早就抓到、是你漏了 graft（回到 Phase E）。不许粉饰。

## 写代码候选的隔离（Prime 特有）

- **写代码候选不在共享主工作区运行**。根代理 checkout / IPython 工作目录里跑并行写码候选 = 撞车 + 抹掉未提交改动。
- 写代码候选必须走 **Sandcastle 隔离**：`.sandcastle/` 的确定性分支 + Docker 沙盒执行，候选各自一条 `sandcastle/...` 分支或独立 worktree；最终合成产物**必须经过 Sandcastle 与人工合入闸**——根代理不 merge、不 push、不动 main。
- 只读候选（设计 / 分析 / 研究 / 文档）走 RLM，不需要 Sandcastle。

## Outputs

- 一份合成 artifact；
- 一份简短 synthesis note（放其旁）：写明基底、graft（含来源候选）、拒绝项、drop-out（如有）、评委 verdict、验证结果。

## 来源

- **上游仓库**：[`cursor/plugins`](https://github.com/cursor/plugins)（GitHub，`pstack/` 目录）
- **固定 SHA**：[`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`](https://github.com/cursor/plugins/tree/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4)（不自动跟随上游 `main`）
- **原始 Skill**：[`pstack/skills/arena/SKILL.md`](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/arena/SKILL.md)（该目录无 `references/` 或 `scripts/`）

## 本地 Prime 改写

- **调用契约**：上游 `disable-model-invocation: true`（Cursor 仅显式调用）→ 删除该字段，改为 `both`（模型可见可自动调用 + `/skill:pstack-arena` 显式入口），符合 #1124/#1130 首批统一 `both` 契约。
- **`run_in_background: true`（Cursor Task 后台）→ RLM 异步 admission**：`rlm('sub-task')` admission 立即返回句柄、结果经 `agent_message` / 文件回流；不保留 Cursor 的 Task/后台语义。
- **模型选择**：上游的 `~/.cursor/rules/pstack-models.mdc`（arena runners / cross-judge pool）与 Cursor slug（`claude-fable-5-thinking-max` / `gpt-5.6-sol-max` / `grok-4.6-fast-xhigh` / `claude-opus-5-thinking-xhigh`）→ 移除，改按 Prime 运行时可用档选模型（`rlm.find_models(...)` 取 selector，子代理默认继承根代理模型）。
- **输出隔离**：上游 `git worktree` / `/tmp/arena-<slug>/candidate-<n>/` → 只读候选走 RLM（各自 `session_dir`），写代码候选走 Sandcastle（确定性分支 + Docker 沙盒 + 人工合入闸）。
- **递归深度**：上游允许候选再扇出；Prime 默认递归深度 = 1，候选与评委都是根代理直接子代理（一层），Pick / Graft / Verify 由根代理亲自执行。
- **上游 principle-skill 交叉引用**：`Laziness Protocol` / `separate-before-serializing-shared-state` / `redesign-from-first-principles` / `prove-it-works` 是上游 principle skills 的名字，本仓无对应 Skill；保留其判据语义（更小表面积 / 分离后序列化 / 手工折叠 / 照常验证），不建同名 Skill。
- **其余流程原样保留**：Frame → Fan out → Cross-judge → Pick → Graft → Verify 六段、rubric 3–6 条、rationale 强制、drop-out 记录、收敛/发散处理。

## 许可证

MIT（Copyright (c) 2026 Lauren Tan），见同目录 `LICENSE.MIT`。
