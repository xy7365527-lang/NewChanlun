# T 算子递归自相似架构 v2（每级别一个 T 实例）

> 2026-06-20 | 编排者裁决：从 T 算子的自相似递归出发重新设计架构，不选 flat-A 也不选 flat-B。
> 综合来源：15-agent workflow（Ground 缠论递归/当前代码/会计守恒/Nautilus + Design 6 节 + Verify 5 视角对抗审查）+ 主循环亲读 mod.rs/types.rs/t_engine.rs/accounting.rs + 原文 26/27/29/33/65 课。
> 关联：`docs/recover_failure_reasoning.md`、`docs/unified_recursive_operator_T.md`、`docs/three_stages_accounting_design.md`。

---

## 0. 认识论与诚实边界（先读，否则全文会被误读为已实现态）

### 0.1 核心命题

T 算子是递归自相似的（第65课 `aₙ=f(aₙ₋₁)`，第33课 `Aₙ₊₁=F(Aₙ)`）。**架构也应该是递归自相似的。** 当前的 flat array（`TPositionEngine.layers: Vec<Layer>` 绝对 ladder 数组）+ 中央 `route_bsp` 是对递归的**模拟**，不是递归本身；"绝对/相对裂缝"是模拟的必然产物（详见 §6）。

### 0.2 认识论等级（强制标注，formalization-validity-domain）

| 内容 | 等级 | 含义 |
|------|------|------|
| 当前 flat 架构的代码事实 | **L0 事实** | 已运行（`t_engine.rs` 等行号皆实测） |
| 递归架构的接口/协议/守恒（本文 §1–§7） | **L0 设计推论** | 从原文 + 现存会计代数**推导**，**未在多实例代码上验证**。当前生产引擎是单实例多 ladder + 单 free 池。本文一切"递归则如此"是必然性约束，**非经验确认**。 |
| 递归能否提升 8 标的 P1 | **L3 待验证** | 必须回测，不可假设（不重复 NRF v5"机制成立但普适性否证"的错误） |

**对比表里凡标"递归"列，读作"若递归化则"，不是"已实现"。** 验收门见 §6.4。

### 0.3 三处对 workflow 设计产出的诚实化纠正（对抗审查触发）

本文与 6 个 design agent 的初稿有**三处实质分歧**，均依对抗审查（Verify 阶段 4 个 critical）+ 编排者裁决纠正：

1. **保留次级别开空**（推翻 agent 初稿的"次级别不开空、做空只在根 flip"）。
   - 编排者裁决：次级别**必须**开空头。原文一致性审查（CRITICAL）证实：第26课:34"先卖后买与先买后卖效果一样""1分钟卖点也参与"、第27课:136-140"对冲"恰恰发生在次级别——agent 初稿把工程经验否证（`project_t_cross_level_coupling_falsified` −89.7% 穿仓）越级提升为原文支持，是 formalization 越界。
   - 正确形态（自相似递归）：**子 T 骑在「回调下跌走势」节点上持空头**——空头绑定一条结构性确认的次级别走势，不是机械地在每个卖点 ε 翻转。−89.7% 穿仓是 flat 架构的绝对/相对错配（空头绑绝对 ladder、平空落错 ladder），**不是"开了太多空头"**；修复是把空头绑回调走势节点（结构性确认本身即门，无门控参数——零操作参数原则，编排者裁决 §8.2），不是删空头（详见 §2、§8.2）。
   - 故 `docs/recover_failure_reasoning.md` 里"sink=减仓到现金、删空头"的方向**已被编排者否定**；我在 `t_engine.rs` 已落的删空头改动**待回退**（§7.0）。该 reasoning 文档的诊断价值（ε 翻转机械化是病根、绝对/相对错配是穿仓根因）保留，但"删空头"的处方作废，换为"空头绑回调走势节点（结构即门）"。

2. **"消除全局态"是概念偷换 → 诚实拆分两类全局态**（推翻 agent 初稿的"局部依赖消除全局态"叙事）。
   - 全局态审查（CRITICAL）：根唯一现金池 + 根唯一三阶段 + 根唯一 flip 权 = 一个中央协调器，名字叫"根"。把它叫"局部依赖"是概念偷换。
   - 诚实拆分：**(a) 坐标全局态**（绝对 ladder、`seen_bsps` 的 level 键）——**应消除**（产生漂移）；**(b) 会计全局态**（`free`/`withdrawn`/三阶段 → 根账本）——**必须保留**，因 NAV 守恒要求单一现金真相源，这是构造性核心不变量、不是缺陷。局部依赖（275号）只适用于**坐标/分派轴**（父子通信），**不适用于会计轴**（根账本集中）。本文不再声称"消除全局态"，只声称"消除坐标全局态、显式集中会计全局态于根账本"。

3. **base case 补 PENDING_CONTAINER 待定态 + 三分支**（修复 agent 初稿的 spawn/flip 二分时序 gap）。详见 §3.5。

---

## 1. T 实例接口定义

### 1.1 设计公理（从 Ground 推导）

| 公理 | 来源 | 接口后果 |
|------|------|---------|
| **A1 仓位骑走势节点，不骑绝对 ladder** | 考古 §4 裂缝 + 65课 aₙ=f(aₙ₋₁) | `TInstance.node: TrendNode`（起止 bar/价区/方向）；**禁存绝对 ladder 索引**。涌现塔涨落时 ladder 漂移，节点身份不变。 |
| **A2 子级方向受父级约束** | 27课:740 父级状态决定子级 BSP 语义 | 子 T 的操作极性由其所骑**走势节点的方向**给出（回调=反父向=空头），不由 BSP 类型独立推断。消除"零方向推断机械翻转"（`project_t_short_leg_regime_function`）。 |
| **A3 涌现优先 relabel，spawn 仅在完成走势连接** | 29课情况一 vs 二/三 + 543/544 | 中枢扩展（情况一）= relabel；完成走势成为父级输入元（情况二/三）= spawn。**且情况一最弱反弹按 29课:32 应退出**（§3.3 强度门）。 |
| **A4 会计全局态集中于根账本** | 会计守恒审计 | `free`/`withdrawn`/`TStage`(权威)/`notional_in`/`earning_cash`/`seen` 单一化于 `TRoot`；实例只持自己的持仓 + campaign 五元组（份额标签）。**这是 §0.3-2 的"会计全局态必须保留"，不假装局部依赖。** |

### 1.2 走势节点身份（A1）

```rust
/// 走势节点身份（仓位骑节点，绝不存绝对 ladder）。对应 types.rs::Unit 的封装体。
/// 重锚键 = start_bar（依赖 append-only 前缀冻结使其稳定——⚠ 该性质未证，见 §8.6）。
pub struct TrendNode {
    pub start_bar: i64,        // 走势节点起 bar（身份重锚键）
    pub end_bar: i64,          // 当前末 bar（生长中走势随 bar 增长）
    pub price_lo: f64,         // 价区下沿 DD
    pub price_hi: f64,         // 价区上沿 GG
    pub direction: Direction,  // 几何方向（H⁰ 形态学，Up/Down）
    // relative_level 仅诊断，绝不作存储/匹配键（A1）
}
```

### 1.3 T 实例 state

```rust
pub struct TInstance {
    // ── 仓位（骑一条走势节点；方向由 node.direction 决定，A2）──
    pub node: TrendNode,        // 骑的走势节点身份（A1）
    pub direction: Polarity,    // 操作极性：Up 走势→Long / Down 回调走势→Short（A2，非 BSP 推断）
    pub units: f64,             // 本实例持有股数 ≥0（禁 sign 位，符号在 direction）
    pub cost_basis: f64,        // 本实例核心成本 per share（可<0；穿0 上报根触发退本金）

    // ── 三阶段 campaign 份额（权威 stage 在根，A4；这里是本实例的归属份额）──
    pub phase_view: TStage,     // 只读视图（权威在根）
    pub notional_in: f64,       // 本实例本 campaign 投入本金 K_k
    pub withdrawn: f64,         // 本实例已退本金份额（物理现金在 root.withdrawn）
    pub earning: f64,           // 本实例③阶段弹药份额（物理现金在 root.free）
    pub pending_recover: f64,   // 已 sink 待回补股数（同股数计数器，第26课:34）

    // ── 拓扑（只跟直接父/子通信，275号——仅坐标/分派轴）──
    pub parent: Option<TRef>,   // 直接父；None = 根（最高涌现级别，唯一独立 flip）
    pub child:  Option<TRef>,   // 直接子（区间套下传目标）；None = 叶（a₀ 邻接层）
    pub lifecycle: TLifecycle,  // Dormant / Active / PendingContainer（§3.5）
}

/// 代际句柄（修复 ABA：dormant 槽位复用会让 pending 升回错误实例，base case 审查 HIGH）。
pub struct TRef { pub slot: usize, pub generation: u64 }
```

**TRef 必须带 generation**：级别收缩时实例转 dormant、槽位可复用（§3.4）；裸 `usize` 索引会产生 ABA——child 的 `pending_recover` 升回到一个被替换的无关新实例。每次槽位从 dormant 复活 `generation++`，派发前校验 `target.generation == ref.generation`，不匹配则走"孤儿 pending 升回根"兜底（根直接平账保 TW 中性）。

---

## 2. 父子通信协议（Reduce / AddBack / Spawn）

所有消息**只在直接父子间同步传递**（坐标/分派轴的局部依赖，275号）。**会计两腿穿过根账本**（A4）——这不矛盾：拓扑是局部的，账本是根集中的（§0.3-2）。`bus` 是**进程内同步调用**（trait 方法直调，发送即在同一 `on_bar` 调用栈处理完），非异步消息队列（§5 裁决用同步对象树，异步会破坏逐 bar 配对）。

### 2.1 自相似的核心：子 T 是回调走势上的一个完整 T

这是与 agent 初稿最大的区别，也是自相似递归的落点：

- 父 T 骑「本级别上涨走势」node，持 **Long 核心**。
- 父级走势由次级别走势交替组成：顺势腿（Up，属核心）/ **回调腿（Down，反父向）**。
- **回调腿上 spawn 一个子 T，骑「回调下跌走势」node，持 SHORT**（这就是"次级别开空"，空头绑定一条确认的回调走势，不是机械 ε 翻转）。
- 子 T 自相似：它在自己的次次级别上再做短差（回调的反弹腿上 Long 小腿……）。turtles down 到 a₀。
- **平空 = 回调走势完成（子 T 自己的底背驰）→ 子 T cover short + 升回父级核心。匹配靠走势节点身份（递归），不靠绝对 ladder。**

这一条同时满足：编排者"次级别开空"、原文 26课:34/27课:140、自相似递归、以及 §6.2 三失效点的根治（匹配按节点身份消除 L3→L4 漂移）。

### 2.2 `Reduce`（子→父：开回调短差时父级减仓）

| 项 | 内容 |
|----|------|
| **触发** | 父级核心走势中**结构性确认**出现一条 Down 回调走势 node（次级别卖点，27课:82"震荡才能产生利润"）。**回调走势的结构性确认本身即门——无成本门/方向门参数**（零操作参数原则，§8.2）。 |
| **动作** | 父级核心 `reduce_at(parent, m)` 减仓 m 股 → 现金；同时在回调 node 上的子 T `add_at(child, m, Short)` 开 m 股空头。**同股数：减 m = 开空 m**（不是 ε 翻转的"父减 + 子开任意"，是恒仓的"减出的份额下放到回调上做空"）。 |
| **m 的取值** | `m = mobile_quota(parent.units)`。⚠ **1/3（MOBILE_FRAC）无原文依据**（原文 26课只说"控制量"未定量，例子是"卖5万股"绝对量），标为 **L2 可调超参**，不归因 26课（原文一致性审查 HIGH）。同股数是守恒律（总减=总回补），1/3 是工程参数。 |
| **守恒** | 父 `free += m·c`、核心市值 `−m·c`；子开空 `free += m·c`、负债 `−m·c`。两腿都 NAV 中性，净敞口从 +u 变 (u−m) 多 + m 空 = u−2m（这正是编排者要的"次级别对冲"）。子 `pending_recover` 不在此记——空头是真持仓，由子 node 完成时 cover。 |

### 2.3 `AddBack`（子→父）——必须拆成两型（会计审查 HIGH）

agent 初稿把回补与升回合一，会计审查指出：升回是**仓位身份迁移（relabel，零现金）**，不是 `add_at`（花 free 买）。拆分：

**(a) 回补型 — 同股数/同金额买回核心**（次级别买点，恒仓回复）
| 项 | 内容 |
|----|------|
| 触发 | 父级核心层自身的次级别买点（核心方向的回调结束），有 `pending_recover>0`。 |
| 动作 | `add_at(parent_core, q, parent.direction)` 从根 free 买回。**按 phase 分流**：CostReduction → `q = pending`（同股数，第26课:34"回复原来的数量"）；EarningShares → `q = earning/c`（同金额，第31课"卖出多少资金买入多少资金"，c'<c 时 q>pending = 增股数）。 |
| 守恒 | 父 Long：`free −= q·c`、市值 `+q·c` ⟹ 0。 |

**(b) 升回型 — 子 T 回调走势完成，relabel 归还核心**（零现金）
| 项 | 内容 |
|----|------|
| 触发 | 子 T 的回调下跌走势**完成**（子自己的底背驰，33课:78）。 |
| 动作 | 子 T 先 cover 自己的空头（`reduce_at(child_short, child.units)`，realized = 高开低平的降成本 alpha），现金回根；然后**子 node 注销，控制权升回父**（父核心已在回补型里恢复，或子的 cover 现金留根供父回补型买回）。**升回是身份注销，不是父级 add_at 买入**——区分清楚避免 free 双花。 |

### 2.4 `Spawn`（涌现新父）

| 项 | 内容 |
|----|------|
| 触发 | 当前最高 T（root，无父）走势完成 ∧ 通过 §3.5 三分支判别为"找到更大容器且方向一致"。 |
| 动作 | spawn 新父 T（骑涌现出的更高走势 node，方向 = `emergent_top().Direction`，**非 BSP 类型**），当前 root 降为新父的 child。**零现金、零股数**——新父 units 来自后续子级升回，spawn 仅建拓扑（平凡 NAV 中性）。 |
| ⚠ 时序缺口 | 见 §3.5：spawn 与"走势完成"是同一事件的两个投影，存在容器确认延迟 → 需 PENDING_CONTAINER 待定态。 |

### 2.5 原子序列（同 bar 同价 c，进程内同步）

```
回调出现@父核心（持多）  ── 同一 on_bar 调用栈 ──
  1. 结构性确认一条 Down 回调 node（结构即门，无门控参数，§8.2）
  2. 父 reduce_at(core, m) ; 子T spawn 于回调 node 持 Short m   # 同股数 m
  3. ΔNAV=0（两腿同价配对）
─────────────────────────────────────────────
回调走势完成@子T（底背驰）  ── 同一 on_bar 调用栈 ──
  1. 子T cover：reduce_at(child_short, m')  # realized=降成本 alpha
  2. 回补型 AddBack：父 add_at(core, q, Long) by phase（同股数/同金额）
  3. 子 node 注销，子T → Dormant
─────────────────────────────────────────────
本级别走势完成@root（顶背驰，无父）：§3.5 三分支（spawn 升回 / flip / PENDING）
```

---

## 3. 级别涌现触发 spawn

### 3.1 触发判据：背驰-转折定理（第29课）

[原文 29课] 背驰将导致：(一) 最后中枢级别扩展、(二) 更大级别盘整、(三) 以上级别反趋势。三者都需"更大级别"作容器。

| 转化 | 原文性质 | 结构动作 | **操作动作（原文一致性审查 HIGH 修正）** |
|------|---------|---------|------|
| (一) 中枢扩展 | "未完成走势的延续，还在一个走势里"(29课:44) | relabel（中枢身份升格） | ⚠ 但 29课:32"最弱反弹仅触及 DD → 找机会马上退出"——**操作上情况一应退出/减仓，不是无条件升格持有**。加"反弹强度门"：仅当反弹重回最后中枢（情况二/三）才升格持有。 |
| (二)(三) | "完成的走势类型……两个走势类型的连接"(29课:44) | **spawn 新父** | 升回新父核心 |

**判别钥匙**（29课:52）：反弹是否重回最后中枢。重回弱 = 情况一；不重回/完成 = 情况二/三。

### 3.2 检测：自下而上 emergent_ceiling × 自上而下区间套（双向交叉确认）

- 自下而上：`emergent_ceiling()`（types.rs:242，用 `completed` 排除生长中走势）较上轮**新增一层** = 结构候选。`emergent_top()`（types.rs:260）给 (level, Direction)。
- 自上而下：区间套（27课）提供父级状态约束子级方向语义（27课:740）。spawn 新父方向由涌现走势方向定，**不由触发 BSP 类型**。
- **单向不充分**：仅结构无方向 → 等待；仅方向无 completed → 中枢仍扩展 = relabel。

### 3.3 spawn 协议 + 情况一退出门

```
spawn(root_top):  # 前置：root 走势 completed ∧ §3.5 判为情况二/三
  dir_new   ← emergent_top().Direction      # 方向由涌现走势，非 BSP（A2）
  node_new  ← 涌现出的更高走势节点
  T_new     ← spawn(node_new, dir_new)      # 零现金零股数
  root_top  → 降为 T_new 的 child           # 区间套父子建立
  # root_top 持仓不动；后续其 BSP 改走 §2 子级短差
```
情况一（最弱反弹）：**不 spawn 也不无条件升格**，按 29课:32 退出/减仓（与 `project_t_emergence_upgrade_ab`"升级=regime方差缩减器非alpha"吻合——无条件升格吃掉了情况一的退出信号）。

### 3.4 反向：级别收缩 → recover 升回 + 休眠（带代际句柄）

- 子 T 回调走势完成 → 全量 cover 升回，子 T → Dormant（保留槽位，`generation++` 待复用）。
- 顶层 `emergent_ceiling` 减层 → 该 ladder 若仍持仓，先 ascend/recover 归并到当前实际最高活跃节点，再休眠。**禁脱锚持仓**。
- 休眠槽位复用必须经 TRef.generation 校验（§1.3 ABA 防护）。

### 3.5 base case：PENDING_CONTAINER 待定态 + 三分支（修复 critical gap）

agent 初稿声称"spawn 找到容器 / flip 找不到容器，二分互斥穷尽"。对抗审查抓出**两个 critical**：

1. **时序 gap**：`emergent_ceiling` 用 `completed` 走势算，而"走势完成"本身就是导致 emergent_ceiling 增层的成因——二者是同一事件的两个投影。走势完成那一刻，更高层是否已 encapsulate 够 3 个单元成中枢（mod.rs:96）未定，存在窗口。
2. **漏掉第三态**：spawn 找到容器但**方向门控拒绝**（涌现方向≠核心方向）——agent 初稿这里会 no-op，持仓悬空。而这第三态恰恰是 **29课情况三"以上级别反趋势"= 真正的 flip 触发条件**。

修正为**待定态 + 三分支**：

```
root_top 走势完成（顶背驰）
   ├── (1) emergent_ceiling 新增层 ∧ 涌现方向 == 核心方向
   │        ⟹ spawn 升回新父（树长高一层）
   ├── (2) emergent_ceiling 新增层 ∧ 涌现方向 ≠ 核心方向
   │        ⟹ flip（= 29课情况三反趋势，核心反向；全树塌缩见 §3.6）
   └── (3) emergent_ceiling 未新增层
            ⟹ PENDING_CONTAINER 待定：持仓冻结（不清不升），
               直到 (i) K 根 bar 内新增层 → 回到 (1)/(2)；
               或 (ii) 反向 base BSP ∧ 仍无容器 ∧ 回抽不重回最后中枢(29课:52)
                      → flip/clear（落地，递归基）
```

flip vs clear 的取舍（避险阀整仓翻转 vs 清仓观望）仍是工程开放轴（`project_t_flip_vs_clear_verdict`），**待 L2 回测裁决，无原文判据**（§8.4）。

### 3.6 根 flip = 原子全树塌缩（会计审查 HIGH）

> ⚠ **§9 精化（编排者 2026-06-20）**：flip（全树清+反向开仓）是次优——正解是 **promote**（转折时当前
> 回调子 T 短头 relabel 为新核心、旧核心清，敞口由回调短头自 t1 连续承载，零真空期）。本 §3.6 的全树
> 塌缩仅作 promote 不可用（转折前无回调先行，罕见）时的退化路径。详见 §9。

根 flip 改变核心方向，**必须在改方向前**了结全部子实例的空头与 pending（否则子级 pending 升回已翻向的根 = 方向污染）：

```
root_flip:
  1. 自顶向下递归：每个子 T 先 cover 空头 + 升回父（按 flip 前的方向）
  2. 全部 pending 清零后，根 clear_all 全树到现金
  3. reset 全树 campaign（§4.5 I4：归还各实例 withdrawn 份额）
  4. 根按新方向 enter；全部子 T → Dormant
  顺序不可换：pending 了结必须在方向改变之前（用旧方向 cover）
```

### 3.7 生命周期状态图

```
   [DORMANT] ─enter/被spawn为父─> [ACTIVE] ─走势完成∧无容器─> [PENDING_CONTAINER]
      ▲ (gen++)                     │  │  │                         │
      │                            │  │  └─同向更高─relabel升格→ACTIVE@high (强度门:情况二/三才升)
      │   回调完成全量cover升回      │  └─走势完成∧容器∧同向─spawn→降为child
      └─────recover+休眠────────────┘  走势完成∧容器∧反向─或─PENDING超时─> flip(全树塌缩)/clear
```

---

## 4. 三阶段会计（per 实例 + 全树守恒）

### 4.1 每实例 campaign 五元组 + 同股数/同金额 by phase

每个 T 实例自有 `(cost_basis, phase_view, notional_in, withdrawn, earning)`（第31课"过程—状态—过程"，每级别走势独立经历降成本→退本金→增股数）。**权威 stage 在根（A4），实例持份额。**

| 阶段 | 触发 | reduce/cover/addback 行为 | Σ\|units\| |
|------|------|------|------|
| ① CostReduction (cost_basis>0) | 建仓起 | 回补型 AddBack **同股数** q=pending | 恒仓守恒 |
| ② CapitalRecovered (cost_basis≤0) | realized 累积穿0 | free→withdrawn 移出在险池，无新交易 | 不变 |
| ③ EarningShares (本金已全退) | withdrawn≥notional | 回补型 AddBack **同金额** q=earning/c | 单调增 |

**recover 升回核心按核心实例 phase 分流**（唯一正确判别钥匙——不按事件类型）：CostReduction 走同股数 `pending`，EarningShares 走同金额 `deploy_earning`。

### 4.2 会计全局态集中于根账本（§0.3-2 落地）

`free`/`withdrawn` 单一化于根：sink/recover 是同一变量 `free_root` 的减加，**结构上不可能漏腿**——这是守恒的根因，是**特性不是缺陷**（会计审查 CRITICAL：诚实声明这是会计全局态，不假装局部依赖）。每实例 `withdrawn/earning` 是"归属本实例的根池份额标签"，物理现金在根。

### 4.3 守恒断言统一为 TW（会计审查 HIGH：不是 NAV）

退本金 `free→withdrawn` 使 in-system NAV 掉 K，**逐 bar 唯一守恒量是总财富 TW = free_root + Σ持仓市值 + withdrawn_root**（与 `t_engine.rs:772` 现状一致——守卫传的是 `total_wealth` 不是裸 nav）。NAV 中性仅在 CostReduction（无退本金）成立。全文守卫一律在**根**用 `total_wealth` 调用——逐实例守卫无法捕捉跨实例配对错误。

| 操作 | 现金流 | 市值/负债 | ΔTW |
|------|------|------|------|
| reduce_at / add_at（同价 c 各方向） | ±m·c | ∓m·c | **0** |
| sink（父减+子开空，同价） | 两腿配对 | 两腿配对 | **0** |
| 升回型 AddBack（relabel） | 0 | units 身份迁移同价 | **0** |
| spawn | 0 | 0 | **0** |
| 退本金 free→withdrawn | −K / +K | 0 | **0**（NAV−K，TW=0） |

### 4.4 强平 by phase 定位到实例

- ①② 逐仓：每实例每层独立判 `c≤basis/SUB_LIQ` (Long) / `c≥SUB_LIQ·basis` (Short)。定位 = (实例, ladder)。
- ③ 全仓：本金在 withdrawn 安全池，连锁判据用**全树** `tree_nav(c)≤0`（杠杆期货应改 ≤maintenance>0）。
- 混合 phase：两判据并行。

### 4.5 全树守恒不变量（Σ over instances）

```
I1 TW 中性（逐 bar，跨阶段唯一守恒）：TW = free_root + Σ_inst Σ_k sign(d)·u·c + withdrawn_root = const   [L0]
I2 Σ|units| 守恒（仅全实例①②恒仓段）：Σ_inst Σ_k u = n_base（sink 开空守 I2；③单调增）
I3 NAV 中性（仅 CostReduction，无退本金时）：ΔNAV=0
I4 本金归属：withdrawn_root = Σ_inst inst.withdrawn ∧ Σ notional_in ≥ withdrawn_root；relabel 时五元组随仓位迁
I5 phase 单向（每实例）：①→②→③（reset 是 campaign 重生非回退）
（无 I6：§8.1 裁决 recover 全量买回不 cap，free 不足=结构 bug fail-loud，无残量可有界）
```

---

## 5. NautilusTrader 对接

### 5.1 选项二（单 Strategy 内部递归 T 树），否决 actor-per-level

T 树是**单一紧耦合会计实体**（父子共享根 free 池）。把级别维拆成 actor + msgbus 是范畴错配：

| 判据 | actor-per-level | **单 Strategy 内部树（推荐）** |
|------|------|------|
| 动态 spawn | `Controller.create_actor` 每涌现一注册 | Rust 内 spawn/relabel，O(1)，对 Nautilus 透明 |
| msgbus 开销 | 每 bar×N 级别 发布订阅+序列化 | 每 bar 仅 4 float 过界一次（`push_bar`） |
| 守恒守卫 | 消息边界破坏逐 bar TW 配对 | `prove_nav_neutral` 在 Rust 树内逐 bar，批↔流共核 bit-exact |

msgbus 价值在松耦合跨进程；级别维是紧耦合单账本，留 Rust。Actor 留给真正独立实体：**每标的一个 Strategy**。

### 5.2 映射 + 共享核心

- `on_bar(Bar) → bridge.feed(o,h,l,c) → TFugueStream.push_bar`（ffi.rs:175）；spawn/relabel 全在 Rust 内，不触 Nautilus actor 注册。
- 父子消息 = 进程内同步直调（trait 方法），零序列化。
- 回测（`run_t_fugue` for-loop push_bar）与实盘（Nautilus on_bar）**共享 `TFugueStreamCore`**（stream.rs:96）= bit-exact 根因。
- 实盘账本真相源在 venue（HL/IBKR）：T 引擎 `step` 输出降级为**目标敞口**，venue 账户经 reconciliation 镜像，差额由 maker executor 收敛（需补 T 引擎"意图输出模式"，是补全不是补丁）。
- 撮合下界口径：回测 `FillModel(prob_fill_on_limit=0.0)`；时间轴 gap 守卫集中在桥接层。

### 5.3 翻案条件

若未来要求各级别**独立资金池 + 异步/多 venue**（`project_t_multiscale_independent_filters` 路线），级别解耦为独立实体，选项一翻案、msgbus 松耦合正合适。这是"选择"类，需编排者裁决（§8.2）。

---

## 6. 架构对比（flat vs 递归）

### 6.1 逐维度

| 维度 | 当前 flat（L0 事实） | 递归（L0 设计目标，**未实现**） |
|------|------|------|
| 持仓存储 | 绝对 ladder 数组 `layers[t_level+BASE_LADDER]`（stream.rs:161） | 仓位绑 `TrendNode`（A1，禁绝对 ladder） |
| 分派 | 中央 `route_bsp` + `nearest_active_parent` 扫 0..MAX_LADDER（t_engine.rs:432） | 父子直接引用 TRef（坐标轴局部依赖） |
| 匹配 | `seen_bsps` 键 `(kind,bar,level)`（level=相对涌现深度，stream.rs:177） | `(kind,bar,node_start_bar)` 节点身份 |
| 级别涌现 | `emergence_upgrade→ascend` relabel 绝对槽位（单向，不下移，:946 幂等） | spawn 实例 + relabel；情况一退出门（§3.3） |
| 会计 | 单 free 池（已天然根集中） | 根账本（**同口径不变**，§6.3） |
| 坐标/相对裂缝 | flat **必然产生**（r* 非单调收缩 mod.rs:96，ladder 钉死不回退） | 节点身份**从坐标层消除** |

### 6.2 三失效点的命运

| 失效点 | flat 根因 | 递归命运（L0 设计，**L3 待验证收益**） |
|--------|------|------|
| 买<卖 做空腿失血 | 无"持多才做空"前提门，BSP 时机误读为方向 | A2 子方向由回调走势节点定 → 机械翻转语法不可表达；**空头绑回调走势节点（结构即门，无参数），生命周期=回调走势生命周期，回调完成自动 cover 升回（§8.2）** |
| 891 noop 不下移 | 绝对 ladder 单向门控，塔收缩不回退 | A1 无绝对 ladder 可"不下移"；塔收缩走 recover 升回+休眠（§3.4） |
| 50% 平空率错配 | 平空落错绝对 ladder | **空头绑回调走势节点**，cover 由子 T 自己的走势完成触发（§2.1），匹配按节点身份 → L3→L4 漂移消除（⚠ 条件于前缀冻结，§8.6） |

### 6.3 守恒口径不变（关键）

两架构会计**口径相同**（根账本，TW 中性）。递归改的是**坐标系**（绝对 ladder→节点身份），不是**会计**。reduce_at/add_at 公式不变，只是寻址对象从 `layers[abs]` 变成 `instance.node`。**这是迁移可建 flat↔递归 bit-exact 守卫的根因。**

### 6.4 可证伪验收门（全局态审查 HIGH：禁把递归列写成已实现）

递归架构"消除坐标全局态"在以下测试 PASS 前**禁声称已实现**：
- **删除 `BASE_LADDER` 常量后引擎仍编译且 bit-exact** ⟹ 绝对 ladder 真消除；
- **`nearest_active_parent` 被 parent 指针替代后无 0..MAX_LADDER 扫描** ⟹ 中央 dispatch 真消除；
- **N4：塔收缩（强制 next.len()<3 触发降级）后已建仓 node 的 start_bar 逐字节不变** ⟹ 漂移真消除（否则只是换形式，§8.6）。

---

## 7. 测试迁移

### 7.0 先决：回退删空头改动

编排者已否定"删空头"方向。`t_engine.rs` 当前的 sink=减仓到现金改动（+ 我加的 `短差减仓到现金_无对冲空头敞口`、`短差减仓无空头腿_价格翻倍不爆仓` 两个对照测试）**待回退**——递归架构保留次级别空头（子 T 骑回调走势持空）。这两个"无空头"对照测试在保留空头的模型下**断言了错误的模型，删除**。

### 7.1 现有 24 测试迁移判决

| 判决 | 数量 | 测试 | 迁移要点 |
|------|------|------|---------|
| **保留** | 6 | 涌现方向门控、全空no_op、子级永不独立翻转、总NAV守恒、空输入守恒、总财富守恒 | 断言对象本就是 §4 全树不变量；仅命名随实例模型调整 |
| **重写** | ~13 | 含绝对 ladder 索引 `layers[k]` / 引擎单例 `eng.pending_recover` / `eng.stage` 的测试 | `layers[k]`→`inst(node)`；`eng.pending_recover`→`child.pending_recover`；`eng.{stage,withdrawn}`→根/实例分流；强平计数→(实例,ladder) |
| **删除** | 3+ | `涌现升级_已在更高ladder不下移`（绝对 ladder 裂缝机制被 A1 消除）；**删空头两测试**（§7.0，断言错误模型） | 机制在递归版不存在 |

⚠ 与 agent 初稿分歧：agent 把"无空头腿"测试当"删除旧 ε 翻转对照"。本文同样删除，但理由相反——不是因为"递归版无空头"，而是因为**递归版有空头（绑回调走势），无空头模型本身是被否定的**。

### 7.2 新增测试（递归特有 + 空头生命周期）

| 编号 | 断言 |
|------|------|
| N1 | spawn 守恒：新父激活零现金流 TW 中性，新父 units==0（来自后续升回） |
| N2 | spawn 方向由涌现走势定，与触发 BSP 类型无关（A2，否定零方向推断） |
| N3 | 父子 reduce 原子性：同 on_bar 栈内单根池减加配对，缺腿 panic |
| N4 | **节点重锚：塔收缩后 node.start_bar 逐字节不变**（§6.4 漂移真消除充分条件，替代被删的"不下移"测试） |
| N5 | base case 三分支：无父才独立 flip；新增层∧反向→flip；无层→PENDING_CONTAINER |
| N6 | 全树 TW 守恒：跨多实例 sink/recover 同价逐 bar==INITIAL |
| N7 | **子 T 空头生命周期**：回调走势 spawn 子 T 持空（同股数 m）→ 回调完成 cover 升回；空头绑 node，cover 由子走势完成触发非绝对 ladder 买点 |
| N8 | campaign 五元组随 relabel 迁移不脱锚 |
| N9 | recover 按 phase 分流：CostReduction 同股数 / EarningShares 同金额 |
| N10 | 本金归属：withdrawn_root == Σ inst.withdrawn |
| N11 | 混合 phase 强平：逐仓与全仓并行 |
| N12 | 根 flip 全树塌缩级联重置（§3.6，pending 在改方向前了结） |
| N13 | **ABA 防护**：dormant 槽位复用后 TRef.generation 校验，孤儿 pending 升回根（§1.3） |
| N14 | **结构即门（无参数）**：子 T 空头仅当回调走势 node 结构性确认时存在；无确认回调走势则不开空（§8.2，零操作参数）。free 不足回补 → fail-loud panic（§8.1，结构 bug 信号，非 cap） |

---

## 8. 未决矛盾（待 escalate / L2-L3 裁决）

对抗审查surfaced 的真实定义冲突与开放轴，按 no-workaround 不在文档内自行消解，显式上浮：

### 8.1 free 不足 = 结构检测 bug，fail-loud（编排者裁决 2026-06-20，已结算）
recover 买回**全量 N units**（同股数），**不 cap、不接受残量、不拒绝 sink**。删除 cap 逻辑（`q=min(pending,free/c)`）+ 删除 I6。
同股数逻辑：卖 N units@P1 收 N×P1，买回 N units@P2。卖点正确则 P2<P1，N×P1 足够买回且有剩余（=降成本利润）。若 P2>P1（free 不够）= 卖点后涨了而非跌了 = 要么卖点错、要么买点太晚。递归架构里子 T 在回调走势完成时平空，**回调终点必然低于起点**，否则那不是回调走势 = 结构判断有误。故 free 不足应 **surface 为异常（fail-loud panic）**——是结构检测 bug 的信号，不是会计选择。

### 8.2 子级空头：递归结构本身就是门（编排者裁决 2026-06-20，已结算）
**不需要成本门/方向门参数。** 子 T 只在回调走势被**结构性确认**（一条 Down 回调 node 形成）时才存在——结构确认本身就是门。子 T 空头生命周期 = 回调走势生命周期：回调走势完成（底背驰）⟹ 子 T 自动 cover 升回归还。
−89.7% 穿仓**不是"开了太多空头"，是 flat 架构的绝对/相对错配**（空头绑绝对 ladder、平空落错 ladder）；修对递归架构（空头绑回调走势节点）空头就被正确管理。**加门控参数违反零操作参数原则（130号 RTAS 同源）。已结算，非开放轴。**

### 8.3 MOBILE_FRAC 1/3（原文一致性 HIGH）
无原文依据，标 L2 可调超参，预注册扫描（与 AND/Structural/OR 三路同构）。同股数是守恒律，1/3 是参数。

### 8.4 最高级别 flip vs clear（base case）—— 已被 §9 解消（编排者裁决 2026-06-20）
~~无原文判据，PENDING_CONTAINER 超时后落地 flip 还是 clear，待 L2 裁决。~~
**解消**：flip 与 clear 都是错的二分。正确形态是**转折的递归原子性**——转折时核心不清仓也不在最高级别
反向开仓，而是「回调子 T 短头 promote 为新核心」，敞口由链连续承载，无真空期。详见 §9。这也解消了
BTC −708% 灾难（宏观 flip 创造牛市核心翻空）。

### 8.5 流式重跑性能 O(S²) 未实测（性能审查 HIGH）
`stream.rs` 模块头声称"每次重跑 O(段数)"被低估——iterate 每次从 a₀ 全量重建整塔 × 每 settled 段触发一次 = **O(S²)**。4.6M bar 从未实测 wall-clock（只有"零 panic"）。递归化前须 L2 性能实测；超预算则引入真增量重跑（复用前缀冻结上塔结果）。

### 8.6 L3→L4 漂移消除 = 条件于"前缀冻结"未证假设（全局态审查 HIGH）
节点身份重锚靠 `start_bar` 稳定，依赖 iterate 的 append-only 前缀冻结——**该性质当前未证明**。N4 测试（§6.4）须在真实数据（L2/L3）上断言塔收缩后 start_bar 不变；若失败，漂移未消除只是换形式，需 escalate（相对涌现 level 非单调 vs 绝对持仓身份稳定的定义冲突）。

### 8.7 会计轴：一个根账本 + 每实例仓位隔离（编排者裁决 2026-06-20，已结算）
**一个根账本 `free`（现金是级别间交换媒介，fungible，非级别专属资源），每个 T 实例独立追踪 units/cost_basis/phase（仓位隔离）。** 现金流：父 T reduce→现金进 free，子 T 开空→从 free 取保证金，子 T 平空→归还 free，父 T add_back→从 free 取。仓位隔离（独立 units/cost_basis/强平）+ 现金池共享（fungible）。守恒律在根级别：`TW = free + Σ各实例持仓价值 + withdrawn`。**已结算——不走每实例独立池（那会破坏现金 fungibility 与逐 bar TW 守恒）。** 这与 §0.3-2 的坐标/会计全局态拆分一致：坐标全局态消除（局部依赖适用），会计全局态=单根账本（fungible 现金，局部依赖不适用）。

---

## 9. 敞口连续性：转折的递归原子性（无真空期）—— flip vs clear 的解消（编排者 2026-06-20）

> 解消 §8.4（flip vs clear）+ 精化 §3.5/§3.6（base case）。同时解释 BTC −708% 灾难的根因。
> 认识论：结构推理（第27/33/37/65 课）= **L0**；它解灾的回测收益 = **L3 待验证**。

### 9.0 问题：顶级 T 实例 clear 后到下一方向之间有真空期吗？

若顶级 T 走势完成就 clear 到现金、再等新走势涌现才入场，则 clear→re-enter 之间有**真空期**——那段
涨跌幅丢失，违反"绩效 = Σ|涨跌幅|"。这是 §8.4 flip-vs-clear 二分隐含的缺陷。本节证明：**真空期不该
存在，因为转折时刻新方向已被下级持有**——flip 与 clear 都是错的，正解是「回调短头 promote 为新核心」。

### 9.1 Q1：L_k 的卖点在结构上对应 L_{k−1} 的什么事件？

[原文第37课]：趋势 a+A+b+B+c，**c 必然是次级别走势**（石猴五点 #2：c 含对 B 的三类买卖点；#4：上涨
c 必创新高）。[原文第37课:22]：「对 c 的内部分析……c 至少包含两个次级别中枢……构成次级别趋势……
就可以继续套用 a+A+b+B+c 的形式进行次级别分析确定 c 中内部次级别趋势的背驰……形成类似**区间套**，
对其后的背驰更精确定位。」

⟹ **L_k 趋势顶背驰（卖点）= c 段（一个 L_{k−1} 上涨走势）创新高后完成 + c 段自身的顶背驰**（c 内部
的 L_{k−1} 趋势背驰，由区间套递归向下到 a0 确认，第27课）。即 L_k 卖点 ⟺ L_{k−1}(c段) 顶背驰 ⟺ …
⟺ a0 精确转折点。**同一个转折点，在各级别逐级收缩确认**（第27课区间套定理）。

### 9.2 Q2：这两个事件同时还是有时序差？—— 有，且下级**领先**

转折点（a0 极值）是**同一根 bar**（区间套底）。但**背驰确认**在各级别有时序差，且方向**反直觉**：
- L_k 的背驰确认需要**反向走势成长为一个 L_{k−1} 走势**（够大才能确认 c 段背驰被反向终结）。
- L_{k−1} 的背驰确认只需反向成长为 L_{k−2} 走势（更小、更早）。

⟹ **下级先确认反转，上级后确认**。设 t0=a0 转折点，反向（下跌）发展：
```
t0: a0 极值（精确顶）
t1: 反向成长为 L1 走势 → c 段(L2)的背驰可确认 → L1 级可入场做空    （早）
t2: 反向成长为 L2 走势 → L3 的背驰确认 → L3 级转折坐实              （晚）
…逐级向上
```
新方向（下跌）在 **L1（t1）就可入场**，远早于 L3（t2）坐实。**时序差 = 相邻级别确认滞后**（反向走势
成长一个级别所需的时间）。

### 9.3 Q3：L_k clear 与 L_{k−1} 进入新方向能否原子？—— 不是原子同时，而是**流水线连续**（下级领先），无真空

不是"同时原子"，而是**下级领先、上级跟进的流水线**——而这恰恰**消除真空期**：
- t1：L1 反向走势确认 = c 段后的回调启动。引擎此刻在 **L1 开空**（这是 §2 的 sink：核心仍持多，回调
  下跌走势上开子 T 短头）。新方向（下跌）的敞口**从 t1 起由子 T 短头承载**。
- t1→t2：核心（L3 多，旧方向背驰未坐实）+ 子 T（L1/L2 短，新方向）并存。这正是第33课"卖点变少"的
  恒仓减仓 + 反向短差对冲段——**敞口连续，无真空**。
- t2：反向坐实为 L2 走势 = 第33课:48「次级别回抽不重回中枢 ⟹ 三类卖点 ⟹ 必须走」= c 段背驰确认为
  **真转折**（非回调）。此刻**旧核心（L3 多）清掉，子 T 短头 promote 为新核心**（relabel，非新建）。

**关键：旧核心 clear（t2）时，新方向已被子 T 短头持有（自 t1）⟹ 零真空期。** "clear 后等待"不存在——
等待期里子 T 已在新方向。flip（清+反向开仓）也不对——它在 t2 才在最高级别开仓，丢了 t1→t2 段且仓位
不连续。正解是 **promote**：回调短头（t1 已开）在 t2 升格为核心。

### 9.4 Q4：时序差多少？怎么处理？

时序差 = **相邻级别确认滞后**（反向走势从 L_{k−2} 成长为 L_{k−1} 所需 bar 数；记忆 `project_interval_nesting_forward`
实测 12b/120b/3000b/50000b 跨级别阶梯）。处理 = **回调短头连续承载 + 区间套判别回调 vs 转折**：
- 回调（第33课:48 次级别回抽**重回**中枢）⟹ recover（子 T 平空升回，旧核心恢复，是降成本短差）。
- 转折（次级别回抽**不重回**中枢 = 三类卖点）⟹ promote（子 T 短头升格新核心，旧核心清）。
两者用**同一个结构测试**（区间套 / 中枢是否被反向走势重回，第33课:48 + 第37课），在 t2 当下判定，不预测。

### 9.5 设计含义：flip → promote；core 从基级别入场、emerge 上移

1. **flip 改 promote**（取代 §3.6 的 clear_all+reverse-enter）：转折时不全树清现金，而是
   「旧核心 reduce 清 + 当前回调子 T relabel 为新核心」。回调短头是新方向的连续载体（t1 已开），
   promote 是仓位身份迁移（零真空、NAV 中性 relabel）。`clear` 仅在**无任何回调子 T 在持**（反向未
   先以回调形态出现）的退化边界才落地——而区间套保证转折前必有次级别回调先行，故此退化罕见。
2. **core 从基级别入场 + emerge 上移**（修 §6.2 granularity / BTC 灾根）：core 不直接钉最高级别，
   而在可操作基级别（move/L1）enter，随走势 spawn relabel 涌现上移（持多骑牛）。

### 9.6 这解释 BTC −708% 灾难（根因 = 违反递归原子性）

当前 `extract_chain` + flip 模型违反 9.1–9.5：core 钉最高级别（L5 宏观），转折走 flip（clear+在 L5 反向
开仓）。后果：(a) 宏观 flip 在 t2（晚）才在 L5 开巨空，且**不经回调短头先行**——丢了连续性；(b) L5 宏观
空头一旦开就持到下次 L5 flip（极晚），在 BTC 牛市被轧 → NAV −608k。

**promote 模型解灾**：反向先以 L1 回调短头出现（sink，小）。BTC 牛市里绝大多数回调**重回中枢** ⟹ recover
平掉（降成本，小）；只有真转折（次级别不重回中枢，BTC 史上罕见）才 promote 为更大空头。**宏观核心翻空
不再凭一个 L5 卖点无条件发生，而是必须反向逐级确认 promote** ⟹ 牛市无界空头从结构上消失。这与 §8.2
"递归结构本身是门"一致：promote 的逐级确认就是门，无参数。

### 9.7 结果包六要素（本节）

- **结论**：转折是递归原子的——L_k 卖点 ⟺ L_{k−1}(c段)顶背驰 ⟺ …⟺ a0 转折点（区间套同一点逐级确认）；
  下级先确认反转、上级后确认（时序差=相邻级别滞后）；新方向自下级回调短头（sink，t1）起连续承载，转折
  坐实（t2）时回调短头 **promote 为新核心**、旧核心清 ⟹ **零真空期**。flip（清+反向开仓）与 clear（清+等待）
  都错，正解是 promote。
- **定义依据**：第37课（c 必次级别 + 区间套递归定位背驰，石猴五点 + L22）；第33课:48（次级别回抽重回中枢
  =回调 recover / 不重回=三类卖点 promote）；第27课（区间套同一转折点逐级收缩确认）；第65课（aₙ=f(aₙ₋₁)
  递归同构）；第26课（恒仓减仓+反向短差对冲段，t1→t2 敞口连续）。
- **边界条件（翻转条件）**：(a) 若区间套不成立（转折前无次级别回调先行，反向以缺口直接跳变）则回调短头来
  不及开，promote 退化为 clear（罕见，第37课:18 连续缺口 b 段情形）；(b) 时序差（相邻级别滞后）若过大
  （记忆阶梯 50000b），t1→t2 间回调短头承载的反向敞口可能不足以覆盖旧核心，净敞口短暂偏旧方向（非真空，
  但非满额新方向）——这是确认滞后税，不可消除（第29课转折须反向走势确认）；(c) promote 的会计（旧核心清
  + 回调短头 relabel 新核心）须 TW 中性，落码须 prove 守卫。
- **下游推论**：(1) rec_engine 的 `flip` 应重写为 `promote`（回调子 T relabel 为核心 + 旧核心 reduce 清），
  §3.6 全树塌缩仅作 promote 不可用时的退化；(2) `extract_chain` 应让 core 从基级别入场（§9.5-2），转折
  由链的回调 promote 表达，而非最高级别 flip；(3) BTC −708% 预测在 promote 模型下大幅缓解（牛市回调 recover
  不 promote）——**L3 待验证**，不可假设（不重复 NRF v5 错误）。
- **谱系引用**：`project_t_flip_vs_clear_verdict`（flip vs clear 开放态——本节解消为 promote 第三态）；
  `project_t_cross_level_coupling_falsified` −89.7%（宏观无界空头，promote 逐级确认解之）；
  `project_interval_nesting_forward`（相邻级别确认滞后阶梯，Q4 时序差实测）；543/544（涌现 relabel）。
- **影响声明**：本节为设计推理（docs 新增 §9），未改代码。落码含义：rec_engine `flip`→`promote` 重写 +
  `extract_chain` core 基级别入场（下一步实装，每步 NT 回测验证）。解消 §8.4，精化 §3.5/§3.6。

### 9.8 L2 实测：promote + emergent 门控落码与"做空陷阱"否定性结果（2026-06-20）

§9.5 落码后经 NautilusTrader BTC（4.6M bar，2017→2021，BH +1380%）逐步验证，产出**否定性结果**
（缩小有效域，比确认更有价值——formalization-validity-domain L2）：

| 模型 | 全量 strat | 敞口（多/空） | 操作 | 机制 |
|------|-----------|--------------|------|------|
| flip（旧 §3.6） | −708% | — | flip 12 | 宏观 L5 翻空持到下次 flip |
| clear_root | −690% | — | clear 11 | clear 后在 chain[0]=反向重入宏观空 |
| **promote（§9.3）** | −862% | 18/82% | promote 3/clear 9 | 回调短头升格为**宏观核心空头**，比 clear 轧得更狠 |
| **+emergent 门控** | **−1069%** | 18.2/81.8% | flip/clear/promote≈0 | **做空陷阱**（见下）|

**emergent 门控**（§9.4 落码）：核心方向权威 = `tree.emergent_top()`（最高**已完成**走势方向，稳定信号），
顶级反转仅当 emergent_dir 也反转才是真转折；chain[0] 反转但 emergent_dir 不变 = 回调/r\* 跳变伪转折 →
核心骑趋势不动。**300k 子集验证有效：93% 持多，+44.82%**（消除伪转折，flip/clear/promote=0）。

**但全量 −1069%（做空陷阱，新揭示的 L2 机制）**：`emergent_top` 用最高**已完成**走势，在最高级别严重
滞后（该走势跨年）。核心一旦在某次真转折翻空（emergent_dir→Short），牛市恢复时 chain[0] 转上但
emergent_dir 仍滞后为 Short → 门控**对称地**把核心锁死空头，直到一个上涨走势在最高级别**完成**（要等
数年）→ 81.8% 时间持宏观空头被牛市轧。300k 未遇真转折（核心始终多）故 +44%；全量遇转折后永久套牢。

**根因质询（§9 promote 的有效域边界）**：三操作模型（flip/clear/promote）+ emergent 门控全失败，根因相同
=**宏观核心一旦净做空即在牛市灾难**。对照 v3 operate.rs +901%（[[project_t_operation_self_replication]]）：
其架构差异是**宏观核心从不净做空**（只多/平，root_direction 持多骑牛），净空头**只来自 1/3 sink 短差**
（次级别回调，有界）。§9.3 的 promote（把空头子 T 升格为**宏观核心**）正是制造宏观净空头 = 灾难根源。

**有效域结论**：emergent 门控有效域 = `emergent_top` 不严重滞后时（短/单调期，如 300k）；全域（长牛 + 中间
真转折）失效。这是 emergent_top 在最高级别的**确认滞后**（区间套时序差 Q4，记忆阶梯 50000b，
[[project_interval_nesting_forward]]）的直接后果——最高涌现级别无非滞后的转折信号（base case 内禀歧义 §9.5）。

**未决（升 §10 待裁决轴）**：宏观核心应否净做空？两读法：
- **读法 A（v3 同构）**：宏观核心只多/平，净空头只经 sink 短差（1/3，有界，[[project_t_short_leg_regime_function]]
  做空腿=亏损唯一来源的跨标的二次确认）。代价：放弃真熊市（2018/2022）的宏观顺势做空。
- **读法 B（§9.3 原文严格）**：26课:34 次级别必开空头 → 转折坐实时 promote 为宏观核心空（保留全做空能力）。
  代价：最高级别确认滞后致做空陷阱（实测 −1069%）。
- **张力**：读法 A 牺牲熊市顺势（有效域=非上行 regime），读法 B 牺牲牛市抗套牢——这是 regime 函数的又一显形
  （非单一最优），非可由定义自决的实现细节 ⟹ 走 §9 再质询（编排者 item 4）。

**影响声明（§9.8）**：rec_engine `promote` + n_promotes（commit 94881db）；rec_driver emergent 门控
（ChainView.emergent_dir + 顶级反转门控）；rec_t_strategy/rec_backtest 敞口分布诊断。33 rust 测试绿。
有效域 L2 标注：emergent 门控仅在 emergent_top 不滞后时有效，做空陷阱是全域否定性结果。

---

## 10. 结果包六要素

1. **结论**：T 算子递归自相似 ⟹ 架构递归自相似——每级别一个 T 实例骑一条走势节点（A1），父子坐标轴局部通信 + 会计轴根账本集中（A4），三消息协议（Reduce 开回调短差/AddBack 回补型+升回型/Spawn 涌现），级别涌现 = spawn 实例（情况二/三）或 relabel（情况一带退出门），base case 三分支（spawn/flip/PENDING_CONTAINER）。flat array 是对此递归的模拟，绝对/相对裂缝是模拟产物，递归从坐标层消除之。

2. **定义依据**：65课 aₙ=f(aₙ₋₁)（递归构成）；27课:740 区间套父级约束子级（A2）+ :136-140 次级别对冲做空（保留空头）；29课:16/44/52 背驰-转折三分类（spawn/relabel 二分）+ :32 情况一退出；26课:34 同股数恒仓 + 先卖后买等效（次级别开空）；31课三阶段（同股数/同金额 by phase）；accounting.rs reduce_at/add_at TW 中性。

3. **边界条件（结论翻转）**：(a) TW 中性仅在同 bar 同价 c + 单根账本下成立——异步/每实例池即破（§5.3/§8.7）；(b) 节点身份重锚依赖前缀冻结（未证，§8.6）；(c) spawn/relabel 判别依赖 29课:52 回抽钥匙，误判则互换；(d) base case flip/clear 无原文（§8.4）；(e) 递归能否提升 8 标的 P1 = L3 待验证，"消除三失效点"指语法层不可表达，不等于回测收益（§6.2）。

4. **下游推论**：(1) 迁移最小改动 = stream.rs:161/173 偏移换 TrendNode 重锚 + seen_bsps 键改 node_start_bar；(2) 守卫提到根用 total_wealth（逐实例无法捕跨实例配对错误）；(3) relabel 必须五元组随仓位迁；(4) recover 按 phase 分流非事件类型；(5) 子 T 空头绑回调走势节点（结构性确认即门，无门控参数——零操作参数原则，§8.2）取代机械 ε 翻转；free 不足回补 = fail-loud（§8.1）。

5. **谱系引用**（曾发生概念分离）：ε 翻转误用于子级开空（`docs/recover_failure_reasoning.md` 诊断保留、删空头处方作废）；跨级别耦合穿仓 −89.7%（`project_t_cross_level_coupling_falsified`，根因=flat 绝对/相对错配；修复=空头绑回调走势节点、结构即门，非删空、非门控参数）；涌现升级 relabel vs spawn（543/544 settled + `project_t_emergence_upgrade_ab`）；recover 全量了结（543 + `project_t_recover_full_vs_quota`）；最高级别 flip/clear 开放态（`project_t_flip_vs_clear_verdict`、`project_highest_level_sigma_frozen`）；绝对 ladder vs 相对涌现裂缝（`project_t_short_close_level_mismatch` + 信号层 segment/move 双重性**待结晶谱系**）；零方向推断已证伪（`project_t_short_leg_regime_function`）；封装恒等式 Move(k)≡Level-(k+1)笔（540）。

6. **影响声明**：本文 = `docs/recursive_t_architecture_v2.md`，纯设计文档，**未改任何代码**。当前 `t_engine.rs` 的删空头改动待回退（§7.0）。本设计若被采纳将约束 `recursive_t/` 递归化重构（stream.rs 偏移映射、seen_bsps 键、TPositionEngine→TInstance 树、强平定位、三阶段下沉实例）。§8 七条未决矛盾需编排者裁决 / L2-L3 回测，落码前不得自行消解。

**认识论等级总结**：§1–§7 = **L0 设计推论**（原文 + 现存会计代数，未在多实例代码 L2 验证）；当前 flat 代码事实 = L0；递归提升收益 = L3 待验证；§8 七条 = 显式未决（escalate / L2-L3）。

---

## 11. 做空/回补 promote 的对称性 —— 最高级别"回补难"的存在论根源（编排者 2026-06-20 重构，否定读法A后）

### 11.0 命题（编排者重构）

读法A（宏观核心只多/平）被编排者**否定**：做空是对的，卖点正确就该做空。问题不在做空本身，在**回补**。
编排者重构命题：**最高级别走势类型的特殊性不在于"不能做空"，在于"回补特别难"。** 三个难点：
(1) 没有父级提供 return address（次级别 recover 回到父级路径清晰；最高级别 recover 回到哪？）；
(2) 最高级别买信号极其稀疏（lad6/7 type1_buy≈0）；(3) 做空容易（卖点到了就开），回来难（买点太慢 + 没有回归路径）。

**提议**：做空是自上而下逐级确认（L3卖点→L2确认→L1执行）；回补应是自下而上逐级 promote（L1买点→L2确认→L3重建）。
这跟 §9 promote 是同一机制方向反过来——§9 是"回调 promote 为空头核心"（导致宏观做空），现在要的是"反弹 promote 为多头核心"。
**关键问题**：这两个方向的 promote 是否对称？对称 ⟹ 宏观做空+对称回补=完整循环=不亏；不对称 ⟹ 问题出在哪？

### 11.1 三层定位裁决（13-agent 工作流：6 原文/结构证据 + 3 视角 + 对抗验证）

| 层 | 对象 | 裁决 | 依据 |
|----|------|------|------|
| **缠论原文（L0）** | 一买/一卖、区间套、转折确认 | **完全对称** | 37课条件4 / 29:52 / 27课（§11.2）|
| **T算子定义域（L0）** | promote/sink/recover 代码 | **方向无关对称** | rec_engine.rs 无 Long/Short 分支（§11.3）|
| **base case 有效域（L2）** | 最高级别（parent=None）× regime | **不对称** | 时间之矢 + 无 return address + regime 放大（§11.4）|

**一句话**：缠论与算子**定义域对称**，最高级别 base case 的**有效域不对称**——formalization-validity-domain 的又一例（定义域对称 ≠ 有效域对称）。

### 11.2 缠论原文层：完全对称（L0，最终权威=博文，逐字核验）

- **一买/一卖严格镜像**（37课:20 唯一带方向条件4成对）：「a+A+b+B+c 是上涨，c 一定要创出新高；是下跌，c 一定要创出新低。」其余 4 条件（同级别中枢/c含三类买卖点/b≤c级别/c含≥2次级别中枢）方向无关、两侧共用。
- **缠师明文断言对称**（29课:52）：「第一类卖点的情况是一样的，只是方向相反。」一买三种后续演化（中枢扩展/更大盘整/反趋势）原样镜像到一卖。
- **区间套定位顶/底同构**（27课）：自上而下精度下钻（大级别背驰段→次级别→…→笔，逐级收缩定位**同一个**转折点）。做空（顶/卖点区间套）与回补（底/买点区间套）同句定理陈述，完全对称。
- **转折确认机制对称**（29课）：背驰-转折定理、"转折须反向走势确认"（确认滞后）对顶/底等强度要求，三处明文"方向相反/反过来即可"。
- **无任何条款使顶背驰比底背驰更易/更难确认**；divergence.rs 检测代码也完全镜像（创新高↔创新低、ZG↔ZD、area_pos↔area_neg、Type1Sell↔Type1Buy）。

**结论**：缠论**不是**不对称来源。「卖点正确就该做空」成立；一买一卖在定义层互为镜像。

### 11.3 T算子定义域层：方向无关对称（L0 源码确证）

- `promote()`（rec_engine.rs:595-635）**通篇无 Long/Short 字面分支**，新核心直接继承子 T 的 units/basis/direction（:617-630 只读不写 direction）。长核心+回调短子 T promote→新空核心；**对称地**短核心+反弹长子 T promote→新多核心，**同一段代码**。
- `sink/recover/enter/rec_reduce/rec_add` 全由 `inst.direction` 参数化分支，无"核心持多"硬编码；`reconcile_chain`（rec_driver.rs:152-159）对长核心 down 回调 sink 短子 T，对短核心 up 反弹**对称** sink 长子 T（`want.dir==flip(cur_dir)`）。
- **推论**：回补的"reverse-promote"**不需要新算子代码**——promote 已方向无关覆盖 short→long。不对称不在算子定义域。

### 11.4 不对称的真来源：base case 有效域（三重合取，非 regime 单独诱导）

缠论对称 + 算子对称，但实测 −1069%、81.8% 持空。不对称源于**最高级别 base case**，三重合取：

1. **存在论时间之矢（结构常量，base case 内禀）**：做空反转的是**已完成（过去）**的旧结构——卖点+回调在过去已被区间套坐实 ⟹ 立即 sink/promote。回补要确认的是**尚未涌现（未来）**的新上涨走势——而"完成"在最高级别意味着**跨年**（§9.8）。第65课：a0 在最高级别只能被**涌现 construct**，不能被 reverse。第29课"转折须反向走势确认"：最高级别**无更高容器**提供 a0 的非滞后确认 ⟹ 顶级走势的"完成"判据本质滞后。**确认一个过去事件 ≠ 等待一个未来事件**——这是时间之矢，不可消解。
2. **无 return address（base case 路由不对称，L0 源码确证）**：`recover()`（rec_engine.rs:497-505）要求 parent_ref **与** child_ref 双解析成功；顶级 parent=None（enter 处 :425）⟹ 顶级空头**无父可 recover**。做空路径（sink/promote）只需 child；回补在顶级**不能是 recover（返回），必须是 promote（构造）**。而 promote 的硬前置（rec_driver.rs:107-114）= root 须有"方向==new_dir 的**现成**回调子 T"。做空侧牛市回调子 T **丰富**（高频满足），回补侧反弹长子 T 升格为最高级别核心**罕见**（缺现成载体 ⟹ 退化 clear_root+下一重跑 enter，断裂）。
3. **regime 放大器（决定后果幅度，非来源）**：长牛里做空载体（回调短子 T）丰富、回补载体（反弹长子 T 升格顶核心）枯竭（lad6/7 type1_buy≈0，全 type3），叠加 emergent_top 跨年滞后 ⟹ 空头不动点的回补触发器**数年不出现** ⟹ 永久套牢。震荡/熊 regime 下退化为**可控的确认滞后税**。

**逐 bar 数据印证**（L2，操作层）：BTC 牛市"short 滑移为负=做空价高于最近5%顶（追空创新高）"，"空头建仓滑移均 −14.5%"；卖点 fire 在新高（频繁），买点要求新低（37课:20 条件4，牛市系统性 fail）⟹ 引擎结构性"开得出空、回不来"。2021→2022 大跌中低级别 480 买点含 31 个 type1_buy 底背驰（定位精确到近谷底），但只被消费为 max 0.000027 股微尘 churn，**无法逐级 promote 重建宏观多头核心**（promote 必须升格"已存在的回调子 T"，而非聚合频繁 L1 买点）。

### 11.5 对称性裁决：不对称是结构常量，后果幅度是 regime 函数

**回答编排者的关键问题**：两个方向的 promote **在定义域对称、在 base case 有效域不对称**。

- **不对称本身**（时间之矢 + 无 return address）= **结构常量**，永远存在，与 regime 无关。
- **不对称的破坏性** = **regime 函数**：长牛致命（永久套牢 −1069%），震荡/熊退化为可控确认滞后税（§9.7 边界条件b）。
- 故"宏观做空+对称 promote 回来=完整循环=不亏"**不平凡闭合**——只在震荡/熊 regime 闭合；长牛 regime 下回补触发器枯竭，循环断裂。

**这正是编排者命题的精确形式**：最高级别"回补难"不是错觉，是 base case（时间之矢 + 无父级）的存在论必然，被长牛 regime 放大为致命。但它**不源于缠论**（缠论一买一卖对称），也**不源于算子代码**（promote 方向无关）——它源于**最高级别没有更高容器**这一递归 base case 的内禀歧义（§3.6/§9.5）。

### 11.6 严格修复形式（编排者 bottom-up 回补的落码语义——四要件，非补丁）

编排者的"自下而上逐级 promote 重建多头核心"是**严格形式**，落码须补 base case 回补的四件缺失语义：

1. **触发器对称化（reverse-promote 真镜像）**：当前 promote 只在"有现成反向回调子 T"时连续升格；做空侧牛市满足、回补侧退化 clear+re-enter。严格回补 = 让**频繁的低级别 type1_buy 聚合为"反弹长子 T"**，经区间套**逐级向上确认**（L1买点→L2确认→…→核心级别）后 promote 为多头核心——即把"自下而上逐级 promote"落码为与做空 promote **镜像的连续触发**，而非依赖 clear_root 后下一重跑的 enter 重入。promote 须能"用频繁 L1 买点 construct 新同向子 T 再升格"，不止"升格已存在的回调子 T"。
2. **promote 条件对称化（核心翻转的充要条件）**：核心**整翻**（full flip）应**只在反向走势涌现到核心级别**（区间套级联完整到顶=真转折）时触发，**两个方向同条件**。否则保持**有界 1/3 sink 短差**（回调/反弹），不涌现到核心级别即 recover。⟹ 单调牛回调永不涌现到顶 ⟹ 核心**本不该翻空**（emergent 门控的 −1069% 正是缺此条件、单信号整翻 + 冻结）；对称地单调熊反弹永不到顶 ⟹ 核心持空。**emergent_top（最高已完成走势，跨年滞后）是错误的核心方向锚——应换为"反向子 T 是否 spawn 到核心级别"的级联判据。**
3. **§8.1 fail-loud 守卫镜像化**：recover 的 assert（rec_engine.rs:522-530）硬编码"回调终点应低于起点"=多头语义；短核心反弹（终点高于起点）未镜像 ⟹ reverse-promote 路径会触发非对称 panic ⟹ 须按 `inst.direction` 镜像不等式。这是算子层残留的方向假设缺陷（定义域对称≠有效域对称在算子内部的微缺陷）。
4. **接受不可消除的确认滞后税**：时间之矢不可消解（29课 转折须反向走势确认）。回补永远滞后于反向走势涌现时间——但这是**对称的税**（做空也滞后于下跌走势涌现）。**非致命的前提是回补 bottom-up**（用频繁 L1 信号早捕恢复），而非等待跨年的顶级完成走势。

### 11.7 结果包六要素（本节）

- **结论**：做空/回补 promote **在缠论原文与 T算子定义域对称（L0），在最高级别 base case 有效域不对称（L2）**。不对称是结构常量（时间之矢 + 无 return address），破坏性是 regime 函数（长牛致命/震荡熊可控）。回补的严格形式 = bottom-up reverse-promote（频繁 L1 买点聚合反弹子 T → 区间套逐级向上确认 → promote 多头核心）+ promote 整翻条件对称化（反向走势涌现到核心级别才整翻，否则有界 sink 短差）+ 守卫镜像 + 接受对称确认滞后税。**循环可闭合，但闭合条件 = promote 条件对称化，非平凡。**
- **定义依据**：37课:20（趋势背驰条件4上涨/下跌创新高低镜像）；29课:52（"一卖和一买一样，只是方向相反"）；27课（区间套顶/底同构定位同一转折点）；65课:182（aₙ=f(aₙ₋₁) 只给递归形式不给方向，a0 最高级别只能涌现 construct）；26课:34（先卖后买/先买后卖恒仓等效——内部短差对称）；rec_engine.rs:595-635 promote 方向无关 / :497-505 recover 需双 ref（base case 无 return address）。
- **边界条件（裁决翻转）**：(a) 若 promote 整翻条件改为"反向走势涌现到核心级别"，则单调牛/熊核心不再被单信号整翻——L3 待验证（不可假设，§9.8 教训）；(b) 若 bottom-up 聚合（频繁 L1 买点 construct 反弹子 T）在真实数据上无法逐级稳定上传到顶（涌现滞后过大），则回补仍滞后——做空陷阱缩小但不消除；(c) 时间之矢的滞后税在极端单边 regime 下即使 bottom-up 也可能不足以覆盖旧核心敞口（§9.7 边界b）。
- **下游推论**：(1) emergent 门控（rec_driver.rs emergent_dir）应**废弃**——emergent_top 跨年滞后是错误的核心方向锚，换为"反向子 T spawn 到核心级别"的级联判据（§11.6-2）；(2) promote 须扩展为"可聚合频繁低级别买点 construct 反向子 T"，不止升格现成子 T（§11.6-1）；(3) recover fail-loud 守卫须按 direction 镜像（§11.6-3）；(4) 这些落码后须每步 NT 回测验证（§9.8 流程），不得假设闭合。
- **谱系引用**：解消 `project_t_flip_vs_clear_verdict`（flip/clear/promote 三态 + 本节 reverse-promote 第四维度=方向对称性）；`project_t_short_leg_regime_function`（做空腿亏损=本节"回补难"的 L3 显形，非"做空错"）；`project_t_cross_level_coupling_falsified` −89.7%（宏观无界空头=缺 §11.6-2 对称整翻条件）；`project_highest_level_sigma_frozen`（最高级别 σ 跨年冻结=时间之矢的 regime 观测量）；`project_interval_nesting_forward`（确认滞后阶梯=时间之矢的级别量化）；§9.8（emergent 门控做空陷阱=本节不对称的实测）。
- **影响声明**：本节为设计推理（docs 新增 §11，13-agent 工作流综合 + 对抗验证），**未改任何代码**。落码含义：废弃 emergent 门控 + promote 整翻条件对称化（反向子 T 涌现到核心级别）+ bottom-up reverse-promote 聚合 + recover 守卫镜像（下一步实装，每步 NT 验证）。精化 §9.3（promote 须条件对称，非单信号整翻）+ §9.8（揭示做空陷阱的存在论根源）。
- **影响声明（认识论等级）**：§11.2/§11.3 = **L0**（原文 + 源码逐字）；§11.4 时间之矢/无 return address = **L0 结构推论**（base case 内禀）；§11.4 regime 放大 + §11.5 后果幅度 = **L2**（BTC 实测 −1069%/+44.82%）；§11.6 修复闭合 = **L0 设计，L3 待验证**（不得假设闭合，§9.8 教训）。
