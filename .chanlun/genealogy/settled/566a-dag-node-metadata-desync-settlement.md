---
id: "566a"
number: 566a
title: "dag.yaml nodes 段 567-575 元数据 desync 回溯结算 + 571/572 ID 对调修复 + TBD 孤儿清理——568号(结算≠认知更新)的索引层实例"
type: "meta-rule"
status: "已结算"   # genealogist 观测层结算（2026-06-26）；执行闭环（2026-06-30，commit c65aaf62d0，#56审查PASS）：dag nodes 段 567-575 + 566a 节点已对齐 settled/ frontmatter，TBD 孤儿已清，meta.json last_mapped→565（impl-566a-2 有消费者，同步更新非删）。规格附件 pending/566a-dag-edit-spec.md 随执行兑现移 settled 归档。
date: "2026-06-26"
settled_date: "2026-06-26"
execution_closed_date: "2026-06-30"   # impl-566a-1/2 执行兑现（commit c65aaf62d0），#56 约束3 审查 PASS（纯工程 audit_exempt）
observer: "genealogist (谱系维护工位, scan-backlog-clearing session)"
source: "[新缠论:元编排层]"
negation_source: genealogist   # 谱系维护扫描发现 dag nodes 段索引与 settled/ frontmatter 权威层不一致
negation_form: expansion   # 568 号(结算≠认知更新)在 session 层显形；本号是其在 dag.yaml 索引持久化层的同构显形——结算事件(git-move pending→settled + 编排者批准)未传播到 dag nodes 段
depends_on: ["568", "575", "559", "094"]
related: ["566", "567", "569", "570", "571", "572", "573", "574", "012", "549", "550"]
topo_effect: "无 negates（纯元数据对齐，非否定）；dag nodes 段 567-575 八条目 status/file/id 修正为与 settled/ frontmatter 一致"
---

# 566a：dag.yaml nodes 段 567-575 元数据 desync 回溯结算

## 状态
已结算（观测层 2026-06-26 坐实规格；执行层 2026-06-30 闭环——commit c65aaf62d0 兑现 dag.yaml + meta.json 修复，#56 约束3 审查 PASS）

## 类型
meta-rule（元编排层 / 谱系索引一致性）

## 来源
[新缠论:元编排层] — genealogist 在 scan 暴露的谱系维护积压清理中发现

## 推导链

1. **scan 报告**前提：`last_mapped=530, total_settled=544`，要求把 566-575 映射进 block-topology。
2. **地面真相核实**（meta.json）：`id_mapping` 实际已连续映射至 **565**；字段 `last_mapped_genealogy: 530` 是**陈旧值**，未随 531-565 的映射动作更新。真正缺映射的是 566-575（与 scan 编号一致，但 last_mapped 的字段值已失真）。
3. **settled/ frontmatter 权威核实**：566-575 共 10 个文件**全部已在 `settled/` 目录**，frontmatter `status: 已结算`，`id` 字段全部正确。frontmatter 注释记录"git-move 完成（Lead 2026-06-23，编排者'你结算即可'批准）"——文件已结算并经编排者批准。
4. **dag.yaml nodes 段失真诊断**：nodes 段 567-575 条目停留在**结算前的旧状态**——多条 `status: 生成态` + `file: pending/...`，与已结算的 settled/ frontmatter 不一致。
5. **结论**：这是 **568号谱系（谱系结算 ≠ 蜂群认知更新）在 dag.yaml 索引持久化层的同构实例**。568 描述 session 内 Lead 未实时摄入结算；本号描述结算事件未传播到 dag.yaml 静态索引。同属 094 号（声明-能力缺口的系统性）又一实例。

## 编号异常的精确清单（dag nodes 段 vs settled/ frontmatter 权威）

| dag 节点 id（修复前） | dag status | dag file | 权威真相（settled frontmatter） | 修复动作 |
|---|---|---|---|---|
| 566 | 已结算 ✓ | settled/566-... ✓ | 一致 | 无需改 |
| 567 | 生成态 ✗ | pending/567-... ✗ | id 567, status 已结算, settled/567-frozen-short-leg-...md | status→已结算; file→settled/ |
| 568 | 生成态 ✗ | pending/meta-rule-settlement-cognition-desync.md ✗ | id 568, status 已结算, settled/568-meta-rule-settlement-cognition-desync.md | status→已结算; file→settled/568-... |
| 569 | 生成态 ✗ | pending/meta-rule-role-boundary-collapse-rlhf-attractor.md ✗ | id 569, status 已结算, settled/569-... | status→已结算; file→settled/569-... |
| 570 | 生成态 ✗ | pending/meta-rule-proxy-...md ✗ | id 570, status 已结算, settled/570-... | status→已结算; file→settled/570-... |
| **TBD（孤儿）** | 生成态 | pending/meta-rule-stopguard-...md | 无独立权威——是 571 碰撞期临时占位 | **删除孤儿节点** |
| **571（标题挂反）** | 生成态 ✗ | pending/571-close-short-...md ✗ | settled/571 frontmatter = **stopguard-block-target**（非 close-short） | title→stopguard; status→已结算; file→settled/571-meta-rule-stopguard-... |
| **572（标题挂反）** | 生成态 ✗ | pending/meta-rule-stopguard-...md ✗ | settled/572 frontmatter = **close-short trinity**（非 stopguard） | title→close-short trinity; status→已结算; file→settled/572-close-short-... |
| 573 | 已结算 ✓ | pending/meta-rule-agent-process-...md ✗ | id 573, settled/573-agent-process-...md | file→settled/573-... |
| 574 | 已结算 ✓ | settled/574-... ✓ | 一致 | 无需改 |
| 575 | 已结算 ✓ | pending/574-clean-slate-...md ✗ | id 575, settled/575-clean-slate-...md | file→settled/575-clean-slate-... |

### 571/572 ID 对调的权威依据

settled/572-close-short-...md frontmatter `collision_note`：
> 571 号曾与并发 genealogist 实例（gen-1/gen-2 两代共存，见 573）二次碰撞。**最终收敛（#74 Lead 权威=Option A，autocompact-fix 工位仲裁 2026-06-23）：stopguard meta-rule=571，trinity=572，process-population=573。**

dag nodes 段是收敛**前**的快照：id 571 错挂 trinity 标题、id 572 错挂 stopguard 标题，且残留 `id: TBD` 的 stopguard 临时占位条目。这正是 **575 号（clean slate 重建消除双代共存的两类后果）** 描述的"命名空间写争用"在索引层的残留物。

## 张力检查（§张力检查）

检查范围 = 本轮涉及谱系（566-575）∪ 显式邻接（568/575/559/094/549/550）∪ Hub（012）。

- **与 568 号**：本号是 568 的索引层实例，**非矛盾——同构强化**。568（结算≠认知更新）+ 566a（索引层同实例）= 同一现象的两个持久化层（session 层 / dag.yaml 静态层）。无不可分层矛盾。
- **与 575 号**：575 描述双代共存争用的两类后果；TBD 孤儿节点 + 571/572 对调是该争用在 dag 索引的**残留显形**。本号是 575 的清理收尾，无矛盾。
- **与 559 号（ceremony-scan-completeness skill）**：559 已覆盖"scan 状态不可观测"；本号的 `last_mapped_genealogy: 530` 陈旧字段是 559 应消费但未消费列表的潜在新条目（见下游推论 impl-566a-2）。需 559 skill 三问检测，非矛盾。
- **结论**：无概念分离信号，无需 /escalate 中断 #1。纯索引层元数据对齐。

## 边界条件

- 若 settled/ frontmatter 与 dag nodes 段**两者皆错**（而非仅 dag 段失真），则不能单方面以 frontmatter 为权威——需回溯 git-move 历史与编排者批准记录裁决。**本号已核实 frontmatter 含编排者批准注释（2026-06-23），frontmatter 为权威成立。**
- 若 571/572 的最终收敛在 #74 之后又被覆盖，则 ID 对调方向需重新核实。**当前 collision_note 是最新权威（#74 Lead Option A）。**

## 下游推论（执行闭环 2026-06-30）

- **impl-566a-1**（行动）：dag.yaml nodes 段 567-575 八条目按"编号异常清单"修正 + 删 TBD 孤儿节点。settlement_status: **closed**（commit c65aaf62d0 兑现；genealogist 复核 dag.yaml nodes 段 2962-3020：567-575 全 `status: 已结算` + `file: settled/`，566a 节点存在，无 `id: TBD` 孤儿；#56 约束3 审查 PASS 纯工程 audit_exempt）
- **impl-566a-2**（选择→559 skill）：`meta.json.last_mapped_genealogy` 字段失真（=530，实际 id_mapping 到 565）。三问检测结论=**有消费者，同步更新**。settlement_status: **closed**（commit c65aaf62d0 已将 meta.json line 552 更新为 `565`；genealogist 复核坐实。非删字段=有消费者，排除 090 声明膨胀）
- **impl-566a-3**（行动→映射管线）：566-575 入 block-topology 的实际映射动作依赖 `map_genealogy_to_blocks.py`，但 **549 号已结算该管线 freeze**（relations.jsonl 沦为未实体化 LFS 指针，须操作者 `git lfs pull` 解冻，settlement_status: pending_infra）。566-575 的区块创建在管线解冻前**结构性阻塞**——与 549 的 freeze 一致，非本号可解。settlement_status: **blocked_by_549_freeze（非本号有效域，维持 549 边界）**

## 影响声明

- **改动**：dag.yaml nodes 段 567-575（commit c65aaf62d0 已执行）；meta.json last_mapped→565（已执行）；新增本谱系记录 + 执行闭环更新；规格附件 `566a-dag-edit-spec.md` 随兑现移 settled 归档。
- **影响模块**：`.chanlun/genealogy/dag.yaml`（索引层，已修）；`.chanlun/block-topology/meta.json`（last_mapped 字段，已修）。
- **不影响**：settled/ frontmatter（已正确，本号以其为权威）；block-topology 区块库（映射被 549 freeze 阻塞，非本号范围）；597-621 概念谱系（本轮形式化簇 own，未碰）。
