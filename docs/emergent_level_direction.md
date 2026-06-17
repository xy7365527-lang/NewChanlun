# 级别涌现决定 Root 方向：从缠论公理到引擎架构的严格推导

> 任务（2026-06-17 编排者）：形式化"级别涌现决定 root 方向"，不是补丁，是从必然性推出。
>
> **上游文档**（已结算）：
> - `orbit_enumeration_completeness.md`（D∞ 轨道枚举 / H⁰=58 / H¹=ℝ / NR-1…7 / 两层闭合）
> - `recursive_fugue_necessity_proof.md`（操作必然性推导链 / 四步 1-cycle 几何）
> - `dialectical_exhaustion.md`（dim H¹=1 / 辩证穷尽）
> - 谱系：540（递归双重性）、541（螺旋覆盖空间）、542（σ-不变）、543（操作 as word）
>
> **本文档的位置**：它是操作层「方向来源」的**结构定理**——把涌现定义、根方向定理、走势方向的
> 双时相、级别正交性（踏空免疫）、四步循环双重会计、D∞ 覆盖空间位置焊接成单一推导链，
> 并据此推出引擎的严格实装（MorphologyAxis 级别涌现）。
>
> **本次重写动机**（v2，opus）：上一版（v1）把「root 方向来自涌现结构」论证为**已经正确**
> （声称 `dir_state` 即走势方向）。这是错的。`dir_state` 来自 `flip_edge` = `trend_last_move_dir`
> = **走势内部最后一个 move 的方向**（`lib.rs:1160`），它在走势内 move 交替时频繁翻转，是噪声，
> 不是走势方向。用它做 root 方向 = 用次级别瞬时方向冒充本级别走势方向 = **级别错配**。
> 这是 ε 对称实装在 ES（−213%）/GC（−122%）踏空的真正根因。本文从必然性推出修复。

---

## 第 0 部分：认识论前置（强制）

### 0.1 等级标注

| 区段 | 内容 | 等级 | 信息增量 |
|------|------|------|----------|
| §1 | 级别递归构造形式定义 | **L0**（缠论公理推导） | 零（同义反复，但焊接定义） |
| §2 | 走势方向的双时相（确立/终结） | **L0**（缠论定义） | **正**（辨清方向 ≠ 尾 move 方向） |
| §3 | Root 方向涌现定理 + 级别正交性 | **L0**（代数+已结算定理） | **高**（踏空免疫的群论形式） |
| §4 | 四步跨级别循环双重会计 | **L0**（群论 word 分析） | **高**（Cycle A vs Cycle B） |
| §5 | D∞ / H⁰ / H¹ 坐标 | **L0**（覆盖空间坐标） | **正** |
| §6 | 引擎实装（MorphologyAxis 级别涌现） | **L1**（代码方案，bit-exact 等价） | **高**（可执行修改） |
| §7 | 踏空失败模式的群论解剖 | **L0**（结构）+ **L3**（经验数据 ES/GC） | **高**（否定性结果定位根因） |
| §8 | 结果包六要素 | — | — |

### 0.2 核心有效域警告

本文档的**结构结论**（§1–§5）属于 **L0 结构必然性**（从缠论定义+群论推导，不依赖经验数据）：
- 「root 方向是涌现属性」是 L0 命题。
- 「方向翻转算子只作用于对应级别（级别正交性）」是 L0 命题。

但「每次涌现都能产生可操作信号」是 **regime-gated 的 L2 命题**（单边上扬时 H¹ 幅度→0，026:80）。
回测 alpha 是 **L3**（真实数据，可否证）。§7 引用的 ES/GC 踏空是 L3 否定性结果——它**定位了根因**
（级别错配），但「修复后是否跑赢」仍待新一轮 L3 验证，本文不预言。

---

## 第 1 部分：级别递归构造的形式定义

### 1.1 基础层 a₀（笔 / bi）

**定义（a₀）**：笔是满足缠论笔定义的最小价格往返单元：
- 顶笔：上凸的 K 线序列，端点满足非包含关系；底笔：下凸。
- 最短有效笔：相邻分型间至少 5 根 K 线（含端点分型）。

笔的**方向** dir(a₀) ∈ {Up, Down}：从底分型到顶分型为 Up，反之为 Down。此级别记为 Level-0。

### 1.2 线段 a₁（Segment）

**定义（a₁）**：线段是至少 3 根同向笔构成的序列，特征序列满足线段划分标准（第67/71/77/78课）。
线段方向 dir(a₁) = dir(第一根笔)。引擎级别 ladder：LADDER_SEG = 2 = FIRST_BSP_LADDER（笔中枢承载层）。

### 1.3 走势类型 Move(k)（Level-k 走势）

**递归定义**（缠论核心，017:40 走势分解定理一）：

**基础情形（k=1）**：
- Level-1 走势 = 至少 3 根相邻 Level-0 笔 + 其中存在至少 1 个中枢。
- 中枢(1) = 连续三段及以上线段的价格重叠区间 [ZD, ZG]。
- Move(1) 的方向 dir₁ = 中枢配置确定的方向（上升中枢序列=Up，下降=Down）。
- 引擎 ladder：LADDER_MOVE = 3 = PENDING_LO（势源下界）。

**递归情形（k≥2）**：
- Level-k 走势 = 以 Level-(k-1) 走势为原子单元构成的走势结构。
- Move(k) 的**构成单元** = Move(k-1) 序列；中枢(k) = 连续三个 Move(k-1) 的价格重叠。
- 引擎 ladder = k + 2（递归层 lid → ladder = lid+2；`organic_signals.py:398`）。
- **关键等式**：$\text{Move}(k) \equiv \text{Level-}(k{+}1)\text{ 的一根笔}$

最后一行是级别递归的**核心恒等式**（洞察3）：走势类型 = 上一级别的一根笔。这不是比喻，是结构
定义：Level-(k+1) 的笔定义域 = Level-k 走势序列（谱系540：同一段在不同级别的双重身份）。

### 1.4 涌现上界 r*(t)

**定义**：在时刻 t，
$$r^*(t) = \sup\{k \geq 1 : \exists\ \text{已形成的完整 Move}(k)\ \text{at or before}\ t\}$$
若无任何已形成走势，r*(t) = 0。

**涌现性质（洞察4：最大级别是涌现的，不是选的）**：
- r*(t) 随时间单调不减（走势形成是不可撤销的历史事件——append-only）。
- 从 1s 笔自下而上递归构成：a₀ → Move(1) → Move(2) → …，每一层是下一层的笔。
- 引擎中 `h0.emergent_ceiling() = max_l = max_ladder + 1`（`signal.rs:189`）= r*(t)+1（上界封顶）。
- `max_ladder` 由递归引擎实时上报（`organic_signals.py:401-402`：递归层涌现即提升）。

**关键澄清**：r* 是「最大级别」的**结构涌现**（哪个级别已形成走势），与该级别走势的**方向**是
两个正交的量。max_ladder 只给级别数，**不给方向**——方向来源见 §2。

---

## 第 2 部分：走势方向的双时相——方向 ≠ 尾 move 方向

> 这是本次重写的核心增量。v1 的根本错误：把「尾 move 方向」当成「走势方向」。

### 2.1 走势方向是走势级稳定量（定理 D-STABLE）

**定理 D-STABLE（L0）**：Move(k) 的方向 dir(Move(k)) 是 Move(k) 的**整体属性**，在 Move(k)
存续期间**恒定**——它不随 Move(k) 内部的 Move(k-1) 子单元方向摆动而改变。

**证明**：dir(Move(k)) 由中枢配置定义（§1.3：上升中枢序列=Up）。一个上涨走势 Move(k) 内部
必然包含上涨子段（向上 Move(k-1)）与回调子段（向下 Move(k-1)）交替。中枢配置在走势完成前
单调（上升走势的中枢依次抬高），故 dir(Move(k)) 在存续期恒为 Up，与内部子段方向无关。∎

### 2.2 双时相：确立与终结（定理 D-PHASE）

**定理 D-PHASE（L0）**：走势方向 dir(Move(k)) 只在两个时相事件改变值：

| 时相 | 事件 | 方向变化 |
|------|------|---------|
| **确立** | Move(k) 形成（第一个中枢 + 第三段同向，r* 达到 k） | 方向首次定为 ±1 |
| **终结** | Move(k) 背驰确认（type1 BSP @ k 向心 confirm） | 方向翻转 ±1 → ∓1 |

在两事件之间，方向是不动点（D-STABLE）。**终结即下一走势的确立**：Move(k) 的顶背驰确认
（上涨终结）同时是反向 Move(k) 的起点（下跌确立）。这是洞察6的形式化：
> 走势终完美 → 走势内部出（反向）买卖点 = 走势已终结（内生的，不需要外部指令）。

### 2.3 尾 move 方向是噪声（引理 D-NOISE）

**引理 D-NOISE（L1，代码事实）**：引擎当前喂入 `dir_state[k]` 的 `flip_edge[k]` 来自
```rust
// lib.rs:1159-1161
fn trend_last_move_dir(&self) -> Option<&'static str> {
    self.inner.moves().last().map(|m| m.direction.as_str())  // ← 尾 move 方向
}
```
即 `dir_state[k] = dir(Move(k-1) 序列的最后一个元素)`。由 D-STABLE 的证明，Move(k) 内部
子段方向 **Up/Down 交替**，故 `trend_last_move_dir` 在 Move(k) 存续期内**频繁翻转**。

**结论**：`dir_state[k] = dir(尾 Move(k-1)) ≠ dir(Move(k))`。用 `dir_state` 做 root 方向 =
用**次级别瞬时方向**冒充**本级别走势方向** = 级别错配。

> **谱系引用**：这是「走势方向代理陷阱」（记忆 `project_trend_direction_proxy`）的精确再现——
> 回测用 swing 笔端点代理走势方向，但缠论正典走势方向由中枢定义。`trend_last_move_dir` 是
> 同一陷阱在 fugue_v3 的实例化：尾 move（次级别 swing）代理本级别走势方向。

### 2.4 走势方向的内生信号（定理 D-SIGNAL）

**定理 D-SIGNAL（L0）**：k 级别走势的**终结**有唯一的内生信号——k 级别 type1 BSP 向心
confirm（`nf_sell[k]` / `nf_buy[k]` fire）：

$$
\text{nf\_sell}[k]\ \text{fire} \iff k\ \text{级别顶背驰确认} \iff \text{Up 走势终结} \implies \text{emergent\_dir}[k] := \text{Down}
$$
$$
\text{nf\_buy}[k]\ \text{fire} \iff k\ \text{级别底背驰确认} \iff \text{Down 走势终结} \implies \text{emergent\_dir}[k] := \text{Up}
$$

**依据**：缠论 type1 BSP 必在走势背驰末端（同级别只有三类买卖点，没有"翻转"——洞察2；
三类买卖点的结构保证见 T49 向心 confirm）。`nf_sell[k]` = k 级别 `nest_sell` 窗口向心
回溯逐圈贯通（`signal.rs:243-249`）= k 级别自己的背驰确认，**与级联武装的 located 区分**
（后者可由更高级别 confirm 向下武装，是 source 链，不是 k 级别自身的终结）。

**初始确立**：第一次 `nf` fire 之前 `emergent_dir[k] = None`（方向未定，行情未启动）。第一个
fire 同时充当「确立」（赋予首个方向）。这是合法的——在 k 级别第一个 type1 确认前，k 级别走势
方向客观上未定（中枢配置未成）。

---

## 第 3 部分：Root 方向涌现定理 + 级别正交性

### 3.1 根方向函数（定理 ROOT）

**定理 ROOT（L0）**：根方向是已涌现最高级别走势方向：
$$\text{root\_dir}(t) = \text{emergent\_dir}[r^*(t)]$$
其中 r* 是当前有已确立走势方向的最高级别。引擎实装（`axis.rs`）：
```rust
fn root_direction(&self) -> Option<Direction> {
    let ceiling = self.emergent_ceiling();              // = max_l = r*+1
    (0..ceiling).rev().find_map(|k| self.direction(k))  // 从高向低找首个有方向级别
}
```
`find_map` 的降级查找：若 `emergent_dir[r*] = None`（最高级别刚涌现、尚无 type1 确认），则用
次高级别的已确立方向作为当前最佳估计。一旦 r* 出现首个 type1 确认，`emergent_dir[r*]` 锁定，
不再降级（`emergent_dir` 单调赋值，从不重置为 None）。

### 3.2 级别正交性 = 踏空免疫（定理 ORTH，本文最高信息增量）

**定理 ORTH（L0）**：方向翻转算子 $T_k$（`nf` fire 翻 `emergent_dir[k]`）**只作用于级别 k**：
$$T_s \cdot \text{emergent\_dir}[r] = \text{emergent\_dir}[r] \quad \forall s \neq r$$
即 $[T_s, \text{read}_{r}] = 0$（次级别翻转与高级别读取对易）。

**推论 ORTH-1（踏空免疫）**：在强 Up regime（r* 级别 Up 走势存续）中，次级别 s < r* 的回调
顶背驰 `nf_sell[s]` fire 触发 $T_s$（`emergent_dir[s]` → Down），但 `emergent_dir[r*]` **不变**
→ root_dir 仍 = Up → **F 不翻空**。次级别背驰只触发 E sink（机动仓短差），核心仓方向不动。

**对比 v1/ε 失败实装**：旧实装用 `sell_source → Short`（出现卖点即翻空），等价于令
$T_s$ 直接改写 root（无视级别）——这违反 ORTH。强 Up regime 中次级别回调顶背驰频繁触发翻空，
核心仓被反复打到空头，单边上涨踏空 ⟹ ES −213% / GC −122%（§7 详解）。

**唯一翻转 root 的条件**：`emergent_dir[r*]` 改变 ⟺ **最高级别 r* 自身出顶/底背驰**
（`nf_sell[r*]` / `nf_buy[r*]` fire）。这才是真正的大级别转折——洞察5+6 在最高级别的实例。

### 3.3 P1：根方向非预设定理

**命题 P1（L0）**：入场操作（F 步骤）不「决定」根方向；入场是读取当前涌现结构的观测行为。

**证明**：
1. F 入场极性 `polarity = f(root_dir)`：f(Up)=Long，f(Down)=Short（`operate.rs` F 步骤）。
2. root_dir 由 §3.1 从 `emergent_dir` 涌现读取，`emergent_dir` 由 §2.4 内生 `nf` 翻转。
3. `nf` 是信号层（H⁰⊗groupoid 内聚）产出，**不经操作层**。
4. 故 F 设置 polarity 是对涌现结构的读取，引擎无任何方向初始化参数
   （`FugueEngineCore::new(floor_ladder)` 只接结构递归基，零方向参数）。∎

**推论 C1（不变量，由 prove_epsilon_symmetry 守卫）**：有仓位期间恒成立
$\text{core\_polarity} = f(\text{root\_dir})$，f(Up)=Long，f(Down)=Short。

### 3.4 σ-ascend 是根方向的级别跟踪（不是方向产生）

当更高级别走势在**同方向**涌现，核心仓归属级别上移（T5/A5 第20环）：
```rust
// operate.rs core_emergent_ladder()
while lad + 1 < ceiling
    && h0.direction(lad + 1) == Some(expected_dir)   // ← 读 emergent_dir[lad+1]（涌现结构）
    && h0.anchor(lad + 1) >= core_entry_bar           // ← dir_anchor[lad+1]（方向确立 bar）
{ lad += 1; }
```
σ-ascend 消费 `emergent_dir`，不产生它（H¹ 操作消费 H⁰ 方向，cd_ℚ=1 两层闭合）。爬升后
`emergent_dir[new_core] == expected_dir` 由 while 条件结构性保证，无需额外断言。

---

## 第 4 部分：四步跨级别循环的双重会计学

### 4.1 两种循环的辨析

**Cycle A（原文「先卖后买」，洞察1）**——完整跨级别全仓进出：
```
Long@k → Step1 本级别卖点 全平多（C-clear）→ Step2 次级别做空（F-short@k-1）
       → Step3 次级别买点 全平空（C-clear@k-1）→ Step4 本级别买点 重做多（F-long@k）
```

**Cycle B（引擎实装，E/D 机制）**——并行跨级别对冲：
```
Long@k（核心仓，从不全清）
  → E（sink=σ⁻¹∘τ）：次级别卖信号 → +Short@k-1（1/λ 份额，穿 ε 缝）
  → D（recover=σ∘τ）：次级别买信号 → −Short@k-1（平空，回纯 Long@k）
```

### 4.2 双重会计性（洞察1：本级别卖点→平多=次级别做空窗口开）

每个操作同时在两级别有会计含义（τ 双重投影，reinterp §1/§5）：

| 操作 | 本级别会计 | 子级别会计 | D∞ word |
|------|-----------|-----------|---------|
| **C** | 结清本级别多头 | 打开次级别做空窗口 | τ@core（边界算子） |
| **E** | 本级别多头锁定（不变） | 次级别做空 1/λ 份额 | σ⁻¹∘τ |
| **D** | 本级别多头解锁（不变） | 次级别空头平仓 | σ∘τ |
| **F** | 本级别新建多头 | 确认次级别买点有效 | β⁺@source |

E+D net word = σ⁻¹τ·στ = σ⁻¹·(τστ) = σ⁻¹·σ⁻¹ = σ⁻²（层内 1-cycle，H₁ 生成元 Δr=−1）。

### 4.3「同级别没有翻转」（洞察2）的精确含义

在一个走势类型内部，C 总是触达**顶端**（最高级别 source = r* 的 type1 confirm），不可能在
中途同级别翻转。这是缠论 BSP 三类型的结构保证（T49 向心确认）。"翻转"只存在于**跨级别**的
会计双重性中（C：本级别平多 ↔ 次级别做空），不存在于同级别内部。

**这正是 §3.2 ORTH 的会计侧表述**：方向是级别局部的，跨级别"翻转"是双重会计的两个投影，
不是同一级别的方向反复。

### 4.4 λ-不变性（Cycle A 是 Cycle B 的 λ→1 极限）

- λ → 1：f=1/λ → 1，机动仓 = 全仓，E=全平、D=全做回，Cycle B 退化为 Cycle A。
- λ = 3（当前，026:80「用其中的 1/3」）：核心仓 2/3 恒持，机动仓 1/3 循环（T50）。
- H¹ 生成元 Δr=−1 在所有 λ 下结构相同（σ-不变，542号），只有仓位分配不同。

---

## 第 5 部分：在 D∞ / H⁰ / H¹ 覆盖空间的精确坐标

### 5.1 三轴 = 三个穷尽工具（operation_route_exhaustion §6B）

| 轴 | 内容 | 穷尽工具 | 本文角色 |
|----|------|---------|---------|
| **H⁰**（morphology） | 笔/段/中枢/走势递归构成 + **方向** | 形态学公理（递归定义） | **方向的唯一源头** |
| **groupoid**（observe） | 信号确认 + 区间套定位 | 范畴极限（逆极限） | 方向终结信号 nf + 时机 source |
| **H¹**（operate） | 操作路线（D∞ word） | 群论 word 枚举 | **消费**方向，不产生 |

**根方向的归属**：root_dir ε ∈ {±1} 是 **H⁰ 属性**（来自形态学公理），不是 H¹ 操作的产物。
这是「根方向是涌现属性，不是操作参数」的代数精确化（cd_ℚ=1 两层闭合 → H⁰ 固持 ⊕ H¹ 循环
穷尽操作空间，无 H² 第三种操作）。

### 5.2 ε 对称性（D∞ 双向 root）

H¹(D∞, ℝ₋) = ℝ，单生成元 Δr=−1。根方向 ε 是 H¹ 模「系数的手性」：
- Long root：ε=+1，生成元 Δr=−1 作用于 ℝ₊ 值；
- Short root：ε=−1，作用于 ℝ₋ 值（D∞ 的 ε 对称）。

硬编码 ε=const 截断 D∞ 为 Z（半个群）；从 root_direction 读 ε 是 H⁰ 涌现结构的直接消费，
**零自由度**。这是 `prove_epsilon_symmetry` 守的不变量（C1）。

---

## 第 6 部分：引擎实装（MorphologyAxis 级别涌现）

> §6 是 L1（代码方案）。流式 vs 批量 bit-exact 由共享 `step/finish` 构造保证；fugue_v3 内部
> 方向来源变更**不影响** spiral/unn（独立 SpiralState 实例，不碰 `signal.rs`/`spiral/engine.rs`）。

### 6.1 实装定理（从 §2–§3 直接推出）

| # | 改动 | 推导依据 |
|---|------|---------|
| **R1** | 新增 `MorphologyState`：跨 bar 持有 `emergent_dir[k]` / `dir_anchor[k]`，每 bar 由 `nf` fire 内生翻转 | D-SIGNAL（§2.4） |
| **R2** | `MorphologyBridge.direction(k)` 改读 `MorphologyState.emergent_dir[k]`（不再读 `signal.dir_state`=噪声） | D-NOISE（§2.3） |
| **R3** | `MorphologyBridge.anchor(k)` 改读 `MorphologyState.dir_anchor[k]` | σ-ascend 一致性（§3.4） |
| **R4** | `root_direction()` 保留降级查找（emergent_dir 单调，最高级别一旦确立不降级） | ROOT（§3.1） |
| **R5** | F 入场 entry_dir 断言**退为 aligned 软门控**（`direction(s)==root_dir` 才入场，陈旧 located 致暂逆则不入场，非 panic） | D-GATE（§6.2） |
| **R6** | 保留 `prove_epsilon_symmetry`（root_dir ⟺ polarity）为唯一 panic 级方向守卫 | C1 / ε 对称（§5.2） |

### 6.2 entry_dir 断言退为软对齐门控（定理 D-GATE）

**定理 D-GATE（L0）**：旧 F 入场断言 `h0.direction(s) == expected_s_dir`（buy_source ⟹
direction(s) 应为 **Down**，「type1 buy 在 s 下跌末端」）在内生方向下**符号反转且不可作 panic**，
应退为**软对齐门控**：`aligned := (h0.direction(s) == Some(root_dir))` 为入场前置，不满足则
不入场（非 panic，等次级别真信号）。

**证明（符号反转）**：
1. buy_source=s ⟹ located_buy[s] 链顶 ⟸ `nf_buy[s]` 曾 confirm（`chain_source`，`signal.rs:89`）。
2. `nf_buy[s]` fire ⟹ `emergent_dir[s] = Up`（D-SIGNAL，§2.4）——而非旧断言的 Down。
3. 旧断言把「s 下跌末端」（type1 buy 的**几何位置**）误作「s 走势方向」；内生语义下 s 走势方向是
   买点**之后**的方向 = Up（D-PHASE：终结即下一走势确立）。故正确对齐是 `direction(s)==Up==root`。∎

**为何软门控而非 panic（关键，避免误杀）**：`direction(s)==root` 在正常情况成立（buy_source ⟹
emergent_dir[s]=Up=root），但存在**陈旧 located 例外**——`nf_buy[s]` confirm 后 located_buy[s]
保留期内 s 级别又出 `nf_sell[s]`（`emergent_dir[s]→Down`），而价未破极值故 located 未 clear。
此时 buy_source=s 仍有效但 `direction(s)=Down≠root`。这不是信号层 bug（located 链与 emergent_dir
是两个合法机制，可暂时不同步），故**不可 panic**；正确处理是**不入场**（等次级别真信号）= 软门控。

**结论**：F 入场前置 = `bsp_fire ∧ aligned`。方向唯一来自 root_direction（P1），source 只决定
**时机/层级**（洞察「sell_source 只决定时机不决定方向」）；aligned 门控额外过滤陈旧 located 逆向
窗口。`prove_epsilon_symmetry`（C1，root⟺polarity）是唯一 **panic 级**方向守卫（守 root→polarity
映射的 D∞ ε 对称，零自由度）。

### 6.3 数据流（engine.rs 组装）

```text
每 bar：
1. signal.process(sig, flip_edge) → frame{ nf_sell, nf_buy, sell_source, buy_source, max_l }
2. morph.emerge(&frame, bar)                     ← R1：nf fire 内生翻转 emergent_dir（新增步骤）
3. MorphologyBridge::new(sig, &signal, &frame, &morph)   ← R2/R3：direction/anchor 读 morph 涌现态
   ObserveBridge::new(&signal, &frame)
4. operate.step(&h0, &obs)                       ← R4/R5/R6：root_direction 决定方向，source+aligned 决定时机
5. outcome.consumed → signal.clear_located_*
```

`flip_edge` 仍传入（`signal.process` 签名不变，spiral 复用 DRY；fugue_v3 的 `signal.dir_state`
被更新但**不被消费**——morphology 改读 `morph_state`）。这是接口复用的合法代价，非声明膨胀。

### 6.4 边界条件（nf 同 bar 双向 fire）

同级别 k 同 bar `nf_sell[k]` 与 `nf_buy[k]` 同时 fire（一个走势同 bar 既顶背驰又底背驰）按
缠论定义不可能（实际 nf 在向心 confirm 中互斥）。`MorphologyState::emerge` 中 `nf_buy` 后处理
（同 bar 共触则覆盖为 Up）——此情形不出现，故无歧义。`emerge` 循环范围 `PENDING_LO..MAX_LADDER`
（nf 仅在势源层 fire，ladder<PENDING_LO 恒 `None`）。

---

## 第 7 部分：踏空失败模式的群论解剖（ES −213% / GC −122%）

> §7 结构部分 L0，经验数据 L3（否定性结果，缩小有效域边界）。

### 7.1 失败链（旧 ε 实装）

```
强 Up regime（ES/GC 单边上涨，r* 级别 Up 走势长期存续）
  → 次级别 s 回调结束，s 级别顶背驰 → sell_source = s
  → 旧实装：sell_source → core_polarity = Short（出现卖点即翻空，无视 s 的级别）
  → 核心仓翻成空头，单边上涨中空头持续亏损
  → 反复触发（每次次级别回调）→ ES −213% / GC −122%
```

### 7.2 群论根因（违反 ORTH）

旧实装令 $T_s$（次级别翻转）直接改写 root，即把级别局部算子误作全局算子。这违反定理 ORTH
（$[T_s, \text{read}_{r^*}] = 0$）。次级别顶背驰 `nf_sell[s]`（s<r*）应只翻 `emergent_dir[s]`，
触发 E sink（机动仓短差做空 1/λ），**核心仓 root 方向不动**。旧实装把「机动仓的次级别短差信号」
误用为「核心仓的 root 翻转信号」——级别错配（§2.3 D-NOISE 的操作侧后果）。

### 7.3 修复的免疫机制（推论 ORTH-1）

内生方向 + root 读最高级别后：
- 次级别 s 顶背驰：`emergent_dir[s]→Down`，但 `emergent_dir[r*]=Up` 不变 → root=Up → 不翻空 ✓
- 仅 r* 级别自身顶背驰（`nf_sell[r*]`）：`emergent_dir[r*]→Down` → root 翻 Down（真正大级别转折）✓

强 Up regime 中 r* 级别长期不出顶背驰（一直涨），故 root 长期 Up，核心仓持多——这正是单边
上涨该有的行为。次级别回调只在机动仓做 1/λ 短差对冲。

### 7.4 残余有效域警告（L2 诚实标注）

免疫的前提：`emergent_dir[r*]` 在行情早期确立为 Up（需 r* 级别早期出过一次 `nf_buy[r*]`=
大级别底背驰）。若行情自始单边涨、r* 级别从无底背驰确认（buy_source 从未级联到 r*），则
`emergent_dir[r*]=None`，root 降级到次级别——残余踏空风险。此为 **L2 regime 依赖**，非 L0
结构缺陷：缠论上 Move(r*) 的形成（max_ladder 达 r*）通常伴随起涨点的高级别买点级联，故风险
在实际数据中较小，但**不为零**——待 §8.3 边界条件之 L3 验证裁决。

---

## 第 8 部分：结果包六要素

### 8.1 结论

1. **走势方向 ≠ 尾 move 方向**（D-NOISE）：`dir_state`（flip_edge=`trend_last_move_dir`）是
   走势内部尾 move 方向噪声；走势方向是走势级稳定量（D-STABLE），只在背驰确认翻转（D-PHASE）。
2. **Root 方向是涌现的 H⁰ 属性**（ROOT）：root_dir = emergent_dir[r*]，由 nf fire 内生维护
   （D-SIGNAL），操作层只消费不产生（cd_ℚ=1）。
3. **级别正交性 = 踏空免疫**（ORTH）：方向翻转算子 $T_k$ 只作用于级别 k，次级别背驰翻不动
   最高级别 → 强 Up regime 不踏空。这是 ES/GC 失败的群论解药。
4. **entry_dir 断言退为软对齐门控**（D-GATE）：旧断言符号反转（内生语义下应断言 direction(s)==root
   而非反向），且因陈旧 located 例外不可作 panic，退为软门控（aligned 才入场，否则等真信号）；
   `prove_epsilon_symmetry`（C1）是唯一 panic 级方向守卫。
5. **六项实装**（R1–R6，§6.1），全部从 §2–§3 的 L0 定理推出，非补丁。

### 8.2 定义依据

- 级别递归 Move(k)≡Level-(k+1)笔：缠论 017:40 走势分解定理一 + 谱系540。
- 走势方向中枢配置：第17课「走势分解定理」+ 中枢定义（§1.3）。
- type1 BSP 在背驰末端：T49 向心 confirm（`signal.rs:103-121`）+ 同级别三类买卖点（洞察2）。
- H¹(D∞,ℝ₋)=ℝ / cd_ℚ=1：`orbit_enumeration_completeness.md` §3.3/§4 A1。
- σ-不变 f=1/λ：542号 + `recursive_fugue_necessity_proof.md`。

### 8.3 边界条件（结论翻转条件）

| 结论 | 翻转条件 |
|------|---------|
| 走势方向 = emergent_dir（nf 内生） | 若 nf fire 与走势终结不等价（信号层 confirm 机制 bug） |
| ORTH 踏空免疫成立 | 若 `emergent_dir[r*]=None` 长期持续（强单边、r* 无底背驰）→ root 降级到次级别（§7.4 残余风险，待 L3） |
| entry_dir 退为软门控 | 若方向恢复为独立外部源（与 nf 解耦，喂稳定走势方向）→ 断言重获 L2 交叉验证信息，可升回 panic |
| root 是 H⁰ 属性 | 若存在不经 H⁰ 信号层产生的方向变化（架构越权） |
| 修复跑赢旧实装 | **未验证**——§7 修复是结构必然，但 L3 alpha 待 8 标的回测裁决（regime 依赖） |

### 8.4 下游推论

1. **fugue_v3 回测口径全变**：root 方向从噪声（尾 move）改为稳定走势方向，F/C/E/D/σ-ascend
   全路径方向输入改变 → 所有 trade 行漂移。这是预期的（修复 bug），不是回归。
2. **spiral/unn 不受影响**：fugue_v3 用独立 SignalState，不碰 `signal.rs`/`spiral/engine.rs`，
   spiral/unn bit-exact 守恒。
3. **`signal.dir_state` 对 fugue_v3 成死状态**：仍被 flip_edge 更新（spiral 需要），fugue_v3
   morphology 不再读。flip_edge 输入保留（签名不变，ffi 零改动）。
4. **prove_epsilon_symmetry 是唯一 panic 级方向守卫**：F 入场 entry_dir 退为软门控（非 panic），
   C1（root⟺polarity）由 ε 对称守卫独家 panic 守护——任何 root_dir 与 polarity 错配立即 panic。

### 8.5 谱系引用

- **540号**（递归双重性，已结算）：Move(k)≡Level-(k+1)笔，同段双重身份。
- **541号**（螺旋覆盖空间，已结算）：(φ,r,ε) 坐标。
- **542号**（σ-不变，已结算）：f=1/λ 级别无关。
- **543号**（操作 as word，已结算）：操作是 D∞ word，不硬编码循环。
- **走势方向代理陷阱**（记忆 `project_trend_direction_proxy`，本次重写的直接先例）：swing/尾 move
  代理走势方向 = 级别错配。本文 §2.3 是其在 fugue_v3 的实例化与修复。
- **本文可能触发新谱系**：「走势方向的双时相 + 级别正交性踏空免疫」尚无对应已结算谱系条目——
  D-NOISE/ORTH 是否升格为语法记录，待编排者裁决（§8.6）。

### 8.6 影响声明

**本文档改动**：重写 `docs/emergent_level_direction.md`（v1 sonnet → v2 opus，核心增量 §2 双时相 +
§3.2 级别正交性 + §7 踏空群论解剖）。

**配套代码实装**（同 commit）：
- `rust/src/fugue_v3/morphology.rs`：新增 `MorphologyState`（内生方向，`emerge` 每 bar nf 翻转）+ Bridge direction/anchor 改源（R1/R2/R3）。
- `rust/src/fugue_v3/engine.rs`：持有 `morph`，每 bar `signal.process` 后 `morph.emerge`（R1 数据流）。
- `rust/src/fugue_v3/operate.rs`：F 入场 entry_dir 断言退为 `aligned` 软门控（R5）。
- `rust/src/fugue_v3/axis.rs`：`root_direction()` 默认实现（降级查找最高有方向级别，R4）。
- `rust/src/fugue_v3/prove.rs`：`prove_epsilon_symmetry` 为唯一 panic 级方向守卫（R6）。

**不修改**（保持正确）：
- `signal.rs` / `spiral/engine.rs`（spiral/unn bit-exact 边界）。
- 三轴 trait 分离（`axis.rs` 接口）、会计层（NAV/守恒）、`FugueEngineCore::new` 零参数。

---

*文档认识论等级：§1–§5 全 L0（形式推导）；§6 L1（代码方案，bit-exact 等价）；§7 结构 L0 + 经验
ES/GC L3（否定性结果定位根因）。修复后的 alpha 为 L3，待 8 标的回测，本文不预言。*
