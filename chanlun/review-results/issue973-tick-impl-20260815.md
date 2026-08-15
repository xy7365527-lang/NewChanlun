# issue #973 tick 对照臂落地 — 实施报告

- 日期：2026-08-15
- 工作区：/tmp/wt-tick-impl（detached HEAD @ main `93017c06a5`，正式实施，非 throwaway）
- 范围：tick loader（生产级）+ 筛选门槛重标定 + 判定链零改复用
- 结论一句话：**seam 跑通——20000 笔 BTC 逐笔成交 → Bar(O=H=L=C) → 现役判定链，产出非平凡结构
  （分型 4617 / 笔 96 / 线段 13 / 中枢 11，判定链代码零改）；并订正 prototype #972 的「线段塌缩到 4」实为
  短样本假象。**

---

## 0. seam（一行）

```
逐笔成交 → Bar(O=H=L=C=成交价, epoch ms/ns 时间戳) → 现役 theta_v0 判定链（分型/笔/线段/中枢，零改）
```

左半边（数据接入）新增 `rust/src/theta_v0/backtest/tick.rs`；右半边（判定链）`parser::parse_layer` +
`classifier::center::center_from_segments` 照搬，**一行未改**。

## 1. tick loader（生产级，替代 prototype 的 throwaway）

新增 `rust/src/theta_v0/backtest/tick.rs`（L1，读 JSON → fail-loud 校验 → 时间序还原 → 量化为 `Bar`）：

- `load_binance_agg_trades`：REST `/api/v3/aggTrades` 数组（字段 `a/p/q/T`，ms）。schema 逐字核到官方
  文档（#968 §一）。
- `load_binance_trades`：REST `/api/v3/trades` 数组（字段 `id/price/qty/time`，ms，严格逐笔）。
- `load_databento_trades`：JSON 数组（`ts_event` ns / `price` / `size`，**可选**）。只断言三个 DBN 通用字段；
  精确字段清单/文件格式/rtype/action/side 过滤未经人工核（#968 §二），落地前须在 portal 核（见 §6 残余）。
- 对齐现役 `data.rs` 的 fail-loud：文件缺失 / JSON 解析失败 / 空数组 / 核心字段缺失 / 价格或量非有限正数 /
  时间戳非正 → `Err`。**不做** #922 统计级坏 tick 清洗（#922 未修，本票用干净样本绕开，见 §6）。
- 时间序还原：按 `(timestamp, trade_id)` 升序排序（Binance 分页拼接后整体可能降序；排序是时间序还原，
  不是清洗，不删不改任何一笔）。`source_index` 重置为 0..n。
- `Bar`：O=H=L=C=quantize(price)，`timestamp` 直接落 epoch ms/ns（不复用 `data.rs` 的 14 位秒编码，
  #971 ③并列），`volume=qty as i64`（对齐 data.rs 截断口径），`untradable=false`。忠实成交量保存在
  `TickFeed.volumes`（f64，与 bars 同索引同长）。

### 样本（fixture，已入仓）

- 文件：`rust/tests/fixtures/btc_agg_trades_20000.json`（2.75 MB）。
- 下载：`GET /api/v3/aggTrades?symbol=BTCUSDT&limit=1000` + `fromId` 向后分页 20 页，共 **20000 笔**；
  按 `(T, a)` 升序还原时间序。
- 时间跨度 **8298 s（≈2.3 h）**，12155 个互异毫秒时间戳，价格区间 63023.64–63096.29。
- 用 20000 而非 prototype 的 5000（~37 min）：**5000 笔太短，线段层因样本短而塌缩**（见 §2 订正）。

## 2. 筛选门槛重标定（本票核心）

### 2.1 扫参表（20000 笔，`new_stroke_min_gap` 扫参）

`cargo test --lib theta_v0::backtest::tick::tests::calibration_sweep_finds_non_collapsed_segments -- --nocapture`：

| gap | strokes | segments | 笔中位尺寸($) | 笔/段 |
|---|---|---|---|---|
| 3（现默认） | 351 | 25 | **0.0100** | 14.0 |
| 5 | 154 | 20 | **0.0100** | 7.7 |
| **10** | **96** | **13** | **2.6700** | 7.4 |
| 20 | 64 | 7 | 5.9200 | 9.1 |
| 50 | 34 | 3 | 10.3700 | 11.3 |
| 100 | 9 | 0 | 15.8000 | — |

### 2.2 订正 prototype #972 的退化点2（重要）

prototype 报告「5000 笔 → 线段 4（塌缩）」。本票用 **4× 样本（20000 笔）** 复测发现：**线段层并不
塌缩**——gap=3 时 25 段、gap=5 时 20 段、gap=10 时 13 段，只有 gap≥50 才因笔被抽得太稀（34/9 笔）而
掉到 ≤3 段。⟹ **prototype 的「塌缩到 4」是 ~37 min 短样本的假象**（约每 5–10 min 出 1 段），不是
线段判定链在 tick 粒度上的结构性退化。

### 2.3 线段层窗口：无须放大（已扫）

- `second_seq_scan_window`（config.rs:57/65）：默认 0 = 无限全扫。实测 0 vs 50 vs 200 vs 1000 输出
  **逐位相同**（gap=3/10/20 各 4 档全同）——窗口已在无穷档，无从「放大」，有限档只会收紧不会放宽。
- `TAIL_WINDOW`（segment.rs:79，const=7）：临时改 7→0（无限）重跑，输出**逐位相同**——线段塌缩/段数
  与 TAIL_WINDOW 无关，瓶颈不在此。它是 `const` 非 config 字段，本票**未改**（改它=改 segment.rs=违
  「判定链零改 / diff 只加新文件」）。

### 2.4 稳定点判据 + 选定值

三个判据合取：

1. **噪声底逃逸**：gap≤5 时笔中位尺寸 = $0.01（= 1 个价格步，纯逐笔噪声主导）；gap=10 起逃出（$2.67）。
2. **线段层非平凡**：gap≤20 时线段 ≥7（非 4 这种塌缩值）；gap≥50 塌缩（≤3）。
3. **笔/段比稳定带**：gap∈[5,20] 时笔/段 ≈ 7–9（稳定）；gap≥50 退化（11.3→∞）。

⟹ **选定 `new_stroke_min_gap = 10`**（首个同时满足噪声底逃逸 + 线段非平凡的档位）。gap=20（7 段、
$5.92）也在稳定带内，是更保守的备选；10 vs 20 的最终取舍留给信号层 #872 接线后的 L2 复核，本票以
「噪声底逃逸」这一可证伪判据取 10。

> 注意：本票**不把默认值写死改到 config.rs:63**（那是「改 Θ」须 change request；且 tick 与 1min 两条
> 链并存，1min 链默认 3 不动）。重标定结论编码在测试的 `RECALIBRATED_GAP=10`（测试内局部 config），
> 生产接线（#960 换装）时再决定「tick 专用 config」的落点。

## 3. 判定链零改（#969 粒度无关的端到端实证）

`git diff --name-only` 仅 `rust/src/theta_v0/backtest/mod.rs`（+3 行：`pub mod tick;` 注册）。分型/包含/
笔/线段/中枢（parser/*、classifier/center.rs、types.rs、config.rs）**零 diff**。

seam 测试（gap=10，20000 笔）实跑读数：merged 12538 → 分型 4617 → 笔 96 → 线段 13 → 中枢 11
（连续三段滑窗喂 `center_from_segments`，完整判据=方向交替 ∧ ZD<ZG 严格非空）。13 段方向严格交替
Down/Up/Down/…，端到端无 panic、无数值溢出（tick_size=1e-8，价格 ~6.3e12 在 i64 范围）。

## 4. 改动面清单

| 文件 | 状态 | 内容 |
|---|---|---|
| `rust/src/theta_v0/backtest/tick.rs` | **新增** | tick loader + 7 个测试（5 单测 + seam + 扫参） |
| `rust/src/theta_v0/backtest/mod.rs` | +3 行 | `pub mod tick;` 注册 |
| `rust/tests/fixtures/btc_agg_trades_20000.json` | **新增** | 20000 笔 BTC aggTrade 样本（2.75 MB） |
| parser/classifier/types/config | **零改** | 判定链照搬（#969 已证粒度无关） |

## 5. 两轴评审

### Standards（fmt / 无 shotgun / 文档格式）

- `cargo fmt --check` 绿；`cargo check --lib` 绿；`cargo check --features backtest_bin` 绿（生产门控路径）。
- 单文件测试：`cargo test --lib theta_v0::backtest::tick::` → **7 passed**（0.08s）。
- 无 shotgun：判定链零 diff；tick.rs 单文件 <500 行，模块头写明 seam/认识论等级/校验口径/volume 口径。
- 实现耦合：只经公共接口 `parse_layer` + `center_from_segments`（无 mock 内部/无测私有）。
- 同语反复：测试断言非平凡结构（量化数学 / 排序还原 / fail-loud / 20000 笔→分型4617→笔96→段13→中枢11），
  不是「喂 X 得 X」。

### Spec（#973 验收逐条）

| 验收 | 结果 |
|---|---|
| seam = tick→Bar(O=H=L=C)→判定链，现役 parser/classifier 复用零改 | ✅ §0/§3 |
| 实跑 seam 测试：结构非平凡（线段数 ≠ 4 塌缩值） | ✅ 13 段（gap=10），测试断言 `!=4` 且 >0 |
| 筛选门槛重标定有理有据（扫参记录 + 稳定点判据） | ✅ §2（表 + 三判据合取） |
| `cargo fmt --check` + 单文件测试绿；判定链零改（diff 只加新文件） | ✅ §5 |

## 6. 照实残余（未含，按票）

1. **力度/背驰接线**：排 #872，本票不做（`classify` 的信号层未接 tick 链验证）。
2. **坏 tick 清洗 #922**：未修。本票用干净样本绕开（fixture 价格/量全部通过结构校验）；逐笔级坏点
   （如单点异常价）仍会直接污染分型/笔，是 #922 的既有漏洞，非本票引入。
3. **Databento Trades schema**：只按 DBN 公开惯例断言 `ts_event/price/size` 三字段，精确字段清单/
   DBN 二进制格式/rtype/action/side 过滤未经人工核（#968 §二「查不到」），落地前须 portal 人工核。
4. **volume 整数域截断**：`Bar.volume = qty as i64` 对小数基准资产量（BTC qty≈1e-4）截断为 0，忠实量
   在 `TickFeed.volumes`（f64）。结构链不消费 volume；接执行层（#872/#960）时须决定 volume 整数域编码。
5. **`new_stroke_min_gap=10` 未写回 config.rs:63 默认**：tick 与 1min 两链并存，1min 默认 3 不动；tick
   专用 config 落点留 #960 换装时定（见 §2.4）。
6. **trade vs aggTrade 取舍**：两 schema 都实现，但只有 aggTrade 有端到端 fixture 实测；trade 流只做了
   单笔单测。
7. **样本单一**：仅 BTCUSDT 一段 2.3h（安静期，价格区间 ~$72）。稳定点判据在单样本上成立，跨品种/高波动
   期的稳健性未验。

## 附：关键行号（spot-check 通过）

- `Bar` 结构 types.rs:49-58 ✓；`quantize` types.rs:24 ✓（tick_size 默认 1e-8）。
- parser 管线 parser/mod.rs:11-13/29-36 ✓；`parse_layer` parser/mod.rs:130 起 ✓。
- `center_from_segments` center.rs:210-231 ✓；`compute_zg` :126 / `compute_zd` :134 ✓（3 线段重叠，ZD<ZG 严格）。
- `new_stroke_min_gap` config.rs:44/63 ✓；`second_seq_scan_window` config.rs:57/65 ✓；`TAIL_WINDOW` segment.rs:79 ✓。
