# 本地 Skill 定制总表

> 本文件是本仓 Skill 的**本地定制与来源登记总表**（人读索引）：登记哪些 Skill 是对上游的
> 分叉/适配（有本地改写），记录上游来源、固定版本、许可证与本地差异。**纯导入未改写的**
> Skill 由 `skills-lock.json` 记录来源与哈希，不在本表重复登记；本表只登记「改了上游内容」
> 或「新增本地内容」的项。
>
> 机器可复现的发现/零 warning 检查见 `scripts/pstack-lite/check-skills.mjs`。

## 登记口径

每项必须记录：

- **Skill 名**：`.agents/skills/<name>/`（或明确写「用户级全局」）。
- **上游仓库 + 固定 SHA**：适配所基于的确切快照。
- **原始 Skill 路径**：上游仓内相对路径（连同 references/scripts 相对文件）。
- **本地改写**：相对上游改了什么、为什么。
- **许可证**：上游许可证（pstack 为 MIT，Copyright (c) 2026 Lauren Tan）。

## 通用策略：不自动跟随上游 `main`

- pstack-lite 所有适配固定到上游 SHA `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`
  （仓库 `cursor/plugins`，`pstack/` 目录）。**不得自动跟随上游 `main`，不设同步机器人。**
- 升级必须：开票 → 比较旧 SHA 与新 SHA 的 diff → 重做 Prime 适配 → 通过
  `check-skills.mjs` + `run-trigger-fixtures.mjs` 靶向验证 → 更新本表与 `skills-lock.json`
  （如适用）。
- 上游删除或改名不影响本地已固定版本；`unslop` 独立升级，不随项目 pstack-lite 更新。

## pstack-lite 索引（首批）

固定上游：`cursor/plugins@fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`，`pstack/` 目录，MIT。

| Skill | 原始 Skill | 上游路径（固定 SHA 下） | 状态 | 本地改写 |
|---|---|---|---|---|
| `pstack-how` | `how` | `pstack/skills/how/SKILL.md`（+`references/`×4） | 待实现 | 待后续票填写 |
| `pstack-arena` | `arena` | `pstack/skills/arena/SKILL.md` | 待实现 | 待后续票填写 |
| `pstack-blast-radius` | `blast-radius` | `pstack/skills/blast-radius/SKILL.md` | 待实现 | 待后续票填写 |
| `pstack-create-verification` | `create-verification-skill` | `pstack/skills/create-verification-skill/SKILL.md`（+`references/`） | 待实现 | 待后续票填写 |
| `pstack-technical-writing` | `technical-writing` | `pstack/skills/technical-writing/SKILL.md` | 待实现 | 待后续票填写 |
| `unslop`（用户级全局 `~/.agents/skills/unslop/`） | `unslop` | `pstack/skills/unslop/SKILL.md` | 已安装（#1127） | 上游逐字一致；SKILL.md SHA-256 `181883e539caec8258ec9129e3ba5f133409144a2cbf2aa361158ab94cfc3441` |

### 合并能力（不新建 pstack-* Skill）

- `architect` 的设计片段 → 并入 `.agents/skills/design-an-interface/` 与
  `.agents/skills/codebase-design/`（保留等价来源说明）。
- `interrogate` 的独立 reviewer / agreement map / lead judgment → 并入
  `.agents/skills/code-review/` 的可选 adversarial 模式。

### 不移植 / 复用现役（首批）

- `why` 推迟到第二批；`recall`、`reflect`、`show-me-your-work`、`automate-me` 复用 Prime
  会话恢复 / continual harness / Wayfinder resolution+roster / `skill-creator`，不移植。
- 全局 `unslop` 保持唯一用户级副本；**项目中不得创建同名副本**（项目级会遮蔽用户级全局，
  见 `check-skills.mjs --self-test` 的同名 shadow 断言）。

## 固定 SHA 记录

- 上游仓库：`https://github.com/cursor/plugins`
- 固定 SHA：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`
- 固定范围：`pstack/` 目录；许可证 `pstack/LICENSE`（MIT，Copyright (c) 2026 Lauren Tan，
  SHA-256 `bc957ca6bee02792566a1a028d105e02e247c6e77cf057061674273da77b200e`，
  逐字复制于 `docs/agents/pstack-lite/LICENSE.MIT`）

## 存量定制条目

> 主机工作副本另有 9 条存量定制登记（见 #1121 决议「当前清单只承认 9 项定制」），尚未纳入
> 版本控制。合入本文件时须与主机那份逐条对账，避免覆盖。当前只登记 pstack-lite 新增面。
