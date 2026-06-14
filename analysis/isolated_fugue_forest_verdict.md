# 逐仓独立声部森林（iso）L3 判决

> 任务（2026-06-14 编排者）："把这个实现了，我需要这个任务里的所有东西都严格实现。"
> 范围裁决（AskUserQuestion）：**新建 `isolated_fugue.rs` 全森林**（保留 v4/URS 不回归）。
> 认识论等级：**L3**（8 标的 × 真实数据 447K–5.5M bars × 跨资产）。

## 1. 结论

四改动全部严格实装并通过验证；**实装正确**（335 单测通过 + ~25M bars 真实数据
零守恒违反），但 **L3 经验否证用户核心前提**："没吃到信号 → 逐仓森林吃更多 →
更赚"——不成立。

- **P1（iso ≥ BH）：1/8**（仅 OKLO +570% vs +307%）。清仓频率/alpha = regime
  函数不可约的第 N 次复现。
- **iso ≻ urs：6/8**（BTC +548.7pp / OKLO +248.1pp 显著）——作为 URS 后继，iso
  是明确改进（E* 清仓回现金 > URS 翻转 churn）。
- **iso ≺ v4：5/8**（v4 审计基座仍最强）；iso ≻ v4 仅 3/8（BRN +29.4 / CL +18.1 /
  BTC +128.3，= 深崩/弱域）。
- **R_capture（iso 比基线多吃信号）：3/8**（ES/GC/BTC）——且多吃的 ES/GC 既不
  跑赢 BH 也不跑赢 v4。**信号捕获与 alpha 正交**（"12 信号丢失机制"被修 ≠ alpha）。

### 完整数据

| 标的 | BH% | iso% | P1 | iso_MDD | spawns | trades | 清仓 | vs_v4 | vs_urs | 多吃信号 |
|------|-----|------|----|---------|--------|--------|------|-------|--------|---------|
| OKLO | 307.1 | **570.0** | ✓ | -71.7 | 258 | 265 | 6 | -125.1 | **+248.1** | ✗ |
| QQQ | 174.6 | 168.2 | ✗ | -23.0 | 19 | 30 | 8 | -27.7 | +2.0 | ✗ |
| BRN | 87.4 | 70.1 | ✗ | -71.8 | 203 | 222 | 12 | +29.4 | +13.6 | ✗ |
| DX | 4.1 | 2.7 | ✗ | -15.9 | 0 | 12 | 8 | -1.2 | -1.7 | ✗ |
| ES | 594.3 | 499.4 | ✗ | -33.7 | 175 | 185 | 5 | -48.4 | +11.0 | ✓ |
| GC | 257.3 | 242.9 | ✗ | -41.3 | 222 | 238 | 11 | -55.0 | -2.1 | ✓ |
| CL | 28.2 | 20.8 | ✗ | -79.6 | 404 | 428 | 13 | +18.1 | +16.1 | ✗ |
| BTC | 1380.4 | **1098.6** | ✗ | -76.6 | 713 | 720 | 6 | +128.3 | **+548.7** | ✗ |

## 2. 定义依据

- **逐仓 voice（§1-§8 + §11 审计）**：v4/URS 已是逐仓 voice 双层会计（§11 审计
  D1 存一次/D6 递归守恒 = CONFORMS）。任务 #3"净额→逐仓"的核心**已存在**——
  iso 的增量是**多声部森林**（root 可多 child，`child_voice_ids: Vec`），非"从
  净额改逐仓"。
- **栈→森林（四改动）**：
  1. per-voice `acted_bar`（去全局 `acted: bool` 互斥）——同 bar 多 level/多 voice
     独立操作（改动1+4）。
  2. Type2 接入区间套窗口（`e.class.side()` 归侧，删 `Sell2|Buy2 => continue`）
     ——中枢回测确认词汇（概念链第12环，改动2）。
  3. `VoiceLedger` 森林 + 守恒律 §8.1（Σ 活跃在手 = N_base）+ §8.3（child P&L ≡
     parent cost_reduction）每 bar 守卫（改动3）。
- **清仓（§6）**：URS E* 涌现归属层 sell_any ∧ 递归确认 ⇒ cascade 全树回现金
  （十年 1-2 次，iso 实测 5-13 次/标的 = 忠于"不清仓原则"）。

## 3. 边界条件（结论翻转条件）

- **P1 翻转**：iso 在 OKLO 成立（高波动新股震荡域）；强单边牛（ES/QQQ/BTC）下
  iso < BH（清仓罕见 → 骑过回撤 → 但仍 < 满仓 BH）。若换震荡 regime 标的，P1
  比例可能上升。
- **iso vs v4 翻转**：iso > v4 仅在深崩/弱域（CL 油崩 / BRN / BTC 极端波动）——
  E* 清仓在崩盘中比 v4 θ 棘轮清仓更及时；牛市侧 v4 的 θ 棘轮少清仓 > iso 的
  E* 频繁回落清仓。
- **R_capture 翻转**：仅当标的结构产生大量同 bar 多 level 信号 ∨ 同级别多声部
  并发时（ES/GC/BTC），森林宽度 > 1 才激活多吃。OKLO depth 直方图 max=5 但
  spawns(258) < urs(441) ⇒ 多吃未发生。
- **MDD 翻转**：iso MDD 普遍劣于 v4（OKLO -71.7 vs -55.2）——§6"清仓十年1-2次"
  使 iso 骑过回撤。若放宽清仓判据（非 §6 忠实），MDD 改善但偏离规格。

## 4. 下游推论

- **若 iso 成立**（作为 URS 后继）：URS 的翻转清仓（C_CLEAR_NOT_FLIP=false）应被
  E* 清仓回现金替代——iso 6/8 ≻ urs 证实翻转 churn 是 URS 的赤字源（v3 翻转死因
  的又一确认）。
- **若 R_capture 否证成立**：信号丢失诊断（12 机制）的"修复"不增 alpha ⇒ 暴露
  守恒（择时 ≈ 伪装 buy-hold）+ 信号捕获与 alpha 正交的第 N 次坐实。栈纪律不是
  bug，是特征（参 `project_shared_position_fugue` L2 否证：放松栈纪律普遍劣化）。
- **v4 仍是默认基座**：iso 不 dethrone v4（5/8 劣于）。iso 价值 = URS 谱系的清仓
  策略修正 + 森林机制存在性 L3 验证（depth 5 实测激活，会计守恒鲁棒）。

## 5. 谱系引用

- §11 审计（`nested_fugue_accounting.md` L194-246）：Phase 3 逐仓嵌套 D1+D6
  CONFORMS ⇒ 地基稳固——iso 是 Phase 3 的森林实装。
- `project_shared_position_fugue`（L2 否证：共享仓位多槽劣于 slice）——iso 是
  **isolated**（逐仓独立 margin）非 shared，与该否证正交；但"放松栈/链纪律"的
  劣化趋势在 iso vs v4 重现（5/8 劣）。
- `project_nrf_v4_strict_accounting`（v4 = P1 3/8 历史最佳基座）——iso P1 1/8 < v4。
- `project_constitutive_throughput_falsified` / `project_nrf_v3_root_flip_falsified`
  （翻转/构成性贯通否证）——iso 的 E* 清仓回现金（非翻转）6/8 ≻ urs 证实翻转赤字。
- 清仓频率/alpha = regime 函数不可约（多个在册判决）——iso P1 1/8 = 第 N 次复现。

## 6. 影响声明

- **新增**：`rust/src/trading/isolated_fugue.rs`（森林引擎 + 12 单测全过）；
  `analysis/isolated_fugue_backtest.py`（L3 回测脚本）；本报告。
- **改动**：`rust/src/trading/positional.rs`（`PolarityMode::Isolated` 变体 +
  `parse("iso")` + 分派 + 两处 match arm）；`rust/src/trading/mod.rs`（注册模块）。
- **零回归**：v4/URS/全部在册模式 bit-exact 测试 335/335 通过；未改任何会计原语
  （nested_fugue.rs/unified_recursive.rs 零改动）；未改 BSP/缠论引擎。
- **未改动**：信号层（`buysellpoint.rs`/缠论引擎）——任务约束遵守。

## 7. 开放轴

- **清仓判据 × MDD**：iso 忠于 §6（罕见清仓）⇒ MDD 深。E* 清仓频率与 MDD 的
  regime 依赖未消融（v4 θ 棘轮 vs iso E* 涌现的 MDD 对照可预注册）。
- **森林宽度激活条件**：哪些标的结构使森林宽度 > 1（ES/GC/BTC R_capture=true）
  vs 退化为链（OKLO/CL）——未做宽度直方图（仅有活跃 voice 数直方图，混淆深/宽）。
- **iso × v4 白名单**：iso ≻ v4 域 {BRN, CL, BTC} = 深崩/极端波动——是否可作
  regime 门控的清仓策略切换（E* vs θ 棘轮）待预注册。
