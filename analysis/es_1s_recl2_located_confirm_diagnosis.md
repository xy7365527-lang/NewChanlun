# ES 1s recL2 located confirm 诊断——"237 个为什么不通过"彻底查

**任务**（2026-06-15）：ES 1 秒数据 238 个 recL2 走势完美（settled），严格区间套 confirm 只通过 1 个，
查 237 个失败原因分布。只诊断，不修复。

**方法**：在 `rust/src/trading/unified_necessity.rs` 的 located confirm 逻辑加只读诊断累加器
（`UNN_DBG_LOCATED`，不改任何操作状态 ⇒ 真实 metric bit-exact：`confirmed[4]=377`
= `n_nest_fire_sell[4]+n_nest_fire_buy[4]=178+199` 逐字对齐，`root_entries=1` 不变）。
ES 1s 11,765,833 bar 全程跑通零 panic（N1–N8 全 bar 成立）。

**认识论等级**：L2（单标的 ES 1s 真实数据，可证伪）。本诊断产出**否定性结果**——
缩小了区间套 type1 链递归的有效域。

---

## 0. 结论（一句话）

**用户前提建立在一个量的混淆上**：「confirm 只通过 1 个」中的 **1 = `total_root_entries`（F 建仓数），
不是 located confirm 数**。located confirm 在 recL2 层实测**通过 377 次**（543 候选的 69.4%）。
而 237/238「不通过」的真相分两层：

1. **confirm 层本身没有瓶颈**：543 个 recL2 候选窗口 → 377 confirm fire / 165 破极值 / 0 残留。
2. **真正的"只有 1 个"在下游**：377 次 confirm 之后，能变成**操作**的只有 1 次 F 建仓——
   被**恒仓架构**（T14 in-place 翻转永不清仓 ⇒ 森林永不空 ⇒ F 至多触发一次）封顶，与 confirm 无关。

且用户问的三条 confirm 失败原因（type1 / 时序 / 递归深度），实测分布是**极端的**：

| 失败原因 | recL2 (k=4) 计数 | 占比 |
|---------|-----------------|------|
| type1 不满足（缺 type1 sell/buy） | **0** | 0% |
| 递归深度不满足（没递归到底） | **0** | 0% |
| 时序不满足（compress ≥ confirm / 破极值早于后续走势） | **165** | **100%** |

**type1 条件和递归深度条件在 11.77M bar 全程从未被触发**——`while cost_gate_open && has_type1`
的循环体执行 **0 次**。原因见 §3（区间套递归在 1s regime 退化）。

---

## 1. located confirm 窗口生命周期实测（recL2 = ladder 4）

```
new_armed(distinct candidate)   [3]=10581  [4]=543   [5]=43  [6]=6
confirmed(located confirm fire)  [3]=10414  [4]=377   [5]=30  [6]=4   ← recL2 通过 377 次，非 1
broke(破极值)                     [3]=152    [4]=165   [5]=13  [6]=2
  broke_type1_stuck             (全零)                                ← 无一例卡在缺 type1
  broke_floor_timing            [3]=152    [4]=165   [5]=13  [6]=2   ← 100% 破极值都是时序
eod_pending                     (全零)
```

收支平衡：recL2 `new_armed 543 ≈ confirmed 377 + broke 165`（差 1 为窗口 confirm 后再武装的边界）。
**没有任何 recL2 候选窗口因 type1 缺失或递归未到底而失败。** 全部失败（165 个）都是 `broke_floor_timing`：
窗口武装当 bar 即到达（平凡的）成本门 floor，但 `since_bar < bar`（540 号"后续走势验证"）要求至少
跨 1 bar，而价格在下一 bar 即破候选极值——**1-bar 时序竞争**。这正是 540 号"先势后定位"严格时序
在做它该做的事：即时反向（无后续走势）的候选被正确拒绝，不是 bug。

---

## 2. 决定性证据：区间套递归恒下探 0 级

```
confirm_floor(到底层)   [2]=10414  [3]=377  [4]=30  [5]=4
stuck_level(卡点层)     (全零)
```

`confirm_floor[f]` 逐层**精确等于** `confirmed[f+1]`：

| 候选层 k | frontier 起点 (k−1) | confirm 落点 floor | 下探级数 |
|---------|--------------------|--------------------|---------|
| 3 move(L1) | 2 | **2**（10414=confirmed[3]） | 0 |
| 4 recL2 | 3 | **3**（377=confirmed[4]） | **0** |
| 5 recL3 | 4 | **4**（30=confirmed[5]） | 0 |
| 6 recL4 | 5 | **5**（4=confirmed[6]） | 0 |

**每个 k 层窗口都在 floor = k−1 确认，下探恰好 0 级，无一例外。** 区间套的 type1 链递归
（`while cost_gate_open(frontier) && has_type1(frontier) { frontier -= 1 }`）的循环体在全程
执行 0 次——因为守卫 `cost_gate_open(frontier)` 第一次判断即为 **false**。

`stuck_level` 全零 + `broke_type1_stuck` 全零，从两个独立角度交叉确认了这一点：递归从不在某层
"卡住等 type1"，因为它从不进入循环。

---

## 3. 根因：1s regime 成本门吞掉全部级别的振幅

```
floor_theta_none(不可测)    (全零)
floor_theta_below(<0.2%)   [2]=10414  [3]=377  [4]=30  [5]=4
```

`cost_gate_open(f) = (theta(f) ≥ SUB_COST_K × SUB_FRICTION_RT = 2 × 0.001 = 0.2%)`。
在每一次 confirm 时，floor 层 frontier 的 `theta`：

- **不是 None（势可测，中枢观测 ≥ 10）**——`floor_theta_none` 全零；
- **而是 < 0.2% 成本门**——`floor_theta_below` 承接全部计数。

即：从 segment(2) 到 recL3(5)，**每一级的中位中枢振幅 θ 都低于 0.2% 往返成本门**。所以对任意候选层 k，
`cost_gate_open(k−1)` 恒假，递归在 frontier=k−1 立即终止于平凡 floor（候选层正下方一级），
`has_type1(k−1)` 从不被求值。

这是 1 秒分辨率的**振幅坍缩**：单根树各级别走势的绝对振幅（θ）被压在 0.2% 成本门之下。
谱系一致：`project_pcf_1s_a0_source_collapse`（尺度不变性——1s 塔变高但成本门 floor 不变）、
`unn_strict_interval_nesting_l2`（recL2 θ≈0.211–0.453%，动态中位在 confirm bar 实测 <0.2%）。

---

## 4. 全称命题验证——次级别买卖点确实存在，但 confirm 不查看它们

用户命题：238 settled recL2 走势完美 ⇒ 其次级别（move L1）必然有买卖点（走势终完美全称命题）。
信号层 BSP 事件 kind 分布（candidate；confirmed 另计）实测：

| 层 | type1（走势完美背驰） | type2 | type3 | confirmed |
|----|---------------------|-------|-------|-----------|
| segment(2) | 1809 | 473 | 55070 | 135529 |
| **move L1(3)** | **985**（533+452） | 2719 | 7850 | 13640 |
| recL2(4) | 21（13+8） | 15 | 509 | 886 |
| recL3(5) | 3 | 3 | 37 | 72 |

**全称命题成立**：move L1（recL2 的次级别）有 985 个 type1 走势完美点 + 2719 type2 + 7850 type3，
买卖点不缺。

**那么"confirm 为什么看不到这些买卖点"？** 答案不是"看不到"，而是"**根本不下探去看**"：

- located confirm 的递归设计是"逐级 type1 下探到成本门 floor"。但 §3 证明成本门在 frontier=k−1
  （= move L1 这一层）即关闭，`while` 循环守卫第一次判断 `cost_gate_open(3)=false` 就退出。
- 因此 `has_type1(3)`（去 move L1 找次级别 type1 买卖点的那一步）**从不被调用**。
- move L1 的 985 个 type1 点客观存在，但它们落在递归终止线（成本门 floor）的**正下方**，
  从未进入 located 链的判据。区间套递归在"次级别之上一层"就停了。

**严格表述**：confirm 看不到次级别买卖点，不是因为它们不存在（存在），也不是因为时序错配
（虽然有 30% 的候选输给时序），而是因为**成本门的有效域（0.2%）淹没了 1s 各级别的振幅有效域**，
使"逐级 type1 下探"这一机制在 ES 1s 上**整体退化为同义反复**——floor 永远平凡地落在候选层下方一级。

---

## 5. 下游：377 located confirm → 1 F 建仓 + 4 翻转 + 0 spawn

located confirm（377 次 cascade_arm 武装 `located_*[2..=4]`）≠ 操作。操作还需各自门控：

| 操作 | 额外门控 | 实测 |
|------|---------|------|
| **F 建仓** | `!any_active`（森林空） | **1**——恒仓 T14 in-place 翻转永不清仓 ⇒ 森林初始入场后永不空 ⇒ F 至多一次（唯一 source=move(L1)=3） |
| **C 翻转** | `sig.sell1[S] ∧ S≥root_ladder ∧ prove_chain` | flips=`[[3,3],[4,1]]`——需 type1 落在 source 层（recL2 仅 21 个 type1 candidate）∧ root 经 T5 涌现爬升后 root_ladder 常已升至 recL4 ⇒ 要求 source≥recL4 |
| **E spawn** | 自层 nf ∧ cost-gated spawn ∧ 非根空头叶节点 | **0**——成本门拒 + 根空头叶节点 skip（T8） |

所以"只有 1 个"是 **F 被恒仓封顶 + C/E 被各自的 type1/级别门控过滤**的合成结果，
**不是 located confirm 失败**。by_ladder 里的 recL2/recL4 "trades" 是那唯一一根 voice 的
T14 翻转 / T5 涌现重标，非独立入场。

---

## 6. 用户数字 238 与 1 的对账（已精确复核）

独立 probe（`analysis/_count_settled_recl2.py`，复核信号层 `settled_seen` 集合大小，
`analysis/data_cache/_settled_recl2_count.log`）：

| 层 | distinct settled move 数 | up | down |
|----|--------------------------|----|----|
| move L1(3) | 2852 | 1472 | 1380 |
| **recL2(4)** | **238** | 131 | 107 |
| recL3(5) | 18 | 11 | 7 |
| recL4(6) | 1 | 1 | 0 |

- **「238」逐字命中** = 信号层 distinct settled recL2 move 数（131 up + 107 down）。用户前提的分母核实无误。
- **「1」= `total_root_entries=1`**（§5），恒仓封顶，非 confirm 数。located confirm 实测 **377**（recL2）。

**238 settled recL2 走势 → 引擎流转链**：238 settled move 在其生命周期发射 BSP 事件 ⇒ 武装 **543**
个 recL2 候选窗口 episode（>238，因每个 settled move 跨 bar 发射多个 type2/type3 事件、且破极值后可
再武装）⇒ **377 confirm fire**（§1）⇒ 经下游门控 ⇒ **1 F 建仓**（§5）。

链条上**没有任何一环是"238 个里只有 1 个通过 confirm"**：confirm 这一环是 543→377（69.4% 通过）。
"只有 1"出现在最末端的 F 建仓环，且由恒仓架构而非 confirm 决定。

---

## 结果包六要素

**1. 结论**：237/238「confirm 不通过」是量的混淆。located confirm 在 recL2 实测通过 377 次（543 的 69.4%）；
"1" 是被恒仓封顶的 F 建仓数（下游，非 confirm）。confirm 层失败（165 破极值）**100% 是时序**
（broke_floor_timing），**0% 是 type1 / 递归深度**。根因：1s regime 成本门（0.2%）高于 segment→recL3
各级振幅 θ ⇒ 区间套 type1 链递归全程下探 0 级、`has_type1` 零次求值、退化为"候选层下方一级平凡 floor"。

**2. 定义依据**：
- located confirm = `unified_necessity.rs` step() 的 `while cost_gate_open(frontier) && has_type1(frontier)`
  下探 + `!cost_gate_open(frontier) && since_bar < bar` 触发（第 14 环区间套 + 540 号先势后定位时序）。
- `cost_gate_open(f) = theta(f) ≥ SUB_COST_K×SUB_FRICTION_RT`（第 16/35 课成本门，0.2%）。
- 走势完美 = type1 背驰（缠师第 6 课"十年 1-2 次"）；`has_type1` 只认 Type1，type2/3 不算。
- 全称命题输入：信号层 move L1 实测 985 type1 + 2719 type2 + 7850 type3 ⇒ 次级别买卖点存在。

**3. 边界条件**（结论翻转条件）：
- 若 SUB_FRICTION_RT 下调使成本门 < 1s 各级 θ（如 <0.05%），`cost_gate_open(k−1)` 转真，
  type1 链递归启动，§1–§4 的"0 级下探 / 0 type1 失败"结论翻转，此时 type1 / 递归深度才可能成为失败因。
- 若改用更粗分辨率（1min）使 θ ≥ 0.2%，递归同样启动（参 1min 域空报告的不同机制）。
- 若放开恒仓（允许清仓回现金），F 建仓数解封，"1" 翻转，§5 的下游瓶颈消失。

**4. 下游推论**：
- 「ES 1s 严格区间套域空/近空」的根因**不在信号层产出级别不够高**（recL4 已涌现，候选 545/43/6 非空），
  也**不在 confirm 递归失败**（377 通过），而在**成本门 vs 振幅的尺度错配** + **恒仓对 F 的封顶**。
- 区间套 type1 链递归在 1s regime 是**死代码**（执行 0 次）——它的有效域要求 θ ≥ 成本门，1s 不满足。
  这是 `formalization-validity-domain` 的实例：递归机制的定义域（所有 regime）⊋ 有效域（θ≥成本门的 regime）。
- 若要让 recL2 located 真正消费 move L1 的 985 个 type1，须解除成本门对 frontier 起点的短路
  （概念问题：成本门 floor 与 type1 链 floor 是两个独立的递归终止判据，当前实现让成本门**先于** type1 链
  终止 ⇒ type1 链被旁路）。**此为概念层缺口，留待编排者裁决，本报告不修复。**

**5. 谱系引用**：
- `project_pcf_1s_a0_source_collapse`：尺度不变性——1s 塔变高但成本门 floor 不变（同根现象）。
- `project_pcf_pending_locate_collapse_fix`：segment 非势源 + 成本门终止递归（PENDING_LO/floor 机制来源）。
- `unn_strict_interval_nesting_l2`：recL2 θ≈0.211–0.453% 与 0.2% 成本门临界（本报告确认动态中位 <0.2%）。
- `project_constitutive_throughput_falsified` / 539 号：strict located 有效域 ⊂ 特定 regime。
- 540 号（先势后定位时序）：`since_bar < bar` 即本报告的 100% 时序失败判据来源。

**6. 影响声明**：
- 新增 `analysis/es_1s_recl2_located_confirm_diagnosis.md`（本报告）。
- `rust/src/trading/unified_necessity.rs` 加只读诊断累加器 `LocatedDiag`（`UNN_DBG_LOCATED` 门控转储）——
  **不改任何操作状态**，真实 metric bit-exact（confirmed[4]=377=nest_fire 和、root_entries=1 不变）。
  诊断插桩在交付诊断后**应回退**（no-patch-mentality：诊断脚手架不留在生产引擎）。
- 不改任何定义、不改任何回测结果、不触碰信号层。诊断性产出，零行为变化。
