# architect 方法并入说明（#1136）

本文件记录把 pstack `architect` 中有净新增价值的片段并入 `codebase-design` 的来源与
本地改写，是 pstack-lite 合并能力的「等价来源说明」（口径见
`docs/agents/pstack-lite/SOURCE-STATEMENT.md` 与
`docs/agents/skill-local-customizations.md`）。本 Skill 保持单方案深模块设计的
触发与输出契约不变，只在其流程内加入新增方法。

## 来源

- **上游仓库**：`cursor/plugins`（GitHub，`pstack/` 目录）
- **固定 SHA**：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`（不自动跟随上游 `main`）
- **原始 Skill**：`pstack/skills/architect/SKILL.md` 及其
  `references/design-red-flags.md`、`references/rationale-template.md`、
  `references/runner-prompt.md`
- **许可证**：MIT（Copyright (c) 2026 Lauren Tan），见
  `docs/agents/pstack-lite/LICENSE.MIT`（上游 `pstack/LICENSE` 逐字节副本）

## 借用片段（并入 SKILL.md 的对应节）

- **设计红旗**（`## Design red flags`）——借自 `references/design-red-flags.md` 的
  四条：shallow module / information leakage / temporal decomposition /
  pass-through method。逐条压缩为「判据 + 征兆」，措辞改写为与本 Skill 既有词汇
  （module / interface / seam / depth）一致。
- **调用方优先**（`## Caller-first sketch`）——借自 `references/rationale-template.md`
  的「Usage (caller's view)」与 `references/runner-prompt.md` 的
  「Caller's usage first / Data structures first / Make boundaries visible /
  Encode invariants in types / Validate at boundaries」。
- **实施中复核**（`## Design-to-implementation loop`）——借自 `SKILL.md` 的
  Phase D（deviations as signal）与 Phase E（scrap when the architecture is wrong，
  含「同形重复摩擦」判据与 scrapping 三步）。

## 本地 Prime 改写

- **去掉 Cursor 专属编排**：上游的 `how → arena → agree → implement → scrap` 总控流程、
  多模型 runner 面板（`claude-fable-5-thinking-max` 等 slug）、`Task`/`AskQuestion`/
  `.cursor` 路径**不并入**。本仓 Wayfinder 是唯一上层编排器；多方案探索由
  `design-an-interface` 承担；本 Skill 只保留单方案深模块设计的词汇与原则。
- **沿用本 Skill 术语**：上游的模块/接口表述改写为 module / interface / seam /
  adapter / depth / locality / leverage，不引入上游第二条术语体系；「deep call chain」
  并入既有「depth-as-leverage」口径。
- **不新增触发**：`description` 未改动，触发契约与原有代表性任务（设计/改进模块接口、
  寻找深化机会、决定 seam 位置、提升可测试性）保持不变；新增方法在已触发的流程内生效。
- **实施副作用不授权**：上游「默认不设 checkpoint 直接实施」不并入——实施仍由
  Wayfinder / spec / implement 票链接管；本处只保留「偏差是信号、同形重复摩擦即重画」的
  设计判据。
- **不创建 `pstack-architect`**：本合并不产生新 Skill，也不与 `pstack-arena` 或
  `design-an-interface` 的多方案扇出重复。

## 未借用（有意排除）

- 上游 orchestrator 的 Phase A（跑 `how`）/ Phase B（跑 `arena`）/ Phase C（checkpoint）
  整段编排；`references/runner-prompt.md` 的候选独立性/工作目录约定（属 `pstack-arena`）。
- `references/rationale-template.md` 的「Synthesis decision（由 arena 填）」节
  （多方案综合属 `design-an-interface`）。
- 固定模型面板与上游 `disable-model-invocation: true`（本仓派生代理默认继承会话模型；
  现役设计 Skills 维持既有调用契约）。
