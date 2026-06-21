# ES 1s recL2 区间套 confirm 断点诊断——176 缺失逐层追踪（commit 8447a2a391）

**任务**（2026-06-15）：ES 1s 数据 238 个 recL2 settled，严格区间套 confirm（每级 type1
递归到底，commit 8447a2a391 去 cost_gate 版）只通过 62 次。查清 176 次缺失里**每一次低层
type1 在哪一级断了**，分类统计断点原因分布。只诊断，不修复。

**与前序诊断的区别**：`es_1s_recl2_located_confirm_diagnosis.md` 诊断的是**旧 cost_gate 逻辑**
（confirm=377，`while cost_gate_open && has_type1` 守卫首判即 false ⇒ while 体执行 0 次 ⇒
`has_type1` 从不求值，断点不可观测）。commit 8447a2a391 **删除 cost_gate**，confirm 递归改为
`while frontier >= FIRST_BSP_LADDER && has_type1(frontier)` 逐级下探。本诊断针对**新逻辑**——
`has_type1` 现在逐级真实求值，type1 链断点**首次可观测**。旧诊断结论不可迁移。

**方法**：unified_necessity.rs 加只读 env-gated 累加器（`UNN_DBG_NEST`），记录每个 nest 窗口
episode 的**死亡 frontier**（= 区间套 type1 链断裂层的充分统计量）+ 死亡时该层 type1 的时序
归属。不改任何操作状态 ⇒ bit-exact（验证见 §0）。ES 1s 11,765,833 bar 零 panic（N1–N8 全成立）。

**认识论等级**：L2（单标的 ES 1s 真实数据，可证伪）。产出**否定性结果**——否证"176=信号层
bug"假设，将 commit 的"1s regime 嵌套链罕见"细化为**时序朝向错配**。

---

## 0. bit-exact 守卫（探针只读性验证）

探针计数与已发布操作计数逐字对齐 ⇒ 探针未改任何操作状态：

| 量 | commit 8447a2a391 发布值 | 本次 `UNN_DBG_NEST` run | 对齐 |
|----|--------------------------|------------------------|------|
| recL2(k=4) fire | 62 | fire_sell 29 + fire_buy 33 = **62** | ✓ |
| move(L1)(k=3) fire | 536 | 296 + 240 = **536** | ✓ |
| recL3(k=5) fire | 7 | 3 + 4 = **7** | ✓ |
| strat_pct | +22.7% | +22.7% | ✓ |
| n_trades / root_entries | 1 / 1 | 1 / 1 | ✓ |
| floor_stops / N1-N8 panic | 0 / 0 | 0 / 0 | ✓ |

---

## 1. 结论（一句话）

**"176 = 信号层 bug" 被否证。** 信号层在**每一级**都产出了 type1（`never=0` 遍布所有层——见 §4），
必然性论证的前提"每级都有 type1"经验**成立**。176 缺失的真相是**时序朝向错配**：confirm 窗口在
高层 candidate 处武装后**向前**搜索次级别 type1，而必然性链所要求的次级别 type1（末段）发生在高层
candidate **之前**（末段先 settle 才使高层走势完成），平均 **13694 bar（~3.8 小时）之前**。前向搜索
**结构上看不到过去的 type1** ⇒ 窗口死在初始 frontier（k−1），**连一级都没下探**就被价格破极值否定。

断点分布（recL2 破窗 424 次）：**98.6%（418/424）断在 move(L1) 层**（frontier 从未从 3 降下），
其中 **100%（418/418）是"末段 type1 在过去"（时序错配），0% 是"信号层从未产 type1"**。

---

## 2. recL2(k=4) 窗口 episode 全账（540 = 62 + 424 + 54，逐字守恒）

用户口径辨析（承前序 §6）：「238 settled」= distinct settled recL2 **move 数**；「62」= recL2
located confirm **fire 数**。两者分母不同——238 个 settled move 在其生命期发射多个 BSP 事件
（type1@settle + 发育中的 type2/type3）⇒ 武装 **540 个窗口 episode**。引擎按 episode 运作，不按
settled-move 运作。故「176」非引擎口径的差；引擎口径的完整账是：

| recL2 窗口 episode 去向 | 计数 | 占比 |
|------------------------|------|------|
| **fire**（完整 type1 链确认到底 = 通过的 62） | 62 | 11.5% |
| **break**（价格破 candidate 极值否定，027:25） | 424 | 78.5% |
| **superseded**（同侧 confirmed 事件到达清窗，让位 confirmed 路径） | 54 | 10.0% |
| eod_residual（run 结束仍 open） | 0 | 0% |
| **合计 episodes** | **540** | 100% |

`nest_arms[4]=545`（含 5 次对已 open 窗口的极值刷新延展）vs distinct episodes 540。

---

## 3. 决定性：断点逐层分布 + 时序归属

### 3.1 recL2(k=4) 的 424 次破窗——断在哪一级

```
破极值死亡@frontier:  f=3(move L1):418 (sell201/buy217)   f=2(segment):6 (sell2/buy4)
断在move(L1)层(f=3)归因:  末段type1在过去(<arm,时序错配)=418   至死信号层从未产move(L1)type1=0
断在segment层(f=2)归因:   segment type1在过去(<arm)=3        至死从未产segment type1=0
```

| 断裂层 (death frontier) | 计数 | 占破窗 | 含义 | 时序归属 |
|------------------------|------|--------|------|---------|
| **f=3（move L1）** | **418** | **98.6%** | frontier 从未从 3 降下——窗口期内当前 bar 在 move(L1) 层**无 type1** | **过去 418 / 从未 0** |
| f=2（segment） | 6 | 1.4% | 收到过 move(L1) type1 降到 segment，卡在 segment 层破极值 | 过去 3 / 从未 0 / 窗口内不可消费 3* |

\* 6 例中 3 例 `last_t1[segment] ≥ arm`：segment type1 在窗口期内出现，但出现在 frontier 仍=3 的更早
bar（彼时尚未收到 move(L1) type1 下探到 2），待 frontier 降到 2 时该 segment type1 已是过去 bar、
不在当前 evrows ⇒ 不触发下探（层间时序的二阶错配，6/424=1.4%，可忽略）。

**关键**：`至死信号层从未产 type1 = 0`（move L1 与 segment 双层皆 0）。**没有任何一次断点是因为信号层
缺 type1**。全部断点（418/424=98.6% 主体）的 type1 **客观存在但在过去**。

### 3.2 普遍规律——所有层窗口都死在初始 frontier（连一级都没下探）

| 候选层 k | episodes | fire | break | break 主死亡 frontier | 该层"过去/从未" |
|---------|---------|------|-------|----------------------|----------------|
| 3 move(L1) | 10455 | 536 | 7308 | **f=2(segment)：7307** | 过去 7307 / 从未 0 |
| 4 recL2 | 540 | 62 | 424 | **f=3(move L1)：418** | 过去 418 / 从未 0 |
| 5 recL3 | 41 | 7 | 30 | **f=4：29**（f=3：1） | 过去 1 / 从未 0 |
| 6 recL4 | 6 | 0 | 6 | **f=5：6** | — |

**统一模式**：每个候选层 k 的窗口，破窗时绝大多数死在 **frontier = k−1**（武装时的初始 frontier）——
**descent 从未开始**。因为下探一级所需的 (k−1) 层 type1 在过去（高层 candidate 武装窗口前已发生），
前向 `has_type1(当前bar)` 在窗口整个存活期返回 false。`从未 = 0` 在所有层成立。

### 3.3 时序错配定量

```
断在move(L1)层且末段type1在过去的窗口（n=419），arm→最近move(L1)type1 平均距离 = 13694 bar
```

最近一次同侧 move(L1) type1 平均在窗口武装前 **13694 bar（1s≈3.8 小时）**。这不是紧邻末段，而是
更早的同侧 type1——因 move(L1) 同侧 type1 稀疏（§4：sell 候选 533+确认 2786）。窗口（多由 recL2
type3 发育事件武装，§4 recL2 type3 candidate=509 ≫ type1=21）在无后续 forward type1 链的时刻
武装，价格随即破极值。

---

## 4. 全称命题验证——每级 type1 确实存在（必然性前提成立）

信号层 BSP 事件 kind 分布（ES 1s，require_settled=True，与 unn 同口径；`has_type1` 认 candidate+
confirmed 全部 type1）：

| 层 | type1 candidate | type1 **confirmed** | 出现 type1 的 bar 数 |
|----|----------------|--------------------|---------------------|
| segment(2) | 1809 (s928/b881) | **58271** | 60080 |
| move(L1)(3) | 985 (s533/b452) | **5064** | **6048** |
| recL2(4) | 21 (s13/b8) | 347 | 368 |
| recL3(5) | 3 | 31 | 34 |
| recL4(6) | 0 | 4 | 4 |

- **move(L1) 在 6048 个 bar 出现 type1**（含 5064 个确认型——即 settled move 的走势完美点）。次级别
  type1 **不缺**。必然性论证"recL2 settled ⇒ 末段 move(L1) settled ⇒ move(L1) 有 type1"的**结论
  端经验成立**（`never=0` 是其引擎侧确证）。
- 缺的不是 type1 的**存在**，而是 type1 与 confirm 前向窗口的**时序共现**：6048 个 move(L1) type1 bar
  中，落在某个 recL2 窗口存活期（窗口破极值前）内的极少（仅 62+6 例使 recL2 窗口下探）。

---

## 5. 必然性论证的精确缺环

用户论证：`高层settled → 末段次级别settled → 逐级有type1 → ∴ 176缺失=信号层bug`。

| 论证环节 | 经验裁决 |
|---------|---------|
| 高层 settled ⇒ 末段次级别 settled | ✓（缠论走势完美定义） |
| 次级别 settled ⇒ 该级有 type1 | ✓（§4：每级 type1 存在，never=0） |
| **∴ confirm 应看到该 type1** | **✗ 缺环——存在性 ≠ 前向时序共现** |
| ∴ 176 = 信号层 bug | ✗ 否证（信号层 never=0，type1 全部存在） |

**缺环**：必然性论证断言了 type1 的**存在**，但 confirm 机制要求 type1 在窗口武装后**向前**出现
（`compress_bar ≤ confirm_bar`，540号先势后定位）。区间套的自然结构是**构件在先、高层在后**
（末段先完成才使高层走势完美），故构件 type1 在高层 candidate（= 窗口武装点）**之前**。存在性
（在过去）不满足前向共现性 ⇒ confirm 看不到。**这是 540 号"时序反了"的同一机制**（[[project_pcf_pending_locate_collapse_fix]]
诊断 segment 抢先武装；此处是上一层级的同构现象——前向 confirm 无法消费过去的末段）。

---

## 6. 这是 bug 还是严格语义？（概念分叉——留待裁决，不擅自归并）

数据定位了断点，但"前向 confirm 看不到过去末段"是 bug 还是正确的区间套语义，是**定义层问题**：

- **读法 A（前向 = 正确）**：confirm 触发应要求高层 candidate **之后**次级别**重新**走出 type1 链
  （= 新的次级别下跌/上涨在发育），才是"当下可操作的区间套定位点"。则 62 fire 是真信号，176 缺失
  是"过去的走势已完成、当下无新次级别确认"的正确拒绝——**非 bug**，commit 的"嵌套链罕见"成立
  （但根因应表述为时序朝向而非振幅/regime）。
- **读法 B（应消费过去末段）**：第64课"递归定位到一个时间、价格的点"指向**已发生**的构件序列
  （高层由其已完成的次级别构成），confirm 应回溯消费构件 type1。则前向窗口是实现错误（时序朝向
  装反）——**是 bug**，须改为在高层 candidate 武装时回溯其已 settle 的次级别链。

两读法对应不同的第14环区间套时序语义，**不可由数据单独裁决**（依赖第64课"定位"的时序朝向定义）。
按 no-workaround / no-patch：此处**停下上浮**，不擅自选一读法打补丁。建议 `/escalate` 或 Gemini
decide()（语法记录类决断）。

---

## 结果包六要素

**1. 结论**：176 缺失 ≠ 信号层 bug（信号层每级 type1 齐备，`从未产type1=0` 遍布所有层）。recL2
540 episode = 62 fire + 424 break + 54 superseded；424 破窗中 **418（98.6%）断在 move(L1) 层
（frontier 从未下探），100% 因末段 type1 在过去（时序错配，平均 13694 bar 前）**。普遍规律：所有
层窗口都死在初始 frontier=k−1，descent 从未开始，因 (k−1) 层 type1 在高层 candidate 武装之前。
根因 = **时序朝向错配**（confirm 前向搜索 vs 末段在过去），是 540号"时序反了"的跨层级同构。

**2. 定义依据**：
- confirm 断点 = `unified_necessity.rs` step() `while frontier ≥ FIRST_BSP_LADDER && has_type1(frontier)`
  下探 + `frontier < FIRST_BSP_LADDER` 触发（commit 8447a2a391 去 cost_gate 版，第14环区间套认知）。
- `has_type1(j)` = 第 j 层当前 bar evrows 有同侧 Type1（第29课:396"下次级别以下找第一类"；
  type2/3 不算）。
- 死亡 frontier 充分统计量：窗口在 frontier=f 破极值 ⟺ 存活期当前 bar 在第 f 层从未现同侧 type1
  （任一 type1 必触发 while 下探）。
- 时序错配判别：`last_t1[f] < since_bar`（最近 type1 在窗口武装前）；540号 `compress_bar ≤ confirm_bar`。

**3. 边界条件**（结论翻转条件）：
- 若 confirm 改为**回溯**消费高层 candidate 已 settle 的次级别链（读法 B），断点分布翻转——418 个
  "过去 type1" 转为可消费 ⇒ confirm 数大增，"时序错配"诊断失效。
- 若 1s 改粗分辨率使高层 candidate 与次级别 type1 时间间隔缩小到窗口存活期内，前向共现率上升，
  break@初始frontier 占比下降。
- 若信号层某级 type1 真缺失（`从未=0` 被打破，即出现 `从未>0`），则该断点确为信号层 bug——本次
  ES 1s 未观测到（所有层 never=0）。

**4. 下游推论**：
- "ES 1s 严格区间套近空（62/540）"的根因**不在信号层产出**（type1 每级齐备），**也不在 confirm 漏看
  当下 type1**（前向 type1 存在时 descent 正确，62 fire），而在 **confirm 窗口的时序朝向与区间套
  构件的自然时序相反**。commit 的"完整嵌套链罕见⇒confirm减少"成立，但根因应从"振幅/regime 稀疏"
  修正为"**时序朝向错配**"（更精确、可证伪）。
- `formalization-validity-domain`：第14环区间套 confirm 的定义域（所有 settled 高层）⊋ 有效域
  （高层 candidate 后次级别**重新**走出 type1 链的子集）。62/540=11.5% 是有效域占比。
- 第14环"区间套定位"的时序朝向（前向 vs 回溯）是未结算的定义点——§6 概念分叉。

**5. 谱系引用**：
- [[project_pcf_pending_locate_collapse_fix]]（540号）：segment 抢先武装坍缩的时序重构——本诊断是其
  上一层级同构（前向 confirm 无法消费过去末段）。540号"压缩↑在先、展开↓在后"的时序在此**正是断点
  根源**：高层 candidate（压缩）在后、次级别构件（应被展开消费）在前，前向展开够不着过去。
- [[project_unified_necessity_engine]]：N5/N6 级联 pending_locate；commit 8447a2a391 第14环认知/
  第16环操作分离。
- 第64课"递归定位到一个时间、价格的点"：定位时序朝向定义（§6 读法 A/B 分叉的源头）。

**6. 影响声明**：
- 诊断产出，**未改任何引擎操作逻辑**。`unified_necessity.rs` 的 `UNN_DBG_NEST` 探针为只读 env-gated
  累加器（bit-exact 验证见 §0），诊断完成后回退——committed 引擎保持 commit 8447a2a391 字节一致。
- 新增分析脚本 `analysis/_es1s_signal_type1_probe.py`（信号层只读统计）。
- 否证 commit message 与原任务隐含的"176=信号层bug"，将 confirm 近空根因从"regime/振幅稀疏"
  修正为"时序朝向错配"；揭示第14环区间套时序朝向（前向/回溯）为未结算定义点（§6 待上浮）。
```
数据源：analysis/data_cache/_unn_es1s_nestdbg.stderr.log（断点直方图）
       analysis/data_cache/_es1s_signal_t1.log（信号层 type1 聚合）
       analysis/data_cache/unn_ES1S.json（引擎计数）
```
