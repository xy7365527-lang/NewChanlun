---
id: "671"
number: 671
title: P2-R2 消 MACD C≥A 预删选择偏差——struct_break_dir 独立方向源 bit-exact 恢复；力度判据本身未接通生产（defer #42）
status: 生成态
type: bias-correction
date: 2026-07-01
depends_on: [615, 663]
links:
  - beichi.md
  - divergence.rs:segments_diverge（★诚实 gap 第17课）
  - interval_nesting_not_called_in_backtest（同构：简化被当完整）
  - "#42（关于背驰.pdf 支配序升级承接点）"
epistemic_level: L1（方向恢复=合成管线验证）; L2 未做（可交易 alpha=W-VERIFY #23）
heterosource: codex-p2-impl-audit-20260701.md（conditional-pass，五维全不阻塞）
---

# 671 号：P2-R2 选择偏差消除（结构方向恢复层）与力度判据 defer 的分层裁定

## 结论

P2 R2 消除 MACD「C≥A 面积一票否决」的**结构方向选择偏差**：`BspPoint.struct_break_dir: Option<Side>`
（`bsp.rs:136`）作为**独立于 MACD 的结构方向源**，让零 bit（C≥A 未背驰）的破中枢候选在
`interp::candidate_dir` 消歧层恢复 Long/Short 方向、进入 μ 样本，而非被 MACD 面积在信号产出前
预删。

**bit-exact 安全**（硬约束 G1/G2/G4/G5/G7a 全闭合）：
- `struct_break_dir` **绝不进** `BspBits`/`MuClass`/`class_index()`/分桶 key（`bsp.rs:131-132` 注释
  锁定；`class_index()` 仍只读 level/delta/i_class/parent_dir/short_swing/position 六维，types.rs:229）。
- buy1/sell1 判据**完全不动**：C<A 置 buy1 六 bit（走 `root_sel`，`struct_break_dir` 一致不改结果，
  护栏1）；C≥A 六 bit 全零 ⟹ `class_index=0`，靠 `struct_break_dir` 在 `candidate_dir` 恢复方向。
- 唯一 GOLDEN 翻转 `0x37d2_45a7_cdc5_505a → 0x56ed_dd65_1c59_5733`（`signal.rs:1616-1618`）是
  **诚实的受控代价**——FNV 对 `{pts:?}` Debug 串计算，新增字段进 Debug ⟹ 哈希变，**非六 bit
  语义变化**。六 bit 不变由 `extract_signals_bit_exact_vs_orig_per_case` 逐案验证（主验收），GOLDEN
  仅快照辅助（方案A）。
- 双触发 (1,1) 防护：`candidate_dir` 严格 `!conf_plus() && !conf_minus()` 只匹配真零 bit，(1,1)
  的 conf_plus=true 被排除（护栏1，codex Q1 接受）。
- B2/S2 构造路径（`signal.rs:412/430`）明确 `struct_break_dir: None`——非破中枢结构候选，无野路径。

## 定义依据

- **缠师第17课**（`017-第17课.md:248/250`）：「用均线或 MACD 看背驰都是**辅助性**的」「背驰 =
  两相邻同向趋势间后者比前者的**走势力度**减弱」——走势力度是判据，MACD 面积只是一个 proxy。
- **615 号**（μ_f ⊊ 缠论严格分类）：L1 μ_f 分类是 L2 缠论分类的子集。P2 veto 是这个 subset 关系
  在 MACD 层的一个实例——MACD 面积门比缠论走势力度判据更窄，把 C≥A 但方向明确的破中枢候选
  预删了。struct_break_dir 恢复的正是这批被 μ_f 层过窄 gate 删掉的样本。

## 诚实边界（L1/L2 分层——no-patch 不粉饰）

### 已闭合（L1，方向恢复层）

方向恢复 = **L1 合成管线验证**：消除了「几何门通过 ∧ A/C 可配对候选因 MACD C≥A 预删」这一
结构方向选择偏差。接通链在片段层闭合（codex Q2 接受）：
`assemble_gamma_with_tower → candidate_dir → 非 Flat → 不被 Flat 过滤 → signals.push`。

**有效域收窄**（codex 强制，G6 已满足）：仅「几何门 ∧ A/C 可配对候选不再因 MACD 预删」，
**非** Γ_struct 全量进样本，**非** canonical 力度判据（支配序）完成。

### 未闭合（有意 defer #42，非本任务返工项）

**力度判据本身未接通生产**（G3/G7b HONEST GAP，codex Q3/Q4 接受分层验收）：
- **G3**：`divergence.rs` 力度原语（`segments_diverge`/`is_divergence`）存在且带**★诚实 gap
  注释**（第17课，编排者 2026-07-01 坐实）——但**背驰仅由 MACD 段面积判定**（Σ|hist|），
  **无独立走势力度判据**（价格振幅/速度/量能）。这是比 P2 veto **更根本的简化**：辅助指标被
  当成背驰唯一判据，与缠师原文相悖。
- **G7b**：`force_features`（`divergence.rs:362`）**生产零调用**——唯一调用点是测试
  （`divergence.rs:769/791/796/801`），死代码。支配序/递归力度签名未接通 selector。

**同构**（既有谱系）：区间套同一性证书零调用（`interval_nesting_not_called_in_backtest`）、
简化被当完整——同一失效模式的第 N 例。

**可交易 alpha = L2 未做**：是否真消除偏差后产生可交易 alpha = W-VERIFY（#23），本任务不声明。

## 边界条件（结论翻转条件——codex 裁定）

1. 发现 B2/S2/其他 BspPoint 构造路径给零 bit 点填 `struct_break_dir=Some` → 双职责 bug 实化 →
   bit-exact 护栏可能失守 → 需返工。
2. `class_index()` 被发现读了 `struct_break_dir`（六 bit 之外）→ bucket key 改变 → 硬约束失守。
3. 有证据 buy1 bit **必须**代表 canonical Weak_Θ 支配序背驰（非 MACD-area proxy）→ scope 裁定
   需重审 → 力度不接通 = R2 未完成（而非分层验收）。

## 下游推论

- μ 样本扩大：C≥A 但方向明确的破中枢候选进入样本，改变 μ 分布。W-VERIFY（#23）须在**扩样本后**
  的 μ̂/LCB95/OOS 上重跑，不能沿用 veto 版分布。
- **codex 防御性建议**（非当前 bug，随 #42 落地）：`struct_break_dir` 双职责（结构方向元数据 +
  零 bit 恢复 trigger），恢复 safety 依赖所有构造点外部约束（非类型保证）。#42 用 newtype 或显式
  恢复资格 flag（如 `dir_recovery: Option<RecoveryReason::MacdCGeA>`）强化类型安全。
- #42（P2 力度层升级）承接点：ForceMeasure 增非-MACD strength 实例（振幅/速度/量能）、支配序
  ForceState 三值、递归力度签名 𝔉_ℓ，MACD area 降为多 proxy 之一（与 P2「MACD 降 feature」同精神）。

## 谱系引用

- **615 号**：μ_f ⊊ 缠论严格分类——P2 veto 是该 subset 关系在 MACD 层的实例（本条直接推论）。
- **beichi.md / divergence.rs `segments_diverge`**：背驰≠MACD 面积的诚实 gap 已在源码就地标注。
- **`interval_nesting_not_called_in_backtest`**（memory）：同构失效模式（简化被当完整），本条是该
  类的又一实例（力度判据零接通）。
- **codex-p2-impl-audit-20260701.md**：异质审计 conditional-pass，五维全不阻塞 R2 crystallize。

## 影响声明

- **改动模块**：`classifier/bsp.rs`（新增 `struct_break_dir` 字段 + 注释）、`classifier/signal.rs`
  （`judge_first_cached`/`make_first_point` 传方向、删死 sidecar、GOLDEN 更新、per-case 测试）、
  `backtest/runner.rs:1486`（字段初始化）。`classifier/interp.rs`（`candidate_dir` 消歧读字段）。
- **未改**：`class_index()`/`BspBits`/`MuClass` 序列化路径（bit-exact 隔离）；`divergence.rs` 力度
  原语（defer #42，仅标注诚实 gap）。
- **谱系效力**：新增本条（生成态，待 #42 力度层接通后可能升级/结算）；不改既有已结算条目。
