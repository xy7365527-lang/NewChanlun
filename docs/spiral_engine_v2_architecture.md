# 螺旋引擎 v2 架构设计（Spiral Engine v2）

> 状态：架构设计（Plan 阶段，不含代码）
> 日期：2026-06-16
> 理论基础：`docs/dialectical_exhaustion.md`（辩证穷尽）· `docs/necessity_derivation.md`（58 不变量）· `docs/orbit_enumeration_completeness.md`（轨道枚举 + H¹）· `docs/concept_movement_chain.md`（23 环）· `docs/nested_fugue_accounting.md`（会计规范）· `analysis/spiral_solution_to_underperformance.md`（F 裁决）
> 参考引擎（不继承实现）：`rust/src/trading/unified_necessity.rs`

---

## 0. 结果包（六要素）与认识论总表

### 0.1 结果包

1. **结论**：v2 是一个以 D∞ 群结构为**第一性原理**的交易引擎。状态空间 = 群作用空间坐标 `(φ, r, ε)`；交易操作 = 群元素在状态上的作用（不是独立的 F/C/D/E/A 函数）；闭合 = H¹ 生成元 `Δr = −1`；prove = 群关系（`h²³=σ`、`τ²=e`、`τhτ⁻¹=h⁻¹`）+ 58 条不变量的运行时断言；会计 = 同一螺旋的第三投影。v2 从头写操作/会计/prove 三层，复用现有信号层（tape/bsp_events）。

2. **定义依据**：
   - 状态三坐标 `(φ∈ℤ/23, r∈{0..10}, ε∈{±1})` 严格对应 `orbit_enumeration_completeness.md §1.1`（R3 提取）与 `dialectical_exhaustion.md §1.1`（R1 提取）。
   - 操作 = Δr=−1 投影来自 `dialectical_exhaustion.md` 第 5 部分 H¹ 投影表（R1）：F/C/D/E/配额已实装为同一 `Δr=−1`（秩 1）。
   - 闭合 = H¹ 生成元来自 `orbit_enumeration_completeness.md §4B.4` 定理 A1：`H¹(D∞,ℝ₋)≅ℝ`，生成元 = 跨级别位移率 `Δr`。
   - 58 不变量 → prove 守卫来自 `necessity_derivation.md §11.2`（R2，N=58 穷尽）。

3. **边界条件（结论何时翻转）**：
   - 若 v2 的 `GroupAction` 抽象**无法表达**某个现有操作（即该操作不是纯群作用），则"操作=群作用"被证伪——这是有价值的否定性结果，须 escalate。**当前已知一例**：A 强平不是群元素（市场被动作用，gap G2）。
   - 若 v2 与现有 unn 引擎在已验证有效域（8 标的 ~25M bar）**不能 bit-exact**，且差异不是已知 gap，则 v2 的操作逻辑与现有引擎数学不等价——须逐处归因。
   - 若强行声明"零经验参数"而成本门 θ 仍是 L2 经验量，则结论退化为声明膨胀（090号），无效。

4. **下游推论**：
   - v2 把"操作是否真的是群作用"从隐含假设变为**可证伪命题**（每个操作必须由 `GroupAction::apply` 产生，否则编译/断言失败）。
   - 会计层（VoiceForest/nav/守恒）与操作层表述正交 ⟹ v2 可复用会计语义而只重构操作触发。
   - 信号层契约（BarSig 8 字段 + bsp_events）固定 ⟹ v2 不改信号层，M2 比价层（D∞×D∞ 直积）留接口。

5. **谱系引用**：
   - 542号（配额 σ-不变 f=1/λ）、539号（根翻空有效域 ⊂ 非上行 regime）、527号（σ 能指碰撞：径向 h²³ vs 手性 ℤ/2）、231号（有效域≠定义域）、090号（声明膨胀）、161号（务实=先验上限）、137号（否定禁令对执行层无效）。
   - 本文档新增 5 个语法记录候选交 genealogist 评估（见 §12），**不 escalate**（纯推论精化，未制造新矛盾）。
   - 已上浮未结算：`2026-06-15-confirm-arming-differance.md`（G3 延异）、`2026-06-16-t41-zero-liq-classification.md`（G2 A 强平）。

6. **影响声明**：本文档新增 `docs/spiral_engine_v2_architecture.md`，不改动任何代码或现有定义。它规定 v2 的模块/数据结构/操作/prove/接口/实装路线。v2 实装将新增 `rust/src/spiral/` 模块树，**不删除** `unified_necessity.rs`（保留为 bit-exact 对照基线，直到 v2 验收通过）。

### 0.2 认识论等级总表（formalization-validity-domain 强制）

| 声明 | 等级 | 含义 |
|------|------|------|
| 群关系 `h²³=σ`/`τ²=e`/`τhτ⁻¹=h⁻¹` | **L0** | 纯代数（D∞ 定义关系），零信息增量，运行时设结构 panic 守卫 |
| 状态空间 `(φ,r,ε)` 三坐标 | **L0** | 定义层 |
| `dim H¹=1`、Burnside 506→46→2→1 | **L0** | 代数计算（Bass-Serre / 逐级 Burnside） |
| N1–N8 会计守恒、prove 全程零 panic | **L2** | 8 标的 ~25M bar 真实数据断言（可证伪，已达成） |
| 与现有 unn bit-exact | **L1→L2** | L1=管线等价（合成）；L2=真实数据逐位等价（验收判据） |
| 策略收益（P1≥BH） | **L3** | 跨标的真实数据（会计正确性 ⊥ alpha，§10.4） |
| `λ=2`（SUB_SPAWN_FRAC=0.5） | **L2 待测** | A₅ 二分递归建模默认，待 T50 涌现 λ 测量精化 |
| 成本门 θ（4 个常数） | **L2 不可消除** | 势存在性判定本质需经验量 |

**关键区分（231号）**：v2 的群论结构在定义域（全部 `(φ,r,ε)` 状态）上 L0 成立 ≠ 在定义域上 L3 经验有效。"会计层零守恒违反"（L2）**不蕴含**"策略有 alpha"（L3）——R7 实测 P1=1/8（仅 OKLO）。v2 的存在论合法性（守恒）与有效域读数（收益）是两个范畴。

---

## 1. 理论基础回顾（v2 的公理层）

### 1.1 群与三坐标

$$G = D_\infty = \langle\, h,\ \tau \mid \tau^2=e,\ \tau h\tau^{-1}=h^{-1} \,\rangle \cong \mathbb{Z}\rtimes\mathbb{Z}/2,\qquad \sigma := h^{23}$$

| 状态分量 | 取值空间 | 群生成元 | 缠论语义 | 群作用 |
|---------|---------|---------|---------|--------|
| **φ 角向相位** | ℤ/23ℤ | `h`（角向推进） | 走势进度，唯一奇点 φ=0（背驰∧手性翻转） | `h: φ↦φ+1` |
| **r 径向级别** | {0..10}，MAX_LADDER=11（有界塔） | `σ=h²³`（级别跃迁） | 级别 = 圈数 = λ^k（笔→线段→走势→…） | `σ: r↦r+1` |
| **ε 手性** | {±1} | `τ`（手性翻转） | 多/空，仅在 φ=0 翻转 | `τ: r↦−r, ε↦−ε, h↦h⁻¹` |

**helix 编码**：把 `(φ,r)` 合并为单一坐标 `n`：`φ = n mod 23`，`r = ⌊n/23⌋`。则 `h: n↦n+1`，`σ=h²³: n↦n+23`。这把"绕一圈角向 = 升一级径向"（T56 `h²³=σ`）编码为单一平移。

**工程常量**（来自 `types.rs:26`）：`MAX_LADDER=11`、`FIRST_BSP_LADDER=2`、`LADDER_MOVE=3`、`PENDING_LO=FIRST_BSP_LADDER+1=3`。径向口径：`0=bar(a0)`、`1=笔`、`2=线段(segment)`、`3=move/L1`、`4..10=递归层 recL2..`。

### 1.2 三轴正交（辩证三环 = 群上同调三轴）

| 辩证环节 | 上同调身份 | 数量 | v2 含义 |
|---------|-----------|------|--------|
| **正题(肯定)** | `H⁰(D∞,M)=M^G`（0-cocycle） | 58 标签（46 轨道） | 不变量（对象不变）= 状态读数 |
| **反题(否定)** | 作用 groupoid（合法/非法变化） | 7 条 NR（设计禁区） | 状态转移合法性（对象动）= 操作约束 |
| **合题(扬弃)** | `H¹(D∞,ℝ₋)`（1-cocycle） | dim=1（秩 1） | 导出不变量（变化模式不变）= **Δr=−1 闭合率** |

三轴**正交、内容不相交**（R3 §6.1）。穷尽 = `(H⁰⊕H¹) × (2×2 groupoid × 46 母线)`。**单一不变量（H⁰）不能区分同轨道内两状态/转移合法性/变化率**——v2 不能只靠状态读数判定操作合法性。

### 1.3 H¹ = 唯一独立的导出不变量（v2 闭合的代数根）

`dim H¹(D∞, ℝ₋) = 1`（Bass-Serre：`D∞=ℤ/2*ℤ/2`，`H¹=V/(V^a+V^b)`，`a=τ, b=τh`；ℝ₋ 模下 dim=1，三独立路径 confirmed）。

**定理 A1**：`H¹(D∞,ℝ₋)≅ℝ`，生成元 = 跨级别位移率 `Δr`。非零 ⟺ D∞ 在级别线上仿射作用（`σ:r↦r+1, τ:r↦−r`）无全局不动点（无"中心级别"）⟺ **"开仓 r=k、闭合 r=k−1" 的 `Δr=−1` 是真导出不变量**（非上边界、非再基化）。

**核心数量自洽**：13 条 sigma 型张力中 **11 条同投影一个 `Δr=−1`（秩 1）**，彼此不独立；2 条被整数股数 H² 量子化破缺（gap G1）。**所有交易操作（F/C/D/E/配额/close_voice）是同一个 `Δr=−1` 的投影**，不是独立设计的函数。

**定理 A2（两层闭合）**：当 2 在观测模 M 上可逆（实/有理观测量：P&L、θ、units-as-real），`H^{≥2}(D∞,M)=0` ⟹ 上同调穷尽 = `H⁰⊕H¹` 两层。**闭合前提是"2 可逆"，不是 cd=1**（cd 路径脆弱）。`cd_ℚ(D∞)=vcd=1`；`cd_ℤ=∞`（含挠）；`H²(D∞,ℤ)=(ℤ/2)²≠0`（整数股数残余 = G1）。

### 1.4 23 环 = `h²³=σ` 闭合接缝

`concept_movement_chain.md` 的 23 环首尾相接构成一次完整级别跃迁。关键节点：

- 环 1–12：角向推进（`h`），从"走势终完美"到"三类买卖点"，构造形态学。
- 环 13–20：径向跃迁（`σ`），引入级别/区间套/操作量/成本门/降成本/并发/赋格/嵌套。
- 环 21–22：手性翻转（`τ`），多空对称/方向交替递归（`ε×r` 耦合）。
- **环 23：循环闭合**（`τ 翻转 + σ 跃迁`）——"出场=翻转=新建仓=回到环 1 但新势新级别"，`h²³=σ` 的闭合接缝。

### 1.5 NR-1~7（设计禁区，v2 不可违反）

| NR | 内容 | v2 禁区 |
|----|------|---------|
| **NR-1** | 纯前向单圈走完 23 环必落 `(0, r+1, ε)`；同级别 `σ=e` 闭合 = 抹掉径向 = 级别坍缩 | v2 不允许同级别原地闭合作为默认转移 |
| **NR-2** | 降成本时间 `Δt(k)=λ^{k-1}(λ−1)>0` 无上界（k→∞）；MAX_LADDER 截断有界 `λ¹⁰` | v2 假设 r 有界（工程截断非 L0） |
| **NR-3** | 多空两书角向不连通，仅 φ=0 手性缝（τ_seam）桥接 | **τ（翻转）只能在 φ=0 触发**；禁纯角向连通到对侧手性 |
| **NR-4** | 观测 ⊥ 操作：向心读 σ⁻¹（读过去内圈，合法）/ 前向读未来（非法）| confirm 须区分"背驰已确认"（向心读）vs"背驰段候选"（前向搜索）；G3 延异未结算 |
| **NR-5** | 顶层 σ 在 r=10 无像；底层 σ⁻¹ 在 r=0（a0）无像（分辨率不可约） | v2 不升级到 MAX_LADDER 之上，不细化 a0 到 r<0 |
| **NR-6** | 操盘同级别周期是 `σ=e` 投影（完整轨迹径向非周期） | v2 不把同级别周期当闭合 |
| **NR-7** | 整数股数 `H²(D∞,ℤ)=(ℤ/2)²` 第二生成元 ≠ w₁²，扬弃在量子化层不闭合 | G1：units 用 f64 实值掩盖；v2 须裁决是否显式 enforce 整数股数 |

### 1.6 5 个 gap（保留不强行闭合，no-patch-mentality）

| gap | 类型 | 状态 |
|-----|------|------|
| **G1** NR-7 整数股数 H² 残余 | 量子化（cd_ℚ=1 预言内） | f64 掩盖，未 enforce |
| **G2** T41/T46 声明膨胀（"零强平无条件" ⊥ 161 次实测强平） | 声明膨胀（090号） | 已 escalate（t41-zero-liq），**A 强平根源** |
| **G3** confirm 时序/延异（共时在场 ⊥ 第 30 课延异） | 形而上学 | 已 escalate（confirm-arming-differance） |
| **G4** T4 第三类（强牛被否定回调既非趋势非盘整） | 潜在第三类（cd_ℚ=1 预言外） | 未裁决，模型不精确信号候选 |
| **G5** θ 配额谱系节点（TR-F 已结算但 settled 节点未落地） | bookkeeping | genealogist 开节点 |

---

## 2. v2 与现有引擎的关系（存在论澄清）

### 2.1 v2 不是什么

- **不是性能优化**：现有 unn 引擎已 bit-exact、8 标的零 panic。
- **不是算法改进**：操作逻辑（F/C/D/E/配额）的数学内容 R1 证实**已经是 `Δr=−1` 投影**，无需改。
- **不是推翻**：会计层（VoiceForest/nav/守恒）与信号层契约保持语义不变。

### 2.2 v2 是什么

v2 = **把 D∞ 群结构提升为引擎的第一性原理**。现有引擎的结构债（R6 提取）是：F/C/D/E/A 实现为 `step()` 内**五个独立命令式代码块**，每块各自读 located/nf 状态并直接 mutate voices，**无统一"群元作用于 (φ,r,ε) 状态"抽象**：

- C 规则有 `Long`/`Short` 两个对称但**代码重复**的分支（各 ~40 行手写 settle）。
- E/D 是逐 voice for 循环，各自内联 gate 表达。
- 五块共 ~5 份独立逻辑 + 大量重复模板。

v2 把它们统一为 `GroupAction` 在 `SpiralState` 上的作用：
- **C = τ 作用**（Long↔Short in-place，`τ²=e`，units 守恒 M=N）——消除两个重复分支。
- **E/D = σ 径向嵌套的正/逆**（spawn 子 voice @ r−1 / 回补返父）。
- **F = 根涌现**（σ 塔起点 @ source）。
- **A = 会计终局**（**非群操作**，市场被动，gap G2——诚实标记，不伪装成群作用）。
- **配额 f = σ-不变性本身**（`Δr=−1` 的 σ-不变，T18×T48×T59）。

### 2.3 v2 的信息增量（为什么值得从头写）

把"操作是否真的是群作用"从**隐含假设**变为**可证伪命题**：
- 每个操作必须由 `GroupAction::apply(&SpiralState) -> SpiralState` 产生。
- 若某操作无法纳入此抽象，则暴露它不是纯群作用 = 有价值的否定性结果（L2/L3）。
- **当前已知一例**：A 强平进不了群（市场作用），这正是 gap G2 的群论形式。v2 让这个 gap 在类型系统层面**显形**（A 不是 `GroupAction` 变体），而非藏在命令式 for 循环里。

### 2.4 v2 的验收判据（三条，缺一不可）

1. **L0/L2**：群关系 prove（`h²³=σ`/`τ²=e`/`τhτ⁻¹=h⁻¹`/`Δr=−1`/配额 σ-不变）+ N1–N8 全程零 panic（8 标的 ~25M bar）。
2. **L1→L2**：与现有 unn 引擎在已验证有效域**逐 trade bit-exact**（操作逻辑数学等价的证明）。每处差异必须归因为已知 gap，否则是 bug。
3. **gap 保留**：G1–G5 不被强行闭合（no-patch-mentality）。A 强平保持"非群"标记。

> **判据 2 的张力与解**：判据 2 要求 v2≡现有引擎，似乎使 v2"只是换表述"。但 v2 的价值不在行为差异，而在**表述的可证伪性**——bit-exact 证明"群作用抽象足以表达全部已验证操作"（除 A），这本身是对"操作=群作用"命题的 L2 验证。若 v2 无法 bit-exact，则命题被否证。

---

## 3. 模块结构（文件拆分）

新增 `rust/src/spiral/` 模块树（与 `rust/src/trading/` 平级，不污染现有引擎）。遵守 coding-style：单文件 200–400 行典型、800 max、高内聚低耦合。

```
rust/src/spiral/
├── mod.rs              # 模块入口 + 公开 API（SpiralEngine, SpiralStream）
├── state.rs            # SpiralState(φ,r,ε) + helix 编码 + 群作用 trait（L0 核心）
├── group.rs            # GroupElement(D∞) + GroupAction enum + 群关系（h²³=σ/τ²=e/τhτ⁻¹=h⁻¹）
├── voice.rs            # Voice（携带 SpiralState）+ VoiceForest（森林拓扑，孤儿不可能定理）
├── accounting.rs       # 会计投影：nav/close_voice/settle/守恒（复用现有语义，§5 第三投影）
├── operation.rs        # 五操作作为 GroupAction 解释：F/C/D/E + A（非群）
├── closure.rs          # H¹ 闭合（P-close）：Δr=−1 检测与兑现 + 三类闭合区分
├── signal.rs           # 信号层消费：BarSig → (φ推进/r跃迁/ε翻转) 事件映射（消费契约，不重写）
├── prove.rs            # prove 守卫体系：群关系 panic + 58 不变量 + 认识论分层
├── params.rs           # 参数 + 认识论标注（a0/λ/成本门θ，每个带 L 等级 + 来源）
├── engine.rs           # SpiralEngine::step（批量+流式共享的驱动循环）
└── ffi.rs              # PyO3 导出（SpiralStream 流式 + run_spiral 批量，bit-exact 构造）
```

**职责矩阵**：

| 文件 | 职责 | 复用现有 | 新写 |
|------|------|---------|------|
| `state.rs` | 状态三坐标 + 群作用接口 | — | 全新（第一性化核心） |
| `group.rs` | D∞ 群元素与关系 | — | 全新 |
| `voice.rs` | voice 森林 | VoiceLedger 语义（R5/R6） | voice 携带 SpiralState |
| `accounting.rs` | nav/close/settle/守恒 | **语义 bit-exact**（R5/R6） | 重新组织为"第三投影" |
| `operation.rs` | F/C/D/E/A | 操作逻辑数学内容 | 群作用抽象封装 |
| `closure.rs` | P-close | helix_centripetal_confirm | H¹ 显式兑现 |
| `signal.rs` | BarSig 消费 | **契约固定**（R8） | 群事件映射层 |
| `prove.rs` | 守卫 | N1–N8 + prove_* | + 群关系 panic |
| `engine.rs` | step 驱动 | UnnStreamCore::step 结构 | 群作用驱动 |
| `ffi.rs` | PyO3 | 批量+流式双入口（R6/R9） | SpiralStream |

---

## 4. 核心数据结构

> 以下是 Rust 签名草图（Plan 阶段，不是最终实现）。遵守 immutability：群作用返回新状态，不原地 mutate。

### 4.1 SpiralState（state.rs）

```rust
/// 螺旋状态 = D∞ 群作用空间坐标。三坐标严格对应 (角向, 径向, 手性)。
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpiralState {
    pub phi: u8,   // 角向相位 ∈ 0..23 (ℤ/23ℤ)，唯一奇点 phi==0
    pub r:   u8,   // 径向级别 ∈ 0..MAX_LADDER (有界塔，11)
    pub eps: i8,   // 手性 ∈ {+1,-1}，仅在 phi==0 翻转
}

impl SpiralState {
    /// helix 编码：n = r*23 + phi；σ=h²³ 退化为 n+=23 的单一平移。
    pub fn helix(&self) -> u32 { (self.r as u32) * 23 + (self.phi as u32) }
    pub fn from_helix(n: u32) -> Self {
        Self { phi: (n % 23) as u8, r: (n / 23) as u8, eps: 1 } // eps 须显式带入
    }
    pub fn is_singular(&self) -> bool { self.phi == 0 } // φ=0：背驰∧可翻转
}
```

> **527号命名分离**：本 `eps`（手性 ℤ/2）≠ 527号 `σ∈{±1}`（走势方向态）；本 `sigma` 永远指径向跃迁 `h²³`。代码中径向算子命名 `sigma_radial` / 手性命名 `eps` / `tau`，禁用裸 `sigma` 指代手性。

### 4.2 GroupElement / GroupAction（group.rs）

```rust
/// D∞ 群元素的正规形：g = h^a · τ^t（a∈ℤ, t∈{0,1}）。σ=h²³ 是 a=23 的特例。
#[derive(Clone, Copy)]
pub struct GroupElement { pub a: i32, pub t: u8 }  // h^a τ^t

/// 合法操作单子的三生成元（NR-4：操作前向 / 观测向心；R1 合法操作单子 M）。
pub enum GroupAction {
    AngularStep,        // h_fwd : φ↦φ+1（前向推进，唯一前向算子）
    ChiralSeam,         // τ_seam: 仅在 φ=0 翻 ε（NR-3，否则非法 panic）
    RadialDescend,      // σ⁻¹ : r↦r−1（向心下沉，闭合落点；H¹ 生成元 Δr=−1）
    RadialAscend,       // σ   : r↦r+1（级别涌现，根向上生长）
}

impl GroupAction {
    /// 群作用：返回新状态（immutable）。非法作用（如 φ≠0 翻 τ）返回 Err。
    pub fn apply(&self, s: SpiralState) -> Result<SpiralState, GroupViolation>;
}
```

群关系（编译期/运行时双重保证）：`τ²=e`、`τhτ⁻¹=h⁻¹`、`σ=h²³`、`τστ⁻¹=σ⁻¹`（M5，根空头爬 Down）。

### 4.3 Voice + VoiceForest（voice.rs，复用 R5/R6 语义）

```rust
/// 一个声部 = 螺旋一圈（赋格一个 voice）。携带其 SpiralState。
pub struct Voice {
    pub state:    SpiralState,    // 该 voice 的 (φ, r=级别, ε=方向)
    pub units:    f64,           // 在手单位（Σ活跃 = N_base；T34 σ-不变 Casimir）
    pub basis:    f64,           // 开仓均价（降成本下降，earning 可穿 0 变负）
    pub capital:  f64,           // 空头冻结现金（非逐市，物理单真值）/ 多头恒 0
    pub entry_bar: i64,
    pub status:   VoiceStatus,   // Active | PendingRecovery | Closed
    pub parent:   Option<usize>,
    pub children: Vec<usize>,    // 森林：root 可多 child
    pub acted_bar: i64,          // per-voice acted（去全局互斥，N2）
    pub realized_pnl: f64,
}
pub enum VoiceStatus { Active, PendingRecovery, Closed }

/// voice 森林：后序遍历保证孤儿不可能定理（T42）。
pub struct VoiceForest { pub voices: Vec<Voice>, pub n_base: f64 }
```

**关键不变量**（R5）：`Σ(active voices.units) = N_base`（记账两侧一致，**非建仓常数恒仓**；N_base 双向重定基：earning +Δ / 亏损 −δ）。`capital` 不实现为逐市 MtM 负债——空头持现金。

### 4.4 Operation（operation.rs）

```rust
/// 五操作。前四个是 GroupAction 的解释；A 是非群（市场被动，gap G2）。
pub enum Operation {
    Enter   { source: u8 },                 // F：根 voice 诞生 @ source（σ 塔起点）
    Flip    { rid: usize },                 // C：τ 作用（root flip，φ=0，M=N）
    CostReduce { parent: usize },           // E：σ⁻¹（spawn 子 voice @ r−1，Δr=−1 落点）
    Cover   { vid: usize },                 // D：σ 逆向闭合（子层 type1 向心确认返父）
    Liquidate { vid: usize },               // A：⚠ 非 GroupAction（capital 耗尽，市场作用）
}
```

> **类型系统层面的诚实**：`Enter/Flip/CostReduce/Cover` 携带 `GroupAction` 语义；`Liquidate` 显式不携带——A 强平在类型上就**不是**群作用，gap G2 在编译期显形。

### 4.5 ClosureKind（closure.rs）

```rust
/// 三类闭合（R1：不能一律按 Δr=−1）。
pub enum ClosureKind {
    CrossLevel,        // sigma 型：Δr=−1 投影（E/D/close_voice）——H¹ 生成元
    ValidityDomain,    // 定义域>有效域：诚实等级重述（A 强平 regime）
    Category,          // 范畴分层：根空头 MtM ⊥ 子空头 frozen（T8）
}
```

---

## 5. 操作的群论表述

> 来源：R1（H¹ 投影表 + Δr=−1）、R7（F 裁决几何）、R2（5 事件 + 每 bar 优先序）、R5/R6（会计）。

**每 bar 优先序**（R2，无 B 否定扫描——第 11 环"操作只在买卖点"，编排者 2026-06-15 删 B）：

```
A 强平兜底（逐空头 voice，会计终局）
→ C 根 type1 平仓/翻转（同级别卖点，τ）
→ D 回补（子 voice 子级别买点，σ 逆向闭合）
→ E 降成本 spawn（持仓期次级别卖点，σ⁻¹）
→ F 根入场（买点，σ 塔起点）
```

### 5.1 F 建仓 = σ 塔起点（根 voice 诞生）

- **群语义**：在 φ=0 奇点初始化一个新 voice，状态 `(0, source, +1)`，`source` = 区间套链顶（`chain_source(located)`）。F 不是单个群元素，而是 σ 塔的一个起点 —— 已是 `Δr` 跨级别定位（向心 confirm 后满仓开 N）。
- **定义依据**：T28（从当下能介入的最高级别进入，`source≥PENDING_LO`）+ T49（向心 confirm，`helix_centripetal_confirm`）。
- **不变量**：`source ≥ PENDING_LO=3`（segment=2 非势源，否则 source 坍缩，project_pcf_pending_locate_collapse_fix）。
- **prove**：`prove_chain`（source≥PENDING_LO panic）。

### 5.2 C 翻转 = τ 作用（root flip in-place）

- **群语义**：`τ: ε↦−ε`，状态 `(0, r, ε) → (0, r, −ε)`。in-place 翻转（不增删 voice，仍单根），M=N 同股数（units 守恒）。
- **定义依据**：T57（`τhτ⁻¹=h⁻¹`，做空=做多镜像）+ T14（根翻转 M=N）+ T24（多空对称）。τ 是 R₂ 对合（`τ²=e`）。
- **NR-3 约束**：**只能在 φ=0 触发**（手性缝 τ_seam），莫比乌斯禁止"同时多空"（只能时序翻转）。
- **prove**：`prove_t14_root_flip`（dir≠old_dir + units 守恒 M=N + 仍单根）+ `prove_chirality_seam`（τ@φ=0）。
- **消除的结构债**：现有 C 的 `Long`/`Short` 两个 ~40 行重复分支 → 统一为 `τ.apply`。

### 5.3 E 降成本 spawn = σ⁻¹（径向下沉，Δr=−1 直接落点）

- **群语义**：`σ⁻¹: r↦r−1`，spawn 子 voice @ `ladder−1`（`sub = p_ladder − 1`，`unified_necessity.rs:974`），子方向交替（环 22，父多→子空）。**这是 `Δr=−1` 的直接落点 = H¹ 生成元本身**。
- **定义依据**：T20（降成本=voice 自层操作，N7）+ T17（区间套）+ 环 17（降成本=低级别走势完美的利用）。
- **配额**：`m_quota = f × p_units`，`f = 1/λ`（σ-不变，级别无关，T18×T48×T59，542号）。
- **prove**：`prove_n7_spawn_self_level`（触发层==voice 层）+ `prove_theta_sigma_invariant`（f=1/λ）+ **`prove_cross_level_closure`（新，Δr=−1）**。
- **有效域边界**：根空头不嵌套降成本（T8：MtM 根空头 ⊥ 子空头 frozen，`is_root∧Short⟹continue`）。

### 5.4 D 回补 = σ 逆向闭合（子层 type1 向心确认）

- **群语义**：子 voice 自层 nf @ `v.ladder` 完成（次级别 type1 向心确认），close 子 voice 返父，`Δr=−1` 闭合在子层（`self_level_counter_fire`）。
- **定义依据**：与 E 对称（环 2 + 环 11 + 环 14）。
- **prove**：`prove_n7_spawn_self_level`（自层）+ `prove_n8_conservation`（守恒）。

### 5.5 A 强平 = 非群操作（市场被动，gap G2 显形）

- **群语义**：**无**。A = capital 耗尽兜底（c≥2×basis，1x 逐仓），同 r，**缺 Δr 跨级别否定退出**。
- **gap G2**：子空头押被否定 candidate 时无 type1 出场 → 落 A 强平。T41（零强平要求子空头在 liq 前出场）⊥ {第 11 环操作只在买卖点 + T43 让 E 在否定-prone 自层 spawn 子空头}——强牛 regime 三者联合不可满足。
- **v2 处理**：A 在类型上不是 `GroupAction`；`prove_t1_no_voluntary_exit` 守"非空→空 ⟹ root_liquidated"。**不自决修复**（恢复否定线 vs 接受 T41 条件 regime），须 escalate 后裁决。已 escalate（t41-zero-liq）。

### 5.6 配额 f = σ-不变性本身

- **群语义**：`f = 1/λ`，级别无关 = `Δr=−1` 的 σ-不变性。`sigma_invariant_quota(p_units)` 刻意不取 sub 参数（编码级别无关性）。
- **定义依据**：T18（配额径向比 1/λ σ-不变）× T48（units = 唯一 σ-不变 Casimir）× T59（σ 自相似）+ T23（递归 step-replication）。
- **prove**：`prove_theta_sigma_invariant`（`m_quota == sigma_invariant_quota(p_units)`，非重言 should_panic 反证 `theta_sigma_invariance_fires_on_level_dependent_quota`）。

### 5.7 操作 → 群元素 → Δr 映射总表

| 操作 | 群元素 | Δr 涌现 | 代码锚（现有） | v2 改动 |
|------|--------|---------|---------------|---------|
| F 建仓 | σ 塔起点 @ source | 跨级别定位（向心 confirm） | `step` F 段 buy_source | 封装为 `Enter` |
| C 翻转 | τ（`τ²=e`） | 跨级别 confirm + 翻 ε | `step` C 段（两分支） | **统一为 τ.apply** |
| E 降成本 | σ⁻¹（r↦r−1） | **Δr=−1 直接落点** | `:974 sub=p_ladder-1` | 封装为 `RadialDescend` |
| D 回补 | σ 逆向闭合 | Δr=−1 闭合在子层 | `self_level_counter_fire` | 封装为 `Cover` |
| A 强平 | **非群** | **缺 Δr（gap G2）** | A 段 for 循环 | **类型上非 GroupAction** |
| 配额 f | σ-不变 | Δr=−1 的 σ-不变 | `SUB_SPAWN_FRAC` | 复用 + prove |

---

## 6. H¹ 闭合（P-close）实现方案

### 6.1 P-close 的本质（R7 经验依据 + R1 代数根）

**闭合 = 否定之否定 = H¹ 的 `Δr=−1` 在次级别**。势耗尽**不在开仓级别 r=k 原地闭合**（NR-1 否定同级别 σ=e 闭合），而是**向心下沉到次级别 r=k−1**（φ 归零）。

经验依据（R7，`spiral_solution_to_underperformance.md`）：
1. **T49 向心回溯**：confirm 一个外圈 candidate@K 需检验内圈（次级别）type1，由展开恒等式 `Δt∝λ^{K-1}(λ−1)>0`，内圈 type1 **必然在过去**（向心读，非前向读未来）。`Δr=−1`（K→K−1）。
2. **F 根入场几何**：`helix_centripetal_confirm`（向心确认），非前向窗口（前向 = 几何错误，重现 418/418 时序错配 + 失控强平）。
3. **子空头出场级别下沉**：D 回补在子自层=segment 的 type1 买点（`Δr=−1`）。

### 6.2 P-close 算法

```
P-close(voice v @ level K):
  1. 势耗尽信号 = located 链顶 confirmed type1 @ source(=K)
     —— 向心 confirm：读 type1_hist[K-1]（次级别内圈，已 settle 的过去），
        helix_centripetal_confirm(type1_hist, K, side, since) 母线贯通。
  2. 判定 ClosureKind：
       sigma 型      → CrossLevel：闭合落点 r=K−1（E spawn / D 回补）
       validity_domain → 诚实等级重述（不兑现 Δr，标 regime 有效域）
       category      → 范畴分层（根空头 MtM vs 子空头 frozen，分别处理）
  3. 兑现（仅 CrossLevel）：
       E: spawn 子 voice @ K−1，方向交替
       D: close 子 voice 返父 @ K−1
  4. prove_cross_level_closure: 硬断言 child.r == parent.r − 1（Δr=−1 恒成立）
```

### 6.3 三类闭合区分（关键：不能一律 Δr=−1）

| ClosureKind | 判据 | 兑现方式 | 例 |
|-------------|------|----------|-----|
| **CrossLevel** | sigma 型张力，势耗尽向心下沉 | `Δr=−1`（E/D） | 降成本、子层回补 |
| **ValidityDomain** | 定义域>有效域，regime 依赖 | 诚实等级重述（不动 Δr） | A 强平（强牛 regime）、T41 条件化 |
| **Category** | 两定理范畴混淆 | 范畴分层 | 根空头 MtM ⊥ 子空头 frozen（T8） |

### 6.4 新增守卫 `prove_cross_level_closure`

```rust
/// 硬断言：每次 CrossLevel 闭合（E spawn / D 子层）的 Δr == −1。
/// 防漂移；与 prove_theta_sigma_invariant 互补（后者守配额 σ-不变，前者守级别 Δr）。
fn prove_cross_level_closure(parent_ladder: u8, child_ladder: u8) {
    assert_eq!(child_ladder, parent_ladder - 1,
        "H¹ closure violated: Δr must be −1 (cross-level), got {}→{}",
        parent_ladder, child_ladder);
}
```

非重言 should_panic 反证：`cross_level_fires_on_same_level_spawn`（同级别 spawn 应 panic）。

---

## 7. prove 守卫体系

> 范式（R6）：**prove 即验收 = 必然性运行时证明，violation=panic**，非回测验收。每条群关系/不变量配一个 panic 守卫，且必须**非重言**（回归防护：调用点用独立内联表达，prove 守它==规范，漂移即 fire；配套 `#[should_panic]` 反证）。

> **prove 面规模（交叉验证警示）**：现引擎 prove 面是 **~22 个**（N1–N8 + `prove_t14`/`a5`/`t1`/`t52`/`t53`/`t50`/`t56`/`t57`/`t58`/`t59`/`t55`/`theta_sigma` + `prove_self_level_symmetric`），**不是 8 个**。若 v2 只继承 N1–N8 会**缺瓦 14 条**（5 个螺旋观测 prove t50/t56/t57/t58/t59 的混合 panic+观测范式 + T1/T52/T53/theta_sigma）。v2 + 新增 `prove_cross_level_closure` ⟹ ~23 个。下表分三类完整列出。

### 7.1 群关系守卫（L0 结构 panic，v2 新增/提升）

| 守卫 | 断言 | 不变量 | 反证测试 |
|------|------|--------|---------|
| `prove_tau_involution` | `τ²=e`：翻两次回原手性，units 不变 | T24/T57 | `tau_fires_on_non_involution` |
| `prove_chirality_angular_inversion` | `τhτ⁻¹=h⁻¹`：做空角向=做多镜像 | T57 | `chirality_fires_on_same_direction` |
| `prove_angular_radial_holonomy` | `h²³=σ`：segment 层无角向圈（`n_nest_fire[FIRST_BSP]==0`） | T56 | `t56_fires_on_segment_angular_cycle` |
| `prove_cross_level_closure`（新） | `Δr=−1`：E spawn child==parent−1 | A1/H¹ | `cross_level_fires_on_same_level_spawn` |
| `prove_chirality_seam` | τ 只在 φ=0（NR-3，P2 禁 τ@φ≠0） | NR-3/T51 | `seam_fires_on_nonsingular_flip` |
| `prove_theta_sigma_invariant` | `f=1/λ` 级别无关（刻意不取 sub） | T18×T48×T59 | `theta_sigma_invariance_fires_on_level_dependent_quota` |

### 7.2 会计守恒守卫（L2，每 bar，复用 N1–N8）

| 守卫 | 断言 | 守 |
|------|------|-----|
| `prove_n1_forest` | ≤1 active root + 无孤儿 + children 反向引用一致 | T39/T42/T22/T23 |
| `prove_n2_per_voice` | 本 bar 操作 id 无重复（per-voice acted） | T21 |
| `prove_n3_type2` | type2 出现数==处理数 | T13 |
| `prove_n4_cost_gate` | floor_stop≡0（纯 θ<friction∨θ=None） | T19/T44 |
| `prove_chain`（N5/N6） | source≥PENDING_LO + compress≤confirm≤bar | T28/T29/T31 |
| `prove_n7_spawn_self_level` | 触发层==voice 层 | T20 |
| `prove_n8_conservation` | `\|Σunits−n_base\|≤1e-6` + `\|nav_post−nav_pre\|≤1e-4` | T33–T39 |
| `prove_a5_relabel` | 涌现重组 units/NAV 不变 + root_emergent_ladder 单调 | T30 |
| `prove_t1_no_voluntary_exit` | 非空→空 ⟹ root_liquidated（A 强平根源 G2） | T1/环 23 |

### 7.3 缺瓦/观测守卫（L2/regime，~观测非 panic 或 should_panic 反证）

`prove_t52_gauge_fix`（区间套规范固定）、`prove_t53_connection_assoc`（连接结合律）、`prove_s11_s9_located`（located 流交替）、`prove_s12_center`（中枢锚良序）、`prove_t50_radial_scaling`（~观测）、`prove_t55_dual_line_observe`（M2 比价，观测型，未接入 step）。

### 7.4 认识论分层（params.rs 集中声明，修复 R6 结构债）

现有引擎"L0 结构 panic vs L2 经验观测"的划界散落在各 prove 注释中。v2 在 `params.rs` 集中声明每条守卫的等级：

```rust
pub enum ProveLevel {
    L0Structural,   // 群关系/定义层，panic（h²³=σ, τ²=e, Δr=−1）
    L2Conservation, // 会计守恒，每 bar panic（N1–N8）
    L2Observation,  // regime 依赖，观测计数非 panic（T50/T57/T59）
}
```

---

## 8. 信号层接口（tape / bsp_events）

> v2 **不重写信号层**，消费现有契约（R8）。信号层是 v2 操作层的**唯一定义域单元**。

### 8.1 每-bar 输入：BarSig（`tape.rs`）

```rust
struct BarSig {
    close: f64,                              // 唯一成交价口径（P5）；NaN 构造期拒绝（T7）
    buy1:  LadderMask,  // u16：第 k 位 = ladder k 本 bar 新触发 confirmed type1 买点
    sell1: LadderMask,  // type1 卖点
    sell_any: LadderMask, buy_any: LadderMask,
    max_ladder: u8,                          // 本 bar 已涌现最高走势级别（出场归属上限 = r 上界）
    type2_buy: bool,
    bsp_events: Option<Box<[Vec<BspEvent>; 11]>>,  // 11 层带锚事件流
    div_events: Option<Box<[Vec<DivEvent>; 11]>>,
    up_move_settled: LadderMask,
}
```

### 8.2 事件载荷：BspEvent（`types.rs`）

```rust
struct BspEvent {
    class: BspClass,     // 六类：Buy1/Buy2/Buy3/Sell1/Sell2/Sell3 (BspKind×Side)
    seg_idx: i64,        // 锚段索引（去重键）
    confirmed: bool,     // candidate/located 分离核心位
    cs: Option<i64>,     // center_seg_start（type1 可 None；type2/3 必 Some）
    zd: Option<f64>, zg: Option<f64>,  // 锚中枢边界（cs⇒zd,zg 同在）
    price: f64,          // 端点价（恒=close）
}
```

### 8.3 信号 → 群事件映射（signal.rs，v2 新增的薄映射层）

| 信号层产出 | 群语义映射 |
|-----------|-----------|
| `buy1/sell1.get(k)` confirmed type1 @ ladder k | φ=0 奇点在 r=k（势源，候选 F/C） |
| `max_ladder` | r 上界（涌现级别，root_emergent_ladder） |
| `confirmed` 位 | candidate（前向搜索，NR-4 观测）vs located（向心确认，操作）分离 |
| `bsp_events[k]` confirmed type1 | 该级别 φ=0 实例（T47 唯一奇点的径向投影） |
| `dir_flips` / `flip_edge[k]` | ε 手性翻转沿（逐层方向滚动） |
| `cs⇒(zd,zg)` | 中枢锚（type2/3 径向圈数判别） |

> **关键约束（NR-4/G3）**：`confirmed` 位不足以区分"背驰已确认"（向心读过去，操作合法）vs"背驰段候选"（前向搜索，前向读未来非法）。v2 必须叠加 located 链（`cascade_arm` + 027:25 否定线过滤）。raw confirmed type1 系统性不交替（8 标的 ~79%）不是 bug。这关联未结算的 G3 延异 escalation。

### 8.4 step 签名（与现有 UnnStreamCore::step 同构）

```rust
fn step(&mut self, sig: &BarSig, flip_edge: &[Option<Direction>; MAX_LADDER]);
```

`MAX_LADDER=11` 是硬编码层维度，所有布尔行用 `LadderMask(u16)` 位掩码、事件用 `[_; 11]` 定长数组——v2 不得改变此维度（否则 marshal 边界与位掩码全失配）。

### 8.5 φ 的存在论位置：隐式 vs 显式（关键裁决）

**问题（接口缺口）**：`BspEvent` 没有 φ 角向坐标字段。`φ∈ℤ/23Z` 的具体相位值（走势推进到第几环）从未在信号层产出或操作层消费。现引擎中 **φ 是隐式的**——confirm fire（走势完美）是 `φ→0` 的代理；`h` 推进（`φ↦φ+1`）在群论是核心算子，但实装中没有显式 φ 状态分量。

**裁决：v2 保持 φ 隐式，`SpiralState.phi` 是观测/标注量，不是驱动量。** 理由：

1. **23 是 A-链建模约定**（若链重切分 N 环则 `h^N=σ`），不是 L0 强制。把"一个走势=23 辩证环节"映射到 bar 序列需要"哪些 bar 对应哪个环"的映射规则，**无 L0 依据**。
2. **显式化 φ ⟹ 引入环边界划分参数 ⟹ 违 231号零参数**（risk）。
3. **隐式 φ 反而更诚实**：操作层只需要 `φ=0` 这个唯一奇点（背驰∧手性翻转，T47），不需要 `φ∈{1..22}` 中间相位。整个操作层只在 φ=0 触发（买卖点），φ≠0 = "走势进行中"，操作层不消费。
4. `phi: u8` 字段保留，但语义是观测/标注量：confirm fire 时记 0，否则记"推进中"（用 confirm 代理，不字面追踪环数），**不作字面 panic 断言**（与 N_CONCEPT_RINGS=23 同口径）。

**结论**：v2 的 SpiralState 是 **(φ_隐式, r_显式, ε_显式)**——r/ε 由信号层 ladder 索引 + dir_flips 直接产出；φ 由 confirm fire 代理 φ=0。`h²³=σ` 的代数强制由 `prove_t56`（segment 无角向圈，eod 结构 panic）守，**不驱动实时 φ 推进**。显式 φ 化是诱惑性补丁，不采纳。

**保留的接口缺口（不强行补，no-patch-mentality）**：

| 缺口 | 内容 | v2 处理 |
|------|------|---------|
| bsp→φ 推进映射空缺 | φ 隐式，无角向坐标字段 | 接受（confirm 代理 φ=0） |
| r 跃迁双向不一致 | 代码 r 跃迁=经验涌现检测（`root_emergent_ladder` relabel）vs 群论 r 跃迁=角向 holonomy（`h²³=σ`） | 保留分离：relabel=会计重组（A5）/holonomy=eod 守卫（T56）；不强行统一 |
| ε 翻转 φ=0 缝无事前门控 | confirmed 位语义负载过重（同时编码 type1/2/3 三判据） | τ@φ=0 由 located 流交替（prove_s11）**事后守**，非事前门控；located 流 ~79% raw 不交替（candidate ∉ located 商空间） |
| DivEvent 趋向维 dφ/ds 缺失 | T11 背驰只兑现力量维（force_a/force_c），φ 角向速度不可达（MACD dif_peak=0） | 操作层用 located 流斜率（`last_trend_slope`）绕过；趋向维永久投影缺口（不补） |
| center_events 稀疏派生 | CenterEvent 从 BSP 锚 diff 派生，无 BSP 锚的中枢不可见 ⟹ r 径向圈数（σ 圈计数）可能漏中枢 | 090号已落盘的有效域收窄；M2 信号层下沉前 r 圈数对接不完整（标注，不补） |

---

## 9. NautilusTrader 集成

> v2 保持 PyO3 **批量 + 流式共享 step/finish** 的构造性 bit-exact 模式（R6/R9）。时间戳↔index 映射、单调性/gap/dup 守卫全部在 Python 边界层（ChanlunBridge），Rust 引擎不碰时间换算。

### 9.1 数据流（与现有 unn 同构）

```
NT Strategy.on_bar(bar)                          [NautilusTrader 挂载点]
  → RecursiveOrchestrator.process_bar(o,h,l,c)   [信号层步进，bar_index 隐式自增]
  → StreamingSignalReader.process_bar → BarSignalI
  → push_signal(SpiralStream, sig, flip_rows)    [Python 拆平坦行]
  → SpiralStream.push_bar(...)                   [v2 引擎流式步进]
  → list[trade11]                                [增量返回新 trade]
  → ChanlunBridge.drain_signals → ChanlunSignal  [仅 confirmed 出操作信号]
  → NT 下单 / on_order_filled                    [路径 B：MakerOptimizer + TradeJournal]
```

### 9.2 PyO3 导出（ffi.rs）

```rust
#[pyclass] pub struct SpiralStream { core: SpiralEngineCore }

#[pymethods]
impl SpiralStream {
    #[new] fn new(floor_ladder: usize) -> Self;     // = 2
    fn push_bar(&mut self, close: f64,
        buy1: u16, sell1: u16, sell_any: u16, buy_any: u16, up_settled: u16,
        max_ladder: u8, type2_buy: bool,
        bsp_rows: Vec<(u8,String,String,i64,bool,Option<i64>,Option<f64>,Option<f64>,f64)>,
        div_rows: Vec<(u8,String,String,i64,f64,f64,f64)>,
        flip_rows: Vec<(u8,String)>) -> Vec<Trade11>;  // 本 bar 新增 trade
    fn finish(&mut self) -> PyDict;                  // = positional_result_to_dict
    fn snapshot(&self) -> (i64, f64, f64, f64, usize); // cur_bar, nav, long_units, short_units, n_voices
}

// 批量：run_spiral(tape, floor_ladder=2) -> PyDict（与 SpiralStream.finish 同结构）
```

### 9.3 trade11 元组（固定契约，破坏即破 bit-exact 验证管线）

```
(ladder:u8, entry_bar:i64, entry_price:f64, exit_bar:i64, exit_price:f64,
 shares:f64, weight_at_entry:f64, deferred_bars:i64, partial:bool,
 exit_reason:str, polarity:'long'|'short')
```

### 9.4 集成约束

- **批量/流式 bit-exact**：`SpiralEngineCore::step/finish` 被两入口共享 ⟹ 构造性保证（非"对齐努力"）。
- **撮合口径**：`FillModel(prob_fill_on_limit=0.0)`（队列末位保守）。unn 路径几何限价固化在 Rust（不用 NT 撮合）；production 路径用 NT 撮合。两路径撮合语义必须可对账或显式声明不可混表。
- **checkpoint/resume**：live 断线恢复重放复用 `bridge.feed_ohlc` 裸入口（重放推进 watermark 不重复产 trade）。
- **两套信号架构裁决（TODO 缺口）**：unn organic_signals 磁带 vs production ChanlunBridge 直读 BSP——v2 须显式裁决合一还是永久分离（`signal_bridge.py:115-123`）。**此为选择类，须 escalate**（不自决）。

---

## 10. 零经验参数审计（诚实，formalization-validity-domain）

> **核心诚实**：v2 **不是"零经验参数"**，而是"**结构参数零自由度（从群推导）+ 不可消除的 L2 经验量（带认识论标注）**"。声称全零参数 = 声明膨胀（090号）。

### 10.1 参数逐项审计

| 参数 | 值 | 可推导 | 等级 | 来源 / 为何（不）可推导 |
|------|-----|--------|------|------------------------|
| 角向环数 | 23 | ✅ 形式 | L0（定量 A-链约定） | T58 `ℤ/23=⟨h⟩/⟨σ⟩` 角向商；23 是 A-链建模约定 |
| MAX_LADDER | 11 | ⚠ 部分 | L0 结构 + 工程截断 | NR-5 有界塔（L0：r 有界）；11 是工程截断（非 L0） |
| FIRST_BSP_LADDER | 2 | ✅ | L0 | T8 segment=第一个有势走势 |
| PENDING_LO | 3 | ✅ | L0 | T28 move(L1)=势源下界（segment 非势源） |
| SUB_SPAWN_FRAC | 0.5 | ⚠ 形式✅/值❌ | L0 形式 + L2 值 | `f=1/λ` σ-不变（L0，零自由度）；`λ=2` 是 A₅ 二分递归建模默认，待 T50 涌现 λ 测量精化为 `1/λ_measured` |
| 成本门 SUB_COST_K | 2.0 | ❌ | **L2 不可消除** | 势存在性判定本质需 L2 经验量（R7/R6） |
| 成本门 SUB_FRICTION_RT | 0.001 | ❌ | **L2 不可消除** | 同上（摩擦率） |
| 成本门 SUB_COST_Q | 0.5 | ❌ | L2 | θ 分位 nearest-rank |
| SUB_COST_MIN_OBS | 10 | ❌ | L2 | θ 分位最小观测数 |
| DEPTH_REF_WINDOW | 50 | ❌ | L2 | 中枢振幅参照窗口 |
| **强平阈值** | **2.0×basis** | ❌ | **L2 设计缺口（须上浮）** | **1x 逐仓保证金率倒数假设（capital 耗尽清算点），ambient 市场微结构（`L_max=1/(D_struct+mm)`），不可从 23/11/2/Burnside/λ 任一群结构推导。当前是内联魔数（`let b=2.0*v.basis`），无命名无标注——交叉验证（critique）揭示原审计遗漏。v2 须升为命名常数 `SUB_LIQ_FACTOR` + L2 标注 + escalate** |
| TYPE1_CONFIRM_RATIO | 0.9 | ❌ | L2（信号层） | 背驰力竭确认比（`buysellpoint.rs`，**非 unn**）；`force_c/force_a≤0.9` 是 MACD 面积经验校准（T11 趋向维永久投影缺口） |
| STOP_FRAC / SUB_EXPIRY | 0.02 / 60 | ❌ | L2（unn 未消费） | iso/旧引擎遗留（`types.rs`）；v2 若复用须审计（2% 止损=风控参数，60bar=pending 过期时间尺度） |
| a0 | 最小周期 | ❌ | **唯一基底经验参数** | T5：离散化的唯一经验参数（径向 σ 平移 gauge 零点） |

### 10.2 诚实结论

- **结构层（零自由度）**：23 / FIRST_BSP_LADDER / PENDING_LO / SUB_SPAWN_FRAC 的**形式** `f=1/λ` —— 全部从群结构（`ℤ/23` 商、势源下界、σ-不变）推导，零自由度。
- **不可消除层（L2）**：a0（分辨率）、λ 的**值**（待 T50 测量）、成本门 θ 的 4 个常数（势存在性判定本质需经验）。R7 明证：零参数、标的无关的 `r_min` 在成本门层不可能（成本门用价格振幅 θ，是 L2 经验量）。
- **ambient 市场层（L2，与螺旋几何正交）**：强平阈值 `2.0×basis`（保证金率）、交易摩擦 `SUB_FRICTION_RT`、TYPE1_CONFIRM_RATIO（MACD 校准）—— 这些是价格 ambient 坐标的属性（价格 ⊥ 螺旋，T15），群结构不预言。**强平阈值是交叉验证新发现的遗漏缺口**：它藏在 A 强平块的内联魔数里，正因为 A 是非群操作（市场作用），它的参数也是非群参数——这与 §2.3 "A 进不了群" 的判断一致，是同一事实的两面。
- **配额角色分离（范畴）**：配额 `f=1/λ`（σ-不变 L0）与成本门 θ（价格振幅 L2）**必须范畴分离**，不可把成本门伪装成势∝r 结构常数。
- **231号**：v2 不新增经验参数（零新参数）。配额已从旧 `θ_sub/θ_total` 全局归一化收敛为 σ-不变 `1/λ`（542号）。强平阈值升为命名常数 `SUB_LIQ_FACTOR` 是**重命名+标注**（不是新增），并须 escalate 其分类。

### 10.3 价格的存在论位置

价格 `c` 是 ambient 嵌入坐标，**不在 `(φ,r,ε)` 中**（T15）。含价格的量（NAV 的 ×c 项）随 r=λ^k 标度，**不是 σ-不变**；唯一 σ-不变 Casimir 是 units（T48）。v2 守恒守卫守 units（`Σunits=N_base`）而非 NAV 总额。

### 10.4 会计正确性 ⊥ alpha（L3）

森林会计零守恒违反（L2，结构鲁棒）**不蕴含**策略有 alpha（R7 实测 P1=1/8，仅 OKLO）。"会计=同一螺旋第三投影"保证存在论合法，不保证收益。**信号捕获增加 ≠ alpha 增加**。三个 BH-underperformance（天花板/子空头/units 稀释）是有效域读数（C1，不修引擎）：
- 天花板（root≤BH）= A₃（恒仓）∧T28（最高级别入场=最晚）∧T49（向心确认滞后）的联合不动点（强牛 regime）。
- 子空头全亏 = T25（父多必子空）撞 ambient 价格单边上行（539号：子空头有效域 ⊂ 非上行 regime）。
- units 稀释 = T34（Σunits=N_base 守恒）+ T18（配额 f=1/λ）几何强制派生量。

---

## 11. 实装路线（分步，每步可验收）

> 约束：每步独立可验收（prove 守卫 + bit-exact 对照）。L0/L1 步零信息增量（管线正确性），L2 步才有否证力。

### Step 0：骨架 + 状态 + 群（L0）
- 建 `rust/src/spiral/` 模块树（mod/state/group/params）。
- `SpiralState(φ,r,ε)` + helix 编码 + `GroupAction::apply`。
- 群关系守卫：`prove_tau_involution`/`prove_chirality_angular_inversion`/`prove_angular_radial_holonomy` + 各自 should_panic 反证。
- **验收**：群关系单测（`τ²=e`、`τhτ⁻¹=h⁻¹`、`h²³=σ`）通过；合成数据（L1，零信息增量）。

### Step 1：会计层移植（L1→L2）
- `voice.rs`（Voice 携带 SpiralState + VoiceForest）+ `accounting.rs`（nav/close_voice/settle/守恒）。
- 语义 bit-exact 复用 R5/R6（`Σunits=N_base`、MtM 根空头、四去向、孤儿不可能定理）。
- 守卫：`prove_n1_forest`/`prove_n8_conservation`/`prove_a5_relabel`。
- **验收**：会计原语对现有 `nav`/`close_voice`/`settle` 逐函数 bit-exact（L1）。

### Step 2：信号层消费适配（L1）
- `signal.rs`：BarSig → 群事件映射（φ=0 奇点/r 上界/ε 翻转/located 分离）。
- 能力守卫（`has_bsp_events` 等）fail-fast。
- **验收**：同一磁带，v2 signal 层产出的"群事件"与现有 located/nf 状态一致（L1）。

### Step 3：五操作作为 GroupAction（L2）
- `operation.rs`：F/C/D/E（GroupAction）+ A（非群，类型上显形）。
- C 统一 `Long`/`Short` 为 `τ.apply`（消结构债 1）；E/D 为 `σ⁻¹`/σ 逆向。
- 守卫：`prove_t14_root_flip`/`prove_n7_spawn_self_level`/`prove_chirality_seam`。
- **验收**：每 bar 优先序 A→C→D→E→F 与现有引擎逐操作等价（L2）。

### Step 4：H¹ 闭合（P-close）（L2）
- `closure.rs`：Δr=−1 检测 + 三类闭合区分 + `prove_cross_level_closure`（新）+ should_panic 反证。
- 复用 `helix_centripetal_confirm`，提升为 H¹ 显式兑现。
- **验收**：每次 CrossLevel 闭合 `child.r==parent.r−1`（L0 panic）；ValidityDomain/Category 不误兑现 Δr。

### Step 5：SpiralEngine::step 驱动 + PyO3（L1）
- `engine.rs`：`step` 驱动循环（群作用顺序应用）。
- `ffi.rs`：`SpiralStream`（流式）+ `run_spiral`（批量）共享 step/finish。
- **验收**：批量==流式 bit-exact（构造性，L1）；trade11 契约固定。

### Step 6：全量验收（L2，核心否证步）
- 8 标的 ~25M bar：群 prove + N1–N8 全程零 panic。
- **与现有 unn 引擎逐 trade bit-exact**（已验证有效域）。每处差异归因为已知 gap，否则是 bug。
- **验收**：bit-exact 通过 ⟹ "操作=群作用"L2 验证（除 A）；不通过 ⟹ 逐处归因或证伪命题。

### Step 7：gap 跟踪 + escalation（保留，不闭合）
- G1（整数股数）：units 用 f64，记录"是否 enforce 整数股数"为语法记录候选（genealogist）。
- G2（A 强平）：保持"非群"标记，跟踪 t41-zero-liq escalation。
- G3（延异）：confirm 武装规则区分向心读/前向读，跟踪 confirm-arming-differance escalation。
- G4（第三类）：保留为模型不精确信号候选。
- G5（θ 配额节点）：genealogist 开节点。
- 两套信号架构合一/分离裁决：**选择类，escalate**（不自决）。

### 依赖图

```
Step 0 (状态+群) ─┬─→ Step 1 (会计) ─┐
                  ├─→ Step 2 (信号) ─┼─→ Step 3 (操作) ─→ Step 4 (P-close) ─→ Step 5 (引擎+FFI) ─→ Step 6 (验收) ─→ Step 7 (gap)
                  └──────────────────┘
```
Step 1/2 可并行（无数据依赖）；Step 3 依赖 0/1/2；Step 4 依赖 3；Step 5 依赖 4；Step 6 依赖 5；Step 7 全程跟踪。

---

## 12. 保留的 gap 与语法记录候选

### 12.1 不强行闭合的 5 个 gap（no-patch-mentality）

G1（量子化）/ G2（A 强平声明膨胀，已 escalate）/ G3（延异，已 escalate）/ G4（第三类，未裁决）/ G5（θ 配额谱系节点）。强行闭合任一 gap = 补丁思维违规。

### 12.2 语法记录候选（交 genealogist 评估，不 escalate）

1. **轨道枚举 + 观测/操作 2×2 四分**（R3，C3 三分被证伪重构）。
2. **同级别闭合否定 → Δr=−1 升格**（NR-1，需先排除 527号 σ 命名冲突）。
3. **不变量-轨道对偶**（541号轨道/转移侧补充）。
4. **A 强平非群算子**（gap G2 的群论形式：合法操作单子 M 只含 `{h_fwd, τ_seam, σ⁻¹_read}` 三生成元，A∉M）。
5. **θ 配额谱系节点**（G5，TR-F 已结算 settled 节点未落地）。

### 12.3 须 escalate 的选择/矛盾（不自决）

- **两套信号架构合一 vs 永久分离**（选择类）。
- **是否显式 enforce 整数股数**（G1，语法记录待裁定）。
- **T41 分类：条件定理 vs 无条件 L0**（G2，已 escalate，被 L3 数据否证的语法记录）。
- **confirm 武装：背驰已确认 vs 背驰段候选**（G3，已 escalate）。

### 12.4 交叉验证揭示的悬空张力（独立 synthesizer 审查，v2 实装须警觉）

> 以下张力由独立综合工作流的 completeness critic 揭示，是跨文件不一致/未结算点。部分须 escalate，部分须在 v2 实装时显式标注归位，**不掩盖、不强行闭合**（no-patch-mentality）。

1. **NR-1 vs 同级别降成本触发的双层性**：E 在**自层 k 触发**（`nf_sell[k]` 在 voice 自层 k 的 φ=0），spawn **落点 r=k−1**。NR-1 警告"同级别 σ=e 退化中点"，但只否定"同级别闭合"未否定"同级别降成本触发"。"同级别降成本触发是否隐含 σ=e"取决于"同级别短差是否当作闭合判据"——此前提**未验证**（orbit_enumeration 自承交信号层补证）。**v2 标注**：E 的触发（`h` 自层观测）与落点（`σ⁻¹` 跨级别闭合）是两个群作用的**复合**，不是单一 `σ⁻¹`；悬空张力，须信号层补证后裁决。

2. **MtM 根空头会计自洽性破裂**：`nav()` 对根空头用 `capital−units×c`（有独立外部市场负债），对子空头用 frozen capital（无独立负债）。会计文档声明"nav 的 capital 形式是物理单真值（无独立 MtM 负债）"与根空头 MtM **直接冲突**——同一 `nav()` 两套口径。T8：frozen-internal（`parent.is_some`）⊥ MtM-external（`parent.is_none`）。**v2 处理**：归入 **category 类闭合**（范畴分层：根空头 vs 子空头是不相交对象类，§6.3），不强行统一为单一 nav 口径；必须保留 `is_root∧Short⟹continue`（根空头不嵌套降成本 E）。

3. **T41 跨文件三方冲突**：`necessity_derivation` 内部 Phase-1（条件定理）⊥ Phase-4（无条件 L0）**自相矛盾**；`dialectical` 归 G2（声明膨胀）；unn 代码实测 fire 强平（161 次）；且强平计数对实装细节敏感（confirmed_root raw 残余，BTC liq 0→216 config 依赖）⟹ **L3 否证本身不稳定**（161 强平数可能含实装 artifact）。**已 escalate（t41-zero-liq）**，v2 不自决；A 强平逻辑实装（Step 3/6）前须等裁决。

4. **23 的认识论地位不一致**：`necessity`/`orbit` 自承"23 是 A-链建模约定（若 N 环则 `h^N=σ`）"，但 `dialectical`/`orbit` 又把 `ℤ/23` 当代数强制（`46=23×2` Burnside 基本域）。**定性 L0（有限基本域）/定量 A-链约定（23 具体值）**。v2 处理：基本域有限性 L0（守 `prove_t58` 结构 panic）/23 具体值不作字面 panic（与 φ 隐式同口径，§8.5）。

5. **N_base 跨文件语言不一致**：会计文档已修（"恒定"=不主动加仓，N_base 双向重定基）；`necessity`/`concept_movement_chain` 多处仍用"恒仓/N_base 恒定"L0 语言。v2 会计层统一采用会计文档读法（双向重定基），必然性层旧读法是过时表述（genealogist 跟踪）。

> **方法论说明**：本节是用工作流并行深读（9 reader）+ 独立双 synthesizer 综合的交叉验证产物。手动综合（主上下文）与独立综合（synthesizer）的差集 = 本节 + §10.1 强平阈值缺口 + §8.5 φ 隐式裁决——这些是单一视角易漏、双视角才暴露的点。完整性批评（completeness critic）的价值在此显形。

---

## 附录 A：58 不变量 → prove 守卫映射（节选）

| 不变量 | 内容 | 群坐标 | prove / 等级 |
|--------|------|--------|-------------|
| T2 | 走势完美三坐标合一 φ→0∧settle∧ε | 三坐标合一 | 定义层（confirmed=since<bar∧helix，无 prove） |
| T14 | BSP 首尾相连无间隙 | ε 在每个 φ=0 翻转 | `prove_s11_s9_located`（L2） |
| T18 | 配额 f=1/λ σ-不变 | 径向 r 定义 | `prove_theta_sigma_invariant`（L2） |
| T24 | 多空对称 | τ 对合 ℤ/2 时序翻转 | `prove_t14_root_flip`（L2） |
| T30 | 根级别涌现=会计重组 | σ 作用 τστ⁻¹=σ⁻¹ | `prove_a5_relabel`（L2） |
| T34 | 股数守恒 Σunits=N_base | σ-不变 Casimir | `prove_n8_conservation`（L2 每 bar） |
| T42 | 孤儿不可能（森林单根） | 径向森林后序 | `prove_n1_forest`（L2） |
| T49 | 展开恒等式→向心回溯 | 径向母线向心 | `prove_chain`+`helix_centripetal_confirm`（L2） |
| T56 | 角径全纯 h²³=σ | 细胞{φ,r} | `prove_angular_radial_holonomy`（L0 结构 panic） |
| T57 | 手性-角向反演 τhτ⁻¹=h⁻¹ | 细胞{φ,ε} | `prove_chirality_angular_inversion`（L0）+ ~观测 |
| T58 | 角向基本域 23 环 | ℤ/23 商 | `prove_t58_angular_basic_domain`（L0 结构 panic） |
| T59 | 尺度不变 σ 自相似 | ⟨σ⟩≅ℤ 平移 | `prove_t59_scale_invariance`（~观测）+ L3 |
| A1（H¹） | Δr=−1 跨级别闭合率 | r 仿射作用生成元 | **`prove_cross_level_closure`（L0，新）** |

> 完整 58 条见 `necessity_derivation.md §11.2`（N=58 穷尽，gap=0）。

## 附录 B：23 环 → 操作映射（节选）

| 环 | 名称 | 群作用 | v2 操作 |
|----|------|--------|---------|
| 1 | 走势终完美 | 推进 φ（初态） | — |
| 11 | 买卖点=完美操作时点 | 翻转 ε | F/C 触发点 |
| 13 | 级别=递归层次 | 跃迁 r（诞生维度） | r 坐标 |
| 14 | 区间套=层间桥梁 | 跃迁 r（收窄） | F source 定位 / P-close |
| 17 | 降成本=低级别完美利用 | 跃迁 r（跨层） | E（σ⁻¹） |
| 20 | 嵌套递归=voice 自相似 | 跃迁 r（spawn） | E spawn 子 voice |
| 21 | 多空对称 | 翻转 ε | C（τ） |
| 22 | 多空嵌套=方向交替递归 | 翻转 ε × 跃迁 r | E 子方向交替 |
| 23 | 操盘三阶段（闭合接缝） | 翻转 ε + 跃迁 σ | C=出场=翻转=新建仓（h²³=σ） |

> 完整 23 环见 `concept_movement_chain.md`。
