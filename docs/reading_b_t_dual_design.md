# 读法B设计:每级别独立腿骑走势消费d_top(T算子construct的对偶consume侧,自相似递归)

> 来源:读法B Design Workflow(wf_09942d92-46e),2026-06-22

I now have the ground truth on the construct (T iterate) and current consume (TRoot single-core ascend). Two key facts verified that the prosecution reports glossed over: (1) the construct `iterate` (mod.rs:83) is a clean single-operator recursion `apply_t` re-applied; (2) the consume side `TRoot.on_bar` (rec_engine.rs:787) is decidedly NOT self-similar — it has hand-written cross-level routing (`nearest_active_parent`, `sink`/`recover`/`drain`) over a flat `instances[level]` array with a single core. The `enable_trend_done_clear` path (A'', line 858) is the 546 deadlock-break, relevant to reading-B's switch semantics.

I have enough to write the design without further reads. Let me verify one more thing — the divergence/d_top computation surface and the `t1buy`/`t1sell` fields confirm d_top per-level is available in `LevelView`. That's already visible in mod.rs/rec_engine.rs (the `view.t1sell[cc]`/`view.t1buy[cc]` usage). Proceeding to the design document.

---

# 读法 B 设计文档：每级别独立腿骑走势消费 d_top（T 算子 construct 的对偶 consume 侧）

## 0. 摘要与定位

**结论**：读法 B = consume 侧自相似递归算子 `g`，对偶于 construct 侧的 T 算子 `f`（`aₙ=f(aₙ₋₁)`）。`g` 在每级别 k 应用同一逻辑「骑 `levels[k]` 当前走势 + 消费 `d_top[k]` 完成切换」，由单算子 `apply_leg(k)` 迭代深度区分级别，**否定 multiscale 的手动 N 个 LevelEngine 展开**。

**认识论等级声明（formalization-validity-domain）**：本文档全部内容是 **L0（设计/架构推导）**。读法 B 能否「破 2/8 + 破做空失血（539）+ 保住强牛」是 **L2/L3 待验证假设**，本文档不预声明有效域。§7 给出否证方案。

**勘察核实（亲自读源，非二手）**：
- construct `iterate`（mod.rs:83-115）= 单算子 `apply_t` 迭代，**真自相似**（已核实 line 89 单一纯函数 + line 104 `current=next` 升级）。
- 当前 consume `TRoot.on_bar`（rec_engine.rs:787-917）= 单核心 + 手写跨级路由（`nearest_active_parent` line 423 / `sink`/`recover`/`drain`），**非自相似**（核实：`route_bsp` line 722 对「有父/无父」分两套逻辑，子级永不独立翻转 line 725）。
- `d_top` 每级别可得：`LevelView.t1buy[k]`/`t1sell[k]`（rec_engine.rs:199-201），当前仅被 A'' 单核心消费（line 865-878，仅 `cc` 一个级别）。

---

## 1. 读法 B = T 对偶的精确形式

### 1.1 construct ↔ consume 对偶表

T 算子 construct 是「**向上封装**」：把级别 k 的完美走势封装为级别 k+1 的一根单元（`encapsulate`, operator.rs:18）。读法 B consume 是其**严格对偶**：在级别 k **骑乘**该级别走势本体（不封装，不向上传递），每级别独立产生方向/切换。

| 维度 | construct（T 算子 f） | consume（读法 B 算子 g） | 对偶关系 |
|------|---------------------|------------------------|---------|
| 递归方向 | 向上：`S_k → S_{k+1}` | 平行：每级别 k 独立骑乘 | f 折叠 ↔ g 展开为持仓 |
| 单算子 | `apply_t(S_k, k, mode)` | `apply_leg(k, view, leg_k)` | 同一算子在每级别 k 应用 |
| 输入 | `S_k`（级别 k 单元序列） | `(levels[k] 走势, d_top[k], leg_k 当前腿状态)` | f 读单元 ↔ g 读走势+腿状态 |
| 核心操作 | `encapsulate`（完美走势→上级笔）| `ride`（走势方向→腿方向）| 封装 ↔ 骑乘 |
| 切换信号 | 走势 `completed`（背驰）→ 升级 | `d_top[k]` 完成 → 腿切换 | 同一 type1 事件的两面 |
| 输出 | `S_{k+1}`（上级单元） | `leg_k'`（新腿方向/仓位） | 上级结构 ↔ 本级持仓 |
| 级别区分 | 迭代深度 k | 迭代深度 k | **共享同一深度坐标** |
| 终止 | `next.len()<3`（r* 涌现） | k > r*（无更高活跃走势级别） | 同一 r* 上界 |

**对偶的精确陈述**：T 的 `f` 把「走势完美」**消解为上级单元的诞生**（信息向上聚合）；读法 B 的 `g` 把「走势完美」**消解为本级腿的方向切换**（信息在本级兑现为收益）。`encapsulate`（construct）丢弃了走势内部的盈亏运动（只留 high/low/area），`ride`（consume）正是去**吃**这段被丢弃的运动——这是对偶的经济学意义：construct 丢弃的 Σ|涨跌幅| 正是 consume 要捕获的。

### 1.2 consume 算子 g 的形式签名

```
g : (Trend_k, DTop_k, LegState_k) → LegState_k'

其中：
  Trend_k    = levels[k].当前活跃走势  (kind, direction)   // construct 产出
  DTop_k     = d_top[k]  (本级走势是否终完美，bool)        // construct 步骤c 产出
  LegState_k = (units_k, basis_k, dir_k, riding_node_k)    // consume 状态

g 的本体（三分支，对偶于 apply_t 四步）：
  1) ride:   若 Trend_k.dir = Up   → 腿应持多（dir_k := Long）
             若 Trend_k.dir = Down → 腿应持空（dir_k := Short）
  2) switch: 若 DTop_k = true（走势终完美）→ 平当前腿 + 反向建腿（多空对称切换）
  3) hold:   Trend_k 未变 ∧ DTop_k = false → 维持腿不动
```

**对偶严格性检验**：
- `apply_t` 步骤 b（`segment_into_trends`）产出方向 → `g.ride` 消费方向。**一对一**。
- `apply_t` 步骤 c（`judge_divergence`）产出 `completed` → `g.switch` 消费 `d_top`。**一对一**。
- `apply_t` 步骤 a（`find_centers`）+ d（`encapsulate`）是 construct 专有（结构生成/聚合），consume 不复制——consume 不再生成中枢，只消费已生成的走势。这是对偶的**非对称残余**：construct 多两步（生成+聚合），consume 只消费方向与切换信号。这不是缺陷，是对偶的本性（生成 vs 兑现）。

---

## 2. 自相似递归骨架：`apply_leg(k)` 单算子

### 2.1 单算子递归（否定手动 N 腿）

```
fn consume_all(view, legs) {
    let r = view.emergent_ceiling();         // construct 给出的 r*
    apply_leg(0, view, legs, r);             // 从 a0 级别递归
}

fn apply_leg(k, view, legs, r) {
    if k > r { return; }                     // 终止：超过涌现上界（对偶 next.len()<3）
    g(k, view, &mut legs[k]);                // 在级别 k 应用单算子 g
    apply_leg(k + 1, view, legs, r);         // 递归到次高级别（迭代深度 +1）
}
```

**与手动 multiscale 的本质区别**：
- multiscale（项目记忆 `project_t_multiscale_independent_filters`）：手动展开 N 个 `LevelEngine`，每个独立实例，代码里写 N 份。
- 读法 B：单函数 `apply_leg` 递归调用自己，`k` 是迭代深度参数。**代码里写一份**，级别由递归深度涌现，N 不是常量。

这与 construct `iterate`（mod.rs:88-111 的 loop）同构：construct 用 `loop { apply_t(current, k); k+=1 }`，consume 用 `apply_leg(k) { g(k); apply_leg(k+1) }`。loop vs 递归是同一自相似的两种写法，本质都是「单算子 × 迭代深度」。

### 2.2 与 construct 共享递归骨架还是独立

**独立递归，共享深度坐标 r\***。理由（formalization-validity-domain：不能声明超出实际的耦合）：
- construct 递归在**结构空间**迭代（`S_k → S_{k+1}`，单元序列）。
- consume 递归在**持仓空间**迭代（`leg_k → leg_{k+1}` 的并列骑乘，不是 leg_k 产出 leg_{k+1}）。
- 二者的**唯一耦合点**是 construct 产出的 `RecursiveTree`（`levels[k]` 走势 + `d_top[k]`）作为 consume 的输入。consume 不反向影响 construct（construct 是 H⁰ 形态学，consume 是 H¹ 操作，设计文档 §0.4 已分轴）。

**结论**：consume 的 `apply_leg` 是独立递归算子，但**严格寄生于** construct 输出的级别坐标系——它的递归深度上界 `r*` 直接取自 `view.emergent_ceiling()`（types.rs:263）。这保证两个递归的「级别」语义同构。

---

## 3. 每级别腿语义 + 547/anchor 共存

### 3.1 每级别腿方向语义

级别 k 的腿 `leg_k`：
- **方向** = 骑 `levels[k]` 当前走势方向（Up→Long / Down→Short）。
- **切换** = `d_top[k]` 完成（本级走势终完美 type1）→ 平 leg_k + 反向建 leg_k。
- **每级别独立** = leg_3 骑 L3 走势，leg_4 骑 L4 走势，互不干预方向。

收益目标：`Σ_k (leg_k 的 |涨| + |跌|)` = 每级别各自吃自己级别那段走势的涨跌幅绝对值。这正是 §1.1 对偶表的「construct 丢弃的运动 = consume 捕获的运动」在每级别的兑现。

### 3.2 是否违反 547？——级别归属澄清（核心）

**547 原文**（项目记忆 `project_cascade_level_misattribution`）：「次级别卖点把整个**主力**仓位翻空」是死锁+做空亏的共同根因；判据应为 `e_level == 最高活跃走势级别` 而非 `>=`。

**读法 B 不违反 547，因为级别归属不同**：

| 概念 | 单核心架构（当前 TRoot） | 读法 B |
|------|----------------------|--------|
| 「主力」定义 | 全局唯一 `highest_active` 那一个核心仓 | **不存在全局唯一主力**——每级别 k 的 leg_k 是「该级别的主力」 |
| 次级别卖点作用对象 | 错误地翻动**全局**主力（547 病灶） | 只翻动**本级别** leg_k，不碰 leg_{k+1}（更高级别腿） |
| 547 禁止的事 | L1 卖点 → 翻 L4 核心（越级翻主力） | **结构上不可能**：L1 的 d_top 只进 g(1)，只影响 leg_1 |

**精确陈述**：547 病灶是「低级别信号越级翻动高级别主力」。读法 B 的每级别腿是**各自级别的主力**——L1 腿是 L1 的主力，L4 腿是 L4 的主力。L1 的 d_top 切换 L1 腿（L1 自己的主力），**从不触碰 L4 腿**。所以读法 B 不是「次级别翻主力」，而是「每级别翻自己」。

这与 547 的修复方向**一致**：547 要求「次级别卖点走次级别 sink，主力不动」。读法 B 把这条推到极致——每级别都有自己的腿，每级别信号只动自己的腿，跨级别根本没有「翻动别级主力」的通道（删掉了 `nearest_active_parent` 路由）。

### 3.3 与 anchor（L4 死扣）共存？

**不共存——读法 B 否定 anchor，但保留 anchor 想保护的东西**。

anchor（`emergent_top` 单核心死扣 L4）的目的：防止低级别 BSP ping-pong 翻动核心（rec_engine.rs:910-916 的 `check_direction` 观测的正是这个失配）。读法 B 用**结构隔离**达成同一保护：L4 腿只由 L4 的走势+d_top 驱动，L1 的 BSP 物理上进不了 g(4)。所以读法 B **不需要 anchor 死扣**——因为低级别信号根本没有通往高级别腿的路由。

**扬弃关系**：anchor 是「单核心 + 防翻动补丁」；读法 B 是「每级别独立腿 + 结构隔离」。读法 B 否定 anchor 的单核心前提，保留 anchor 的「高级别腿不被低级别噪声翻动」诉求，提升为「每级别腿天然隔离」。

---

## 4. 资金 / 仓位架构

### 4.1 资金分配：单一 free 池 + 每级别仓位隔离（继承 v2 三裁决）

依据 `project_recursive_t_architecture_v2` 编排者三裁决之一：「单根账本 free + 每实例仓位隔离」。读法 B 继承：

- **单一 free 池**（`TRoot.free`，rec_engine.rs:234）：所有级别腿共享一个现金池，NAV = free + Σ_k sign(dir_k)·units_k·c。**不分 per-level free**。
- **每级别仓位隔离**：`legs[k]` 各自的 units/basis/dir 独立（对偶于 `instances[k]` 已有的隔离）。
- **不是 multiscale 平均 8 槽**：multiscale 把 INITIAL/8 平均分给每级别（`project_t_multiscale_independent_filters`），读法 B 否定这一点——资金不预先切分，而是**按腿建仓时序从单一 free 池动态领取**。

**初始建仓资金规则**（待 §7 验证哪种最优，本文档给出严格候选，不预声明）：
- 候选 A（动态全额）：每级别腿建仓时从 free 领当前可用额度的一份额。问题：哪一份？需配额规则。
- 候选 B（σ-不变配额，对偶 construct）：construct 的中枢三段 → 配额 f=1/λ（λ=3，MOBILE_FRAC，rec_engine.rs:39）。consume 对偶：每级别腿领 free 的一个 σ-不变份额。这与 construct 的 σ-不变性形成对偶（`project_prove_guards_four_l2` 的 `sigma_quota` 守卫可移植到 consume 侧）。

### 4.2 sink/recover 是否还需要？——**读法 B 取代跨级短差**

**结论：读法 B 下 sink/recover 被每级别独立骑乘取代，应删除**。

依据（严格推导，no-patch-mentality：不保留死代码）：
- sink/recover 的存在理由（当前架构）：单核心架构下，只有一个核心仓在市场，要吃次级别回调必须「父减仓 + 子开反向短差」（sink, rec_engine.rs:613）。这是**单核心的补偿机制**——因为没有独立的次级别腿，只能从核心借仓做短差。
- 读法 B 下：次级别本来就有自己的独立腿 leg_{k-1}。次级别回调（L_{k-1} 走势向下）→ leg_{k-1} 自己骑空（g(k-1) 的 ride 分支）。**不需要从父级 sink 借仓**。
- 跨级短差（sink/recover）= 单核心架构的 workaround。读法 B 的每级别独立骑乘是 sink/recover 想模拟的东西的**本体**。保留 sink/recover = 在已有独立腿之上再叠加借仓短差 = 重复 + 死锁风险（`project_deadlock_dual_open_target` 的核心多腿被 sink 几何衰减成僵尸正是这个叠加的病）。

**澄清「多空双开多重赋格」目的的实现**（`project_deadlock_dual_open_target` 编排者校正）：编排者要的「每级别卖点→平多+次级别做空，吃每级别涨跌幅绝对值」——读法 B 用**每级别独立腿的方向切换**实现，而非「核心 sink + 子开空」。L3 卖点（L3 d_top 完成）→ leg_3 从多切空（吃 L3 下跌）；同时 leg_2 骑 L2 走势（L2 可能还在涨或已转跌），leg_2 独立切换。多空双开 = **不同级别的腿同时持不同方向**（leg_4 多 + leg_3 空），这是「多重赋格」的本体，不需要 sink 从单核心借。

---

## 5. 与 multiscale 的区别 + 为何可能破 2/8 + 做空失血

| 维度 | multiscale 手动 N 腿（旧，2/8） | 读法 B + d_top |
|------|------------------------------|----------------|
| 算子 | 手动展开 N 个 LevelEngine | 单算子 `apply_leg(k)` 递归 |
| 切换信号 | 机械 BSP Z₂ 翻转（type1 翻转 only） | `d_top[k]` 区间套精确（条件2 过滤 Consolidation）|
| 切换时机 | 走势 `completed` 或机械翻转（晚/错） | d_top 三条件（链贯通+本级趋势+力度衰减）|
| 资金 | INITIAL/8 平均预切 | 单 free 池动态领取 |
| 跨级 | 各自独立但仍机械翻转 | 结构隔离，无跨级翻动 |

**为何 multiscale 死在 2/8（CL/DX）+ 强牛踏空**（`project_t_multiscale_independent_filters` 诊断）：
1. **机械 BSP 翻转**：强牛中途次级别 type1 卖点 → 机械翻空 → 踏空主升浪。
2. **做空失血**：高级别 sink 短差在强牛回补率 0%（rL2+）→ 结构性套牢（`project_deadlock_dual_open_target` FaceB）。

**为何读法 B 可能破这两个**（**L0 推导，L2/L3 待证**）：
1. **破机械翻转踏空**：d_top 条件2「本级趋势非 Consolidation」（`cseg-dtop` 报告）显式过滤生长盘整中的误触发。强牛中 L4 走势在生长（Consolidation）→ d_top[4]=false 恒成立 → leg_4 不切换 → **保住强牛主升浪**。机械翻转没有这个过滤，所以踏空。
2. **破做空失血**：读法 B 删 sink/recover（§4.2）。做空只发生在「某级别走势确实向下」时（g 的 ride 分支按真实走势方向）。强牛中 L4 走势向上 → leg_4 恒多，不做空。做空失血的根源（强牛中从核心借仓开空）被结构性消除——因为没有借仓通道，只有「真走势向下才做空」。

**关键证伪点（formalization-validity-domain 强制）**：上述「可能破」是 L0 推导。真实风险——d_top 在子级别贯通但**每级别腿是否真能各自吃到正收益**是 L2/L3 问题。`cseg-dtop` 报告已指出第二层根因「消费级别⊥贯通级别」：d_top[≤2] 贯通但 core L4 不消费。读法 B 让每级别腿各自消费自己的 d_top[k]，理论上消解此根因，但**per-level P&L 是否真正都正是待测的**（§7 判据）。

---

## 6. 实装方案

### 6.1 数据结构（rec_engine.rs 新增，OFF 时 bit-exact）

```rust
/// 读法 B 每级别独立腿（对偶于 TInstance，但语义=骑该级别走势的独立主力，非短差/核心）。
#[derive(Debug, Clone, Copy)]
struct Leg {
    level: usize,
    direction: Polarity,
    units: f64,
    basis: f64,
    riding_node: TrendNode,   // 当前骑的该级别走势节点
    active: bool,
}
```

在 `TRoot` 新增（与现有 `instances` 并存，OFF 时不触碰）：
```rust
legs: Vec<Leg>,                  // 每级别独立腿（读法 B 专用）
enable_reading_b: bool,         // env T_READING_B 置位启用，默认 false
pub per_level_long_pnl: [f64; MAX_LEVEL],   // §7 验收：每级别多头腿盈亏
pub per_level_short_pnl: [f64; MAX_LEVEL],  // §7 验收：每级别空头腿盈亏
```

### 6.2 单算子 `apply_leg` + `g`

```rust
impl TRoot {
    /// 读法 B 单算子递归（对偶 construct iterate）。
    fn consume_legs(&mut self, view: &LevelView, c: f64) {
        let r = self.reading_b_ceiling(view);   // 取 emergent_ceiling
        for k in 0..=r.min(MAX_LEVEL - 1) {      // 迭代深度 = 级别（循环=递归的展开形式）
            self.apply_leg(k, view, c);
        }
    }

    /// g：级别 k 的腿消费 levels[k] 走势 + d_top[k]。
    fn apply_leg(&mut self, k: usize, view: &LevelView, c: f64) {
        let trend = view.nodes[k];                       // Trend_k
        let d_top = view.t1sell[k] || view.t1buy[k];     // DTop_k（type1 完成）
        // 1) switch: d_top 完成 → 平当前腿 + 反向建腿（多空对称）
        if d_top && self.legs[k].active {
            self.close_leg(k, c);                        // 吃完本段，记 per_level pnl
            self.open_leg_opposite(k, view, c);          // 反向骑下一段
            return;
        }
        // 2) ride: 腿空 ∧ 有走势 → 按走势方向建腿
        if !self.legs[k].active {
            if let Some(node) = trend {
                let dir = dir_to_polarity(node.direction);
                self.open_leg(k, dir, node, c);
            }
            return;
        }
        // 3) hold: 维持
    }
}
```

资金领取（`open_leg`）：从单一 `free` 池按 §4.1 候选 B 的 σ-不变配额领取，建仓后记 `riding_node`。`close_leg` 平仓，realized 按 dir 计入 `per_level_long_pnl[k]`/`per_level_short_pnl[k]`。

### 6.3 接线（on_bar 分叉）

```rust
pub fn on_bar(&mut self, view, bar, c) {
    self.cur_bar = bar; self.last_close = c;
    let tw_pre = self.total_wealth(c);
    if self.enable_reading_b {
        self.liquidation_check(view, c);   // 强平边界保留（账户级 NAV≤0）
        self.consume_legs(view, c);        // 读法 B 路径
    } else {
        // ... 现有 A/A''/A'/B 全套，逐字不动（bit-exact OFF）
    }
    self.prove_tw_neutral(tw_pre, c);
}
```

**bit-exact OFF 保证**：`enable_reading_b` 默认 false，OFF 时 `legs`/`consume_legs` 完全不执行，现有 `instances` 路径逐字不变。守卫 TW 中性两路径共用。

### 6.4 守卫移植（对偶）

- `prove_sigma_quota`（每级别腿配额 σ-不变，对偶 construct 中枢三段）。
- `prove_relabel_invariant` 不适用（读法 B 无 ascend relabel）。
- 新守卫 `prove_leg_isolation`：g(k) 只写 `legs[k]`，断言不写 `legs[j≠k]`（547 隔离的可观测形式）。
- TW 中性守卫复用（`prove_tw_neutral`）。

---

## 7. L3 验证方案

### 7.1 四路对照（8 标的 × 3 模式 × 4 架构）

| 路径 | env | 架构 |
|------|-----|------|
| OFF | 全 OFF | flat 基线（BTC 权威基线 `project_t_engine_btc_baseline`） |
| ANCHOR | 默认 | 单核心 + emergent_top 死扣 |
| A_PATH | `T_NO_TREND_DONE_CLEAR` 关 / cascade flag | 单核心 + cascade 死锁解（547/546） |
| READING_B | `T_READING_B` | 每级别独立腿 |

8 标的：BTC/CL/BRN/GC/DX/ES/QQQ/OKLO（沿 `project_t_engine_btc_baseline` 标的集）。3 模式：Structural/And/Or。

### 7.2 判据（formalization-validity-domain：可否证）

**判据 1（吃每级别 = 核心目标）**：READING_B 下 `per_level_long_pnl[k]` 和 `per_level_short_pnl[k]` **每级别都正**。
- 否证：若某级别 k 的 short_pnl 持续负 → 该级别做空失血未破（不是 regime 借口，是该级别消费有问题，沿 `cseg-dtop` 验收标准）。

**判据 2（破做空失血 539）**：READING_B 在 CL/BRN/ES（539 否证标的）的空头腿不再系统性失血。
- 对照：A_PATH/ANCHOR 在这些标的的 short_pnl_by_level（rec_engine.rs:280 已有字段）。
- 否证：若 READING_B short pnl 仍 ≪ 0 → 删 sink/recover 不足以破 539，做空失血根在别处（需重新 escalate）。

**判据 3（保住强牛）**：READING_B 在 BTC/OKLO（强牛标的）不踏空——leg_4/leg_top 在主升浪恒多。
- 度量：强牛标的最高级别腿的 long 持仓时长占比 + d_top[top] 触发次数（应罕见，因 Consolidation 过滤）。
- 否证：若 BTC READING_B ≪ BH 且最高腿频繁切换 → 条件2 Consolidation 过滤未生效或本级 d_top 仍误触发。

**判据 4（破 2/8）**：READING_B 的 P1（超 BH 标的数）> multiscale 的 2/8。
- 否证：若 ≤ 2/8 → 自相似递归 + d_top 精确切换相对手动 N 腿无增量，读法 B 假设被否证（缩小有效域边界，这是有价值的否定性结果）。

### 7.3 诊断观测（沿 count_chiral 先例，纯计数不改信号）

- `per_level_{long,short}_pnl[k]`：每级别盈亏画像（判据 1）。
- leg 切换次数 by level：查 d_top 消费频率（对照 construct 的 type1 计数）。
- 每级别腿持仓时长 by direction：查骑乘 vs 踏空（判据 3）。

---

## 8. 结果包六要素

1. **结论**：读法 B = consume 侧自相似递归算子 `g`（`apply_leg(k)` 单算子递归），对偶于 construct 的 T 算子 `f`。每级别 k 的独立腿骑 `levels[k]` 走势（ride）+ 消费 `d_top[k]`（switch），删 sink/recover（被每级别独立骑乘取代），单一 free 池 + 每级别仓位隔离。

2. **定义依据**：第65课 `aₙ=f(aₙ₋₁)`（construct 自相似，mod.rs:14）→ 对偶得 consume 自相似 `g`；走势方向由中枢定义（步骤b `segment_into_trends`）→ 腿方向；type1 走势终完美（步骤c `judge_divergence`）→ d_top 切换信号；区间套三必要条件（第27课，d_top 条件2 非 Consolidation）→ 精确切换。

3. **边界条件（结论翻转处）**：
   - 若 §7 判据 1 否证（某级别 short_pnl 持续负）→ 删 sink/recover 不足以破做空失血，读法 B 的「每级别独立骑乘取代跨级短差」假设被推翻。
   - 若判据 4 ≤ 2/8 → 自相似递归相对 multiscale 无增量，读法 B 整体被否证。
   - 若 d_top 在 top 级别恒 false（核心驻留生长走势，`cseg-dtop` 第二层根因），最高腿可能永不切换 → 需重审 d_top top 级别可计算性。

4. **下游推论**：若读法 B 成立 → (a) sink/recover/drain/ascend/emergent_top 单核心机制整体可删（H¹ 操作轴重写）；(b) 547/546/anchor 三谱系被「结构隔离」统一扬弃；(c) construct↔consume 对偶成为 H⁰↔H¹ 的形式化桥（设计文档 §0.4 分轴的具体实现）。

5. **谱系引用**：本设计触及多条已发生概念分离的谱系——547（cascade 翻错级别 `project_cascade_level_misattribution`，读法 B 用级别归属澄清规避）、546（僵尸核心死锁 545号结晶 + `project_deadlock_dual_open_target`，读法 B 用每级别独立腿消解）、539（根翻空有效域⊂非上行，`project_unn_root_flip_mtm`，读法 B 用「真走势向下才做空」规避字面翻空）、543（recover 全量 vs 配额开放轴，读法 B 删 recover 使此轴归空）、v2 三裁决（`project_recursive_t_architecture_v2`，资金继承）。

6. **影响声明**：本文档是 L0 设计，不改任何代码。实装将新增 `Leg` 结构 + `consume_legs`/`apply_leg`/`g` + `enable_reading_b` flag + per_level pnl 字段（rec_engine.rs），OFF 时 bit-exact 现有 `instances` 路径。影响模块：rec_engine.rs（主）、prove_guards.rs（新守卫 `prove_leg_isolation`）、backtest runner（READING_B 路径）。不改 construct（mod.rs/operator.rs/center.rs/trend.rs/divergence.rs 全不动）。

**相关文件（绝对路径）**：
- `/Users/silencehan/Projects/NewChanlun/rust/src/recursive_t/rec_engine.rs`（实装主体）
- `/Users/silencehan/Projects/NewChanlun/rust/src/recursive_t/mod.rs`（construct iterate，对偶参照，不改）
- `/Users/silencehan/Projects/NewChanlun/rust/src/recursive_t/prove_guards.rs`（守卫移植）
- `/Users/silencehan/Projects/NewChanlun/rust/src/recursive_t/operator.rs`（encapsulate，对偶参照，不改）
- `/Users/silencehan/Projects/NewChanlun/docs/bsp_consumption_redesign.md`（multiscale 旧设计，对照）

→ 接下来：本设计文档为纯 plan 产出，交付给编排者裁决「读法 B 是否进入实装」+「§4.1 资金配额候选 A/B 选择」+「§4.2 删 sink/recover 是否批准（涉及 543 开放轴归空）」三个选择类决断。这三项是价值判断（多合理方案），不可自决，须 escalate。

---

# Codex异质对审

源码核实完毕。现在开始逐点审查。

---

## Codex 异质审查：读法 B（T 对偶自相似递归）设计——六点判定

**审查模式：review（设计忠实度 + 逻辑自洽 + 可达性诚实）**

---

### 1. T 对偶是否真对偶

**判定：类比部分成立，但「严格对偶」声明有过度延伸。**

核实 construct `iterate`（mod.rs:83-115）：`apply_t` 是纯函数，四步骤 a-b-c-d 是**结构生成**操作（找中枢/识走势/判背驰/封装）。`encapsulate`（operator.rs:18）把走势压缩为上级单元，方向和振幅被保留，走势内部的逐 bar 运动被丢弃。

设计文档的对偶表核心主张：「construct 丢弃的 Σ|涨跌幅| 正是 consume 要捕获的」。这个类比**经济学上直观，但算子形式上不严格**。对偶在范畴论意义下要求函子，`f: S_k → S_{k+1}` 与 `g: (Trend_k, DTop_k, LegState_k) → LegState_k'` 的输入/输出空间异构——一个是单元序列空间，一个是（走势×信号×仓位）空间。这两个空间之间没有对偶态射（没有可逆函子把 `f` 的输入变成 `g` 的输出）。

**实质性问题**：`g` 的切换信号 `DTop_k`（设计文档 §1.1）定义为 `view.t1sell[k] || view.t1buy[k]`（rec_engine.rs:199-201 核实，两个字段确实存在）。但这是 construct **已产出**的 type1 信号——`g` 并不与 `f` 对偶，它是**寄生于** `f` 产出的信号消费者。寄生消费者和数学对偶是两件事。设计文档 §2.2 自己也说了「独立递归，共享深度坐标 r*」——这更准确，但和 §1.1 的「严格对偶」相矛盾。

**结论**：「对偶」作为设计隐喻成立（把 f 产出的信号消费掉），作为严格数学声明不成立。步骤 a（找中枢）和 d（封装）在 consume 侧没有对应，设计文档称之为「非对称残余」——这个承认是正确的，但「非对称残余」存在时，严格对偶的标签应改为「对应消费」或「配套信号消费算子」。不影响实装可行性，但影响向下游传递的声明准确性。

---

### 2. 自相似递归是否真自相似

**判定：核心主张为真，但迭代形式掩盖了一个实质性差异。**

construct `iterate`（mod.rs:88-104）是 `loop { apply_t(current, k) }`，每次迭代的**输入是上次迭代的输出**（`current = next`，line 104）——真自相似，`f(f(f(·)))`。

设计文档的 `apply_leg(k)` 骨架（§2.1）：

```
for k in 0..=r { g(k, view, c); }
```

这里每次迭代的输入**不是上次迭代的输出**——`g(k)` 写的是 `legs[k]`，`g(k+1)` 写的是 `legs[k+1]`，两者读写的是**不同的 `legs` 槽**，互不依赖。这是**并列独立更新**，不是 `f(f(f(·)))` 的组合。

construct 的自相似性：`aₙ = f(aₙ₋₁)`——第 n 次迭代以第 n-1 次迭代的产出为输入，信息向上传递、聚合。  
设计文档的 `apply_leg`：`leg_k' = g(k, view, leg_k)`——每个 k 的迭代只更新自己那一槽，不向相邻槽传递任何信息。

这是**每级别独立更新**，不是**迭代组合**。用 Rust 写出来本质是个 `for` 循环，和 multiscale 的「手动 N 个 LevelEngine」在计算结构上完全相同——只是写法从显式 N 份变成了一个 `for` 循环。这是代码组织的改进，**不是自相似递归语义的改进**。

设计文档的辩护（§2.1 结尾）：「loop vs 递归是同一自相似的两种写法」——这混淆了 construct 的 loop（`current = next`，上一步产出是下一步输入）和 apply_leg 的 loop（每步独立，无跨步传递）。两种 loop 形式上都叫循环，语义完全不同。

**影响**：「否定 multiscale 的手动 N 个 LevelEngine 展开」这一主张在代码组织层面成立（写一份比写 N 份好），在自相似递归语义层面不成立（每级别独立，与 multiscale 同构）。「多重赋格」的正确性不依赖于「自相似」标签——每级别独立腿的经济学逻辑可以独立成立，不需要借「T 的对偶」来论证。

---

### 3. 547 冲突：级别归属澄清是否成立

**判定：澄清在结构上成立，但有一个重要的前提问题需要核实。**

547 原文（`project_cascade_level_misattribution`）：次级别卖点把整个主力仓位翻空的根因是「`e_level >= cc` 太松」。读法 B 的回应：每级别腿各自是自己级别的主力，L1 信号只进 `g(1)`，物理上进不了 `legs[4]`。

这个澄清**结构上成立**。但有一个前置问题：`d_top[k]` 在什么条件下为 true？

从 rec_engine.rs:199-201（`t1buy[k]`/`t1sell[k]`）和 LevelView 的构造方式——这些字段由 construct 的 `iterate` + signal 流水线填充。如果 L1 走势完成，`t1buy[1]` 或 `t1sell[1]` 为 true，`d_top[1]` 为 true，`g(1)` 切换 `legs[1]`。**L4 的腿不受影响**。547 隔离在读法 B 的信号路由中确实物理成立。

但需要注意：设计文档的 `apply_leg` 是顺序循环（k=0 到 k=r），如果 `legs` 是共享数组且 `g(k)` 写 `legs[k]`，每级别的写入路径是隔离的。这需要实装时严格保证 `legs[k]` 只被 `g(k)` 写（设计文档 §6.4 提出的 `prove_leg_isolation` 守卫是正确的处置方式）。

**潜在漏洞**：设计文档 §4.2 说「删 sink/recover」但 A'' 路径（rec_engine.rs:865-878 `enable_trend_done_clear`）的 `clear_all` 是**全部级别的腿同时平掉**。如果读法 B 复用这个 `clear_all`，那么最高级别的 d_top 触发时，所有级别的 legs 会被同时清空——这是「高级别 d_top 触发低级别腿平仓」，是 547 逻辑在读法 B 版本下的复现。设计文档没有处理这个情况。

---

### 4. 做空失血（539）：读法 B 能否真正破

**判定：机制假设有内在逻辑，但核心风险被设计文档低估。**

539 的根因（`project_unn_root_flip_mtm` + `project_t_short_leg_regime_function`）：在强上行 regime 中，做空腿持续失血。multiscale 的失血来源是**机械 BSP 翻转**——每次次级别卖点触发 L_{k-1} 翻空，强牛主升浪中次级别 type1 卖点频繁出现，导致每级别都有短暂的空腿在主升浪中失血。

读法 B 的不同之处：骑走势方向（`levels[k].direction`），只有该级别走势**真的向下**时才做空。这比机械 BSP 翻转更保守。如果 L4 走势是 Up（上涨走势），`g(4)` 给 `legs[4]` 的方向是 Long——不做空，539 在 L4 层不触发。

但 539 在低级别（L1/L2）仍可能存在：强牛中 L1 走势会有短暂的 Down 段（调整），此时 `g(1)` 给 `legs[1]` 方向 Short，`legs[1]` 做空调整段——这段如果被 d_top[1] 切换及时，能吃到回调；如果 d_top[1] 在主升浪恢复后才触发，`legs[1]` 的空头就在主升浪中失血。

**关键问题**：低级别（L1/L2）的空头腿什么时候关闭？答案是「d_top[1] = true 时」，即 L1 走势终完美（底背驰）时。如果 L1 走势的背驰判断正确且及时（真正的调整段确认完成），空头腿会在正确时机平掉。但如果背驰判断有滞后（`cseg-dtop` 报告已指出「消费级别⊥贯通级别」的第二层根因），L1 空腿的关闭时机可能仍然有问题。

539 在读法 B 下不是被消除，而是**从跨级错配变成同级判断**——同级判断的准确性取决于 d_top 的精度，这是 L2/L3 待测问题。设计文档 §5「d_top 条件2 Consolidation 过滤」能帮助 L4 腿不在生长走势中做空，但低级别腿的 539 风险仍在。

---

### 5. multiscale 2/8 教训：是否实质区别

**判定：有一个实质区别（d_top vs 机械 BSP），但另一个区别（资金分配）被高估。**

**实质区别（d_top）**：multiscale 用机械 BSP 翻转——L_{k} 的任何卖点触发 `legs[k]` 翻空。读法 B 用 d_top——L_{k} 的走势完美（type1，背驰条件满足）才触发 `legs[k]` 切换。这个区别是实质的：背驰条件比 BSP 存在更保守，过滤了更多的中途伪信号。d_top 的条件2 Consolidation 过滤（`project_t_multiscale_independent_filters` 诊断）是读法 B 相对 multiscale 的主要优势。

**非实质区别（资金分配）**：设计文档声称 multiscale「INITIAL/8 平均预切」是问题，读法 B「单 free 池动态领取」是改进。从 `project_t_multiscale_independent_filters` 的 2/8 否证记录看，失败的主要原因是「机械 BSP 翻转踏空 + 做空失血」，**不是**资金预切分的问题。multiscale 如果改用 d_top 切换而不改资金分配，很可能也能改善。设计文档把资金分配改进也列为优势，但它不是 multiscale 失败的根因，属于过度归因。

**残留风险**：multiscale 的 L3 失败记录（`project_t_multiscale_independent_filters`）包含「CL/DX 2/8 是 regime 函数」——震荡 regime 下低级别高频翻转赚，趋势 regime 下做空失血。读法 B 的 d_top 在震荡 regime 中消费频率会相对高（走势短促，完美判断更频繁），在趋势 regime 中消费频率会相对低（走势延伸，Consolidation 过滤保住主升浪）。这个 regime 依赖性和 multiscale 同构，读法 B 并未消除这个依赖，只是把触发门槛从 BSP 提高到了 type1。

---

### 6. 可达性诚实评估

**判定：§7 验收方案结构良好，但三个可达性风险在设计文档中被低估。**

**风险 A（d_top top 级别稀疏）**：`project_pcf_1s_a0_source_collapse` 和 `project_unn_btc_spawn_throwback` 的记录表明，最高级别的信号在强牛中可能几年才触发一次。如果 d_top[r*] 只在 BTC 主牛市结束时触发一次，`legs[r*]` 在主牛中恒多（好事），但意味着 `legs[r*]` 永不切换——它退化为 buy-and-hold。这本身不是坏事，但设计文档把「吃每级别涨跌幅绝对值」作为目标，顶层腿永不切换意味着顶层没有「涨跌幅绝对值」被消费。

**风险 B（资金分散导致单腿规模小）**：N 个独立腿同时持仓，每腿从单一 free 池按「σ-不变配额」分配。如果 N=5（涌现 5 级），每腿约 free 的 1/λ≈1/3，第一腿建完 free 剩 2/3，第二腿建完 1/3，第三腿建完 0……更高级别腿无资金可建。设计文档的候选 A/B 配额规则「待 §7 验证」——但这个问题在 L0 层面就已经可以发现：σ-不变配额（每腿 1/3 free）在 N>3 时耗尽 free，设计文档没有处理 N>3 时的资金可行性。

**风险 C（clear_all 与腿隔离的矛盾）**：如上 §3 指出，A'' 路径的 `clear_all` 如果被读法 B 复用，会全清所有 legs，破坏 547 隔离声明。设计文档 §6.3 的 `liquidation_check` 接口不明确这里用的是全清还是单腿清。

---

## 总结判定

**是否可进入实装：有条件。须先解决三个前置问题，否则实装可能在 L3 的风险 B 和 C 处直接失败。**

| 审查项 | 判定 |
|--------|------|
| T 对偶严格性 | 类比成立，严格数学声明不成立。应改称「配套信号消费算子」。不阻断实装 |
| 自相似递归真伪 | 每级别独立更新，不是 `f(f(f(·)))` 组合。与 multiscale 计算结构同构，只是代码组织改进。不阻断实装，但「否定 multiscale」的论证需降级 |
| 547 冲突 | 信号路由隔离结构上成立，但 `clear_all` 路径破坏隔离（设计文档缺口）。**须修复才可实装** |
| 539 做空失血 | 机制假设合理，但低级别空腿的 539 风险仍在，d_top 滞后是未解的精度问题。L2/L3 可测，不阻断实装 |
| multiscale 区别实质性 | d_top vs 机械 BSP 是实质区别，资金分配改进是非实质区别。Regime 依赖性与 multiscale 同构未消除 |
| 可达性风险 | 风险 B（资金耗尽，N>3 时 σ-不变配额不可行）在 L0 层面已可发现，**须在设计层解决再进实装**。风险 A 和 C 须在实装中显式处理 |

**两个须前置解决的问题：**

1. **资金配额可行性（风险 B）**：若 N 个独立腿同时建仓，每腿 1/λ 配额在 N>λ 时耗尽 free。需在 §4.1 的资金设计中明确：(a) 最多允许同时持仓的腿数上限，或 (b) 改为总配额 1/N × free（每腿平均），或 (c) 按级别高低优先分配（高级别腿先领）。三个候选都有不同的 L3 含义，须在设计层裁决，不能留到实装后验。这是 L0 层面的代数可行性问题，不是 L2/L3 才能发现的问题。

2. **clear_all 与腿隔离的共存（风险 C）**：读法 B 路径在强平和 d_top 清仓时需要明确是「清全部 legs」还是「只清触发 d_top 那一级别的 leg」。前者破坏 547 隔离承诺，后者需要新的 campaign 边界语义（不再是整个账户的 campaign，而是每级别独立的骑乘周期）。这个语义选择影响三阶段会计的整个结构，是设计层决断，不是实装细节。

**影响声明**：本审查发现两个 L0 层面的设计缺口（资金可行性 + clear_all 边界），一个声明过度延伸（「严格对偶」→「配套消费算子」），一个计算结构错误（「自相似」→「每级别独立更新」）。前两个缺口须在设计文档补全后再进实装；后两个是标签问题，不影响实装路径。