# 递归引擎实装审计 —— 代码 vs 设计文档逐条核验

> 审计日期：2026-06-21
> 审计对象：`rust/src/recursive_t/` 的 `rec_engine.rs` / `rec_stream.rs` / `rec_driver.rs`（+ 关联 `types.rs` / `divergence.rs` / `backtest_run.rs`）
> 审计基准：`docs/bsp_consumption_redesign.md`、`docs/three_phase_unified_design.md`、`docs/recursive_t_architecture_v2.md`、`docs/drain_and_three_stages.md`、`docs/three_stages_accounting_design.md` + `docs/memory_snapshot/` 全部快照 + G1–G5 遗漏检查表
> 方法：11 维度并行审计 agent（读设计文档 + 读代码 + 对照 flat `t_engine.rs`）→ 49 条「不一致」结论各做独立对抗复核 → 主审计员独立地基核验交叉验证。130 条 finding（81 一致 / 28 偏离 / 9 遗漏 / 9 退化 / 3 无法确定）。
> 认识论等级：本报告全部为 **L0**（逐字源码对照 + 设计文档文本对照，零数据依赖）。不涉及回测收益（L2/L3）。

---

## 0. 执行摘要：一个贯穿所有维度的元发现

**当前递归引擎代码不是设计文档的实现，而是 flat 引擎 `t_engine.rs` 的逐行 bit-exact 镜像。** 这是 commit `953d8dfdb3`（2026-06-21「flat 完全递归化」）的产物，`rec_engine.rs:1-29` 头部注释明确声明：

> 「删除所有递归引擎自创的约束（C1 永不翻空 / candidate / 连续 level=父−1 / 方向 gate），逐行对照 flat 重新实装。」

这导致**审计基准本身是分裂的**——5 份设计文档不是一份连贯的规格，而是一条**演化序列，且互相矛盾**：

| 设计文档 | 提出的核心主张 | 当前代码的选择 |
|---|---|---|
| `bsp_consumption_redesign.md` §9.11 (C1) | 核心持多骑牛**永不翻空**，所有卖点都是短差(sink/recover)，无 Z₂ 翻转 | **反转**：恢复 flat 的 flip（整仓翻空） |
| `bsp_consumption_redesign.md` §9.10/9.11 | 嵌套 `TInstance.child` 单链 + 递归归还子树 | **反转**：回退 flat 扁平 `instances[level]` 数组 |
| `three_phase_unified_design.md` | `account_campaign(role)` 拓扑判 role + 方向对称(空头也降成本) + drain 消融 + CarrierModel | **几乎全部未实装**：仍是 flat 的 `account_reduce(dir)`、方向不对称、drain 保留 |
| `recursive_t_architecture_v2.md` §9/§11 | promote / reverse-promote / PENDING_CONTAINER / emergent 门控 / TInstance 骑 TrendNode 禁绝对 ladder | **删除/反转**：flip 取代 promote，绝对 level 数组取代 TrendNode 寻址 |
| `drain_and_three_stages.md` | （分析文档）drain 是 flat 现有合法算子 | **采纳**：drain 保留（与上面 three_phase 的"drain 消融"直接冲突，代码选了保留侧） |

**因此对用户问题「代码是否实装了设计文档写的内容」的诚实回答是双层的：**

- **基础设施层（5 份文档与 flat 共识的部分）= 一致**：sink/recover/drain 齿轮、三阶段会计上移到 TRoot 总体口径、逐仓/全仓强平 by phase、fresh BSP 消费、route_bsp 分层路由、区间套 top-down、单根 free 池 + 每实例隔离、TW 守恒 panic 守卫、标的无特化、H⁰ 2/3 + H¹ 1/3。这些确实落码且对照 t_engine.rs 验证为逻辑等价（非仅注释声称）。
- **"改进 flat"层（3 份文档相对 flat 的增量主张）= 大面积遗漏/退化/偏离**：C1 永不翻空、嵌套 child 链、方向对称三阶段、account_campaign 拓扑接口、promote/PENDING_CONTAINER、TInstance 骑节点、drain 消融、CarrierModel —— 这些被后续「复现 flat」裁决有意废弃或从未落码。

这**不是静默篡改**——`rec_engine.rs:1-29` 头注释、commit message、单测名（`核心级卖点_enter_short_无c1`）都把"删 C1、复现 flat"记录在案。但它意味着：**这三份设计文档的文本与当前代码语义已系统性脱节，文档未随代码同步更新。**

> ⚠️ **审计期并发编辑警告**：`types.rs::emergent_top` 在本次审计窗口内被并发会话**改了至少一次**（详见 §4.2）。所有涉及 emergent_top 的结论为审计时刻（2026-06-21）工作区快照，是移动靶。

---

## 1. 总览统计

| 分类 | 数量 | 含义 |
|---|---:|---|
| **一致** | 81 | 设计点确实实装（且对照 flat 验证等价） |
| **偏离** | 28 | 实装了但跟设计语义不一致（方向/范围/键/阈值不符） |
| **遗漏** | 9 | 设计点完全没实装 |
| **退化** | 9 | 曾按某版设计实装，后被「复现 flat」裁决改回/绕过 |
| **无法确定** | 3 | （复核后 2 项归入一致、1 项归入遗漏） |

按维度：

| 维度 | 一致 | 不一致 | 主要不一致项 |
|---|---:|---:|---|
| BSP 消费重构 | 5 | 7 | C1 翻空、嵌套 child、删 gate、type2/3 区分 |
| 三阶段统一设计 | 4 | 10 | account_campaign、方向对称、drain 消融、CarrierModel |
| 递归架构 v2(结构) | 4 | 5 | TInstance 骑节点、A2 方向、情况一退出门 |
| 递归架构 v2(父子) | 9 | 5 | promote、PENDING_CONTAINER、节点身份键 |
| drain+三阶段 | 11 | 4 | 文档 §4.4 自身偏离、诊断计数退化 |
| 三阶段会计设计 | 17 | 0 | （会计核心全一致，= flat 逐行镜像） |
| G1–G5 检查表 | 4 | 6 | G1/G2 遗漏、G5 偏离 |
| memory 约束(组1) | 6 | 4 | 净额翻转、操作点↓域↑、永远在市场 |
| memory 约束(组2) | 7 | 4 | 必然性 prove 缺失、方向对称、相邻级别守卫 |
| 逐仓强平 rec vs flat | 8 | 2 | （仅诊断层差异，会计/阈值全一致） |
| emergence_top 语义 | 6 | 2 | 判据演化、与文档不符 |

---

## 2. 五份设计文档逐份核验

### 2.1 `bsp_consumption_redesign.md` —— BSP 消费重构

**一致（已正确实装）：**
- **fresh BSP 消费**（§9.8）：`rec_stream.rs:90,246-269` 用 `bsp_seen: HashSet<(bar,level,kind)>` 跨 bar 去重，只消费本次重跑新 fire 的 BSP，非 `all_bsps()` 全历史累积。修复 commit `c650f289b7` 的"机械触发根因"。✅
- **buy/sell 含 type1/2/3 全 6 类**（§1.1）：`rec_stream.rs:261-268` 按 `bk%2` 映射，偶→buy 奇→sell（含三类买/三类卖）。✅
- **route_bsp 分层路由 + top-down**（§9.2/§B）：`rec_engine.rs:789-803` 高 level 先；`route_bsp:649-708` 用 `nearest_active_parent` 分子级(区间套约束 sink/drain/recover/no-op)/核心级(enter/ascend/flip)。逐行对照 flat `t_engine.rs:589-689` 等价。✅
- **sink/recover 齿轮咬合**（§1.3/§9.3）：`rec_engine.rs:573-598`(sink, m=u/3)、`600-629`(recover 全量平清升回)。✅
- **持仓 per-level + 三阶段总体**（§8）：per-level P&L 诊断字段 + 三阶段在 TRoot（`rec_engine.rs:226-234`）。✅

**不一致：**

| # | 设计点 | 代码实际 | 判据 | 严重度 | 位置 |
|---|---|---|---|---|---|
| B1 | **C1：核心持多永不翻空，所有卖点=短差，无 Z₂ 翻转**（§1.2/§9.11/§10 中心裁决） | route_bsp 核心级反向 BSP 走 flip：`clear_all` 全塔平仓 + 反向 enter + `n_flips++`，核心**可翻空** | **退化**（复核：代码忠实复现 flat，是 953d8df 有意删 C1 把它判为伪矛盾——文档终态被相反裁决取代） | 高 | `rec_engine.rs:682-704`，头注释 `:19-20`，单测 `rec_driver.rs:143-149/184-194` |
| B2 | **type2/3→加仓/持有确认 h⁺**（§1.2/§9.6 操作字母表） | route_bsp 签名仅 `is_buy:bool`，type 信息在 `rec_stream.rs:261-268` 折叠成 buy/sell 后丢弃，所有买点同质处理，无 type2/3 专门分支 | **遗漏**（复核确认；= flat，但 §1.2/§9.6 明确要求区分） | 中 | `rec_engine.rs:649-708`，`rec_stream.rs:261-268` |
| B3 | **嵌套 TInstance.child 单链 + 递归归还子树**（§9.10/§9.11 修复1，-238%→+502%） | TInstance 无 child 字段，`instances` 是按绝对 level 索引的扁平 Vec（= flat layers），无 reconcile_recursive、无递归归还 | **退化**（复核确认；嵌套结构曾实装后被 953d8df 撤回扁平数组） | 高 | `rec_engine.rs:156-167,219-221` |
| B4 | **删 direction gate，固定做空 mob=Short**（§9.11 修复2，消除逆势 Long 腿） | sink 仍 `mob=flip_pol(pdir)`，route_bsp 仍按 pdir 判 is_reduce——direction gate 完整保留；父空时买点→sink 开 Long 短差（逆势 Long 腿仍存在） | **退化/偏离**（复核：原判退化→改判偏离；头注释声称删 gate 与代码实际相反） | 高 | `rec_engine.rs:581,655-658,662` |
| B5 | **信号层 11281 全 6 类 BSP 全部被消费**（§0 北极星/§1.1） | 有 `if bl<MAX_LEVEL(=8)` 守卫，level≥8 的 BSP 静默丢弃不消费（= flat ceiling） | **偏离→一致**（复核：ceiling 刻意对照 flat；但严格说"全消费"未达成） | 中 | `rec_stream.rs:262`，`rec_engine.rs:44-48` |
| B6 | 区间套 candidate 武装机制（§3/§9.2） | `rec_stream.rs:51,145-157` candidate 预留=`usize::MAX` 固定，试验已回退；route_bsp 不用 candidate | 退化→一致（复核：candidate 是 953d8df 删除的自创约束） | 中 | `rec_stream.rs:51,145-157` |

### 2.2 `three_phase_unified_design.md` —— 三阶段统一设计

> ⚠️ 关键澄清：该文档的**主命题**（三阶段从 per-instance 上移到 TRoot 总体口径）**已实装**——TRoot 持 `stage/core_cost_basis/notional_in/withdrawn/earning_cash`，TInstance 无 phase 字段。但文档提出的**具体机制**（account_campaign 接口 + 方向对称 + 拓扑 role + settle_and_increment）几乎全部未实装。代码用 flat 的 `account_reduce(dir)`。

**一致：**
- **RecStage 三态机**（CostReduction→CapitalRecovered→EarningShares）单一全局状态机，穿0切退本金，退尽切增股数：`rec_engine.rs:91-105,422-426,446-466`。✅
- **逐仓/全仓由总体 phase 单向驱动**：`rec_engine.rs:719-775`。✅
- **信号层方向对称**（divergence 全程 `t.direction` 参数化）：`divergence.rs:199-217,341-367`。✅
- **enable_three_stage 默认开**（env `T_NO_THREESTAGE` 关），未被绕过：`rec_engine.rs:288-289`。✅

**不一致：**

| # | 设计点 | 代码实际 | 判据 | 严重度 |
|---|---|---|---|---|
| TP1 | **唯一焊接点 `account_campaign(role:ReduceRole,…)`**（§1/§3） | 全仓 grep 零命中。用 flat `account_reduce(dir:Polarity,…)` 按被减层方向分派 | **遗漏**（复核确认） | 高 |
| TP2 | **role 由拓扑身份判**（核心 spine vs 有 parent 的腿；§3 对抗修正#1 明确 REFUTE direction 判别） | 用 direction 判：Long→降成本/Short→leg_pnl，正是被 REFUTE 的方式 | **偏离→一致**（复核：代码=flat，文档已被相反裁决取代） | 高 |
| TP3 | **降本分母 `core_units_on_spine()` 跨实例 spine 汇聚** | 用 `core_long_units()` = Σ 所有 Long 方向层（按方向汇聚，非 spine），与 flat `t_engine.rs:287-289` 逐字相同 | **偏离**（复核确认） | 高 |
| TP4 | **方向对称：空头核心也走完全相同降成本/退本金/增股数**（§0.1.2/§3/§5） | account_reduce 不对称：Long 走三阶段，Short 只 `short_leg_pnl += realized`；空头核心 reduce 永不降 core_cost_basis、永不退本金、永不进状态机（= flat `t_engine.rs:344-350`） | **偏离**（复核确认；这正是文档 §12 要根治的不对称本身，代码完整继承 flat） | 高 |
| TP5 | core_cost_basis 多空都演化（空头镜像降成本） | 空头 campaign 的 core_cost_basis 在 enter 时初始化为入场价后**冻结**（死值），只被 Long-reduce 降低 | **偏离**（复核确认） | 高 |
| TP6 | **settle_and_increment() 总结算原子（u_new>u_old）**（§4.2） | 不存在；最高级别翻转走 flip：clear_all + enter，数值上近似但 reset_campaign 把 core_cost_basis 清 NaN（旧核心降成本成果丢失，对抗修正#3 指出的问题） | 偏离→一致（复核：= flat，文档方案未采纳） | 中 |
| TP7 | **drain 消融**（§8.8，删 drain 并入 account_campaign） | drain 作为独立 τ 原子保留（`rec_engine.rs:631-646`），route_bsp 仍调 `self.drain` | **遗漏**（复核确认；与 `drain_and_three_stages.md` 直接冲突，代码选保留侧） | 中 |
| TP8 | **CarrierModel trait（SpotCarrier 默认）**（§5） | 无 trait，rec_reduce/rec_add 硬编码 SpotCarrier 语义（Long=+m·c/Short=−m·c） | **遗漏**（复核确认；文档 §5 line160 自己声明"本轮不引入"，属设计预期未实装） | 低 |
| TP9 | 增股数持续增/总结算增方向对称（§4.1/§4.2） | deploy_earning 仅多头父级触发（`pdir==Long` 门控），空头不增股数 | 偏离→一致（复核：deploy_earning 的方向门控是 flat 语义，文档"方向对称"绑的是降成本不是增股数） | 中 |

### 2.3 `recursive_t_architecture_v2.md` —— 递归架构 v2

> 该文档引用的行号（promote `rec_engine.rs:595-635`、account_core_reduce `:359-384`）匹配 commit `94881db`，**不匹配 HEAD**——文档描述的是 cc017e4 + 953d8df 之前的 promote 架构，已被整体删除回退为 flat。

**一致：**
- **on_bar 主循环顺序**：A 强平(按 phase 切逐仓/全仓) → A' emergence_upgrade → B route_bsp(top-down)：`rec_engine.rs:713-806`，对照 `t_engine.rs:694-772`。✅
- **涌现升级不依赖该级别 BSP fire**，方向门控仅核心极性==涌现方向才 ascend：`rec_engine.rs:556-571,777-786`。✅
- **会计全局态集中根账本**（free/withdrawn/stage 单一于 TRoot）：`rec_engine.rs:218-234`。✅（注：文档 §8.7「每实例独立 cost_basis/phase」被 three_phase_unified 推翻，代码跟随后者=全局，正确。）
- **sink(u/3)+spawn 短差 / recover 全量升回 / drain / 区间套子级永不独立翻转 / 单根 free 池 + 每实例隔离 / 零操作参数（无门控参数残留）**：与 flat 及 v2 §2/§8.2/§8.7 一致。✅
- **强平 by phase 定位**（①②逐仓 per-instance basis / ③全仓全树 nav≤0）：`rec_engine.rs:718-775`，= v2 §4.4。✅

**不一致：**

| # | 设计点 | 代码实际 | 判据 | 严重度 |
|---|---|---|---|---|
| AV1 | **A1：TInstance 骑 TrendNode、禁存绝对 ladder 索引** | TInstance 同时存 `node` 与 `level`，但操作寻址全走 `level` 数组下标（= flat 绝对 ladder）；node 为结构残留（id_key=start_bar，无操作消费者） | **偏离**（复核确认） | 高 |
| AV2 | **A2：子级方向由所骑走势节点方向给出，非 BSP 类型推断** | 子级方向由 pdir 推（`:653-658`），核心方向由 is_buy 推（`:686`），sink mob=flip_pol(pdir)——方向由 BSP/父向推，非节点 | **偏离**（复核确认） | 高 |
| AV3 | **A3/§3.5：spawn 实例 + 情况一退出门 + PENDING_CONTAINER 三分支（升回新父/flip 反向/PE 持有）** | emergence_upgrade 仅单向 ascend 无退出门；route_bsp 核心反向直接 flip 无三分支；grep TRef/PENDING/generation 无残留 | **遗漏/退化**（复核确认） | 高 |
| AV4 | **§9 promote（回调短头 relabel 升格新核心，零真空）+ §11 reverse-promote** | flip(clear+反向enter) 取代 promote；grep promote/reverse_promote/emergent_dir 全空 | **退化**（复核确认） | 高 |
| AV5 | **§6.1/N4：匹配键从 (kind,bar,level) 改为节点身份(kind,bar,node_start_bar)** | bsp_seen 键仍含 level（`rec_stream.rs:90,250`），instances 按 level 索引；id_key 定义存在但无消费者 | **偏离**（复核确认） | 中 |
| AV6 | **§2.3(b) 升回型 AddBack = 仓位身份迁移(relabel，零现金)** | recover 用 rec_reduce+rec_add（双腿现金转移），非零现金 relabel | 退化→一致（复核：= flat recover，文档 relabel 型未采纳） | 中 |
| AV7 | emergent_ceiling（completed 走势最高级别+1）应被消费 | rec_engine/rec_stream/rec_driver 操作热路径无调用（热路径用 emergent_top）。**更正**：全仓库非死代码——backtest_run.rs:298/383、backtest.rs:292、t_engine_run.rs:419（诊断）、mod.rs:186/202/259（测试）、fugue_v3/axis.rs:28/44（trait+默认实现）、morphology.rs:90、operate.rs:40 均调用 | **偏离**（仅 rec 热路径不用，非死代码；详见 [[545-emergent-top-direction-anchor-instability]]） | 低 |

### 2.4 `drain_and_three_stages.md` —— drain+三阶段（分析文档）

> 该文档是对 **flat 现有 drain 的分析**（非新设计提案），故 rec 复现 flat ⟹ 与本文档描述高度一致。

**一致（11 项）：** drain 触发三条件(子级+反父向 BSP+持同父向遗留仓)、drain=单腿 reduce u/3 无 add 回腿、drain 走 account_reduce、drain/sink/recover 三者边界、drain 净抽干 Σ|units|↓、三阶段第一阶段=sink→recover、第二阶段负 basis 免强平、逐仓强平条件、drain 现金落单一 free 池、MOBILE_FRAC=1/3。全部对照 `t_engine.rs:572-584,589-689` 等价。✅

**不一致：**

| # | 设计点 | 代码实际 | 判据 | 严重度 |
|---|---|---|---|---|
| DR1 | 文档 §4.4 论断"drain 不降 basis、是三阶段反向退化算子" | 代码 drain→account_reduce(Long) 在 CostReduction 下**会**下调 core_cost_basis、甚至触发退本金——与文档论断矛盾（flat 自身注释 `t_engine.rs:573` 也反驳 §4.4） | **偏离**（doc-vs-code，文档论断错误，非代码错） | 中 |
| DR2 | 文档 §4.4"第三阶段增股数未实现" | 当前代码**已实现**增股数（enter_earning/deploy_earning/earning_cash），文档此论断已过时（= flat 也已实现） | **偏离**（文档过时） | 中 |
| DR3 | drain 诊断计数（flat 区分 sell_drain/buy_drain） | rec 仅保留单一 n_drains，删了 op_diag 细分 | **退化→遗漏**（复核；诊断层非会计层） | 低 |
| DR4 | 文档引用 flat 行号（drain@280-287 等） | 已全部失效，drain 现在 `t_engine.rs:572-584`（文档与代码脱节） | 偏离（文档可验证性） | 低 |

### 2.5 `three_stages_accounting_design.md` —— 三阶段会计设计

**全部 17 项一致**——这是审计中唯一零不一致的维度。会计核心（字段、stage 转移、退本金、增股数、reset、account_reduce 分流、clamp、守恒守卫）与 flat 逐行镜像、语义等价（非仅注释声称）：

- **cost_basis/basis 物理分离**（§2）：core_cost_basis（可<0，演化）与 basis（per-instance 恒>0，强平基准）是两个字段。✅
- **字段完整性**：core_cost_basis / notional_in / withdrawn / earning_cash 全在 TRoot，名义与 flat 一致；stage 命名 RecStage（flat 命名 TStage，三变体逐位对应）。✅
- **守恒律**：TW=free+Σsign·u·c+withdrawn，`prove_tw_neutral` 是**真 assert!（panic）验收**，容差公式与 flat `prove_nav_neutral` 逐位相同；**rec 比 flat 守卫更密**（sink/recover/drain 后各调 + on_bar 末调，flat 仅 step 末调）——更严不偏离。✅
- 退本金 try_withdraw_capital、enter_earning、deploy_earning、reset_campaign、保证金模式耦合（§4.6）均一致。✅

> 注意：本文档与 `three_phase_unified_design.md` 的关系——本文档描述的是 flat 已实现的会计原子（rec 忠实复现），而 three_phase 提出的是 flat 之上的"方向对称 + account_campaign"重构（未实装）。两者不矛盾：会计**机制**一致（§2.5），会计**方向对称性**缺失（§2.2 TP4）。

---

## 3. 特别关注四点

### 3.1 逐仓强平（CostReduction 逐仓 / EarningShares 全仓切换）—— ✅ 完全一致

`rec_engine.rs:718-775`（on_bar A 段）与 flat `t_engine.rs:709-745`（step A 段）**bit-exact 镜像**，逐条对照（非注释声称）：

- **EarningShares → 全仓**：in-system `nav(c)≤0` 连锁全平所有 level（`rec_engine.rs:720-743`）。触发判据 `nav<=0`：`:322`。
- **CostReduction/CapitalRecovered → 逐仓**：per-level 独立 basis 判，多头 `c>0 ∧ c≤basis/SUB_LIQ_FACTOR`（`:751`），空头 `c≥SUB_LIQ_FACTOR·basis`（`:752`，多空对称）。
- **SUB_LIQ_FACTOR=2.0** 两侧同源（`crate::fugue_v3::SUB_LIQ_FACTOR`，`rec_engine.rs:32` / `t_engine.rs:52`）。
- **全仓账户级抵消会计**：cascade 遍历所有 level 调 account_reduce（被减层方向各自核算），核心多头利润 − 短差空头亏 = net 在 in-system NAV 上自然抵消，与设计 §6 描述一致。
- 唯一非语义差异：rec 用 `rec_reduce` 直接返回 realized，flat 用 `reduce_at + realized_total()` 差分，数值等价但 flat 多产 trade 记录/per-ladder realized 数组（诊断层）。

**结论：与 flat 一致，与 architecture_v2 §4.4 / three_stages §4.6 设计一致。无遗漏/退化/偏离。**

### 3.2 emergence_top —— 判据震荡（已结晶为谱系 545）

> **更正（2026-06-21 复核）**：本节初稿把审计期工作区临时读到的判据当作"当前"。复核确认：那些是**未提交的工作区实验，审计窗口内被并发会话 revert 回提交版**。`types.rs` 现已干净（`git status` 无 M，`git diff HEAD` 空）。下文以**提交基线**为准。

**提交基线（HEAD 953d8df = b590d4884f 的 types.rs）`emergent_top`：**
```rust
i = rposition(level 有任何 t.completed 走势)        // completed，任意 kind（含盘整）
last = levels[i].next_units.last()
return Some((last.level, last.direction))            // level=i+1（Move(i)≡Level-(i+1)笔），dir=末封装单元方向
```
即指向**最高「含至少一个 completed 走势」级别的封装单元**（level=i+1，方向取末封装单元）。由 commit b590d4884f 引入，953d8df 未触碰，与 flat `stream.rs` 共享同一函数（bit-exact）。

**对照设计文档 architecture_v2 §3.2**：文档定义 emergent_top = "最高**已完成(completed)**走势方向"。提交基线**用 completed ✓**，方向取自末封装单元（next_units.last）而非走势本身——与设计一致（封装恒等式）。判定：**一致（提交基线）**。

**判据震荡（审计期观测，已结晶 [[545-emergent-top-direction-anchor-instability]]）**：审计当日观测到 emergent_top 在工作区出现 ≥3 个未提交实验形态（`!zhongshus.is_empty()` 含盘整返回 i / 仅趋势 kind 无 completed / `completed && 真趋势` 返回 i+1），并在审计窗口内被并发会话改动 ≥2 次后 revert 回提交基线。**根因**：emergent_top 一个能指承载三所指——(a) r\* 涌现上界、(b) 核心方向锚、(c) H¹ 升级归属信号；其中 (b) 方向锚已被 architecture_v2 §11.6 否定（−1069% 做空陷阱，应换级联判据）却未从 emergence_upgrade 的方向门控物理移除，导致判据在 (b) 上反复打补丁、始终无法稳定。详见谱系 545。

**消费侧（一致）**：flat `stream.rs` 与 rec `rec_driver.rs:89-93` 共享同一个 `types.rs::emergent_top()`；on_bar A' 段(`rec_engine.rs:777-786`)在 BSP 路由前调 emergence_upgrade，仅当「核心 dir==涌现 dir 且 核心 level<target」才 ascend，符合 H¹ 自下而上升级归属设计。`emergent_ceiling`（用 completed，types.rs:242-248）**存在但无任何调用者 = 死代码**（AV7）。

### 3.3 三阶段会计字段（cost_basis/phase/notional_in/withdrawn）—— ✅ 字段全在，且 = flat

| 字段 | rec_engine.rs | flat t_engine.rs | 语义一致 |
|---|---|---|---|
| `core_cost_basis`（cost_basis） | :229,413-414,519 | :209,318-319,458 | ✅ 净现金口径，可<0，演化 `-= realized/rem` |
| `stage`（phase, RecStage） | :89-105,227 | :104-119,203(TStage) | ✅ 三变体逐位对应 |
| `notional_in` | :228,518,447 | :204,457,358 | ✅ campaign 投入本金 |
| `withdrawn` | :232,452-456,491 | :213,363-368,410 | ✅ 退本金移出池，reset 归还 |
| `earning_cash` | :230,432,473 | :215,340,388 | ✅ 增股数纯利润子账 |

**全在 TRoot（总体口径），TInstance 无 phase 字段** —— 与 flat 一致，且实现了 three_phase_unified 的主命题（三阶段上移）。**缺口**：仅这些字段，没有 three_phase_unified 提的 `campaign_direction` / `campaign_*` 前缀字段、没有 account_campaign 接口（§2.2 TP1-TP9）。

### 3.4 BSP 消费（view.buy/sell type1/2/3 + route_bsp 分发）—— 含全类但不区分 type

- **view.buy[j]/sell[j] 含 type1/2/3 全部**：`rec_stream.rs:261-268` 按 BSPKind 奇偶映射（偶 0/2/4→buy，奇 1/3/5→sell），三类合并。✅
- **fresh 消费**：bsp_seen 跨 bar 去重，只新 fire 的进 view。✅（§3.1）
- **route_bsp 分发**：`nearest_active_parent(j)`：`Some(p)` 子级走区间套约束（is_reduce 按父向判→sink/drain；同父向→recover/no-op），`None` 核心级走 enter/ascend/flip。对照 flat 等价。✅
- **缺口**：route_bsp 只看 `is_buy:bool`，**不区分 type1/2/3**（type 信息在 rec_stream 折叠后丢弃）。设计 §1.2/§9.6 要求 type2/3→加仓确认(h⁺) 未实装（B2 遗漏）；level≥8 BSP 被 ceiling 丢弃（B5）。

---

## 4. G1–G5 遗漏检查表结果

| 约束 | 结果 | 证据 |
|---|---|---|
| **G1 摩擦地板按级别** | **遗漏** | rec_engine/rec_stream/divergence/backtest_run 全文零 friction/floor/摩擦/地板；回测层零 fee/commission/slippage；sink 配额恒 u/3 不按级别切割；无最低可操作级别门。低级别短差在摩擦下=噪声的保护**完全不存在**。`rec_engine.rs:147-150,574-598` |
| **G2 单边上扬 H¹→0** | **遗漏** | 操作层无任何让短差幅度/recover 率随级别或 regime 衰减的机制；MOBILE_FRAC=1/3 恒定，recover 全量 m=units。divergence 的 level 区分是信号层"是否产 type1"判据，非操作层 H¹ 振幅衰减。`rec_engine.rs:36,577,608` |
| **G3 一次满仓100%+同仓叠加短差** | **一致** | enter 一次性 `m=free/c` 全额建仓（`:503-524`）；单一 `self.free` 标量池（`:221-222`）；sink/recover/drain 全在同一 free 上叠加，非独立资金池。✅ |
| **G4 僵尸空头 recover 率级别衰减（修好了吗）** | **一致/已修** | recover 对所有 sub 级别一视同仁，m=units 全量平清，无级别衰减（`:602-629`）；§9.11 修复1 递归归还子树曾落码（但注：该嵌套版后被 953d8df 回退为扁平，见 B3——当前 recover 是 flat 式单层全量，对所有级别仍一视同仁）；提供 recover_by_level/short_pnl_by_level 诊断。✅ |
| **G5 双层记账同数（子空头 P&L ≡ 父降成本金额）** | **偏离** | sink 与 recover 是两笔时间/价格不同的 reduce，realized 各自独立；代码**无任何断言**保证"子短差 realized = 父降成本量"。prove_tw_neutral 只保证单次转移 NAV 中性，非双层同数恒等。`rec_engine.rs:574-598,602-629` |

---

## 5. memory_snapshot 核心约束逐条核验

| 约束 | 结果 | 说明 |
|---|---|---|
| **嵌套递归赋格（同笔双层记账/资金守恒/平多≠开空）** | 部分偏离 | sink/recover 确是同一物理交易双层记账，reduce(平多)与 add(开空)是两个独立会计动作（非符号翻转）✅；但「逐仓独立空头 voice」「子空头 P&L≡父降成本同一数字」**未实装**（单 campaign 净仓位模型，short_leg_pnl 与 core_cost_basis 两本独立账）；核心翻转用 clear_all+净额 enter = memory 点名批评的「净额翻转丢掉递归嵌套信息」 |
| **递归正则化（操作点来自下一层/域来自上一层）** | 偏离 | 操作点(BSP)按其自身产出级别 `t.level` 路由，非「level j 操作点来自 j-1」；域(emergent_top)取**最高**趋势级别供 ascend，方向与「域来自上一层(次级别)」相反 |
| **H⁰核心 2/3 + H¹机动 1/3** | 一致 | quota=u/3（`:36,147-150`），sink 下放 1/3，核心保留 2/3 ✅ |
| **绩效=Σ\|涨跌幅\|（永远在市场有方向）** | 部分一致 | flip 是 clear+反向 enter 无 FLAT 态（LONG↔SHORT）✅；但首次 enter 前、recover/drain 平掉全部仓位后仍可空仓，非严格"永远在市场" |
| **区间套自上而下定位** | 一致(结构) | 子级永不独立翻转(`:650-680`)，只核心级 flip；top-down 路由(`:789-803`)✅；但 candidate 武装机制被删（B6），是结构即门非显式 candidate |
| **标的无特化（零 per-asset 参数）** | 一致 | backtest_run.rs 只有「标的→数据文件路径」表 + BT_SYMBOLS 白名单，无 per-asset 参数分支；引擎常量全标的无关 ✅ |
| **必然性累积（prove 函数是验收标准）** | 偏离 | flat 配套有完整 `fugue_v3::prove` 守卫族（prove_cross_level_closure/prove_sigma_quota/prove_conservation 全 panic），但 rec_engine **只实装 prove_tw_neutral 一项**；memory 必然性清单（逐仓独立森林/全三类 BSP/成本门/区间套定位/先势后定位 8 条）未落为 prove 守卫 |
| **概念自我运动（走势终完美→新走势）** | 一致 | route_bsp 核心级反向 = clear 全塔 + 反向 enter 的辩证推导（`:696-703`），核心级是唯一独立翻转点 ✅ |
| **D∞ 群三坐标（φ/r/ε）** | 偏离/隐式 | 代码只有 ε（极性 flip_pol，`:79-85`）和 r（level/MOBILE_FRAC=1/λ，`:35-36,163`）以注释形式隐式出现，φ（相位/方向）无显式坐标，无显式三坐标结构 |
| **齿轮耦合（Z₂、相邻级别反向）** | 偏离 | sink/recover mob=flip(pdir) 方向反父向(ε=−1)✅；但路由的是 nearest_active_parent（**可非相邻**），相邻性(Δr=−1) 不被强制（= flat，但 prove_cross_level_closure 声明的相邻守卫缺失） |
| **滤波器（per-level P&L 追踪）** | 一致 | short_pnl_by_level/recover_by_level/sink_by_level 字段存在且被写入（`:253-264,595,619,623`）✅ |
| **三阶段方向对称（空头也降成本）** | 偏离 | 空头核心走 short_leg_pnl，不走降成本三阶段（= flat 已验证语义，但与 memory「方向对称」相悖）。与 §2.2 TP4 同 |

---

## 6. 所有不一致项汇总（按分类）

### 遗漏（9，设计点完全没实装）
1. **TP1** account_campaign(role) 唯一焊接接口 —— 高
2. **TP7** drain 消融 —— 中
3. **TP8** CarrierModel trait —— 低（文档自己声明本轮不引入）
4. **B2** type2/3→加仓确认 h⁺ —— 中
5. **AV3** PENDING_CONTAINER 三分支 + 情况一退出门 —— 高
6. **AV7** emergent_ceiling 仅 rec 热路径不消费（**非死代码**，已更正：backtest/测试/fugue_v3 在用） —— 低
7. **G1** 摩擦地板按级别 —— 高
8. **G2** 单边上扬 H¹→0 衰减 —— 高
9. **DR3** drain 诊断细分计数（sell_drain/buy_drain） —— 低
+ 必然性 prove 守卫族（memory 组2，prove_cross_level_closure 等）—— 中

### 退化（9，曾实装后被「复现 flat」裁决改回）
1. **B1** C1 核心永不翻空 → 恢复 flat flip —— 高
2. **B3** 嵌套 TInstance.child 链 + 递归归还 → 回退扁平数组 —— 高
3. **B4** 删 direction gate 固定 Short → 恢复 flip_pol(pdir) —— 高
4. **AV4** promote/reverse-promote → flip 取代 —— 高
5. **AV3** spawn 实例 + 退出门 → 单向 ascend —— 高
6. **AV6** §2.3(b) relabel 型升回 → 双腿现金转移 —— 中
7. **B6** candidate 武装机制 → 删除 —— 中
8. **DR3** drain 诊断细分 → 单计数 —— 低
> 注：退化项**全部是 953d8df「flat 完全递归化」的有意结果**（删自创约束、复现 flat），有 commit/注释/单测在案，非静默回归。

### 偏离（28，实装但语义不符；列高/中严重度主项）
- **TP3** 降本分母 core_long_units（按方向）≠ core_units_on_spine（按拓扑） —— 高
- **TP4** 方向不对称（空头不降成本，= flat） —— 高
- **TP5** 空头 core_cost_basis 冻结死值 —— 高
- **AV1** TInstance 用绝对 level 数组寻址（node 是残留） —— 高
- **AV2** 方向由 BSP/父向推，非走势节点 —— 高
- **AV5** 匹配键含 level，非节点身份 start_bar —— 中
- **G5** 双层记账无同数断言 —— 高
- **memory 递归正则化** 操作点按自身 level 路由、域来自更高层非次级别 —— 中
- **memory 嵌套赋格** 净额翻转丢递归信息 —— 高
- **emergence_top** 判据比文档更严(completed+真趋势+i+1)且审计期不稳定 —— 中
- **DR1/DR2/DR4** drain 文档 §4.4 论断与代码矛盾 + 文档过时/行号失效 —— 中/低
- **memory D∞/齿轮相邻性/方向对称** —— 中

---

## 7. 结果包六要素

1. **结论**：当前递归引擎（rec_engine/rec_stream/rec_driver）是 flat `t_engine.rs` 的 bit-exact 镜像（commit 953d8df），**忠实实装了 5 份设计文档与 flat 共识的基础设施层**（sink/recover/drain 齿轮、三阶段总体会计、逐仓/全仓强平、fresh BSP 消费、区间套 top-down 路由、单根 free 池、标的无特化、TW 守恒 panic 守卫、H⁰2/3+H¹1/3），但**系统性未实装 3 份文档相对 flat 的"改进"主张**（C1 永不翻空、嵌套 child 链、方向对称三阶段、account_campaign 拓扑接口、promote/PENDING_CONTAINER、TInstance 骑节点、drain 消融、CarrierModel）。130 条核验：81 一致 / 28 偏离 / 9 遗漏 / 9 退化。

2. **定义依据**：每条 finding 引用精确 `file:line`（见 §2-§6），对照判据为「设计文档文本 vs 当前工作区代码」+「rec vs flat t_engine.rs 等价性」。涉及缠论概念的设计点（三阶段第31/48课、区间套第27课、级别独立第3课、方向对称第26课）均溯源至文档引用的原文。

3. **边界条件（结论翻转）**：
   - 若审计基准从「设计文档」改为「代码自己声明的 spec（= flat，见 rec_engine.rs:1-29 头注释）」，则 9 条退化 + 部分偏离翻转为「一致」——这正是对抗复核阶段多条改判的原因。**用户问的是 vs 设计文档（Framing A），故本报告以设计文档为基准；但每条都标注了 = flat 的事实。**
   - 若 `types.rs` 的并发编辑继续，§3.2/§4 emergence_top 结论失效（移动靶）。
   - G1/G2 判遗漏的前提是"设计文档要求 per-level 摩擦/H¹ 衰减"——若该要求被后续裁决也判为伪约束（如 C1），则降为"有意不实装"。

4. **下游推论**：
   - 三份文档（bsp_consumption / three_phase_unified / architecture_v2）的**文本已与代码脱节**，应标注"被 953d8df 复现 flat 裁决取代"或同步更新，否则后续读者会误以为 C1/方向对称/promote 在代码里。
   - **方向不对称三阶段（TP4/TP5）是已知的 §12 不对称根因**，代码继承 flat 的不对称——若要恢复 memory「方向对称」约束需重写 account_reduce（这是定义层改动，非 bug 修复，应走裁决）。
   - **G1/G2/G5 + 必然性 prove 守卫缺失**是真实功能缺口（非 flat 镜像问题——flat 也没有）：摩擦地板、H¹ 衰减、双层同数断言、prove_cross_level_closure 等都未落码。
   - drain 在 three_phase（消融）与 drain_and_three_stages（保留分析）间的矛盾，代码已事实裁决为"保留"——该矛盾应在文档层标注解消。

5. **谱系引用**：本审计触及多个曾发生概念分离的领域——
   - C1「核心永不翻空」vs flat「可翻空」：`[[project_recursive_t_architecture_v2]]`（953d8df 把 C1 判为伪矛盾）；
   - 方向对称三阶段 vs flat 不对称：`[[project_t_short_leg_regime_function]]`（做空腿亏损 L3 = §12 不对称的经验后果）、`[[project_unn_root_flip_mtm]]`（根翻空 MtM 会计）；
   - 嵌套 child vs 扁平数组：`[[project_t_cross_level_coupling_falsified]]`（−89.7% 穿仓 = 绝对/相对错配）；
   - emergence_top 判据演化：`[[project_recursive_t_architecture_v2]]`（编排者多次根因修复，completed→有中枢→真趋势）。
   - 不确定是否有 G1/G2/必然性-prove 相关谱系记录——这些是设计文档 §9.7 补遗的 memory 约束，谱系状态待 genealogist 确认。

6. **影响声明**：本报告 = 新增文档 `docs/rec_engine_implementation_audit.md`，**未改任何代码**（git 状态与审计前一致，5 个 .rs 文件的 M 状态在审计前已存在）。本审计揭示：(a) 三份设计文档与代码脱节，需文档侧同步或标注取代；(b) G1/G2/G5/必然性-prove 四个真实功能缺口（flat 与 rec 共缺）；(c) `types.rs::emergent_top` 在审计窗口内被并发会话改动后 revert 回提交基线，判据反复震荡（根因=一个能指承载 r\*/方向锚/H¹升级三所指，方向锚所指已被否定却未移除），**已结晶为谱系 [[545-emergent-top-direction-anchor-instability]]**；(d) `emergent_ceiling` **非死代码**（更正初稿：rec 热路径不用，但 backtest 诊断/测试/fugue_v3 轴在用）。不改变任何已结算定义。

---

## 附录：审计方法与可信度

- **并行审计**：11 维度 agent（BSP 消费 / 三阶段统一 / 架构 v2 结构 / 架构 v2 父子 / drain / 三阶段会计 / G1-G5 / memory 组1 / memory 组2 / 逐仓强平 / emergence_top），各读对应设计文档全文 + 代码 + 对照 t_engine.rs。
- **对抗复核**：49 条「不一致」结论各派独立 skeptic agent 复核，默认怀疑误判、亲自重读代码。复核改判了若干「退化/偏离→一致」（理由：代码忠实复现 flat，文档已被相反裁决取代），本报告保留**原判（vs 设计文档）为主判据 + 复核（vs flat 真实 spec）为旁注**，因用户问题明确是「vs 设计文档」。
- **主审计员独立地基**：5 份设计文档全文读 + rec_engine/rec_stream/rec_driver/types/t_engine 核心代码直读 + 常量/prove/标的无特化 grep + flat account_reduce 对照——与 agent 结论交叉验证，纠正了 1 处 agent 误判（emergent_top 当前判据，agent 读的是中间态，本报告以审计结束时工作区为准）。
- **可信度边界**：emergence_top 在审计窗口内被并发改动（移动靶）；agent 读取的代码行号在并发编辑下可能与最终 HEAD 有 ±数行偏移；本报告所有结论为 2026-06-21 审计时刻工作区快照。
