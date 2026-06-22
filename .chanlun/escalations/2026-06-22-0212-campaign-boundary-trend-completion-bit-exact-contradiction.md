---
id: escalation-2026-06-22-0212
title: "campaign 边界重定义（走势完成→reset_campaign）与 rec≡flat bit-exact 约束不可同时满足——死锁修复触及方向决策权归属（546§9.2）"
type: 矛盾上浮
status: 生成态
date: 2026-06-22
负责工位: CC session（死锁修复实装工位）
related_genealogy: ['546', '545', '539', '538']
related_definitions: ['fengkong', 'chanlun-trading-system', 'zoushi']
related_docs: ['docs/recursive_t_architecture_v2.md §9/§11.6']
---

## 矛盾报告

### 矛盾描述

死锁修复工位的任务三约束**两两可满足、三者不可同时满足**，构成一个构成性矛盾
（no-workaround.md 触发）：

1. **修复方向**：campaign 边界从「units 几何衰减归零」重定义为「**走势完成 → 强制
   reset_campaign**」（546 号 §9.1 + filter-spec §11.6）。
2. **对称约束**：flat（`t_engine.rs`）与 rec（`rec_engine.rs`）**对称改**，保持
   **rec≡flat bit-exact**（新行为下两引擎仍逐位一致）。任务明示这**不是**被否定的
   取向 Y（单边改 rec 弃 bit-exact）。
3. **不复活已否定锚 + 不动 EPS**：不得用 emergent_top 作核心方向锚（545 号已否证，
   −1069% 做空陷阱）；不得动 EPS 阈值（filter-spec §九.3，会同时破坏 545 方向锚）。

**为什么不可弥合**：实现「走势完成 → reset_campaign + enter 重建」必须在操作层有一个
「**核心走势完成**」触发器。穷举 flat 与 rec **共享的信号通道**（这是 bit-exact 的必要
条件——触发器只能用两引擎都有的信号），只有三个候选，每个都撞死一条硬约束：

| 候选「走势完成」触发器 | bit-exact 可行？ | 解死锁（BTC 2019+ 不再踏空）？ | 是否复活已否定锚 / 触发 escalate？ |
|---|---|---|---|
| **A. emergent_top 反转 vs 核心方向** | ✅（`types.rs:282-289` 两引擎同函数，逐位一致） | ❌ 单调牛中 emergent_top=最高已完成走势**跨年滞后**（[[project_highest_level_sigma_frozen]] 最高级别σ跨年冻结），BTC 2019+ 牛市恒为 Long，反转**数年不 fire** → 不解死锁 | ❌ 用其方向锚 = 545 复活（−1069%） |
| **B. 核心级反向 BSP（现存 flip 路径）** | ✅（已存在） | ❌ 死锁正因核心 ascend 到塔顶（跨年-罕见层），核心级反向 BSP 几乎不 fire（545 机制）；让**子级**反向 BSP 触发核心 reset = 把回调误判为走势完成 → 复活过度交易 | ⚠ = §11.6-2「promote 条件对称化」= **方向决策权归属**（546§9.2 /escalate） |
| **C. per-level TrendNode/走势完成 标志** | ❌ **flat 的 `TSignalView` 无 node 通道**（仅 buy/sell/emergent_top）；且两引擎**均无** `completed` 标志导出到信号层（`TrendNode` 结构无 completed 字段，completed 留在 tree 内不外露） | （若实装）✅ | ❌ rec-only = **取向 Y**（单边改 rec 弃 bit-exact），任务明令禁止 |

**结论**：在 flat∩rec 共享信号的有效域内，**不存在**一个既 bit-exact 可行、又能在
目标 regime（BTC 2019+ 牛市）真正解死锁、又不复活 545 已否定锚的「走势完成」触发器。
A 无效（跨年冻结）且复活 545；B 落到 §9.2 方向决策权（/escalate）；C 破 bit-exact
（取向 Y）。任务自身的退出条款已预见此情形：「若对称修在 flat 架构上不可行（flat
无法表达走势完成边界），停下走 /escalate 上浮」——**正是 C 撞墙 + A/B 不达标**。

### 双方论证

**立场 A（直接 TDD 修，把「走势完成」实装为操作逻辑）：**
- 依据：546 号 §9.1 登记修复方向「走势完成 → 强制 reset_campaign + enter 重建」；
  编排者已授权解死锁方向。
- 但：该方向**未指定**「走势完成」用哪个信号判定。穷举共享信号（上表）后，每个具体
  触发器都触及概念决策（方向锚归属）或破坏 bit-exact，**无法在不做概念决策的前提下
  落码**。546§9.1 自身标注「不声明有效性…不擅自实装（待 L2 实验闭合）」。

**立场 B（停下 /escalate）：**
- 依据：546 号 §9.2 明示「§11.6 级联判据落码…涉及**方向决策权归属**（emergent_top
  vs 级联 vs route_bsp 核心级 flip 三者的边界），是 `/escalate` 范畴（架构 + 决策权），
  待编排者裁决。生成态。」539 号 §9.5「新开放轴…未实装、未验证，预注册不擅测」、
  §五.2「不擅自实装…须独立授权后落码」。
- 「走势完成」在 fengkong/chanlun-trading-system 中 = **退出条件 = 买入程序的判断
  条件被否定**（生成态定义），把 campaign 边界从「units 归零」改为「走势完成」**改变
  该概念的含义/边界** → 触及定义 → 须 /ritual + 方向决策权裁决，非纯实现逻辑。

立场 A 与 B 的真分歧：**「走势完成」的判定信号是哪个，由谁决定**——这恰是 546§9.2
登记的「方向决策权归属」，不是实现层可自决的细节。

### 涉及的定义

- `.chanlun/definitions/fengkong.md`（生成态）：**退出条件 = 买入程序的判断条件被
  否定**（第13课），与盈亏无关、只与走势结构有关；成本归零三阶段；终点 = 超大级别
  卖点一次性清仓。→ campaign 边界的概念对应物。
- `.chanlun/definitions/chanlun-trading-system.md`（生成态）：同上，交易策略视角。
- `.chanlun/definitions/zoushi.md`：走势类型/走势完成（走势终完美）的级别递归依据。
- 当前引擎的 campaign 边界（`highest_active()==None` via 几何衰减归零）**不是**上述
  定义的任何条目——它是实装捷径，与 fengkong 退出条件脱钩（546§五 separation 否定）。

### 谱系比对结果

- **546 号**（本死锁的结晶记录，settled）：§9.1「解锁 enter 修复方向」= 生成态，
  「不擅自实装（待 L2 实验闭合）」；§9.2「§11.6 落码 = 方向决策权归属 = /escalate
  范畴，待编排者裁决」；§9.3「动 EPS 是 no-patch 禁止的治标」。**本上浮 = 546§9.2
  登记的 /escalate 的实际触发**，非新发现。
- **545 号**（settled）：emergent_top 作核心方向锚已否证（−1069%）；候选 A 复活之。
- **539 号**（settled）：清仓频率是 regime 函数，无一普适（§9.4 四形态确认）；
  「走势完成→reset」的有效域同属此谱型——任一具体触发器是 regime-laden 选择，
  §9.5「不擅自实装…独立授权后落码」。
- **538 号**（settled）：单根账本 + 资金守恒解锁递归——reset_campaign 的 withdrawn
  回流（`free += withdrawn`）依赖此守恒；本修复触及同一会计骨架，须保守恒。
- **历史先例处理方式**：546/539/545 三号一致裁决「登记方向 + 不擅自实装 + 待编排者
  独立授权」。**本次与先例相同**——故按先例 /escalate，不单边实装。

### Lead 的建议方案

死锁是真实的（546§一/§二 L0+L2：BTC n_enters 冻结 4 / 8.5 年，踏空 +1343pp），
**必须解**；但「走势完成」的判定信号是方向决策权问题，建议编排者在以下三条中裁决
（按对 bit-exact / 已否定锚 / 工作量的影响排序）：

1. **【推荐】§11.6-2 promote 条件对称化（级联判据）**：核心整翻只在「**反向走势区间套
   级联涌现到核心级别**」时触发；否则保持有界 1/3 sink 短差（回调/反弹不涌现到顶即
   recover）。这是 546§9.2 + 545§开放轴1 + 539§9.5 共同指向的严格形式。
   - 代价：需在**两引擎共享信号**中表达「反向级联是否到达核心级别」。当前共享信号
     （buy/sell/emergent_top）**不足以**表达级联深度——需新增一个**两引擎都能产出**
     的级联信号通道（否则破 bit-exact）。即：先扩 flat 的 `TSignalView`（加级联/节点
     通道）使其与 rec 对称，再对称落码。这是 §11.6-1「bottom-up reverse-promote」的
     前置工程。有效域 L3 待验证（§11.6 边界条件 a/b/c，不得假设闭合）。
2. **走势完成 = emergent_top 反转**（候选 A，仅作 reset 触发、re-entry 方向交给下个
   BSP）：bit-exact 最省，但**跨年冻结 → 不解 BTC 牛市踏空**，且仍以 emergent_top
   编码方向（545 边缘）。建议**否决**（不达死锁解除目标）。
3. **接受 rec≠flat（取向 Y）**：单边在 rec 用 `nodes` 通道实装走势完成。**编排者已
   否定**，列出仅为完备。建议**否决**。

若编排者选 1，建议同时授权：(i) 把「走势完成」正式纳入 fengkong 退出条件的实装映射
（走 /ritual 广播 campaign 边界 = 退出条件，非 units 归零）；(ii) §11.6 四要件配套
（reverse-promote 聚合 + 守卫镜像 + 接受对称确认滞后税）；(iii) 每步 NT 回测验证
（§9.8 教训，不得假设闭合）。

### 需要决断的问题（领域语言）

**「核心走势完成」（走势终完美，触发 reset_campaign + 重建新走势仓位）应由哪个判据
判定，且该判据如何在 flat 与 rec 两引擎对称表达（保 rec≡flat）？**

具体即三选一 + 一前置：
- (a) 反向走势经**区间套级联涌现到核心级别**（§11.6-2，需先扩 flat 信号通道使其能与
  rec 对称表达级联深度）；
- (b) **emergent_top 反转**（跨年滞后，已知不解牛市踏空，且贴近 545 已否定锚）；
- (c) 其他（编排者从零给出）。

以及：**campaign 边界从「units 几何衰减归零」改为「走势完成」是否确认为 fengkong
退出条件（生成态定义）的含义变更**？若是，应走 /ritual 广播此定义边界，再据裁定的
判据 (a/b/c) 对称落码——而非由实装工位擅自选定判据。

---

## 影响声明

- **未改任何代码、未改任何定义文件**（no-workaround：撞构成性矛盾即停，先上浮）。
- **未 commit**（等 Lead 汇总）。
- 新增本上浮文件 `.chanlun/escalations/2026-06-22-0212-campaign-boundary-trend-completion-bit-exact-contradiction.md`。
- 触及（待裁决后）：`rec_engine.rs`/`t_engine.rs`（route_bsp 核心级 / reset_campaign /
  信号视图 `TSignalView`/`LevelView`）、定义 `fengkong.md`/`chanlun-trading-system.md`
  （campaign 边界 = 退出条件，走 /ritual）、谱系 546§9.2 闭合。

## 认识论等级

- 死锁机制 = **L0**（546 逐字源码 + 本工位复核 rec_engine.rs:45/151/403-405/746-747/503-513）。
- 信号通道不对称（flat 无 node/completed 通道，仅 buy/sell/emergent_top；emergent_top
  两引擎同函数 bit-exact）= **L0**（源码 + 信号构造路径核验 stream.rs / rec_stream.rs /
  rec_driver.rs / types.rs:282-289）。
- emergent_top 跨年冻结 → 候选 A 不解牛市踏空 = **L2**（[[project_highest_level_sigma_frozen]]
  + 546§三 BTC 牛2 +0.1% vs BH +1711%）。
- 「走势完成→reset」有效域 = **生成态**（546§9.1 + 539§9.4 regime 函数，未声明有效性）。

---

## 强化（实装前 Codex 异质对审，2026-06-22 03:53）：选项1 内在 liveness 缺口

> 本节由死锁修复实装工位（topo: session-4130a713/deadlock-cascade-fix）补写。
> 编排者已就本上浮裁决「方案1（选项1 §11.6 级联判据）」。落码前按编排者要求做
> Codex（gpt-5.5 / reasoning=xhigh）实装前方案对审，**独立收敛于本上浮的边界条件
> (b)/(c) 并新增一层概念矛盾**：选项1 本身解不了任务目标。完整交互见
> `.chanlun/review-results/codex-decide-20260622-0353.md`。

### 新发现：死锁有两个正交面，选项1 只覆盖其一

| 面 | 死锁机制 | 选项1（反向级联 flip/promote）能解？ |
|---|---|---|
| **Face A 方向锚**（核心在**真转折**翻错/翻早卡死） | emergence_upgrade 缺 flip 态 + emergent_top 跨年滞后做空陷阱（545/§9.8） | **能**（但须用正确判据，见下；机制 R 被否决） |
| **Face B liveness**（**单调牛**核心几何衰减→1e-6 僵尸 + withdrawn 永久锁定 + free 闲置 + `enter` 永不再触发） | `enter` 仅在 `highest_active()==None` 触发（rec_engine.rs:761）；单调牛无 reset_campaign | **不能**（结构上不可达） |

**为什么选项1 不能解 Face B（Codex Q1 = Yes，源码 L0 确证）**：选项1 严格形式只在
「反向走势区间套级联**到核心级别**=真转折」时 full flip→clear_all→reset_campaign。
**单调牛中反向级联永不到顶** ⇒ 不 flip ⇒ 不 reset_campaign ⇒ withdrawn 仍锁、free 仍闲置、
僵尸核心仍 active ⇒ `enter` 永不再触发。而**任务目标恰是「2019+ 牛市重新入场」= Face B**。
⟹ **选项1 与任务目标错配**：选项1 治「真转折翻错」，治不了「单调牛衰减闲置」。

把触发放宽到 cc-1「解冻」= 把牛市回调当核心反转 = 复活 §9.8 −1069% 做空陷阱（Codex Q4）。
⟹ 不能用放宽选项1 的方式凑出 Face B 的解（no-workaround）。

### 推荐机制 R 被 Codex 否决（Q2）

实装工位初拟的机制 R（`t1*[cc-1]` + cc-1 活跃反向短差 → flip）**不成立**：它是「核心紧下方
单层 type1 + 既有短差」的**局部触发器**，不表达级联深度/ancestry/真转折判别，可能读到陈旧短差。
正确判据须**信号层显式** `reverse_spawn_to_core(cc, rev_dir)`（反向结构由底层逐级 fresh spawn
穿过 cc 逻辑级别，携 ancestry）——即本上浮方案1 代价段所述「**先扩 flat∩rec 共享信号通道**
（`cascade_buy_to[]/cascade_sell_to[]`），再对称落码」。读引擎内部 `instances/layers` 仓位状态
**不能**替代结构级联信号（Q5：致命点不是索引偏移，是「路径依赖仓位状态 ≠ 结构级联信号」）。

### A'' trend-done-clear 不应与选项1 并行（Q3，no-patch）

当前工作区未提交的 A''（rec_engine.rs:858 / t_engine.rs:838，early-return clear）缺 §3.5
`emergent_ceiling 新增层/方向/PendingContainer` 判别，不是干净的 branch3，且在 route/promote 前
抢先清仓。若落选项1，A'' 应被**替代**或降级为正式 PendingContainer 的退化 clear fallback，
**不得作为第二套死锁补丁并行存在**（保留两个针对同一死锁的并行机制 = 补丁思维）。

### 正确动作优先级（Q4，若 Face A 落码）

`promote`（严格级联确认后升格既有反向子 T）> `clear`（无可 promote 时）> **绝不**用局部
cc-1 事件直接 full reverse enter（复活做空陷阱）。

### 需编排者**再裁决**的问题（在原裁决「选项1」之上新增）

原裁决（选项1）基于本上浮表格做出，**当时未做实装前 Codex 对审**，故未知选项1 的 Face B 缺口。
现请编排者就以下三选一再裁决：

- **(α)** 接受选项1 只解 Face A，**另开独立方案解 Face B**（liveness）——需指定 Face B 的方向：
  ① 修 recover 欠触发（核心 ascend 到塔顶后次级别买点结构不再产生 same-parent recover，核心只
  sink 不 recover → 衰减；这是「核心卡塔顶」反馈进 liveness）；② campaign 边界周期性归还 withdrawn；
  ③ 放宽 `enter` 条件（不止 `highest_active()==None`）。**每条都触及 settled 会计骨架/方向决策权
  → 须独立授权**（538/539/546）。
- **(β)** 选项1 不是任务目标的解，**重新选择死锁解法**（直面 Face B liveness 为主）。
- **(γ)** 其他（编排者从零给出）。

并请同时裁决：Face A 若要落码，是否授权**扩 flat∩rec 共享信号通道**（`cascade_to[]` + ancestry，
工程量 = §11.6-1 bottom-up reverse-promote 前置），以及 A'' 的去留（替代/降级 fallback）。

### 结果包六要素（本节）

1. **结论**：编排者选定的【选项1 §11.6 级联判据】经实装前 Codex（gpt-5.5/xhigh）异质对审 +
   源码 L0 复核，确认**结构上不能达成任务目标（2019+ 牛市重新入场）**——死锁含正交两面，选项1 只
   覆盖 Face A（真转折方向锚），覆盖不到 Face B（单调牛 liveness）。推荐机制 R 被否决。**停止落码，
   再上浮裁决。**
2. **定义依据**：§11.6-2（核心整翻只在反向走势涌现到核心级别=真转折）；rec_engine.rs:761（`enter`
   仅 `highest_active()==None`）；§9.8（emergent 门控/宏观净空头 −1069% 做空陷阱）；545（emergent_top
   方向锚否定）；本上浮表格（A/B/C 三候选共享信号不足，方案1 需扩信号通道）。
3. **边界条件（结论翻转）**：若 2019-2024 BTC **存在真转折**（2021 顶 / 2022 底），选项1 会在该处
   flip→reset→re-enter，**部分**解冻 n_enters（转折点附近）——但 2019-2021 单调升段的衰减/闲置
   不解。故「选项1 完全解死锁」**伪**，「选项1 在转折点部分解冻」**真**——这是 L3 实测可量化的
   （需回测确认转折点是否触发 + 单调段是否仍闲置），不可假设闭合（§9.8 教训）。
4. **下游推论**：(1) 对**任务目标**——选项1 单独落码 = 把 Face B 缺口留到后面（161号务实/补丁思维），
   不达「滤波器吃到每级别涨跌幅」的前置（中间级别滤波器仍被僵尸核心阻塞）；(2) 对 **bit-exact**——
   正确 Face A 判据须扩共享信号通道（不读引擎 state），扩通道本身可保 flat≡rec（对称扩），只在 flip
   触发时偏离；(3) 对 **P5/l2-verify-script**——本工位**未改引擎**，P5 当前跑的引擎（含 A'' 未提交）
   不需因本工位重跑。
5. **谱系引用**：545（emergent_top 方向锚震荡，本上浮 + 本强化的上游）；539（清仓频率 regime 函数）；
   546（死锁结晶 + §9.2 方向决策权归属）；§9.8（−1069% 做空陷阱）；project_t_short_leg_regime_function
   / project_t_cross_level_coupling_falsified（做空腿/无界空头 = Face A 误触的历史显形）；
   project_highest_level_sigma_frozen（最高级别跨年冻结 = 选项1 在单调段不 fire 的同源）。
6. **影响声明**：未改任何引擎代码 / 定义文件。新增本强化节 + Codex 对审记录
   `.chanlun/review-results/codex-decide-20260622-0353.md`。触及（待裁决后）：rec_engine.rs/t_engine.rs
   （route_bsp/emergence/reset_campaign）、信号层 stream.rs/rec_stream.rs（`cascade_to[]` 通道）、
   A'' 去留、546§9.2 闭合。认识论等级：Face A/B 正交 + 选项1 不覆盖 Face B = **L0**（源码 + Codex
   独立复核）；选项1 在转折点部分解冻 = **L2/L3 待回测**。
