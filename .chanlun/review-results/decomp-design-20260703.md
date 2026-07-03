# 走势类型分解 + CurrentMove 流式状态机——设计文档（Task #143 阶段一）

**日期**：2026-07-03 | **工位**：ws-decomp | **规格来源**：`docs/formal-chain/一类买卖点.pdf`（§6 分解算法 / §8-Q1/Q8 裁决 / §9.2 Move 结构 / §10 不锁死证明）+ `.chanlun/review-results/type1-zero-external-review-20260703.md`（漏斗坐实死点=趋势门）
**性质**：设计先行，零生产代码改动。阶段二实装等 #142（中枢延伸）落地后接线。

---

## 0. 问题与修复位置

现状三处代码实现同一个错误谓词 AllTrend（全历史累积中枢链全链同向）：

| 位置 | 函数 | 消费者 |
|---|---|---|
| `rust/src/theta_v0/classifier/level.rs:62` | `classify_move(centers) -> MoveOutcome` | `mod.rs:216`（classify_impl 全量路径） |
| `rust/src/theta_v0/classifier/mod.rs:597` | `classify_move_incremental`（O(1) 续判，含吸收锁死注释「一旦破裂不可恢复」） | `mod.rs:1351`（增量塔路径） |
| `rust/src/theta_v0/classifier/divergence.rs:529` | `trend_class(centers) -> TrendClass` | `signal.rs:627`（L0 τ 门）、`signal.rs:763`（type1_funnel_dx 探针，cfg(test)） |

PDF §2 已证明 AllTrend 是吸收锁死谓词（∃j, r_j∉{全Up,全Down} ⟹ ∀t'>t, trend_class=None），外审漏斗坐实 BTC 全历史在数据第 1–17 天锁死。§6 的修复：把中枢链**分解**为 maximal 走势类型块 C_ℓ = B₁⊕B₂⊕…⊕B_k，趋势门只作用于当前/刚完成的块（#144 消费）。本设计给出分解的数据结构、算法、流式状态机、迁移表、增量策略、测试计划。

---

## 1. Move 结构定义（PDF §9.2：M = (C_a,…,C_b, kind, dir, status)）

新文件 `rust/src/theta_v0/classifier/decompose.rs`：

```rust
/// 走势类型块状态（Q8：status ∈ CurrentMove 五元组）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveStatus {
    /// 当前走势类型（链尾块，尾中枢/尾关系仍可变——#142 延伸或新关系可改写它）。
    Active,
    /// 已完成（后继块已开启：关系标签切换处即本块右边界，前缀不可变）。
    Completed,
}

/// 走势类型块（PDF §9.2 M=(C_a..C_b,kind,dir,status)；中枢下标闭区间指向本级 centers）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveBlock {
    /// 块 span 首中枢下标（含）。span 与相邻块共享边界中枢（见 §2 ownership/span 二投影）。
    pub start_center: usize,
    /// 块 span 尾中枢下标（含）。Trend 块的「最后一个中枢」= centers[end_center]（Q8 裁决）。
    pub end_center: usize,
    pub kind: MoveKind,            // Trend | Consolidation（types.rs 现有枚举，复用）
    pub dir: Option<Direction>,    // Trend 携方向；Consolidation = None（第31课盘整无方向）
    pub status: MoveStatus,
}
```

- 复用 `types::MoveKind` 与 `types::Direction`，不新增平行枚举（no-patch：不留两套走势类型词汇）。
- `MoveOutcome`（level.rs）与 `TrendClass`（divergence.rs）在阶段二**删除**——它们是 AllTrend 谓词的返回类型，谓词删除后类型无存在理由。`HigherCenterCandidate/Degenerate` 语义由分解自然吸收：0 中枢 = 空分解；混合链 = 多块序列（不再是级别整体退化裁决）。

## 2. maximal 块分解算法（PDF §6）

**输入**：canonical 中枢链 `centers = (C_1..C_m)`（#142 延伸吸收后的产物——延伸修复是本分解的前置，否则 94.4% overlap 碎片污染关系链，PDF §4）。
**关系标签**：`R_i = classify_relation(C_i, C_{i+1})`（center.rs 现有 GG/DD 判据，Q3 裁决「GG/DD 口径保留」）∈ {UpContinuation, DownContinuation, LevelExpansion}。三值记为 {Up, Down, X}。

**算法**（单趟 O(m)）：把关系序列 R_1..R_{m-1} 切成 **maximal 等标签 run**；每个 run [p..q]（关系下标）产一个块，span = 中枢 [p..q+1]：

- run 标签 ∈ {Up, Down} → `MoveBlock{kind: Trend, dir: Some(d)}`（span ≥2 中枢自动满足趋势定义「≥2 依次同向中枢」）。
- run 标签 = X → `MoveBlock{kind: Consolidation, dir: None}`（PDF §6 盘整块情形2：相邻关系为 overlap/expansion 的序列——在递归塔中这正是上级中枢的种子，见 §3 触发4）。
- m == 1（无关系）→ 单块 `Consolidation` span=[0..0]（PDF §6 盘整块情形1：无同向后继的单中枢）。
- m == 0 → 空分解（无块，CurrentMove 不存在）。
- 链尾块 status=Active，其余 Completed。

**span 与 ownership 双投影**（完备性的精确表述）：

相邻 run 共享边界中枢（[Up,Down] 中 C_2 既是上涨块尾中枢又出现在下跌块 span 首）。这是缠论语义本身：「走势终完美」——转折中枢完成前一走势类型，同时是后一类型的关系起点。两个投影：

- **span**（结构域，供 #144 趋势门取 C_last/C_prev）：块 j 的 span = [p_j .. q_j+1]，相邻块 span 交叠恰一个边界中枢。
- **ownership**（分区，完备性口径）：中枢 C_i 归**其闭合关系所在的块**——C_i 属于包含 R_{i-1} 的块；C_1 属于 B_1。即转折中枢归前块（前一走势在它处完成）。

**完备性定理**（property test 验证对象，§6）：
1. 关系分区：每个 R_i 恰属一个 run（maximal run 切分的定义性质）。
2. 中枢分区：ownership 下每个 C_i 恰属一块（i≥2 由关系分区唯一性直推，C_1 显式归 B_1）。
3. maximality：相邻块标签不同（否则违反 maximal，run 应合并）。
4. 覆盖：⊕ 无遗漏（∑ owned = m）无重叠（分区定义）。

**#144 趋势门接口**（本设计对下游的输出契约）：

```rust
/// 全量分解（单一来源；增量走 resume 路径，见 §5）。
pub fn decompose(centers: &[Center]) -> Vec<MoveBlock>;

/// 当前/刚完成趋势块（PDF §9.3 Type1Cand 条件1-2 的取块口径）：
/// 链尾块是 Trend → Some(尾块)；链尾块是 Consolidation 或空 → None。
/// 「最后一个中枢」= centers[block.end_center]，prev = centers[block.end_center-1]
/// （span 保证 Trend 块 end_center ≥ start_center+1 = ≥2 中枢，条件2 定义性成立）。
pub fn last_trend_block(blocks: &[MoveBlock]) -> Option<&MoveBlock>;
```

注意：#144 的门是「当前**或刚完成**」趋势块——若链尾是刚被 X 关系闭合的 Trend 块（新盘整块刚开启、仅 1 个自有新中枢），破中枢段仍可能是该 Trend 块的一类离开段。此窗口语义（刚完成块的可用时限）属 #144 的 Type1Cand 条件3-4 判据域，本层只保证 `blocks` 序列如实携带 Completed 块，不做过滤。

## 3. CurrentMove 流式状态机（Q8 四触发边界）

流式态（每级一份，进 `LevelCache`）：

```rust
pub struct DecomposeState {
    /// 冻结块前缀（只含冻结关系 R_1..R_{m-2} 构成的完整 run；见 §5 冻结不变量）。
    frozen_blocks: Vec<MoveBlock>,
    /// 冻结关系数（= 已处理到的关系下标上界）。
    frozen_rels: usize,
}
```

`CurrentMove_t`（Q8 五元组）= 分解输出的链尾 Active 块，不是独立数据结构——单一来源，避免状态机与批式分解两套真值。

**事件驱动转移**（事件源 = #142 的中枢构造 resume 路径）：

| # | Q8 触发 | 事件 | 转移 |
|---|---|---|---|
| 1 | 当前中枢停止延伸 | #142 non-extension（d_j>ZG ∨ g_j<ZD） | 不闭块——只是 C_m 定稿，使 R_{m-1} 从临时变冻结（§5）。它是触发 2/3 的前置。 |
| 2 | 新 canonical 中枢在延伸关系之外出现 | centers 追加 C_{m+1} | 算 R_m = Rel(C_m, C_{m+1})：与尾块标签同 → 尾块 end_center+=1（走势延续）；异 → 尾块 status=Completed，开新块 span=[m..m+1]（新标签 Up/Down→Trend，X→Consolidation）。 |
| 3 | 关系从同向延续变为 overlap/反向 | 事件2 的「异标签」分支 + #142 延伸改写 C_m 外缘使**临时关系 R_{m-1} 翻转** | 后者：回退临时尾（§5），按新 R_{m-1} 重开尾块。冻结前缀不动。 |
| 4 | 更高级别中枢/盘整形成 | 递归塔：本级 X-run 的中枢在 ℓ+1 级凑齐窗口成中枢 | 无需本级额外动作——X-run 块已记 Consolidation；ℓ+1 级中枢形成是塔的既有 compose 路径（LevelExpansion 关系正是升级种子）。对应关系在测试中断言（X-run 块 span ⊆ 某 ℓ+1 中枢的构成窗口）。 |

初始/边界转移：空 → 首中枢 C_1 → 单块 Consolidation Active（盘整块情形1）；单块 Consolidation(1 中枢) + 同向 R_1 → **升格**为 Trend span=[0..1]（单中枢+同向后继 ≠ 盘整块，PDF §6 情形1 反面）；+ X 关系 → 保持 Consolidation，end_center+=1（情形2 多中枢盘整）。

## 4. 接口迁移表

| 现状 | 阶段二处置 | 迁移后 |
|---|---|---|
| `level.rs classify_move/MoveOutcome/outcome_to_kind/all_adjacent` | **删除**（AllTrend 谓词全家） | `decompose.rs::decompose` |
| `divergence.rs trend_class/TrendClass/all_same_relation` | **删除**（同一谓词的第二份拷贝） | `last_trend_block(&decompose(centers))` |
| `mod.rs classify_move_incremental + LevelCache.cached_outcome` | **删除/替换** | `LevelCache.decompose_state: DecomposeState`（§5） |
| `mod.rs:281,1352 moves: Vec<MoveKind>`（≤1 元素） | 语义升级 | `LevelState.moves` = 全部块的 kind 投影（真序列）；同时新增 `LevelState.move_blocks: Vec<MoveBlock>`（#144/#145 消费方向与 span，econ_positive.rs:124 注释抱怨的「MoveKind::Trend 丢方向」由此消解） |
| `signal.rs:627 extract_signals_with_hist` τ 门 | #144 改写（本任务不动） | 门输入从 `trend_class(centers_sorted)` 换 `last_trend_block`；C_last/C_prev 从块 span 取（不再是全局链尾两中枢） |
| `signal.rs:763 type1_funnel_dx` 探针（cfg(test)） | #144 随门更新，环1 计数从 AllTrend 换块口径 | 漏斗 parity 断言保持 |
| `mod.rs:2040+ type1_funnel_census_btc` / `:2059-2073` 关系统计 | 诚实重算（诊断口径变） | 对拍改用 decompose |
| `l3_pi_probe.rs:172,203 lv.moves.len()` 统计 | 无代码改动，数值诚实变化（0/1 → 块数） | 诊断输出，无下游判据消费 |
| `complete/state.rs:228 rec_struct.moves: Vec<(MoveKind,usize,usize)>` | 生产写入方为零（仅 mod.rs:193-196 测试写）——placeholder，接线时直接以 MoveBlock 投影填充 | `(kind, span_start, span_end)` 与 MoveBlock 天然同构 |
| `lib.rs MoveTuple` / `src/moves.rs`（pyo3 桥，旧引擎谱系） | **不动**——非 theta_v0 谱系，不在本修复序 | — |
| `recursive_tower.rs` | 本任务零改动（#142 在此实装延伸；分解只消费其 canonical centers 输出） | compose/sub_moves 结构不变 |
| GOLDEN | 诚实重算（LevelState PartialEq 含 moves，位形必变） | 阶段二结果包附前后 diff 与归因 |

递归塔耦合面澄清：`classify_impl` 的递归推进（`units = project_to_units(&upper_moves)`）**不消费走势裁决**——outcome 只投影进 LevelState.moves，删除 AllTrend 不触碰塔的级别构造/终止逻辑。改动被限制在「裁决投影 + τ 门」两点上。

## 5. 增量路径（T^inc == T^full 维持策略）

沿用 #142 已立的单一来源模式（`detect_centers_windowed_resume`：全量 = start_i=0 的 resume）：

- **单一来源**：`decompose_resume(centers, state) -> &[MoveBlock]`；`decompose(centers)` ≡ 空 state 的 resume。T^inc == T^full **定义性成立**，非对拍性成立。
- **冻结不变量**：一旦 C_{m+1} 种子出现，C_m 永不再变（#142 non-extension 定稿）⟹ 关系 R_1..R_{m-2} 不可变 ⟹ 只含冻结关系的 run/块不可变（前缀冻结）。唯一可变量：临时关系 R_{m-1}（C_m 延伸改写 DD/GG 可翻转其标签）。
- **每 bar 续算 O(1)**：state 持冻结块 + 冻结关系数；每 bar 只重算临时尾（R_{m-1} 的标签 + 尾块开/合/升格），至多触碰链尾 1-2 个块。中枢定稿事件把临时尾并入冻结前缀。
- **回退路径**：centers 回缩/前缀改写（cascade_reset、`LevelCache::clear`）→ `DecomposeState` 清空全量重算——与现有 `cached_outcome`/`cached_bsp_key` 同一回退纪律，bit-exact 合法退化。
- **性能边界**：全量 O(m) 单趟、增量摊还 O(1)/bar，替换的 `classify_move_incremental` 同为 O(1)——无回归。

## 6. 测试计划

配合 #144（不锁死证明的**门级**测试归 #144；本层提供分解级性质）：

1. **分解完备性 property test**（proptest 随机中枢链，含随机 GG/DD 几何）：关系分区/中枢 ownership 分区/maximality（相邻块异标签）/Trend 块全内部关系同向且 span≥2/⊕ 重组无遗漏无重叠。
2. **不锁死前件**（PDF §10，供 #144 复用）：构造「早期 overlap + 后续局部同向 run」链，断言 `AllTrend=0 ∧ last_trend_block=Some(Trend(d))`——旧谓词锁死、新分解产块。牛市窗 2 sell1 的全历史复现（跑 B 反差闭合）是 #144 验收项。
3. **T^inc == T^full**：随机事件序列（追加/延伸改写尾中枢外缘/回缩）驱动 resume，与全量 `decompose` 逐字段 bit-exact。
4. **Q8 四触发单元测试**：每触发一例——延伸定稿不闭块 / 新中枢同向延续 / 同向→overlap 闭块开盘整 / 临时关系翻转回退重开；加 X-run 块与 ℓ+1 中枢窗口的对应断言（触发4）。
5. **边界**：m=0/1、单中枢升格 Trend、链尾刚闭合 Trend + 新开盘整、全链同向（分解=1 块，与旧谓词唯一重合情形 bit 一致）。
6. **GOLDEN 诚实重算**（阶段二）：BTC 漏斗探针 parity + 全量/增量对拍 harness 复跑。

## 7. 结果包

- **结论**：分解设计如上——关系 run 切分 + span/ownership 双投影 + 事件驱动流式尾块 + resume 单一来源。
- **定义依据**：PDF §6（maximal 块 + 盘整块两情形）、§9.2（M 五元组）、Q8 裁决（「最后一个中枢」相对当前走势类型 + 四触发）、Q1 裁决（门作用域=分解后当前块）、Q3 裁决（GG/DD 口径保留）；第17课趋势定义、第18课分解定理一、「走势终完美」（转折中枢归前块的 ownership 依据）。
- **边界条件**（结论翻转条件）：①若 #142 落地后 canonical 链仍有大比例 X 关系（延伸吸收不充分），分解产出以盘整块为主，趋势门开启率仍可能低——那是延伸实装问题而非分解算法问题，但会要求重审「X-run=盘整块」与「升级为高级别中枢」的边界（Q8 触发4 从对应断言升级为显式状态）；②若 #144 发现「刚完成趋势块」窗口语义需要块内追溯（不止链尾 1 块），`last_trend_block` 接口需扩展为带时限参数；③若编排者裁定转折中枢归**后**块（ownership 反向），分区证明重写但 span/门口径不变。
- **下游推论**：#144 的趋势门与 Type1Cand 五条件可直接建立在 `last_trend_block` 上；#145 的 A/C 次级别走势类型化获得 MoveBlock 方向来源（Q7 高级别方向=次级别走势类型方向）；#146 重跑的前提对象（全历史一类>0 或诚实为 0）从此建立在正确定义域上（formalization-validity-domain：门谓词的有效域从「全历史链」修正为「当前块」）。
- **谱系引用**：type1 零触达谱系（#116→#119→#141 外审→本修复序）；记忆锚 `project_gap3_*`/`project_frontier_resume_bt_too_late`（frontier resume 时机先例）；`.chanlun/review-results/type1-arch-audit-20260703.md`。
- **影响声明**：本文档零代码改动。阶段二将删除 3 处 AllTrend 谓词拷贝、新增 decompose.rs、升级 LevelState.moves 语义、更新 GOLDEN——影响 classifier（level/divergence/mod/signal）与全部消费 LevelState.moves 的诊断路径；不触碰 recursive_tower 结构、closed_loop、旧引擎 pyo3 桥。
