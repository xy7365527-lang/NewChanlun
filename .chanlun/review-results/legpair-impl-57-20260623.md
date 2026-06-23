# #57 reading-B 腿模型重写为多空双开（LegPair）—— 审计 + 设计 + 实装 rationale

topo_address: L0.53.1 | parent_callback: legpair-impl → main | 任务节点（#53 子 DAG）
认识论等级标注：审计=L0（读码事实）；设计=L0（架构推导）；CL bull 验收=L2（单标的真实数据）。

---

## 第一部分：当前 reading-B 腿模型审计（L0，读码firsthand验证 #53 selfsimilar-impl 结论）

### 结论
当前 reading-B 腿模型（`enable_reading_b`，rec_engine.rs `consume_legs`/`g(k)`/`Leg`）**绑 d_top 走势完成/背驰链，不绑买卖点+反向买卖点+否定线**。#53 审计结论firsthand坐实。需结构性重写（MAJOR）。

### 逐项坐实（代码位置）
| 维度 | 当前实现 | 代码位置 | 目标 |
|------|---------|---------|------|
| 腿数量 | 每级别**单** `Leg`（`legs[k]`），L↔S 翻转 | rec_engine.rs:302-326,445 | 每级别**一对** LegPair（多腿+空腿共存） |
| 开（open） | `dir_to_polarity(node.direction)`几何方向 | rec_engine.rs:1140-1145 (`g` ride 分支) | 该级别**买点**(区间套定位)→多腿 / **卖点**→空腿 |
| 平（close/switch） | `d_top[k]` 区间套链贯通真顶/真底（走势完成链 OR 背驰段链） | rec_engine.rs:1124-1139 (`g` switch 分支) | **反向买卖点**(区间套定位，对称) |
| 止损 | **仅** NAV≤0 账户级强平 | rec_engine.rs:1333-1345 | **否定线**(中枢边界 ZG/ZD，回试回中枢=假突破) |
| churn 门控 | **无**；d_top 链**贯通**(complete)即翻 | `g` 无门控 + divergence.rs:539 `nest_chain_complete` | 链**破坏**(type3 突破中枢+回试不回)=转折→动核心 / 链**完整**=回调→不动核心 |

**关键概念发现（churn 触发器语义反转）**：当前 `d_top` 在区间套链**贯通**（`nest_chain_complete`=true，背驰/完成嵌套钻取到 a0）时 fire → 翻腿。目标 churn 门控用区间套链**破坏**（type3=突破中枢+回试不回=嵌套断裂）→ 动核心。二者不只是极性反转，而是**不同原语**：背驰链贯通（divergence.rs d_top）≠ 突破中枢回试不回（operator.rs detect_type3）。需新建 type3 链破坏检测，与 d_top 完成链正交。

### 可复用积木（firsthand坐实存在）
- `operator.rs:44 detect_type3`：中枢离开+回抽不破 ZG/ZD = Type3Buy/Type3Sell（携 bar/price/level）= **链破坏原语已实装**。
- `Zhongshu{high,low,gg,dd}`（types.rs）：ZG/ZD = **否定线原料**。
- `LevelView.buy/sell/t1buy/t1sell[k]`：rec_stream.rs:407-419 从 fresh BSP 填（buy: bk 偶；sell: bk 奇；t1*: type1 子集）。reading-B 当前**不消费**（`g` 只读 nodes + d_top）。type3 子集当前**未 surface** 到 LevelView（需新增 t3buy/t3sell）。
- `divergence::nest_chain_complete`/`divergence_window`/`select_child_in_window`：区间套链机器（**保留**，repurpose 为买卖点区间套定位；删的是 d_top fn，非整个链机器——但删 d_top 后这些函数若无其它消费者将变 dead，实装时一并处理）。
- `prove_leg_isolation`(rec_engine.rs:330)+`prove_tw_neutral`：守卫，扩展到 LegPair。
- `geom_tower_quota`(rec_engine.rs:1054)：单 free 池几何塔配额。

### bit-exact 契约（坐实）
OFF 基线 = `instances` 路径（route_bsp/sink/recover/flip/enter/ascend）。reading-B 走**独立的** `legs`/`consume_legs` 路径（on_bar fork @ rec_engine.rs:1331 `if self.enable_reading_b`）。重写 reading-B 路径**不触碰** OFF instances 路径 ⇒ `EngineConfig::off()` bit-exact 自动保持（fork else 分支逐字不动）。

---

## 第二部分：子蜂群分解评估（sub-swarm-ceremony，L0）

### 结论：#57 实装层**原子**（不分解为 ≥2 独立子任务），扁平退化特例，记录理由。

### 候选子工作独立性分析（团队lead 列 ①LegPair结构 ②开平绑买卖点+区间套 ③否定线止损 ④链破坏churn门控）
| 候选 | 写目标 | 对①的依赖 |
|------|-------|----------|
| ①LegPair 结构 | `LegPair` struct + open/close 原子函数 | —（spine） |
| ②开平+区间套 | `g_pair(k)` open/close 分支 + rec_stream 区间套定位信号 | 硬依赖①（写 LegPair） |
| ③否定线止损 | `g_pair(k)` stop 分支 + LevelView ZG/ZD 注入 | 硬依赖①（写 LegPair） |
| ④churn门控 | `g_pair(k)` 核心移动 gate（包裹 open/close 决策） | 硬依赖①（gate 同一 decision fn） |

### 原子性理由（274号终止条件1 + 275号局部依赖 + no-workaround）
②③④ 全部**写同一 struct（LegPair）+ 编辑同一算子函数体（g_pair(k) 的交错分支）**：
- ②③④ 对①是**硬写-写依赖**（struct 必须先存在）；
- ②③④ 不是带 clean interface 的独立模块，而是**同一 decision 函数的交错分支**（open/close/stop/gate 互相条件嵌套）；
- 并行 4 个文件改 agent 会在**同一 `g_pair(k)` 函数体 + 同一 struct** 上冲突，且各自产出**不可组合的半成品**（partial g_pair 不编译/不闭合守恒）——违反 no-workaround（半成品）+ 局部依赖（每节点须 build on 前置）。

clean-interface 分解需先完成 g_pair 重写设计——而那正是核心工作本身 ⇒ 分解前置=premature。

### 四类节点已由父 #53 DAG 提供（不在 #57 重复布设）
- 任务节点 = #57（本节点）
- 审查节点（约束3 异工位）= #59
- 异质审计节点（约束4 codex-challenger）= #60
- 结晶节点（约束1a/1b）= #61
- L3 验证 = #58

⇒ sub-swarm-ceremony 五特征 DAG 在 **#53 层已满足**；#57 是其中的原子任务节点。无需在 #57 层再造子 DAG（否则=重复四类节点）。

---

## 第三部分：目标架构设计（待实装，下文随实装更新）

（实装进行中——见后续 commit 与本文件更新）
