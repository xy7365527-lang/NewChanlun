# issue #972 tick 对照臂 prototype — 端到端跑通报告

- 日期：2026-08-15
- 工作区：/tmp/wt-tick-proto（detached HEAD @ main 76d4956f3b，throwaway）
- 范围：只做父会话指定四步，判定链（theta_v0）代码**零改**，只新增脚本 + 只读 bin probe。
- 结论一句话：**能。逐笔成交（O=H=L=C=成交价）直接喂现役 theta_v0 判定链，跑出非平凡结构**
  （分型 1287 / 笔 92–410 / 线段 4 / 中枢 2，端到端无 panic）。

## 步骤 1 — 下载 BTC tick 样本

- Binance 公开 API（无需 key）：`GET /api/v3/aggTrades?symbol=BTCUSDT&limit=1000`，
  `fromId` 分页向后翻 4 页，共 5 页 × 1000 = **5000 笔**。
- 落盘 `/tmp/wt-tick-proto/btc_ticks.json`（原始 aggTrades 数组，字段 a/p/q/f/l/T/m/M）。
- 时间跨度 2,158.8 s（≈36 min），价格区间 62999.90–63051.78，3266 个互异毫秒时间戳。

## 步骤 2 — tick → Bar 最小转换（不碰 Rust）

- 新增 `/tmp/wt-tick-proto/scripts/tick_to_bar.py`：每笔 aggTrade → 一根 Bar
  `O=H=L=C=float(p)`，`volume=float(q)`，`date=str(T)`（ms 时间戳字符串）。
- **不做聚合、不做清洗**。唯一额外动作 = 按 `(T, aggTradeId)` 升序排序——
  Binance aggTrades 分页拼接后整体是降序，喂时间序判定链前必须还原；排序是时间序还原，
  不是清洗。
- 输出 parallel-array schema（与 `analysis/data_cache` 同款）：
  `{symbol, opens, highs, lows, closes, volumes, dates}`，落盘 `/tmp/wt-tick-proto/btc_ticks_bars.json`。
- 自检：5000 bars，时间单调不减，span 2158.8 s。

## 步骤 3 — 喂现役判定链（核心）

- 新增 `/tmp/wt-tick-proto/rust/src/bin/tick_proto_probe.rs`（只读入口，theta_v0 零改）。
- 读 `btc_ticks_bars.json` → 按 `tick_size=1e-8` 量化 → `Vec<Bar>` → `parser::parse_layer`。
- 对 `new_stroke_min_gap ∈ {1,2,3}` 各跑一遍（config.rs:44 字段 / :63 默认 3——父会话指出
  tick 下 3 根=3 笔语义过短，须试值看笔数变化）。
- 中枢：把 `layer.segments` 复刻 `classifier::segment_to_unit`（pub(crate) 语义，bin 侧不可见，
  逐字段同款投影）成 `UnitRange`，滑窗取连续三段喂 `classifier::center::center_from_segments`
  （center.rs:210，完整判据 = 方向交替 ∧ 全三段核心严格非空 ZD<ZG，compute_zg/zd :125/:133）。
- 入口逐字复用现役 API：`parse_layer(&bars, &config)` → `fractals/strokes/segments`（各为
  `Rc<Vec<_>>`），判定链内部代码一行未动。

## 步骤 4 — 跑 probe 记录输出

命令：`cargo run --quiet --bin tick_proto_probe`（rust/ 目录，构建 ~19s 冷启动，跑 0.7s）。

| new_stroke_min_gap | merged_bars | fractals | strokes | segments | centers(3段重叠) |
|---|---|---|---|---|---|
| 1 | 3017 | 1287 | 410 | 4 | 2 |
| 2 | 3017 | 1287 | 162 | 4 | 2 |
| 3 | 3017 | 1287 | 92 | 4 | 2 |

线段明细（gap=1，交替 Up/Down/Up/Down）：

```
dir=Up   si=8    ei=416   start=63032.00  end=63037.92
dir=Down si=416  ei=1481  start=63037.92  end=62999.90
dir=Up   si=1501 ei=2074  start=63015.70  end=63043.01
dir=Down si=2136 ei=2627  start=63048.50  end=63033.17
```

中枢明细（gap=1，滑窗 3 段产生 2 个，均通过完整判据）：

```
center [si=8,   ei=2074] zd=63032.00 zg=63037.92 gg=63043.01 dd=62999.90
center [si=416, ei=2627] zd=63033.17 zg=63037.92 gg=63048.50 dd=62999.90
```

## 结论：tick 喂现役判定链能不能跑出非平凡结构？

**能。** 四层结构数量全非零、端到端无 panic：

- 分型 1287（≈43% 的 merged bar 是局部极值）——tick 粒度下几乎每个价格抖动都是分型，极密；
- 笔 92–410（随 `new_stroke_min_gap` 1→3 单调下降）——该 config 重标定只影响笔层；
- 线段 4（三个 gap 值都不变）——4 段交替 Up/Down/Up/Down；
- 中枢 2（4 段滑窗 3 段，两个窗口都通过「方向交替 ∧ 全三段核心严格非空」）。

判定链粒度无关（#969）在本样本上得到了一次端到端实证：逐笔单点 K 线（high==low）能正常穿过
包含合并 → 分型 → 笔 → 线段 → 中枢，全程无 panic、无数值溢出（tick_size=1e-8，价格量化后
~6.3e12，远在 i64 范围）。

## 退化 / 阻塞点（照实）

1. **分型层极密**：1287 分型 / 3017 merged bar ≈ 43%。tick 粒度下分型是「价格序列的每个局部
   极值」，信息量大但噪声密度同样大。结构非平凡 ≠ 信号可用，下游信号/背驰须自行消化该密度。
2. **线段层塌缩（本次主要退化点）**：笔 410（gap=1）→ 线段仅 4；笔 92（gap=3）→ 线段仍 4。
   67 课特征序列的线段结束条件在 tick 粒度下极少触发，线段层把大量笔折叠成极少数长线段
   （~36 min 只有 4 段，约 9 min/段）。「非平凡」主要集中在分型/笔两层，线段层很薄。
3. **中枢只有 2 个且高度重叠**：4 段只有 2 个滑窗（seg0-1-2、seg1-2-3），两者区间大段重叠
   （si 8–2074 vs 416–2627）——是同一 4 段跨度的两个窗口视图，不是两个独立依次递进的中枢。
   核心宽度 ~$6（63032.00–63037.92）≈ 价格的 0.01%，严格非空但极窄。
4. **gap 只影响笔层**：本样本里 `new_stroke_min_gap` 1→3 使笔 410→92（~4.5×），但线段/中枢
   数量不变。这是样本经验观察，非一般性结论；正式标定仍需 #969/#971 的口径票。
5. **volume/untradable 未进结构链**：qty 原值在 JSON 里，probe 中 `Bar.volume=0`、
   `untradable=false`。grep 证实 parser/center 的结构接受判据只消费 OHLC（high/low），
   不消费 volume/untradable，故本 prototype 结构数不受影响；接执行层时小数 volume（qty≈1e-4
   BTC）的整数域量化须另行处理，非本 prototype 范围。
6. **数据源 caveat**：aggTrades 是「同价 100ms 聚合成交」，非原始逐笔 trade；5000 笔仅 ~36 min，
   样本短。ticket 验收里「与同区间 1min 链结构对照」未在本 prototype 做（父会话只排四步），
   留后续。

## 未含（照 ticket，留后续票，本 prototype 不做）

力度/背驰接线（排 #872）、坏 tick 清洗（#922）、1min 链同区间对照、trade vs aggTrade 取舍、
tick 链转生产（#960）。

## 新增文件清单（判定链零改）

- `scripts/tick_to_bar.py`（新）
- `rust/src/bin/tick_proto_probe.rs`（新，只读入口）
- `btc_ticks.json` / `btc_ticks_bars.json`（样本数据，untracked）
- 本报告

`git status --short` 仅 4 个 untracked 文件，无任何 tracked 文件被改。
