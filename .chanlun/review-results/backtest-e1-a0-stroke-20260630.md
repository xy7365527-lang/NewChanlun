# backtest-e1: a0=Segment→Stroke 单变量回测判决（L2）

date: 2026-06-30
task: #68
engine: recursive_t `t_backtest_8x3`（rust/src/recursive_t/backtest_run.rs:358）
data: BTC `btc_1m_full.json` (4.6M bar) + CL `cl_1m_databento_10y.json` (3.67M bar)，1min 粒度
变量: a0 来源单变量（`T_A0=segment` baseline vs `T_A0=stroke`），其余全固定
**L 等级: L2**（真实 1min 数据，双标的，可证伪，携信息增量；非 L3——不可外推到"a0=笔普适劣化"）

## 结论

**E1 否证成立。a0=Stroke 不提升，反而灾难性劣化。**

| 标的 | 模式 | a0=Segment (baseline) | a0=Stroke | trades(seg→str) |
|------|------|----------------------|-----------|-----------------|
| BTC | Structural | **+70.9%** | **−100.0%** | 2679 → 17753 (6.6x) |
| BTC | AND | +66.9% | −100.0% | 2664 → 17624 |
| BTC | OR | +208.1% | −100.0% | 2763 → 18107 |
| CL | Structural | +64.0% | −98.7% | 1934 → 11685 (6.0x) |
| CL | AND | +54.2% | −98.9% | 1934 → 11610 |
| CL | OR | +24.8% | −97.8% | 2026 → 11982 |

BH: BTC +1628.8%, CL +85.1%（两 a0 相同，仅作参照）。
r\*（涌现顶层）：BTC seg=5→str=5，CL seg=5→str=6。

否证判据（filter-spec 锁定）= a0=Stroke 下 BTC P1 final_nav < +664% → **−100.0% 远低于阈值，否证**。
（注：filter-spec 写的 +664% 基线与本引擎实测 baseline +70.9% 不符，见边界条件 §2。无论用哪个基线，−100% 都是否证。）

## 定义依据

- 第65课 aₙ=f(aₙ₋₁)：a0 是递归塔底座。`build_a0_from_strokes`(backtest.rs) 用 confirmed 笔，`build_a0_from_segments` 用 confirmed&&Settled 线段（526号）。
- 引擎数据流：`iterate`(mod.rs:83) → `run_backtest`(backtest.rs:321) → `apply_bsp`(backtest.rs:252)。`apply_bsp` = 纯多头，买点全额建多/卖点分级减仓（最高级别清仓、否则减1/3配额），backtest.rs:281 显式"简单版不做空"。
- 机制：笔比线段细碎 ~6.6x（BTC strokes=328068 vs segs=39782 = 8.2x 原始；进 a0 后 BSP 6611→45475 = 6.9x）。纯多头 apply_bsp 在每个细碎笔级 BSP 全额翻动仓位 → 被微观噪声碾碎，BTC 直接归零。

## 边界条件（结论翻转条件）

1. **若开平仓逻辑改为带级别/方向门控**（如 rec 引擎的 LongEntry::CrossLevel）→ 细碎笔信号会被门控过滤，否证可能翻转。但那是另一条引擎（见谱系 §cascade），不是 E1 指定引擎。本否证仅对 `t_backtest_8x3`(纯多头 apply_bsp) 成立。
2. **filter-spec +664% 基线存疑**：本引擎当前 baseline(segment) BTC Structural = +70.9%，与 filter-spec 报告的 +664% 差一个量级。若 +664% 来自旧版引擎/不同配置，则该阈值对当前引擎无效——但 a0=Stroke 的 −100% 低于任何正基线，否证不依赖阈值具体值。
3. 若改 PerfectionMode（非 Structural）→ AND/OR 同样 −100%/−98%，否证不翻转。

## 下游推论

- **E2/barspec 1s 路线失去 a0=笔的依据**：filter-spec 的"1s+笔组合"假设以"a0=笔提升捕获率"为前提（line 64/107）。本 L2 在 1min 上即否证 a0=笔提升 → 1s 上笔更细碎，只会更劣（filter-spec line 61 自承"1s 产生更多 L0/L1 笔，触发次数线性增加"）。E2/barspec-impl 的 a0=笔分支不再有实证支撑。
- a0=Segment 是当前 `t_backtest_8x3` 引擎的正确底座（confirmed&&Settled 过滤掉细碎噪声 = 隐式信噪门）。

## 谱系引用

- 531号（bar 粒度维度）、547/546号（cascade 级别错配）、556/557号（顶层稀疏 regime）。
- **cascade 范畴错位（本任务自检发现，已上浮 Lead）**：filter-spec line 64/107/101/125 声明"547 cascade 修复是 E1 可解读前提"，指 `rec_engine.rs::cascade_reverse_to_core`。但 (a) 该函数名源码不存在（漂移，真位置=`rec_engine.rs:81-100` LongEntry::CrossLevel）；(b) cascade 门控所在的 rec 引擎只被 rec_driver/rec_stream 调用，**不在 E1 指定引擎 `t_backtest_8x3` 的代码路径上**（`iterate→apply_bsp` 零处触及 rec_engine）；(c) cascade 攻的病理"翻空主力/逆向开多"在纯多头 apply_bsp 里不存在 → **cascade 对 E1 引擎范畴不适用**。因此 E1 否证结果**不依赖 cascade 是否生效**，可解读。filter-spec 该声明需 genealogist 修正适用域（formalization-validity-domain：有效域≠定义域）。

## 影响声明

- 只读回测，**未改任何源码**。`T_A0` 切换是既有 wiring（backtest_run.rs:370-372），仅设环境变量。
- 写 JSON 副作用：`analysis/data_cache/t_backtest_{BTC,CL}_{structural,and,or}.json` 被 stroke 版覆盖（最后一次运行为 stroke）。如需保留 segment baseline JSON，重跑 `T_A0=segment BT_SYMBOLS=BTC,CL`。
- 否证 filter-spec 的"a0=笔提升"假设 → 影响 roadmap 主线 signal_resolution_1s_bi_a0 的 E2/barspec 分支取舍（归 Lead/编排者）。
