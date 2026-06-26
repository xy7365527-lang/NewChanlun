# 督导回路（Supervision Loop）

> 编排者裁定（2026-06-26）：「teach 里面的问题也要反过来回到你那里去，他相当于督导。」

## 这是什么

teach 每一课结尾的「**带走的锋利问题**」不只是给编排者学习用——它**反向施加于蜂群自身的产出**，成为一道**证伪闸**：任何工位声明「我把 X 忠实做完了」，必须先用对应的锋利问题盘问自己，过不了就是未完成（且若声明了已完成 = 声明膨胀，090号）。

教学方法 = 督导工具。lesson 把「识破声明膨胀」的方法结晶下来；本回路把这些方法**作用回**蜂群的代码与裁决。它与 no-patch / no-workaround / formalization-validity-domain / quality-guard 同向，是它们的**可操作锋刃**。

## 常设规则

1. **每出一课 → 本台账追加一行**：登记该课的锋利问题。
2. **每条「已忠实完成」声明 → 必须先过对应锋利问题**：工位在报「done/faithful」前，自答台账里所有相关锋利问题，把见证（witness）随报告附上。
3. **过不了 = 未完成**：不写 workaround，不声明膨胀。失败即开/挂一个被追踪的缺口任务。
4. **L 等级强制**：每个「过了」必须标 L0（类型/骨架）/ L1（缠论规则真编码）/ L2（真实数据撑）——只过 L0 不等于忠实。

## 三层识破法（lesson 0002 §3，督导通用模板）

对任何「我把 X 忠实实现了」：
1. **反退化见证**：给一个**具体例子**证明它没退化（给不出 = 退化/聚合）。
2. **哪一层真理**：L0 骨架（类型/函数自动成立）/ L1（缠论规则真编码）/ L2（真实数据）？
3. **边界条件**：什么情况下结论翻——翻了就是没覆盖那个情况。

## 活台账

| 课 | 锋利问题（摘要） | 施加于（蜂群产出） | 裁决 | 产出缺口/任务 |
|----|------------------|---------------------|------|----------------|
| 0001 | classify 的纤维**恰好**是行为等价类吗？↔ 两个方向都兑现？是 L0 schema 还是 L2 撑？ | codex `Origin/CompleteClassification.CompleteClassifier` | **FAIL** — ↔ 是结构字段假设，全仓从未对具体缠论分类器构造/兑现；`BehEquiv` 在 TrendCompleteClassification 一次未现；真实分类器只证 L0 函数平凡；后向（行为极小性）很可能为假。纯 L0 标签，零 L2。 | **#91** |
| 0002 | 给我一个**输出上级序列长度 ≥ 2** 的具体例子 + 每个上级的 **≥3 个不同下级 witness**。 | cc-foundation-faithful 的 `composeStep`（#87 ChanlunInstantiation §1） | **FAIL** — `composeStep xs := if len≥3 then [compose xs]`，输出长度永远 ≤1（单窗口聚合摘要）；只证单 window soundness，未证 `MovesComposedFrom` 全约束（∀uppers WellFormed + upperMoves.length*3 ≤ lowerMoves.length）。曾声明「忠实完成」= 声明膨胀。 | **#89** |
| 0004a | 「这是自动的」——平台真发那个事件吗（查 `platform_support`：true 还是 false/partial）？触发 hook 是 GUARD（echo systemMessage 就 exit）还是 LAUNCHER（真 spawn 工位）？若 ≠true 且 GUARD，则不是自动的，靠某 Lead/人手动认领兜底——说出兜底人是谁、漏了会怎样。 | **通用督导工具**（施加于任何蜂群「X 是自动/事件驱动」声明，含 Lead 自身的「结构工位已自动起」声明） | **可施加（物证已立）** — `.chanlun/dispatch-dag.yaml` 31× `platform_support` 全 false/partial（true=0）；`.claude/hooks/` 30 脚本全 `-guard/-verify/-prompt/-dispatcher`，无 LAUNCHER（`post-write-edit-dispatcher.sh` 终点 `systemMessage`+`exit 0`，不 spawn）；DAG 自承 `:285` 手动认领（D策略）。真相：自动=Lead 手动解释 DAG（033号），Lead 欠执行=唯一承重点。施加判据：报「结构工位事件驱动已生效」须指出该路径的 platform_support=true 或一个真 LAUNCHER；否则诚实标「D策略手动认领」。 | spec-execution-gap 活教材（声明=自动/实际=手动）；谱系 082/073a/412 |
| 0004 | 「S_Θ 闭环已装配/已实装」——给我一个**已实例化的 `HybridComponents`**，其 `transition` 真改 ledger（Dynamics.δ + Accounting 接进去）。给不出 = 闭环是空环，L2 形式落点悬空。两条增量除 `rfl` 外是否只剩「类型签名强制 intent 读 Class + T 写回完整态」这一条？ | `Strict/HybridStep.lean`（#86 骨架 / #88 重基 pending / #94 引擎实装 in_progress） | **待施加**（骨架 PASS-L0：编译通过零错误，`policy_factors_through_classify:164`+`hybridStep_is_closed_transition:183` 把 Chain 并列合取 `∧` 升级为闭环复合 `∘`，约束前移到接口签名 `intent:HybridState→Class→Intent:84`；但 `transition` 仍是抽象字段，`:264` 自承 adapter 实例化「后续」——闭环实例化=未完成，#88/#94 在途）。施加判据：任何报 #88/#94 done 须附已实例化 transition 改 ledger 的 witness。 | **#88 / #94（追踪中）** |

> 注：#89 经 codex xhigh 78k 裁决为 A（可忠实焊接，非结构矛盾，pattern 已知），但仍**未实装**——督导裁决「FAIL（未完成）」成立，去风险≠已完成。

### 裁决记录（PASS）

- **#91（2026-06-26，cc-gap91-classlimits，方向B）→ RESOLVED-by-honest-downgrade**：过督导闸——给出可判定反例（`BSPLabels.x_2bOnly` vs `x_2b3b` 语义不同，canonical 分类器 `Strict.BSP.IGlobal` 判同类 type2，`chanlunTrace x := x.leftCenter` 区分二者行为，by decide）。**没硬凑「↔ 成立」，而是机器证明 ↔ 对缠论非平凡 trace 不可满足（9 定理，`Foundation/CompleteClassificationLimits.lean`）+ 把 `CompleteMinimalClassification` 降级出 canonical**。L0 + 谱系物证（615/231），零 L2。`lake build` 53 jobs 绿（15 def/theorem，#print axioms 仅 propext）。这是「诚实降级 > 虚假兑现」的范例：缺口的正确解可以是「证明它不可满足并诚实标注」，不是「假装满足」。补强：`both_directions_fail` 证 ↔ 双向皆假（缠论分类与行为**正交**），完整否定 lesson 0001「两方向都兑现?」=否。

### 督导对 Lead 生效（双向回路物证）

回路不只盘问工位，也盘问 Lead。已记录两次 Lead 声明被工位督导纠正：
- **「方向A 丢弃 4 个机器可检验实例化」**（cc-direction-decide 驳）：ChanlunInstantiation.lean 两版相同，A 不丢实例化；A 真实代价=重映射 + 丢 theta_v0 Lean-Rust 对齐。
- **「codex 版 BehEquiv 双重悬空（两 namespace）→ 当前仓库」**（cc-gap91 驳）：双重定义仅是 codex from-origin worktree 属性；当前主仓库只有一个 `def BehEquiv`（HybridStateMachine:32）。

两次都是「声明与实际不一致」被即时纠正（090号）。督导回路的价值在于它对**所有**蜂群产出生效，含 Lead 自身——这正是编排者「他相当于督导」的本意。

- **#89（2026-06-26，cc-gap89-recursion，方向B）→ PASS（窗口化递归忠实实装）**：lesson 0002 的锋利问题（「给我长度≥2 + 每上级≥3 不同 witness」）被**机器见证**回答——`xs9`（9 个不同 segment）→ `composeStep` 产 **3 个**上级（`xs9_multi_upper`，rfl）+ 每上级 3 个**不同**下级（`xs9_each_upper_three_distinct_subs`，injection+omega）+ `MovesComposedFrom` 全约束（3*3≤9）+ 真派生中枢（非 `canG=[]`）。从「单窗口 length≤1 聚合退化」升级为窗口化多上级序列，54 jobs 绿零 sorry。对比上轮 cc-foundation-faithful 的 FAIL（`[compose xs]` 单窗口聚合）——同一锋利问题，这次给出了见证。**督导闸把「去风险（裁决A）」与「已实装（机器见证）」分开，逼出了真实装。**

## 后续

每新增一课，在此追加一行；缺口入 TaskList 追踪。本回路的元模式（教学方法结晶 → 反向督导）可由 meta-observer 适时上升为蜂群常设结构。
