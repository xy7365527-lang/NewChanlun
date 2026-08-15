# issue #969 tick 单位K线改动面勘察（只读研究）

> 目标：theta_v0 判定链从 Bar(1min) 换 tick 单位K线（逐笔成交，O=H=L=C=成交价）的改动面。
> 范围：parser（分型/包含/笔/线段）+ classifier（中枢构造 + 判定路径）。
> 产出：三问逐条答 + 改动面清单（file:line）。
> 日期：2026-08-14。作者：只读研究子代理（本体直办，未派发）。

---

## 问题 1：分型/包含/笔/线段是否粒度无关？

**结论：四层全部粒度无关，逐笔单位K线（O=H=L=C）下可照搬，算法零改。**

证据（每个文件核心判定只碰 Bar 的 open/high/low/close/volume/untradable + source_index，
`timestamp` 仅作 tie_key 排序（types.rs:62-64），无任何时长/秒/1min 算术）：

1. **包含 inclusion.rs**：`process_inclusion`（inclusion.rs:103-156）只比较 `high/low`
   （`contains` :38-40，`merge` :49-66，`strict_dir` :73-81）。无时间假设。
   - tick 语义：O=H=L=C ⟹ `contains(a,b)` ⟺ `a.price == b.price`（互含仅发生在同价），
     即**连续同价 tick 被合并成一根**；不同价 tick 永不包含。行为自然、无退化。
2. **分型 fractal.rs**：`detect_fractals`（fractal.rs:35-71）只做严格 `high/low` 四向比较。
   - tick 语义：high==low==price ⟹ 四条件退化为「price 严格高于/低于左右」二重冗余，
     分型 = **逐笔（合并后）局部严格极值**，照搬成立。「第三根 K 收盘确认」在
     close==price 下自动成立，无需改。
3. **笔 stroke.rs**：`build_strokes`（stroke.rs:76-107）在分型序列上交替配对，间隔用
   `source_index` 差（`gap_ok` :62-65，`config.new_stroke_min_gap` 默认 3，config.rs:44/63）
   ——**原始 K 计数**，非时间。无时长假设。
   - 注意：`new_stroke_min_gap=3` 是「中间 ≥3 根原始K」；tick 下这 3 根是 3 笔成交，
     笔会极短。这是**配置值重标定**问题（微改 config），不是代码改。
4. **线段 segment.rs + feature_seq.rs + second_kind.rs**：67 课特征序列法全部在笔的
   价格区间（`Interval`/`stroke_interval` :104-112，`three_stroke_overlap` :130，
   `feature_elements` :161，`divide_segments` :459）。`TAIL_WINDOW=7`（segment.rs:79）是
   **笔数窗口**不是时长。`second_seq_scan_window`（config.rs:57/65）是笔数窗口。无时长假设。

**照搬结论**：分型=逐笔局部极值、笔=交替极值、线段=67课特征序列——三者照搬，判定代码零改。
唯一需要重标定的是 `new_stroke_min_gap`（tick 下 3 根=3 笔，语义过短）。

---

## 问题 2：中枢是否已按「3 线段重叠」建？tick 链是否照旧复用零改？

**结论：已按「3 线段重叠」建（口径 B，637号），且中枢构造完全在整数 tick 价域，tick 链照旧复用、零改。**

证据：

- 057:26 逐字（`docs/chanlun/text/blog/057-第57课.md:26`）：
  「……每一线段就是其次级别走势类型，三个线段重合部分就构成最小分析级别的中枢。」✓
- `center_from_segments`（classifier/center.rs:210-231）：输入三个 `UnitRange`（来自线段，
  含方向），两支合取：
  1. **方向交替** `dir_alternates`（center.rs:153-155，判据支 :212）；
  2. **全三段核心严格非空** `compute_zd < compute_zg`（:217-221）。
  其中 `compute_zg = min(三段 hi)`（center.rs:125-127，:126 `a.hi.min(b.hi).min(c.hi)`）、
  `compute_zd = max(三段 lo)`（center.rs:133-135，:134 `a.lo.max(b.lo).max(c.lo)`）——
  **这就是「3 线段重叠」**（三段价格区间公共交集）。
- 现役中枢建在**什么构件上**：建在 **parser 的 `segments`（线段）**上。classifier 把
  线段经 `segment_to_unit`（classifier/mod.rs:261-274）转成 `UnitRange`，L0 层用
  `detect_centers_complete`（classifier/mod.rs:448-450）→ `center_from_segments`；
  上级层用 `detect_centers_geometric` → `center_from_window`（几何路径，无方向交替，
  classifier/mod.rs:467-469；center.rs:252-268）。分派在 classifier/mod.rs:600-603
  （`is_l0 ? center_from_segments : center_from_window`）。
- tick 复用：`center_from_segments`/`compute_zg`/`compute_zd` 只碰 `Tick`（i64 价格）与
  `Direction`，零时间/时长输入 ⟹ tick 链**照旧复用、零改**。方向交替在 tick 线段上照常
  成立（线段方向由笔交替方向而来，粒度无关）。

---

## 问题 3：判定链里还有没有依赖 bar 时长/秒/1min 的地方？

**结论：parser + classifier 判定路径里没有。全部命中项要么是测试/性能探针，要么是「bar 计数」而非「bar 时长」。**

逐类核对（grep 60/seconds/time/bar_seconds 全命中项）：

1. **数据加载层（判定链之外）**：`backtest/data.rs` 的 `BAR_GRANULARITY` 表（:55-62，全
   1min=60s 文件）、`bar_seconds`/`bars_per_year`（:83-93，CAGR 年化基数）。这是数据摄入 +
   指标层，不进 parser/classifier 判定；tick 迁移时这里是**数据接入面**（新增 tick 数据源 +
   粒度参数），不是判定代码改。
2. **测试/性能探针（cfg(test)/profile，不参与判定）**：parser/profile.rs:3/37（ES 1min 基准）、
   parser/mod.rs:347（测试加载 OKLO 60s）、classifier 各 `elapsed()`（mod.rs:6263/6276 等，
   signal.rs:5089 等——纯计时）。全部测试/性能代码，非判定。
3. **名字带 time 但实为 bar index**：`canonical.rs` 的 `confirm_time`（:40，实际用
   `end_idx as Timestamp`，:107）、`nest.rs` 的 `judge_at`/「clock」（`usize` bar 序号，
   nest.rs:436/915）、`NestInterval` 的 `start_time/end_time`（cand_predicate.rs:874-928，
   测试数据里的区间**序号**）。均非时长算术。
4. **「bar 计数」而非「bar 时长」的阈值（粒度无关，但值需重标定）**：
   - `new_stroke_min_gap=3`（stroke.rs:62-65，config.rs:44/63）——原始 K 根数；
   - `UPGRADE_TOTAL_SEGMENTS=9`（recursive_tower.rs:64，第33课段数阈值）——段数；
   - MACD(12,26,9)（divergence.rs 头部「MACD(12,26,9)」，config MacdConfig）——**bar 计数
     EMA 周期**，非时长；tick 下照样运行但语义变（L2/L3 标定问题，非代码阻塞）。
   - `second_seq_scan_window=0`（config.rs:57/65）——笔数窗口。
5. **文档标签「L0=1分钟线段账本」**（纯 doc，无逻辑）：parser/mod.rs:130、classifier/mod.rs:31/238/529。
   tick 迁移后这些注释需要改成「L0=tick 线段账本」——**doc 级微改**，零逻辑影响。

**判别一句话**：判定链里的所有「时间」只有两种形态——(a) `Timestamp`（i64，只做 tie_key 排序，
types.rs:20/62-64，无算术）；(b) `source_index`/bar 序号（计数）。没有任何一处用
`bar 时长 × 序号`、`序号 / 60`、`seconds` 参与判定。

---

## 改动面清单

### 零改（判定逻辑不动，tick 链照搬复用）

| 文件 | 范围 | 理由 |
|---|---|---|
| parser/inclusion.rs | 全文件 | 只碰 high/low，tick 下同价合并自然成立 |
| parser/fractal.rs | 全文件 | 只碰 high/low，tick 下=局部严格极值 |
| parser/stroke.rs | 全文件 | 只碰分型价格 + source_index 计数 |
| parser/segment.rs | 全文件 | 只碰笔价格区间 + 笔数窗口 |
| parser/feature_seq.rs | 全文件 | 笔数窗口，无时长 |
| parser/second_kind.rs | 全文件 | 复用 feature_seq，无时长 |
| parser/canonical.rs | 全文件 | confirm_time=end_index 代理，无时长 |
| parser/tail.rs | 全文件 | 只碰价格/序号 |
| classifier/center.rs | 全文件 | 只碰 Tick 价格 + Direction，已是 3 线段重叠 |
| classifier/recursive_tower.rs | 全文件 | 段数阈值（9），无时长 |
| classifier/divergence.rs | 判定代码 | MACD 用 close 序列（bar 计数周期），无时长 |
| types.rs | Bar/Fractal/Stroke/Segment/Center | 逐笔 O=H=L=C 可直填（types.rs:49-58） |

### 微改（配置/文档/接入面，非判定逻辑）

| 文件 | 位置 | 改什么 |
|---|---|---|
| config.rs | :44 / :63 | `new_stroke_min_gap` 默认 3 —— tick 下须重标定（3 根=3 笔过短）。只改默认值或接 tick 专用配置，不动 stroke.rs 逻辑 |
| config.rs | :57 / :65 | `second_seq_scan_window`（0=无限）——tick 下笔数剧增，可能须设有限窗；值级微改 |
| parser/mod.rs | :130 | doc「L0=1分钟线段账本」→「L0=tick 线段账本」 |
| classifier/mod.rs | :31 / :238 / :529 | 同上三处 doc 标签 |
| backtest/data.rs | :55-62 / :83-93 | tick 数据接入：新增 tick 数据源 + 粒度参数（bar_seconds=1s 或逐笔文件）；判定链不读这里 |
| nautilus/bar_adapter.rs | :46-67 | tick 接入点：新增/扩一个 `TradeTick → Bar(O=H=L=C)` 适配器（或复用 from_nautilus_bar 的 OHLC 填入路径） |

### 查不到 / 不编

- 未发现任何「bar 时长/秒/1min」参与 parser+classifier 判定的生产代码（grep 全命中项均已归类于上）。
- 未评估 backtest 数据文件是否已有 tick 数据源（#969 只问判定链改动面，数据源筹备不在本范围）。

---

## 附：spot-check 父会话已给行号

- Bar 结构 types.rs:49-58 ✓（`pub struct Bar` 在 :49，字段 50-58）。
- parser 管线 mod.rs:11-13（fractalsOf/strokesOf/segmentsOf 契约）✓、mod.rs:29-36（子任务清单 2/3/4）✓。
- center_from_segments 的 ZG/ZD：父会话记 center.rs:120/129 是**文档注释起行**；函数体精确行：
  `compute_zg` 体 center.rs:126（`a.hi.min(b.hi).min(c.hi)`）、`compute_zd` 体 center.rs:134
  （`a.lo.max(b.lo).max(c.lo)`）；`center_from_segments` 在 center.rs:210。
- 057:26 逐字 ✓（三个线段重合部分就构成最小分析级别的中枢）。
