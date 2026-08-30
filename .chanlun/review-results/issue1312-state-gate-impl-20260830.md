# #1312 状态门控第二预注册：门状态导出（tape v3）+ 空腿/多腿状态闸 + 重跑件（2026-08-30）

> **状态**：实现已落地并验证（Rust 门列导出器 + Python 状态闸 + 第二轮 driver）；
> **7 标的重跑仍卡在同一外部数据依赖**（沙盒无 `DATABENTO_API_KEY`，7 个 1min 数据文件不在仓内）——
> 与 #1309 第一轮实现期同一卡点，复现证据与解除条件见 §5。
> **本票「正确」口径**：忠实执行 #1282 预注册，不是「实验必须过」——第二轮过与不过都原样报。
> **上游**：#1279（M1 实验线）；判据 = #1282 冻结文本（跑前不改一字）；第一轮无门基线 = `issue1309-dual-open-backtest.md`（1/7 不过）。

## 0. 结论摘要（TL;DR）

1. **tape v3 门状态列已落地**（要做 ①）：`rust/src/trading/gate_state_dump.rs`——从 v2r 磁带
   逐 bar 驱动**生产函数本体**（`trend_exhaustion.rs` 的 `up_unexhausted`/`down_unexhausted`，
   `fatigue_gate.rs` 的点态状态），落 `_tape_v3_gates_{SYM}.bin` 三列。判据零实现，推进序逐字
   复刻 `runner.rs` 主循环。
2. **Python 驱动 v2 状态闸已落地**（要做 ②）：`analysis/gate_state_columns.py`（读列 + 进场准入）
   + `fugue_alpha_diagnosis._run_swing_leg(..., gate)`——空腿进场拒于 `l2_up_unexhausted=true`、
   多腿镜像拒于 `l2_down_unexhausted=true`；`gate=None` 与第一轮逐位等价。
3. **第二轮 driver 已就绪**（要做 ③）：`analysis/m1_e_futures_dual_gated_backtest.py`——判据/窗口/
   基线一字不动，产出第二轮结果 + **两轮对照表**（逐品种 Δ 百分位）+ 门 sanity 表 + 双跑自检行。
4. **门 sanity 已核验**（要做 ④）：Rust 侧 7 把测试锁（含"趋势延续段 unexhausted 恒 true"、
   "列 = 生产函数读数"）+ Python 侧 12 把（含"unexhausted 区间内零逆势单"）+ 跨语言接缝
   端到端实跑（§4.3）。真实数据上的抽验已内建为 driver 的硬断言 + 报告 §3，数据就位即出。
5. **fatigue 臂能力缺失（如实报，不代理）**：`FatigueGate` 清空路径(1) 依赖磁带 `run_high` 行，
   当前信号层（`organic_signals.py`）不产出该行 ⇒ fatigue 列整列写哨兵、开臂即 fail-fast。
   详见 §3。
6. **跑批卡点**（外部依赖，与 #1309 同源）：无 Databento key、7 个数据文件不在仓内 ⇒ 第二轮
   7 标的**无法在沙盒执行**。解除条件 + 续跑命令见 §5。

## 1. 门状态导出（要做 ①）

### 1.1 落点与推进序

`rust/src/trading/gate_state_dump.rs`（`#[cfg(test)]`，沿 `trade_behavior.rs` 的 dump 先例）：

```text
D3 方向行滚动推进 → TrendExhaustion::observe → CenterBook::ingest
  → CenterBook::negate_pending_departure → FatigueGate::observe → 读三列
```

逐字复刻 `runner.rs` 主循环同序；读数时点在同 bar 全部推进之后——与 runner 把 `l41`/`book`
只读借用交给 FLAT/ARMED/LONG 分派的时点一致。`hard_type3` 取自在册变体 `V2r`（本磁带的在册
消费配置，与 `trade_behavior.rs` 同源），不另立配置口径。

**零新判据**：本模块只做「驱动 + 读数 + 列式落盘」，三列全部是生产函数返回值。Python 侧一行
判据都不写（防④列判据分叉 = 本票设计要点）。

### 1.2 级别口径

门列取 **ladder 4**。映射正本 `analysis/fugue_version_i.py:145`【正文】：
`0=bar 1=bi 2=segment 3=走势(L1)；递归 L → ladder L+2（L2→4 … L8→10）`。
E 择时的 `l2_flip_long/short` 由 L1 走势 settle 端点喂 L2 PH 产出
（`analysis/m1_e_rust_engine.py`），即 L1 之上一级 = recL2 = ladder 4——门列与择时列同级别，
无级别错配。

### 1.3 列文件格式（v3）

```text
header: magic u64 = 0x4E43545056334700 ("NCTPV3G\0")
        n_bars u64, ladder u64, fatigue_available u64（0/1）
        first_close f64, last_close f64   ← 与驱动侧 closes 的接缝校验
cols:   l2_up_unexhausted n*u8 / l2_down_unexhausted n*u8 / fatigue_state n*u8
        （fatigue: 0=Fresh 1=Fatigued 255=能力缺失）
```

接缝校验是必需项：门列来自 `_dump_tape_rust.py` 的清洗后 bar 序列，driver 的 closes 来自
`load_ohlc` 同款清洗——不对齐即错位读数，故 bar 数/首尾 close 任一不符即 fail-fast。

## 2. 状态闸（要做 ②）

| 腿 | 进场判据（#1282 冻结，零改动） | #1312 新增状态闸 |
|----|------------------------------|-----------------|
| 多腿 | `down_move_settled ∧ entry_div_ok` | 拒于 `l2_down_unexhausted = true` |
| 空腿 | `up_move_settled ∧ exit_div_ok` | 拒于 `l2_up_unexhausted = true`（fatigue 臂开时另需衰竭段内） |

- 闸**只在进场处**加准入，出场/回补判据零改动——41课门是"不在未衰竭趋势里开逆势仓"的开腿
  约束，不是离场约束（与 `level_sub_lou.rs:367` / `level_operating_unit.rs:1260` 的生产消费点同位）。
- `gate=None` ⇒ 第一轮（#1309）逐位等价，测试锁在 `tests/test_state_gate_columns.py`。

## 3. fatigue 可选臂：能力缺失如实报（不代理）

`fatigue_gate.rs` 头部【能力边界】：清空路径(1)「创新高 ∧ 不背驰」依赖磁带 `run_high` 行与
div 力度字段；**当前磁带（`organic_signals.py`）无 `run_high` 行** ⇒ 生产 runner 对
`rev_gate=true` 在入口 capability guard fail-fast，且该模块**明令不提供**"用 close 代理
run_high"的降级路径（代理 = 双真相源）。

全仓核查确认无任何生产者写 `SignalTape::run_high`：

```text
$ grep -rn "run_high: Some\|run_high = Some" rust/src --include="*.rs"
（零命中；tape.rs:14 行注亦记「当前信号层不产出 → None」）
```

本票遵同一界：
- Rust 侧：`run_high` 缺失 ⇒ fatigue 列整列写 `255` 哨兵，**不驱动** FatigueGate、不代理；
- Python 侧：`StateGate(use_fatigue=True)` 在列不可用时构造即 `RuntimeError`（fail-fast）。

⇒ **fatigue 臂在当前磁带上不可跑**，这是能力边界而非本票遗漏。解除路径 = 信号层产出
`run_high` 行（属信号层扩面，非本票范围；若编排层要该臂，需另票给磁带加 D3 run 高点行）。
臂的代码路径已就绪并有测试锁（列可用时空腿进场额外要求 Fatigued），`--fatigue` 开关已接。

## 4. 验证面（沙盒可做部分，全部实跑）

### 4.1 Rust

- `cargo test --lib`：**2911 passed / 0 failed / 161 ignored**（全量不降）。
- 新增 7 把锁（`gate_state_dump::tests`）全绿：
  - `up_trend_continuation_window_stays_unexhausted` —— 趋势延续段内 `up_unexhausted` 恒 true
    （门在拦），盘整背驰落地即开，新段起点重置回关；
  - `down_trend_continuation_window_stays_unexhausted` —— 方向镜像；
  - `columns_equal_direct_production_readings` —— 列 = 手工驱动生产函数的逐 bar 读数（零重实现锁）；
  - `center_book_participates_in_gate_reading` —— confirmed Sell3 坐实后门开（中枢账本真被喂到）；
  - `fatigue_column_is_unavailable_sentinel_without_run_high` —— 能力缺失哨兵；
  - `fatigue_column_tracks_production_state_with_run_high` —— 能力在时跟随生产点态；
  - `missing_dir_rows_is_fail_fast` —— dir 行缺失即报错（同 runner capability guard）。
- `cargo fmt --check` 通过；`cargo clippy --lib --all-targets` 对 `gate_state_dump.rs` **零命中**
  （该跑另有 3 项 clippy error，全部落在 `theta_v0/{parser/segment.rs,strategy/campaign_book.rs,
  backtest/wverify_run.rs}` 的既有测试代码，与本票改动面无关，未触碰）。

### 4.2 Python

- `pytest tests/test_state_gate_columns.py`：**12 passed**（列读写往返、三项接缝校验 fail-fast、
  空/多腿拦截语义、"整段延续 ⇒ 逆势腿零单"、无门路径与 `run_swing_trading(MODE_NONE)` 逐位等价、
  双跑逐位自检、fatigue 臂 fail-fast 与开臂语义）。
- driver 缺输入路径实跑：打印 7 个缺数据文件 + 7 个缺门列 + 解除条件，退出码 **2**。

### 4.3 跨语言接缝端到端实跑

合成 8-bar v2r 磁带（ladder4 方向行 Up→Down→Up，bar3 一个 Consolidation×Up 背驰）→
真实 Rust 导出器 `cargo test gate_state_dump_one -- --ignored`（`GATE_DUMP_SYM=SYNTH`）→
真实 Python 读列：

```text
[SYNTH] 8 bars | L2(ladder=4) up_unexhausted=5 (62.50%) down_unexhausted=8 (100.00%)
        | fatigue=能力缺失（磁带无 run_high 行）
up   [1, 1, 1, 0, 0, 0, 1, 1]     ← 延续段恒 1；背驰 bar3 起转 0；bar6 新段重置回 1
down [1, 1, 1, 1, 1, 1, 1, 1]
fat  [255 ×8]
seam check：closes 少一根 ⇒ ValueError（拒绝错位 join）
```

**未做**：真实 7 标的跑批（缺数据，见 §5）——门开率、Δ 百分位、真实数据门 sanity 抽验均待数据。

## 5. 卡点报告（外部依赖，复现证据 + 解除条件 + 续跑命令）

### 5.1 复现证据

```text
$ env | grep -ciE "databento"          → 0（无 DATABENTO_API_KEY / DATABENTO_KEY）
$ ls analysis/data_cache/              → 仅 venue_fee_* 5 个文件，无 *_1m_databento_10y.json
$ uv run python -c "import newchan_rust"  → ModuleNotFoundError（Rust 扩展未构建）
$ PYTHONPATH=src:analysis uv run python analysis/m1_e_futures_dual_gated_backtest.py
卡点：第二轮跑批的输入不齐（外部依赖）。
  缺期货 1min 数据文件（…7 个…）
  缺 v3 门状态列（…7 个…）
exit=2
```

### 5.2 解除条件

1. Databento 订阅 key 注入沙盒（`DATABENTO_API_KEY` 或 `DATABENTO_KEY`）；或编排层把 7 个数据
   文件（`es/gc/cl/zn/usd6e/brn/dx_1m_databento_10y.json`）放入 `analysis/data_cache/`（gitignored）；
2. 构建 Rust 扩展（`newchan_rust`，信号层与磁带 dump 均依赖）。

### 5.3 续跑命令（数据就位后）

```bash
cd /home/agent/workspace
# 1) Rust 扩展
cd rust && uv run maturin develop --release && cd ..
# 2) 数据就位（有 key 时拉取；或直接放文件）
DATABENTO_API_KEY=... PYTHONPATH=src uv run python scripts/fetch_1m_databento_10y.py all
# 3) v2r 磁带（信号事件流）
PYTHONPATH=src uv run python analysis/_dump_tape_rust.py ES GC CL ZN 6E BRN DX
# 4) v3 门状态列（Rust 生产函数现算）
cd rust && cargo test --release gate_state_dump_prereg7 -- --ignored --nocapture && cd ..
# 5) 第二轮跑批（门控双开；--fatigue 开可选臂，当前磁带下会 fail-fast，见 §3）
PYTHONPATH=src:analysis uv run python analysis/m1_e_futures_dual_gated_backtest.py
# 6) 产物
#    .chanlun/review-results/issue1312-state-gated-backtest.json
#    .chanlun/review-results/issue1312-state-gated-backtest.md（含两轮对照表 + 门 sanity 表）
```

## 6. 预注册冻结文本逐条对照（#1282 票面，第二轮）

| 冻结项 | 票面 | 本票执行 | 核对 |
|--------|------|----------|------|
| 假设 H | 操作层对称扩展至下跌侧（次级别顶背驰进空、L2 底背驰回补，联立门方向镜像） | 判据零改动，仅进场加 41课门状态闸（在册生产判据 #1267/#1264，非新判据） | ✅ |
| 过判据（唯一） | 7/7 标的百分位全部越过各自随机中位（符号检验单侧 p=0.0078） | driver 按 `分位>50` 计数 + 7/7 判定 + p=0.0078125，与第一轮同口径 | ✅ |
| 前后对照（Q3.1） | 同标的同窗配对，逐标的记变化量 | 两轮对照表：第一轮归档 vs 本跑无门重算 vs 第二轮门控，逐标 Δ | ✅ |
| 随机基线（Q3.2） | prereg-windows-v0 百分位法（含 #1281 补 ZN/6E 窗） | `large_bootstrap_percentile_dual` 零改动复用 | ✅ |
| 数据与窗口 | 7 标的 10 年 1min，窗口照 prereg-windows-v0 + §9 补录 | `SYMBOL_FILES` 全量 1min，与第一轮同窗同文件 | ✅ |
| 纪律附款与排除项 | 跑前不搜索/不调参/不挑窗；结果原样入档；不加对照品种、不中途改判据 | 判据/窗口逐字取自冻结文本，零改动；结果原样入档 | ✅ |

## 7. 交付物清单

- [x] tape v3 门状态列导出（Rust 生产函数现算，`gate_state_dump.rs` + 7 把锁）
- [x] Python 门列消费面 + 状态闸（`gate_state_columns.py` + `_run_swing_leg(gate=…)` + 12 把锁）
- [x] 第二轮预注册 driver（判据/窗口/基线一字不动；两轮对照表 + 门 sanity 表 + 双跑自检行）
- [x] 门 sanity 核验（Rust/Python 双侧锁 + 跨语言接缝端到端实跑）
- [x] fatigue 可选臂（代码就绪 + 能力缺失 fail-fast，如实报为能力边界）
- [ ] 7 标的第二轮真实跑批结果 + 两轮对照数值（**卡点**：外部数据依赖，见 §5）

**待解除卡点后**：跑批 → #1282 票面贴第二轮结果与判定（过与不过都原样报）。
