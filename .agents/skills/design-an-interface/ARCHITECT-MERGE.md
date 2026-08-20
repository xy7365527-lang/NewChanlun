# architect 方法并入说明（#1136）

本文件记录把 pstack `architect` 中有净新增价值的片段并入 `design-an-interface` 的来源与
本地改写，是 pstack-lite 合并能力的「等价来源说明」（口径见
`docs/agents/pstack-lite/SOURCE-STATEMENT.md` 与
`docs/agents/skill-local-customizations.md`）。本 Skill 保持接口多方案探索的
触发与输出契约不变，只在探索流程内加入新增方法。

## 来源

- **上游仓库**：`cursor/plugins`（GitHub，`pstack/` 目录）
- **固定 SHA**：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`（不自动跟随上游 `main`）
- **原始 Skill**：`pstack/skills/architect/SKILL.md` 及其
  `references/design-red-flags.md`、`references/rationale-template.md`、
  `references/runner-prompt.md`
- **许可证**：MIT（Copyright (c) 2026 Lauren Tan），见
  `docs/agents/pstack-lite/LICENSE.MIT`（上游 `pstack/LICENSE` 逐字节副本）

## 借用片段（并入 SKILL.md 的对应位置）

- **调用方优先**（`## Workflow` 第 1/2 步）——借自 `references/rationale-template.md`
  的「Usage (caller's view)」（usage 写在 type sketch 之前、二者不一致时改 sketch）
  与 `references/runner-prompt.md` 的「Caller's usage first」；落地为
  需求收集阶段先写 README 式 usage + 2–3 个真实调用点，子代理输出格式把 usage 提到
  签名之前。
- **设计红旗筛查**（`## Workflow` 第 4 步）——借自 `references/design-red-flags.md`
  的四条（shallow module / information leakage / temporal decomposition /
  pass-through method）；落地为比较前先筛候选，红旗命中即修订或拒绝。
  （红旗的完整判据在 `codebase-design` 的「Design red flags」节，本 Skill 引用之，
  不重复抄录。）

## 本地 Prime 改写

- **去掉 Cursor 专属编排**：上游 `how → arena → agree → implement → scrap` 总控流程、
  多模型 runner 面板与 `.cursor` 路径**不并入**；本 Skill 仍是「3+ 并行子代理出不同
  接口方案 → 比较 → 综合」的既有流程，只在其内加入 usage-first 与红旗筛查。
- **不新增触发**：`description` 未改动，触发契约与原有代表性任务（设计 API、探索接口
  方案、比较模块形状、`design it twice`）保持不变。
- **不创建 `pstack-architect`**：本合并不产生新 Skill，也不与 `pstack-arena` 的
  同题 bakeoff 扇出重复——本 Skill 只做接口形状的多方案探索，不覆盖实现竞争。
- **与 `codebase-design` 分工写明**：本 Skill = 多方案探索；`codebase-design` =
  单方案深模块设计。两处 SKILL.md 的 scope 声明互指，避免同题双触发。
- **Prime 异步结果修复（#1136 后续行为门）**：删除旧 `Task tool` 同步语义，改为根代理
  持有 3+ 个独立 RLM admission；每个 child 在 admission 前取得根 session-dir 内唯一绝对
  结果文件与显式读/工具/时间/输出预算。结果按完整消息 → 同内容文件 → child 最终 JSONL 回收；
  admission handle 不算结果，缺任一结果即停止比较，父不得代写。此段及确定性契约夹具是
  本地 Prime 改写，不是上游借用片段。

## 未借用（有意排除）

- 上游 orchestrator 流程（Phase A/B/C/D/E）与 `references/runner-prompt.md` 的候选
  独立性约定（属 `pstack-arena`）。
- `references/rationale-template.md` 的完整模板（Problem / Shape / Synthesis decision /
  Tradeoffs / Alternatives / Open questions / Next step）——只借「usage 先行」单点；
  其余输出契约维持本 Skill 既有「签名 + usage + 隐藏内容 + trade-off」格式。
- 固定模型面板与上游 `disable-model-invocation: true`。
