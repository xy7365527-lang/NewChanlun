# 级别涌现决定 Root 方向：从缠论公理到引擎架构的严格推导

> 任务（2026-06-17 编排者）：形式化"级别涌现决定 root 方向"，不是补丁，是从必然性推出。
>
> **上游文档**（已结算）：
> - `orbit_enumeration_completeness.md`（D∞ 轨道枚举 / H⁰=58 / H¹=ℝ / NR-1…7 / 两层闭合）
> - `recursive_fugue_necessity_proof.md`（操作必然性推导链 / 四步 1-cycle 几何）
> - `dialectical_exhaustion.md`（dim H¹=1 / 辩证穷尽）
> - 谱系：540（递归双重性）、541（螺旋覆盖空间）、542（σ-不变）、543（操作 as word）
>
> **本文档的位置**：它是操作层「方向来源」的结构定理——把涌现定义、根方向定理、四步循环
> 双重会计、D∞ 覆盖空间位置三者焊接成单一推导链，并严格分析引擎当前实装与理论的差距。

---

## 第 0 部分：认识论前置（强制）

### 0.1 等级标注

| 区段 | 内容 | 等级 | 信息增量 |
|------|------|------|----------|
| §1 | 级别递归构造形式定义 | **L0**（缠论公理推导） | 零（同义反复，但焊接定义） |
| §2 | Root 方向涌现定理 | **L0**（代数+已结算定理） | **正**（消除「预设 root」假设） |
| §3 | 四步跨级别循环双重会计 | **L0**（群论 word 分析） | **高**（辨清 Cycle A vs Cycle B） |
| §4 | 在 D∞ / H⁰ / H¹ 的坐标 | **L0**（覆盖空间坐标） | **正**（把原文操作精确到群论位置） |
| §5 | 引擎差距与架构推论 | **L1**（代码分析，非经验验证） | **高**（可操作的修改方案） |
| §6 | 结果包六要素 | — | — |

### 0.2 核心有效域警告

本文档的结论属于 **L0 结构必然性**（从缠论定义+群论推导，不依赖经验数据）。
「根方向是涌现属性」是 L0 命题，不是 L2 假设。
但「每次涌现都能产生可操作信号」是 regime-gated 的 L2 命题（单边上扬时 H¹ 幅度→0，026:80）。
本文只声明前者。

---

## 第 1 部分：级别递归构造的形式定义

### 1.1 基础层 a₀（笔 / bi）

**定义（a₀）**：笔是满足缠论笔定义的最小价格往返单元：
- 顶笔：上凸的 K 线序列，端点满足非包含关系
- 底笔：下凸的 K 线序列
- 最短有效笔：相邻分型间至少 5 根 K 线（含端点分型）

笔的**方向** dir(a₀) ∈ {Up, Down}：从底分型到顶分型为 Up，反之为 Down。

此级别记为 Level-0（a₀ = L0 基元）。

### 1.2 线段 a₁（Segment）

**定义（a₁）**：线段是至少 3 根同向笔构成的序列，其特征序列满足线段划分标准（第67/71/77/78课）。

线段方向 dir(a₁) = dir(第一根笔)。

### 1.3 走势类型 Move(k)（Level-k 走势）

**递归定义**（缠论核心）：

**基础情形（k=1）**：
- Level-1 走势 = 至少 3 根相邻 Level-0 笔 + 其中存在至少 1 个中枢
- 中枢(k=1) = 连续三段及以上线段的价格重叠区间 [ZD, ZG]
- Move(1) 的方向 dir₁ = 中枢配置确定的方向（上升中枢序列=Up，下降=Down）

**递归情形（k≥2）**：
- Level-k 走势 = 以 Level-(k-1) 走势为原子单元构成的走势结构
- Move(k) 的**构成单元** = Move(k-1) 序列
- 中枢(k) = 连续三个 Move(k-1) 的价格重叠
- **关键等式**：Move(k) ≡ Level-(k+1) 的一根笔

最后一行是级别递归的**核心恒等式**：走势类型 = 上一级别的一根笔。这不是比喻，是结构定义：Level-(k+1) 的笔定义域 = Level-k 走势序列。

### 1.4 涌现上界 r*(t)

**定义**：在时刻 t，
$$r^*(t) = \sup\{k \geq 1 : \exists \text{ 已形成的完整 Move}(k) \text{ at or before } t\}$$

若无任何已形成走势，r*(t) = 0。

**涌现性质**：
- r*(t) 随时间单调不减（走势形成是不可撤销的历史事件）
- 但「当前活跃」的最高级别走势可以是 r*(t)-1（若最高级别走势已完成转向，当前在构建新走势中）
- 引擎中：`h0.emergent_ceiling()` = `max_l` = `max_ladder + 1` = 活跃信号的上界

**信号层对应**（L1，代码映射）：
```rust
// spiral/signal.rs line 189
let max_l = (sig.max_ladder as usize + 1).min(MAX_LADDER);
// ↑ 这是 r*(t)+1（上界封顶），直接从引擎产出的 max_ladder 读取
```

---

## 第 2 部分：Root 方向作为涌现属性

### 2.1 根方向函数

**定义（D1：根方向函数）**：
在时刻 t，给定已涌现的最高活跃级别 r*，根方向为：

$$\text{root\_dir}(t) = \text{dir}(\text{Move}(r^*(t))) \in \{Up, Down\}$$

其中 dir(Move(k)) 是当前 Level-k 走势的方向。

**关键：根方向不是常量。** 它是关于 t 的函数：
- 当更高级别走势涌现时（r* 增加），root_dir 可能改变
- 当当前最高级别走势完成（型变）时，root_dir 改变

**信号层对应**（L1）：
```rust
// spiral/signal.rs line 133+182-187
pub dir_state: [Option<Direction>; MAX_LADDER],

// process() 中方向更新：
for lad in 0..MAX_LADDER {
    if let Some(d) = flip_edge[lad] {
        self.dir_state[lad] = Some(d); // ← root_dir 的更新来源
        self.anchor_state[lad] = bar;
    }
}
```
`dir_state[k]` = 当前 Level-k 走势方向。这是 root_dir 的载体。

### 2.2 根方向非预设定理（P1）

**命题 P1**：入场操作（F 步骤）不「决定」根方向；入场是读取当前涌现结构的观测行为。

**证明**：

（1）F 步骤触发条件 = `obs.buy_source()` 返回 Some(s)，即区间套定位链已武装到 s 级别的 type1 买点已 confirmed。

（2）区间套定位链武装到 s 级别的 type1 买点，等价于：Level-s **最后完成走势方向为 Down**（第三段下跌背驰触发买点；`trend_last_move_dir()` 返回 Down，门控 `dir_state[s]=Down`）。

（3）因此 F 入场时，`h0.direction(buy_source) == Some(Down)` 是必然成立的前提条件（不是巧合，是类型保证）。

（4）F 步骤设置 `core_polarity = Polarity::Long` 是对 `h0.direction(buy_source) = Some(Down)` 的读取，不是预设。

**推论 C1（不变量）**：在有仓位期间，恒成立：
$$\text{core\_polarity} = f(\text{h0.direction(core\_ladder)})$$
其中 f(Down) = Long（买点在下跌末端），f(Up) = Short（卖点在上涨末端）。

这是隐式不变量——代码当前未显式断言（见 §5 差距分析）。

### 2.3 σ-ascend 是根方向跟踪机制

当更高级别走势在同方向涌现时，核心仓归属级别上移：

```rust
// fugue_v3/operate.rs: core_emergent_ladder()
while lad + 1 < ceiling
    && h0.direction(lad + 1) == Some(expected_dir)  // ← 读涌现结构
    && h0.anchor(lad + 1) >= core_entry_bar
{ lad += 1; }
```

`expected_dir` 来自 `core_polarity`——正是 §2.1 中的 root_dir 同方向检查。这不是「预设的」级别爬升，而是每 bar 重新从形态学轴读取当前涌现结构。

**因此**：σ-ascend（T5/A5 第20环）的实装已经体现了「root方向是涌现属性」——它通过 `h0.direction(lad+1)` 实时读取，而非依赖预存的高位级别参数。

---

## 第 3 部分：四步跨级别循环的双重会计学

### 3.1 两种循环的辨析（重要区分）

**Cycle A（原文描述，用户洞察链）**：完整的跨级别进出：
```
Long@k 持仓
  → Step 1：本级别卖点 → 全平多仓（C-clear，了结本级别）
  → Step 2：次级别入场做空（F-short @ k-1，次级别短差开始）
  → Step 3：次级别买点 → 全平空仓（C-clear @ k-1，了结次级别）
  → Step 4：本级别买点 → 重新做多（F-long @ k，回到本级别）
```
这是缠论原文「先卖后买，降低成本」的全仓版本。

**Cycle B（引擎当前实装，E/D 机制）**：并行的跨级别对冲：
```
Long@k 持仓（核心仓，从不全清）
  → E（sink = σ⁻¹∘τ）：次级别卖信号 → +Short@k-1（1/λ 份额）
  → D（recover = σ∘τ）：次级别买信号 → -Short@k-1（平空，回到纯Long@k）
```
这是缠论「成本降低」的部分仓版本——核心仓（多数）持续持有，机动仓（1/λ）循环短差。

### 3.2 两种循环的 D∞ 表达

在螺旋坐标 (φ, r, ε) 中：

**Cycle A 的 D∞ path**：
| 步骤 | 操作 | D∞ word | 坐标变化 |
|------|------|---------|---------|
| 初始 | Long@k | — | (φ₀, k, +1) |
| Step 1 | C-clear | τ@k（边界算子） | (φ₀, k, +1) → flat |
| Step 2 | F-short@k-1 | β⁻@k-1 | flat → (φ₀, k-1, -1) |
| Step 3 | C-clear@k-1 | τ@k-1 | (φ₀, k-1, -1) → flat |
| Step 4 | F-long@k | β⁺@k | flat → (φ₀, k, +1) |

Cycle A 的净 word = τ@k · β⁻@k-1 · τ@k-1 · β⁺@k。Δr = 0（回到原级别）但穿越了两个级别。

**Cycle B 的 D∞ path**：
| 步骤 | 操作 | D∞ word | 效果 |
|------|------|---------|------|
| 初始 | Long@k | — | (φ₀, k, +1)，全仓 |
| E（sink） | σ⁻¹∘τ | 1/λ 份额下沉 | +Short@k-1（并行），核心 Long@k 不变 |
| D（recover） | σ∘τ | 1/λ 份额回升 | -Short@k-1（平空），回到纯 Long@k |

Cycle B 的 E+D net word = σ⁻¹τ · στ = σ⁻¹ · (τ·σ·τ) = σ⁻¹ · σ⁻¹ = σ⁻² → 这是层内两次往返，H₁ 1-cycle。

### 3.3 为何两种循环都是「跨级别的」

**Cycle A** 的跨级别性是显然的：步骤 1→2 从 k 降到 k-1，步骤 3→4 从 k-1 升回 k。

**Cycle B** 的跨级别性稍微隐蔽：
- E（σ⁻¹∘τ）= 进入 k-1 级别的短差，意味着从 Long@k 视角读到了 k-1 级别的卖信号
- D（σ∘τ）= 从 k-1 级别买信号触发，意味着 k-1 级别的下降走势完成
- 整个 E+D 循环在 k 和 k-1 之间「咬合」

这正是用户洞察链中「两个级别的齿轮咬合」的精确群论表达。

### 3.4 双重会计性（每步的两个级别含义）

每个操作步骤**同时**在两个级别上有会计含义（τ 双重投影，reinterp §1/§5）：

| 操作 | 本级别会计 | 子级别会计 |
|------|-----------|-----------|
| **C** (τ@core_ladder) | 结清本级别多头仓位 | 打开次级别做空窗口 |
| **E** (σ⁻¹∘τ) | 本级别多头锁定（不变） | 次级别做空 1/λ 份额 |
| **D** (σ∘τ) | 本级别多头解锁（不变） | 次级别空头平仓 |
| **F** (β⁺@source) | 本级别新建多头 | 确认次级别买点有效 |

「没有同级别翻转」的含义：在一个走势类型内部，C 总是触达顶端（sell1 confirmed at core_ladder），不可能在中途同级别翻转。这是缠论 BSP 三类型的结构保证（T49 向心确认）。

---

## 第 4 部分：在 D∞ / H⁰ / H¹ 覆盖空间的精确坐标

### 4.1 H⁰（形态学轴）= 级别结构本身

H⁰ = 58 个不变量（T49/T50 等），其中**与根方向直接相关的**：

- **T5 涌现**（第20环）：更高级别走势在同方向涌现 → core_emergent_ladder 上移（σ-ascend）
  - 对应 `h0.direction(lad+1) == Some(expected_dir)` 检查
- **T49 向心 confirm**：从高向低的区间套定位确认，保证买卖信号方向与级别结构一致
  - 对应 `obs.buy_source()` 返回的 source 级别
- **T50 成本**（θ门控）：子级别振幅参照，决定 E 操作的有效域
  - 对应 `h0.theta(sub)` 读取

H⁰ 的核心作用：**提供方向信息的客观源头**。`MorphologyAxis.direction(ladder)` 直接暴露每个级别的当前走势方向，这是 root_dir 的唯一合法读取来源。

### 4.2 groupoid（观察轴）= 信号确认的态射

groupoid 层（`ObserveAxis`）提供：
- `buy_source()` / `sell_source()`：区间套定位链确认的最高 source 级别
- `nf_buy(ladder)` / `nf_sell(ladder)`：向心 confirm 传递信号

**根方向与 groupoid 的关系**：
- F 入场时，`buy_source()` 必然满足 `h0.direction(buy_source) = Some(Up)`（类型保证）
- C 清仓时，`sell_source()` 必然满足 `h0.sell1(sell_source) = true`（T49 保证）

groupoid 是「读取根方向的接口」，不是「存储根方向」的地方。

### 4.3 H¹（操作轴）= D∞ word 处理器

H¹(D∞, ℝ₋) = ℝ，单生成元 Δr = -1（跨级别闭合率）。

**dim H¹ = 1 的操作含义**：
- 机动仓的所有操作路线都在同一个 1 维结构上（没有第二个独立维度）
- 这意味着：E/D 的「哪个级别」不增加自由度——所有级别的 E/D 循环都等价于 Δr = -1 生成元的倍数
- σ-等变性：Level-k 和 Level-k+1 的 E/D 循环是同一生成元的「σ 上推」，结构相同

**根方向在 H¹ 中的位置**：
- 根方向 ε ∈ {±1} 是 H¹ 模的「系数的手性」
- Cycle B（E/D 循环）的 Long root：ε = +1，生成元方向 Δr = -1 作用于 ℝ₊ 值
- Cycle B 的 Short root：ε = -1，生成元方向 Δr = -1 作用于 ℝ₋ 值（D∞ 的 ε 对称性）

这正是引擎注释中「ε 对称双向 root」（operate.rs L1–L17）的群论精确化。

### 4.4 两层闭合 H⁰ ⊕ H¹（cd_ℚ = 1）

由 `orbit_enumeration_completeness.md` §4 结论（cd_ℚ(D∞) = 1，两层闭合）：

- **H⁰ 层**（核心仓）= 58 不变量的「稳定分量」——走势方向、锚点、级别本身
- **H¹ 层**（机动仓）= Δr = -1 生成的「循环分量」——E/D 跨级别短差循环

**根方向的归属**：
- 根方向 ε 是 H⁰ 属性（来自形态学公理，不是 H¹ 操作的产物）
- H¹ 操作（E/D/C）消费根方向，但不产生根方向
- 这就是「根方向是涌现属性，不是操作产物」的代数精确化

**无 H² 的含义**：cd_ℚ = 1 保证不存在第三层操作结构。操作空间完全被 H⁰（固持）⊕ H¹（循环）穷尽，没有「第三种操作」的余地。

---

## 第 5 部分：引擎差距与架构推论

### 5.1 当前引擎的架构分析

**现状**（L1，代码映射）：

```rust
// fugue_v3/operate.rs
pub struct OperateEngine {
    core_polarity: Polarity,  // ← 预存的根极性
    core_ladder: usize,       // ← 根方向所在级别
    ...
}
```

`core_polarity` 的生命周期：
- 设置：F 入场时，`Polarity::Long`（buy_source）或 `Polarity::Short`（sell_source）
- 读取：D/E/C/σ-ascend 每步都读
- 清除：C 清仓后概念上无效（但 `has_position()` 保证不会被读）

**隐式不变量**（当前未断言）：
```
∀ bar 使得 has_position() = true：
  core_polarity = f(h0.direction(core_ladder))
  其中 f(Some(Up)) = Long, f(Some(Down)) = Short
```

### 5.2 差距分析

**差距 G1：core_polarity 的不变量未显式验证**

当前 `core_polarity` 从入场信号类型设置（buy_source → Long），不从 `h0.direction(source)` 直接读取。两者应该等价，但没有运行时断言：

```rust
// 当前代码（operate.rs F 入场部分）：
self.core_polarity = Polarity::Long;  // 从信号类型推断

// 严格做法：
let dir = h0.direction(s).expect("F 入场时 source 级别应有方向态");
let polarity = match dir {
    Direction::Up => Polarity::Long,
    Direction::Down => Polarity::Short,
};
self.core_polarity = polarity;  // 从 h0 直接读取
```

如果信号层有 bug（buy_source 在无 Up 方向的级别武装），当前代码不会发现，严格做法会 panic。

**差距 G2：σ-ascend 后 core_polarity 与 h0.direction(core_ladder) 的同步**

σ-ascend 上移 `core_ladder`（relabel 无交易），但 `core_polarity` 不变（这是对的——方向不变）。但新的 `core_ladder` 处的 `h0.direction(core_ladder)` 是否必然等于原来的 `core_polarity`？

从 `core_emergent_ladder()` 代码：
```rust
while lad + 1 < ceiling
    && h0.direction(lad + 1) == Some(expected_dir)  // ← 只有方向匹配才爬升
    && h0.anchor(lad + 1) >= core_entry_bar
{ lad += 1; }
```

是的，σ-ascend 只在方向相同时爬升，所以爬升后 `h0.direction(new_core_ladder) == Some(expected_dir)` 必然成立。这个不变量是**结构性保证**的，不需要额外断言。

**差距 G3：Cycle A（全仓进出）vs Cycle B（核心仓+机动仓）的选择未形式化**

当前引擎实装 Cycle B，但缠论原文的「先卖后买」描述更接近 Cycle A。两者的关系：

- **Cycle A 是 Cycle B 在 λ=1 时的极限情形**：当机动仓比例 f=1/λ=1 时，全仓都是「机动仓」，E 操作 = 全平，D 操作 = 全仓做回来，退化为 Cycle A。
- 当前引擎 λ > 1（每次 τ 只转移 1/λ 份额），所以实装的是 Cycle B，核心仓始终持有。
- 这不是 bug，是设计选择（H⁰⊕H¹ 两层分离，T48 守恒不要求全平）。

**此差距需要形式化**（而非修复）：在引擎文档中明确声明「λ > 1 时为 Cycle B；极限 λ → 1 退化为 Cycle A」。

**差距 G4：MorphologyAxis 缺少 root_direction() 便利方法**

`h0.direction(h0.emergent_ceiling() - 1)` 是计算根方向的两步操作，没有单一接口：

```rust
// 建议在 MorphologyAxis 添加：
fn root_direction(&self) -> Option<Direction> {
    let ceiling = self.emergent_ceiling();
    (0..ceiling).rev().find_map(|k| self.direction(k))
}
```

这是便利方法，不是缺失功能。但添加后可以：
1. 在 F 入场时显式从 root_direction() 读取极性（修复 G1）
2. 在每步提供一个可断言的「当前根方向」供调试

### 5.3 架构推论：严格修改方案

**修改 A（必要，修复 G1）**：在 F 入场时从 `h0` 读取方向，不从信号类型推断：

```rust
// 在 F 入场逻辑（operate.rs）中：
fn enter_long(&mut self, s: usize, bar: i64, price: f64, h0: &dyn MorphologyAxis) {
    // 严格：从 h0 读取方向，而不是假设 buy_source → Long
    let dir = h0.direction(s).expect(
        &format!("prove_f_direction_source: F 入场 source={s} 级别应有方向态（h0 pre-condition）")
    );
    self.core_polarity = match dir {
        Direction::Up => Polarity::Long,
        Direction::Down => Polarity::Short,
    };
    self.core_ladder = s;
    self.core_entry_bar = bar;
    // ... 仓位建立
}
```

这把「root 方向来自涌现结构」从隐式约定变为显式代码路径。

**修改 B（可选，增强可观测性，修复 G4）**：为 `MorphologyAxis` 添加 `root_direction()` 默认实现：

```rust
pub trait MorphologyAxis {
    // 现有方法...
    
    /// 根方向：最高活跃级别的当前走势方向（默认实现，可 override）。
    fn root_direction(&self) -> Option<Direction> {
        let ceiling = self.emergent_ceiling();
        (0..ceiling).rev().find_map(|k| self.direction(k))
    }
}
```

**修改 C（形式化，非代码变更，修复 G3）**：在引擎文档中添加：

```
// H¹ 机动仓循环参数 λ：
// λ → 1：Cycle B 退化为 Cycle A（全仓进出，原文「先卖后买」极限）
// λ = 3（当前）：核心仓 2/3 持续，机动仓 1/3 循环（T50 / 1/3 原则）
// λ-不变性：H¹ 生成元 Δr=-1 在所有 λ 值下结构相同，只有仓位分配不同
```

### 5.4 「根voice不应该存在」的精确含义

用户洞察链第 5 条：「根voice不应该存在：没有预设的 root，方向从涌现的级别结构中读取。」

精确解读（L0）：

1. **「根voice」指的是预设的方向参数**，不是「不能有根仓位」。F 入场必然建立一个顶层仓位（核心仓），这不是「根voice」的问题。

2. **「不应该存在」的是**：把根方向作为引擎初始化参数（如`engine.new(direction=Long)`）传入。当前引擎已经是「零操作参数」（`FugueEngineCore::new(floor_ladder)` 只接受结构递归基，不接受方向）——这个设计已经正确。

3. **剩余的「预设」形式**：`core_polarity` 存储字段看起来像预设，但实际上是「入场时读取一次的涌现快照」。差距 G1 的修复让这个语义更明确：把它变成「每次都从 h0 计算」而非「入场时存储」。

4. **更彻底的「无预设」形式**：把 `core_polarity` 完全改为方法：
   ```rust
   fn core_polarity(&self, h0: &dyn MorphologyAxis) -> Polarity {
       match h0.direction(self.core_ladder) {
           Some(Direction::Up) => Polarity::Long,
           Some(Direction::Down) => Polarity::Short,
           None => panic!("prove_polarity_defined: 有仓位时 core_ladder 方向态应存在"),
       }
   }
   ```
   这完全消除预存字段，每次使用时从 h0 实时读取。代价：每步多一次 trait 调用（可忽略）。收益：「根方向 = 涌现结构快照」的语义变为代码事实。

---

## 第 6 部分：结果包六要素

### 6.1 结论

1. **Root 方向是 H⁰ 属性，不是操作参数**：`h0.direction(core_ladder)` 是根方向的唯一合法源头，`core_polarity` 字段是其存储快照。

2. **两种跨级别循环在 λ=1 极限处统一**：Cycle A（全仓进出）是 Cycle B（核心仓+机动仓）在 λ→1 时的退化，当前 λ=3 的 Cycle B 是严格实装，不是近似。

3. **三个架构修改**：
   - 修改 A（必要）：F 入场从 `h0.direction(s)` 读极性
   - 修改 B（可选）：`MorphologyAxis.root_direction()` 便利方法
   - 修改 C（形式化）：文档化 λ-不变性

### 6.2 定义依据

- 级别递归定义：缠论 §017:40 走势分解定理一（每段走势=次级别走势序列）
- H¹(D∞, ℝ₋) = ℝ：`orbit_enumeration_completeness.md` §4 A1 定理
- dim H¹ = 1：`dialectical_exhaustion.md` §1
- σ-ascend T5/A5：`recursive_fugue_necessity_proof.md` L4
- cd_ℚ = 1 两层闭合：`orbit_enumeration_completeness.md` §3.3

### 6.3 边界条件

本文档结论翻转的条件：

| 结论 | 翻转条件 |
|------|---------|
| core_polarity = f(h0.direction(core_ladder)) 是不变量 | 若信号层在无 Up 方向的级别发出 type1 buy BSP（信号层 bug） |
| Cycle B 严格正确 | 若缠论原文明确要求「全仓清空后才能做反向」（Cycle A 唯一合法）→ 需核查026:80原文 |
| σ-ascend 维护不变量 | 若 `h0.direction(lad+1)` 在爬升发生后立即变为 None（信号层时序 bug） |
| 根方向是 H⁰ 属性 | 若存在不经过 H⁰ 信号层就产生的方向变化（架构越权） |

### 6.4 下游推论

1. **prove_f_direction_source**：修改 A 使 F 入场多一个硬断言，任何「方向不存在的级别武装 BSP」会立即 panic，比现在更早暴露信号层 bug。

2. **core_polarity 字段冗余性**：若实施修改 A 的「彻底形式」（改为方法），`core_polarity` 字段消失，`OperateEngine` 不再存储冗余的方向快照。这简化了状态机。

3. **ε 对称性（fugue_v3 注释已声明）**的理论基础得到补全：ε 对称 = Long root 和 Short root 使用相同 H¹ 生成元，只是 ε ∈ {±1} 不同。这由 `axis.rs StepOutcome` 注释「ε 对称双向 root」已正确表达。

4. **无第三种操作**的保证：cd_ℚ = 1 → H⁰⊕H¹ 两层闭合 → 操作空间被穷尽 → 不会出现「三级别同时持仓但不是 H¹ 结构」的意外情况。

### 6.5 谱系引用

- **541号**（螺旋覆盖空间，已结算）：(φ, r, ε) 坐标定义 + Burnside cascade 506→46→2→1
- **543号**（操作 as word，已结算）：操作是 D∞ 的 word，不是硬编码循环
- **540号**（递归双重性，已结算）：同一段在不同级别的双重身份（构成单元 / 次级别内容）
- **542号**（σ-不变，已结算）：f=1/λ 的 σ-不变性，即机动仓比例与级别无关

暂无与「根方向涌现性」直接相关的已结算谱系——本文档可能触发新谱系写入（见 §6.6）。

### 6.6 影响声明

**本文档改动**：
- 新建 `docs/emergent_level_direction.md`（本文件）
- 未修改任何代码文件

**建议修改（需独立 commit）**：
- `rust/src/fugue_v3/operate.rs`：F 入场时从 `h0.direction(s)` 读极性（修改 A）
- `rust/src/fugue_v3/axis.rs`：添加 `MorphologyAxis::root_direction()` 默认方法（修改 B）

**不需要修改**：
- 三轴 trait 分离设计（`axis.rs`）——已正确
- `core_emergent_ladder()` 实现——已正确实现 σ-ascend
- `StepOutcome` ε 对称结构——已正确
- `FugueEngineCore::new()` 零参数设计——已正确实现「无预设 root」

---

*文档认识论等级：全文 L0（形式推导）+ L1（代码映射）。无经验数据支撑，不需要 L2/L3 验证。*
