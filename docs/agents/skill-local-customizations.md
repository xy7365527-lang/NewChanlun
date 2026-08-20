# skills-lock 本机定制清单

> 正本：本文件是 `.agents/skills/` 相对上游「本机定制」的唯一清单。
> 最后更新：#1070（2026-08-18）
> 关联台账：`skills-lock.json`（记上游原件哈希）

## 口径

- `skills-lock.json` 的 `computedHash` 是**同步时上游原件**的哈希，用于检测上游更新；
- 本仓 `.agents/skills/` 是「上游 + 本机定制」的合成形态，凡下表所列 skill，其哈希与台账**失配是合法状态**（失配=定制，不是漂移）；
- 下表之外任何 skill 的哈希失配都按漂移对待，应排查。

## 现行定制（2026-08-18 实测 HEAD vs 上游形态逐字节 diff，共 9 处）

| skill | 差异文件 | 定制内容 | 定制日期 |
|---|---|---|---|
| grilling | SKILL.md, agents/openai.yaml | 提问风格保留「一次一问」，不用上游「一轮多问/frontier」 | 2026-08-14 |
| triage | SKILL.md | 同上（一次一问） | 2026-08-14 |
| loop-me | SKILL.md | 同上（一次一问） | 2026-08-14 |
| wayfinder | SKILL.md | 同上（一次一问）；另含本仓 wayfinder 纪律对齐 | 2026-08-14 |
| code-review | SKILL.md + ADVERSARIAL-*.md | 术语用「PRD」，不用上游「spec」；并入 pstack `interrogate` 可选对抗评审模式（#1137：独立 reviewer / 统一 rubric / agreement map / lead judgment，Prime RLM 异步改写） | 2026-08-14 / 2026-08-20 |
| to-spec | SKILL.md | 同上（PRD 术语） | 2026-08-14 |
| claude-handoff | SKILL.md | 同上（PRD 术语） | 2026-08-14 |
| setup-matt-pocock-skills | SKILL.md, issue-tracker-github/gitlab/local.md | 同上（PRD 术语） | 2026-08-14 |
| implement | SKILL.md | 去掉 `disable-model-invocation: true`（本仓要求 implement 进模型目录） | 2026-08-14 |

## 已追平的历史定制（3 处，截至 2026-08 全局同步不再有差异）

| skill | 当时定制 | 现状 |
|---|---|---|
| codebase-design | Agent 工具族本机定制 | 已与上游一致 |
| improve-codebase-architecture | 同上 | 已与上游一致 |
| wizard | disable-model-invocation 定制 | 已与上游一致 |

## 操作纪律

- 上游同步（`setup-matt-pocock-skills`）后必须重新跑一遍本清单的 diff 核验，把「现行」与「已追平」两表更新到最新事实；新钦定的定制同步登记到「现行」表。
- 上游形态参照物（2026-08-06 全局同步副本）现备份于 `~/.agents/skill-conflict-backup-20260818/`（不入仓）。
- 同步来源与流程见 #2026-08-14 roster 记录（`.chanlun/agent-roster-20260814.md`「Matt Pocock skills 更新」行）。

## pstack-lite 适配登记（#1130）

pstack-lite 是对上游 `cursor/plugins`（`pstack/` 目录）的 Prime 适配族，共用一个固定上游
SHA 与「不自动跟随上游 `main`」的同步策略。机器可复现的发现/诊断分类检查（合入门
`unexpectedDiagnostics=0`）见 `scripts/pstack-lite/check-skills.mjs`；触发夹具 runner 见
`scripts/pstack-lite/run-trigger-fixtures.mjs`。

### 登记口径

pstack-lite 每项登记必须记录：

- **Skill 名**：`.agents/skills/<name>/`（或明确写「用户级全局」）。
- **上游仓库 + 固定 SHA**：适配所基于的确切快照。
- **原始 Skill 路径**：上游仓内相对路径（连同 references/scripts 相对文件）。
- **本地改写**：相对上游改了什么、为什么。
- **许可证**：上游许可证（pstack 为 MIT，Copyright (c) 2026 Lauren Tan）。

### 同步策略：不自动跟随上游 `main`

- pstack-lite 所有适配固定到上游 SHA `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`
  （仓库 `cursor/plugins`，`pstack/` 目录）。**不得自动跟随上游 `main`，不设同步机器人。**
- 升级必须：开票 → 比较旧 SHA 与新 SHA 的 diff → 重做 Prime 适配 → 通过
  `check-skills.mjs` + `run-trigger-fixtures.mjs` 靶向验证 → 更新本表与 `skills-lock.json`
  （如适用）。
- 上游删除或改名不影响本地已固定版本；`unslop` 独立升级，不随项目 pstack-lite 更新。

### pstack-lite 索引（首批）

固定上游：`cursor/plugins@fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`，`pstack/` 目录，MIT。

| Skill | 原始 Skill | 上游路径（固定 SHA 下） | 状态 | 本地改写 |
|---|---|---|---|---|
| `pstack-how` | `how` | `pstack/skills/how/SKILL.md`（+`references/`×4） | 已实现（#1131） | 删除 Cursor model slug / 同步 `Task` 返回 / `readonly`；根代理 `rlm` 扇出 + `agent_message`/文件回流，admission handle 不算结果；explainer 收敛到根代理；简单问题 inline |
| `pstack-arena` | `arena` | `pstack/skills/arena/SKILL.md` | 已实现（#1132） | 调用契约 `both`（删 `disable-model-invocation`）；Cursor Task 后台改 RLM 异步 admission + 消息/文件回流；模型池与 Cursor slug 改 `rlm.find_models(...)`；只读候选走 RLM、写码候选走 Sandcastle 隔离 + 人工合入闸；按默认递归深度 1 做扁平扇出（Pick/Graft/Verify 归根代理）；其余流程原样保留 |
| `pstack-blast-radius` | `blast-radius` | `pstack/skills/blast-radius/SKILL.md` | 已实现（#1133） | 去 `disable-model-invocation` 改 `both` 契约；`why`/`arena` 引用改 `git`/`gh` 与 `pstack-arena` 转交口径；结构查询接 Serena+codebase-memory，补代码搜索；新增「潜在破坏面」与「实际验证证据」两节分离，grep 零命中不冒充行为证明 |
| `pstack-create-verification` | `create-verification-skill` | `pstack/skills/create-verification-skill/SKILL.md`（+`references/feature-map-example/`×3） | 已实现（#1134） | 命名 pstack-*；移除 `disable-model-invocation` 用 `both`；生成路径 `.agents/skills/verify-<app>/`；接 Prime `skill-creator`；新增授权闸与隔离；维护命令改写。详见 SKILL.md「本地 Prime 改写」节 |
| `pstack-technical-writing` | `technical-writing` | `pstack/skills/technical-writing/SKILL.md` | 已实现（#1135） | 见 `.agents/skills/pstack-technical-writing/SKILL.md`「本地 Prime 改写」；`both` 契约（移除 disable-model-invocation）；正文四层原样保留 |
| `unslop`（用户级全局 `~/.agents/skills/unslop/`） | `unslop` | `pstack/skills/unslop/SKILL.md` | 已安装（#1127） | 上游逐字一致；SKILL.md SHA-256 `181883e539caec8258ec9129e3ba5f133409144a2cbf2aa361158ab94cfc3441` |

### 合并能力（不新建 pstack-* Skill）

- `architect` 的设计片段 → **已并入**（#1136）`.agents/skills/design-an-interface/` 与
  `.agents/skills/codebase-design/`，等价来源说明见两 Skill 根目录的
  `ARCHITECT-MERGE.md`（固定 SHA、借用片段、本地改写；MIT 指向
  `docs/agents/pstack-lite/LICENSE.MIT`）。借用范围：调用方优先、接口草图、
  设计红旗、实施中复核；**不并入**上游 orchestrator 流程
  （how→arena→agree→implement→scrap）与多模型 runner 面板。两 Skill 的
  `description`/触发契约未改，仅新增正文方法并互指「多方案探索 vs 单方案深模块」分工。
- `interrogate` 的独立 reviewer / agreement map / lead judgment → 并入
  `.agents/skills/code-review/` 的可选 adversarial 模式（**已实现 #1137**：SKILL.md
  「两种模式与升级判定」「对抗评审模式」节 + `ADVERSARIAL-MODE.md` /
  `ADVERSARIAL-REVIEWER-PROMPT.md` / `ADVERSARIAL-RUBRIC.md` /
  `ADVERSARIAL-CODE-QUALITY.md` / `ADVERSARIAL-LEAD-JUDGMENT.md`，固定上游 SHA
  `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`，MIT，未创建 `pstack-interrogate`）。

### 不移植 / 复用现役（首批）

- `why` 推迟到第二批；`recall`、`reflect`、`show-me-your-work`、`automate-me` 复用 Prime
  会话恢复 / continual harness / Wayfinder resolution+roster / `skill-creator`，不移植。
- 全局 `unslop` 保持唯一用户级副本；**项目中不得创建同名副本**（项目级会遮蔽用户级全局，
  见 `check-skills.mjs --self-test` 的同名 shadow 断言）。

### 固定 SHA 记录

- 上游仓库：`https://github.com/cursor/plugins`
- 固定 SHA：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`
- 固定范围：`pstack/` 目录；许可证 `pstack/LICENSE`（MIT，Copyright (c) 2026 Lauren Tan，
  SHA-256 `bc957ca6bee02792566a1a028d105e02e247c6e77cf057061674273da77b200e`，
  逐字复制于 `docs/agents/pstack-lite/LICENSE.MIT`）
