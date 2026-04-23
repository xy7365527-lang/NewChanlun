# 24 条 tensions_with 边审查报告

- **topo_address**: v70-swarm/tensions-scan-24
- **工位**: P3 — 谱系张力审查
- **审计日期**: 2026-04-24
- **审计边界**: 只读审查；不修改任何 .chanlun 文件

---

## 1 数据来源与"24"的由来

ceremony_scan.py 第 1608-1616 行的 `tensions_found` 计数是**谱系文件数**（不是边数）：对 `.chanlun/genealogy/settled/*.md` frontmatter 扫描 `tensions_with:` 非空即计 1。复现得 24 个文件。

实际登记的**张力边**分布在两处：

| 来源 | 数量 | 覆盖状态 |
|---|---|---|
| `.chanlun/tension-audit.yaml`（主登记表） | 15 条边，14 份谱系 | 已分类（v160-swarm 审计） |
| 新增谱系 frontmatter（2026-03-11 之后） | 9 条边（10 份文件，含 132 号文档示例） | **未入主登记表**——审计缺口 |

24 份文件的分布：
- **14 份**在 tension-audit.yaml 中已审计（007/008/009/012/013/019d/062/064/065/139/166/167/168/169）
- **1 份**（132）frontmatter 的 tensions_with 是文档示例，不是真实边
- **9 份**（416/424/425/428/429/434/437/438/476）是 2026-03-11 至 2026-03-17 新增谱系，frontmatter 已带 tensions_with，但从未注册入 tension-audit.yaml

## 2 三项审查结论（按 24 文件分类）

### 2.1 已审计且 resolved（9 条边，7 份文件）

| # | from | to | 状态 | 审计来源 | 仍有效？ | 建议行动 |
|---|---|---|---|---|---|---|
| 1 | 007 | 008 | resolved by 010 | tension-audit.yaml | 否（已消化） | 保留历史记录，无行动 |
| 2 | 007 | 009 | resolved by 010 | tension-audit.yaml | 否 | 保留历史记录，无行动 |
| 3 | 008 | 009 | resolved by 010 | tension-audit.yaml | 否 | 保留历史记录，无行动 |
| 4 | 012 | 013 | resolved by 020 | tension-audit.yaml | 否 | 保留历史记录，无行动 |
| 5 | 012 | 019d | resolved by 019d（自结算） | tension-audit.yaml | 否 | 保留历史记录，无行动 |
| 6 | 013 | 012 | resolved by 020（反向边） | tension-audit.yaml | 否 | 保留历史记录，无行动 |
| 7 | 166 | 139 | resolved by 166（内部结算） | tension-audit.yaml | 否 | 保留历史记录，无行动 |
| 8 | 167 | (异步自指=Che vuoi) | resolved by 167（内部结算） | tension-audit.yaml | 否 | 保留历史记录，无行动 |
| 9 | 169 | (话语结构判定) | resolved by 169（内部结算） | tension-audit.yaml | 否 | 保留历史记录，无行动 |

### 2.2 已审计且 historical（4 条边，3 份文件）

065 框架被 068 否定，四条关联边随框架失效（132 号 `valid_until` 机制已规范化）。

| # | from | to | 状态 | valid_until | 升级为 sever/split？ | 建议行动 |
|---|---|---|---|---|---|---|
| 10 | 062 | 065 | historical | 068 | 否（框架整体被否定即失效，无需升级边类型） | 保留历史记录 |
| 11 | 064 | 065 | historical | 068 | 否 | 保留历史记录 |
| 12 | 065 | 062 | historical（反向） | 068 | 否 | 保留历史记录 |
| 13 | 065 | 064 | historical（反向） | 068 | 否 | 保留历史记录 |

### 2.3 已审计且 ongoing（2 条边，2 份文件）

| # | from | to | 分类 | 备注 | 仍有效？ | 建议行动 |
|---|---|---|---|---|---|---|
| 14 | 139 | 原则0的ESC权 | ongoing（选择类） | 需编排者价值判断是否接受 ESC/分类权分离 | 是 | **观察中**，不处理；如持续无进展可触发 `/escalate`（选择类） |
| 15 | 168 | alienation前提缺失 | ongoing（选择类） | 168 号提出的"089 扬弃≈alienation→separation"有条件映射，需编排者概念决策 | 是 | **观察中**，不处理；同上 |

### 2.4 新增未审计（9 条边，9 份文件）——**审计缺口**

这些边在 frontmatter 登记但未入 tension-audit.yaml。下表为本次审查的初步分类（仅依据谱系文件内容，不等同于主登记表的正式分类）：

| # | 源谱系 | 目标 | frontmatter 建议分类 | 判定依据 | 建议行动 |
|---|---|---|---|---|---|
| 16 | 416 | operator-insight: "谱系写入本身就是穿越" | structural（非对称：对象不是谱系号） | 编排者洞察与 416 号的穿越/写入关系张力，无对向谱系 ID | 语法记录候选——下次 topology-manager 需决定是否定义非 ID 目标的张力类型 |
| 17 | 424 | 425 | ongoing（逆向已出现） | 424 号 S_net 持久化实装 ↔ 425 号 S_net 入图问题，双方都 settled | 待 topology-manager 登记为 ongoing（双向） |
| 18 | 425 | 399 | ongoing | 425 号声明共现边对穿越密度贡献为零 ↔ 399 号耦合振荡的声明，双方 settled | 待登记，需判断 459 号审计是否已消化 |
| 19 | 428 | 427 | ongoing | 428 号接力观察 ↔ 427 号未完成，"观察连续性张力" | 待登记；若 427 后续有闭合则可转 resolved |
| 20 | 429 | 425 | ongoing | 声明已实装但溯源未持久化，与 425 入图问题的关联 | 待登记 |
| 21 | 434 | 425 | ongoing | thinking 循环需要足够边密度 ↔ 425 入图问题 | 待登记 |
| 22 | 437 | 425 | ongoing | 回流管道依赖共现边入图机制 ↔ 425 入图问题 | 待登记 |
| 23 | 438 | 实装审计待定 | **resolved by 459** | 438 号 frontmatter 已明文 `status: resolved, resolved_by: '459'` | 待 topology-manager 登记为 resolved，不再是 ongoing |
| 24 | 476 | 178 | ongoing | 178 号声明 block topology 为 primary 存储 ↔ 476 号指出 JSONL 仍是 primary（迁移断裂） | 待登记，与工程实装进展耦合 |

## 3 分类汇总

| 分类 | 数量 | 处置 |
|---|---|---|
| resolved（含主登记与 frontmatter 明文结算） | 9 + 1（#23） = 10 | 保留为历史记录，tension-audit.yaml 需补录 #23 |
| historical（框架否定） | 4 | 保留为历史记录，无行动 |
| ongoing（选择类，需编排者决策） | 2 | 观察中；长期未消化可升级 `/escalate` |
| ongoing（新增未登记，工程/概念未决） | 7（#17 #18 #19 #20 #21 #22 #24） | 待 topology-manager 登记入主表；部分可能已被后续谱系消化但未交叉引用 |
| 语法记录候选（非 ID 目标） | 1（#16） | 需 topology-manager 决定张力目标类型扩展规则 |

## 4 结论（简化版六要素）

### 4.1 结论

- 24 份谱系文件共声明 **15 条主登记边 + 9 条 frontmatter 新边 = 24 条**；**10 resolved / 4 historical / 9 ongoing / 1 语法记录候选**（#23 已被 459 号结算但未回填主表）
- `tension-audit.yaml`（2026-03-05 生成）相对当前谱系规模（截至 484 号）**严重滞后**：9 条新张力边从未纳入审计
- 无发现**未识别的概念分离**（经 CLAUDE.md 逐条核对，所有新增边属于 S_net 生命周期/器官原则/观察连续性等工程-概念耦合领域的已登记张力）
- **无需 `/escalate`**（审计缺口是工程行动类，非概念矛盾类）

### 4.2 边界条件（分类翻转条件）

- **resolved → ongoing** 翻转：若被引用的解决者谱系（如 010/020/459）自身被否定或 `topo_effect` 含 `sever:`，则下游 resolved 状态失效
- **historical → 重新激活** 翻转：若 068 号被再次否定（范式回滚），065 框架相关的 4 条历史边需重新分类
- **ongoing（选择类）→ resolved** 翻转：编排者作出概念决策（139 ESC 权分离 / 168 alienation 映射）
- **ongoing（新登记）→ resolved** 翻转：后续谱系交叉引用并闭合（如 #23 已由 459 号完成，但主表未反映）
- **#16 语法记录**翻转：若 416 号 `operator-insight` 字符串目标被扩展为支持非 ID 张力目标的 schema，则升格为合法边

### 4.3 影响声明

- 本次审查**只产出审查报告**，未改动任何代码、schema、谱系文件、tension-audit.yaml、relations.jsonl
- 产出路径：`.chanlun/pdf-review/tensions-audit-24.md`
- 下游推论（留给 topology-manager 的工程行动，本工位不执行）：
  1. 将 #16–#24 的 9 条新边补录入 `tension-audit.yaml`，保持单一事实源
  2. 对 #23（438↔459）回填 resolved 分类
  3. 对 #17–#22 #24 六条 ongoing，在下一轮审计中查验是否已被 440/442/458/459/460 等后续谱系消化（交叉引用检查）
  4. `scan_ongoing_tensions` 当前仅读 tension-audit.yaml，将错过 9 条新边的 ongoing 状态——**这是工程层缺口**：主登记表未回填导致 ceremony_scan 的 ongoing 扫描遗漏
