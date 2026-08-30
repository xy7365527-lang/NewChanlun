# #1314 DX 门列/驱动 closes 6 根错位：口径核对 + 对齐修复（2026-08-30）

> **上游**：#1312 第二轮状态门控实验的 DX 缺口（driver 在 `gate_state_columns.py` 的
> bar 数接缝校验处 raise，DX 门控回测未跑；其余 6 品种已出结果）。
> **状态**：要做①（核对）**以代码层定论 + 落地诊断件**交付；要做②（对齐）**已落地并测试**；
> 要做③（重跑 DX + 6+1 全表入档）**卡在同一外部数据依赖**（沙盒无 1min 数据/无门列/无
> Rust 扩展），复现证据与续跑命令见 §4。

## 0. 结论摘要

1. **两侧清洗口径确实不同**（代码层已核实，见 §1）：驱动侧只剔 nan 且 `zip` 截到最短数组；
   dump 侧另剔任一 OHLC ≤0 的 bar。⇒ **同一份 json 内容下 dump 的保留集是驱动的子集，
   门列 bar 数不可能多于驱动**。
2. 因此 DX 实测「门列 2,058,424 > 驱动 2,058,418」**不可能由清洗口径差解释**，只能是
   两侧读到的不是同一份文件内容（磁带/门列 vintage 与当前 json 不同），或 json 各数组
   长度不齐（驱动 `zip` 截尾、dump 不截）。哪一种、多出的 6 根落在头/尾/内部——须在
   有数据的环境上跑对齐诊断定论，故本票落地 `analysis/_diag_gate_bar_alignment.py`。
3. **由票面既有事实可反推：多出的 6 根含尾部**。#1312 已落的截取逻辑只在
   「门列末 close == 驱动末 close」时截头；DX 走到了 raise ⇒ **末 close 不锚**。而内部
   多出会同时保住首末两根（末 close 必锚），故内部-only 已被排除 ⇒ 尾部有多出。
4. **对齐已按该反推落地**（§2）：`_align_start` 加首锚臂（首 close 锚定 ⇒ 舍弃尾部
   `n−len(closes)` 根），与已有的末锚臂（舍弃头部）对称，两端都不锚仍 fail-fast。
5. **顺带修掉一个真错**：#1312 的截头臂把整块 `raw` 平移 `3×skip` 字节后仍按
   `off=HEADER_SIZE` 读三列——列在文件里是三段连续区，整体平移会**跨列错位**，且还
   多跳了一个 header 长度。本票改为三列各自按同一位移切片，并加逐 bar 值回归锁。
6. **另补一处隐患**：多出的 bar 在内部时首末两根都在，#1312 的末锚臂会**静默截头**
   产出全程错位的 join。现改为"两端都锚定但 bar 数不等 ⇒ 显式拒绝并转诊断"。

## 1. 要做①：6 根差的口径核对（代码层）

| 侧 | 入口 | 清洗 | 遍历 |
|----|------|------|------|
| 驱动 | `analysis/m1_e_futures_backtest.py:86-93` `load_ohlc` | 只剔任一 OHLC 为 **nan** 的整根 bar | `zip(o,h,l,c)`（**截到最短数组**） |
| dump（门列） | `analysis/_dump_tape_rust.py:69-86` `load_ohlc_with_ts` | 剔 nan **另剔任一 OHLC ≤0** 的 bar | `range(len(closes))` |

- 门列 bar 数 = v2r 磁带 bar 数（`rust/src/trading/gate_state_dump.rs:209`
  `let n = tape.bars.len();`），磁带 bar 数 = `_dump_tape_rust.py` 清洗后的 closes 数
  ⇒ 门列 n 就是 dump 口径的读数，无第三口径插入。
- ⇒ **dump ⊆ 驱动**（剔除集超集）。测试锁：
  `tests/test_gate_bar_alignment.py::test_dump_kept_is_subset_of_driver_kept`。
- ⇒ 「门列比驱动多 6 根」在同一份文件内容下**不可能**发生；剩余两种可能（文件 vintage
  不同 / json 数组长度不齐致驱动侧 `zip` 截尾）由诊断件在有数据处判定。
- `analysis/gate_state_columns.py:125-127` 原注称两侧是"同款清洗"——与源码不符，本票已
  按实际口径改写该行注（防后来者据错注推断）。

### 诊断件：`analysis/_diag_gate_bar_alignment.py`

单标的一次跑出：原始数组长度表 → 两口径各自保留数与差因分解（仅因 ≤0 多剔几根 / `zip`
截尾几根）→ 门列头（n、ladder、首末 close）→ **逐 bar 时间戳比对**（dump 侧 ts 边车
`_tape_v2r_{SYM}_ts.i64` ↔ 驱动侧同过滤器的 ts）→ verdict 与处置建议：

| verdict | 含义 | 处置 |
|---------|------|------|
| `EQUAL` | 逐位相同 | 无需处置 |
| `HEAD_EXCESS` | 多出的全在头部 | 截头（`_align_start` 末锚臂自动） |
| `TAIL_EXCESS` | 多出的全在尾部 | 截尾（`_align_start` 首锚臂自动） |
| `INTERIOR_EXCESS` / `MIXED_EXCESS` | 含内部多出 | 下标 join 不可救，修 dump 口径后重落 |
| `DRIVER_SUPERSET` / `NOT_NESTED` | 门列过期 / 互不包含 | 重跑 dump |
| `NO_DATES` / `NO_TS_SIDECAR` / `DATES_TOO_SHORT` | 逐 bar 比对的输入不齐 | 只出 close 锚定读数，先补件/修数组 |

多出的 bar 逐根打 `gate_idx + epoch + 本地时刻`（前 20 根），可直接对照数据源核对。
诊断件只依赖标准库（不 import `newchan_rust`/numpy），**在最需要它的缺件环境下也跑得动**。

**不齐即报、不跟着崩**（评审修正）：json 数组长度不齐是本票两个候选假设之一，而 dump
口径按 `len(closes)` 遍历不截尾——真 dump 在这种 json 上会越界崩。诊断件若照搬那次崩溃，
就在最该出报告的输入上出不了报告，故改为 `dump_n=None + ohlc_arrays_ragged=True` 报出
（这一条本身即"磁带出自另一份 json"的实证）；`dates` 短于驱动保留的最大下标同理走
`DATES_TOO_SHORT`，不抛 IndexError。

`align_report` 的下标分桶以**门列 ts 两两不同**为前提，而两侧 loader 都只按 json 文件
顺序遍历、不排序不去重（该前提由数据源保证、不由代码保证），故 `dup_ts_gate` /
`dup_ts_driver` 一并报出，非零时报告显式声明 verdict 不可信。

口径复刻声明：诊断件逐字复刻上表两个 loader 的清洗分支（源行号已写入模块头）——复刻是
诊断必需（诊断对象就是"两个口径的差"，无法靠调用其中之一得到）；复刻的是**数据清洗**，
不是缠论判据，判据仍一律在 Rust 生产函数侧，本件零判据。两个 loader 改动须同步本文件。
唯一一处刻意不对称：`extract_arrays` 有 bars-schema 分支，而驱动侧 `load_ohlc` **没有**
（bars-schema 上驱动会 KeyError）——7 个期货标的都是 parallel-array，故本票范围内两侧读
同一分支；若日后有标的换 schema，`driver_kept` 的复刻即失真。

## 2. 要做②：对齐落地（`gate_state_columns._align_start`）

```text
n == len(closes)                       → start = 0，首末两端都核（#1312 原行为不变）
n > len(closes) ∧ 只有末 close 锚定     → start = n − len(closes)（舍头；只核末端）
n > len(closes) ∧ 只有首 close 锚定     → start = 0，取前 len(closes) 根（舍尾；只核首端）
n ≠ len(closes) ∧ 首末两端都锚定        → ValueError：多出的在**内部**，截哪端都错位
n < len(closes)                        → ValueError：门列窗口不足，无可截取的对齐窗
两端都不锚                              → ValueError：不同源/不同窗
                                         （三条错误都带差值、两端锚定读数、诊断命令）
```

`n < len(closes)` 单列一条是评审修正：原实现把它并入"两端都不锚"文案，而该情形下首
close 完全可能锚定（同一条错误信息里就打着 `门列首/末 = 10.0/11.0，驱动首/末 = 10.0/12.0`），
文案与自带读数自相矛盾，会把排查引向"数据源不同源"而非真因"门列窗口比驱动短"。

「两端都锚定但 bar 数不等」这一条是 #1312 的**隐患补掉**：内部多出时首末两根都在，
原逻辑会命中末锚臂**静默截头**，产出一份全程错位的 join。现改为显式拒绝并转诊断。

- 三列各自按 `start` 切片：`up = raw[H+start : H+start+m]`、
  `dn = raw[H+n+start : …]`、`fat = raw[H+2n+start : …]`（`n` 为**原始** bar 数 = 列跨距，
  `m = len(closes)`）——修掉 #1312 的整块平移错位。
- 锚定只核首末两根（列文件头只存这两个 close），**内部是否另有剔除由诊断件判定**，
  `_align_start` 不越权推断——这一界写进了函数 docstring。
- 已对齐的 6 个品种走 `n == len(closes)` 分支，逐位行为与 #1312 完全一致（零影响）。

## 3. 验证（沙盒实跑）

```text
$ uv run --with pytest pytest tests/test_gate_bar_alignment.py tests/test_state_gate_columns.py -q
31 passed
```

新增锁 `tests/test_gate_bar_alignment.py`（19 把）：
- 清洗口径：dump 保留集 ⊆ 驱动保留集；驱动 `zip` 截最短 vs dump 按 closes 长度遍历；
- `align_report` 六个 verdict + 多出 bar 的下标/时间戳定位；重复 ts 自报；
- **截头臂 / 截尾臂逐 bar 值正确**（三列各自切片——这两把即 #1312 平移错位的回归锁）；
- bar 数相等时首末两端仍都核；两端都锚定但 bar 数不等 ⇒ 拒绝（内部多出）；
  两端不锚 ⇒ fail-fast 且信息里点名诊断命令；门列少 bar ⇒ 拒绝理由不自相矛盾；
- `diagnose` 端到端（合成 json + 门列 + ts 边车，尾部多 2 根 → `TAIL_EXCESS` + vintage 提示）；
- 数组不齐 / `dates` 过短 ⇒ 出报告而非 IndexError（评审补的两把）；
- 缺件 ⇒ 卡点报告 + exit=2。

既有 `tests/test_state_gate_columns.py` 12 把全绿；其中 `seam_checks_fail_fast[bars]` 的
驱动切片改为**两端都不锚**的 `closes[1:3]`（首锚臂落地后 `closes[:3]` 会命中截尾对齐，
不再 fail-fast）——该用例锁的语义仍是"锚不上即拒"，未放松。

Rust 侧本票零改动（`cargo check` 无需跑；门列生产函数与格式一字未动）。

## 4. 要做③：卡点报告（外部依赖）

### 4.1 复现证据（本沙盒实跑）

```text
$ ls analysis/data_cache/          → 仅 venue_fee_* 5 个文件，无 *_1m_databento_10y.json、
                                     无 _tape_v2r_DX.bin / _tape_v2r_DX_ts.i64 / _tape_v3_gates_DX.bin
$ env | grep -ciE "databento"      → 0
$ python3 -c "import newchan_rust" → ModuleNotFoundError（Rust 扩展未构建，rust/target 不存在）
$ PYTHONPATH=src:analysis uv run python analysis/_diag_gate_bar_alignment.py DX
卡点：对齐诊断的输入不齐（外部依赖）。
  - [DX] 缺 1min 数据文件 …/analysis/data_cache/dx_1m_databento_10y.json
exit=2
$ PYTHONPATH=src:analysis uv run python analysis/m1_e_futures_dual_gated_backtest.py
卡点：第二轮跑批的输入不齐（外部依赖）。 …（7 缺数据 + 7 缺门列）… exit=2
```

⇒ DX 的 6 根差**落在头/尾/内部的实测定论**、DX 门控回测结果、6+1 全表数值，均无法在本
沙盒产出（#1312 实现期同一卡点）。本票交付的是"定论所需的判据与工具 + 对齐逻辑"，
数据侧一就位即可一次跑出。

### 4.2 解除条件

1. 7 个 1min 数据文件就位（`analysis/data_cache/`，gitignored）或注入 `DATABENTO_API_KEY`；
2. 构建 Rust 扩展 `cd rust && uv run maturin develop --release`；
3. `_tape_v2r_DX.bin` / `_tape_v2r_DX_ts.i64` / `_tape_v3_gates_DX.bin` 就位（下方步骤 3-4）。

### 4.3 续跑命令（数据就位后，按序）

```bash
cd /home/agent/workspace
# 1) Rust 扩展
cd rust && uv run maturin develop --release && cd ..
# 2) 数据就位（有 key 时拉取；或编排层直接投放文件）
DATABENTO_API_KEY=... PYTHONPATH=src uv run python scripts/fetch_1m_databento_10y.py all
# 3) v2r 磁带 + ts 边车（DX 单标的即可）
PYTHONPATH=src uv run python analysis/_dump_tape_rust.py DX
# 4) v3 门状态列（Rust 生产函数现算）
cd rust && GATE_DUMP_SYM=DX cargo test --release gate_state_dump_one -- --ignored --nocapture && cd ..
# 5) ★ 先跑对齐诊断，把 6 根差的落点（头/尾/内部）与时间戳原样入档
PYTHONPATH=src:analysis uv run python analysis/_diag_gate_bar_alignment.py DX
# 6) 重跑第二轮 driver（另 6 品种自动 resume 缓存，只算 DX；write_report 出 6+1 全表）
PYTHONPATH=src:analysis uv run python analysis/m1_e_futures_dual_gated_backtest.py
# 7) 产物
#    .chanlun/review-results/issue1312-state-gated-backtest.json / .md（6+1 全表 + 两轮对照 + 门 sanity）
```

**步骤 5 的 verdict 决定步骤 6 是否可直接跑**：
- `TAIL_EXCESS` / `HEAD_EXCESS` ⇒ `_align_start` 自动截取，直接跑第 6 步（driver 会打印
  一行截取记档：舍弃哪端多少根 + 锚定读数）；
- `INTERIOR_EXCESS` / `MIXED_EXCESS` / `NOT_NESTED` / `DRIVER_SUPERSET` ⇒ **下标 join 不可救**，
  须先按诊断输出修 dump 口径（或重落磁带/门列）再跑，不得强行对齐。

## 5. 交付物清单

- [x] 要做①：口径核对代码层定论（dump ⊆ 驱动 ⇒ 多出不可能来自口径差）+ 尾部多出的反推
      + 对齐诊断件（逐 bar 时间戳定位头/尾/内部，缺件即卡点 exit=2）
- [x] 要做②：`_align_start` 首锚臂（截尾）+ 末锚臂列切片修错 + 内部多出/两端不锚
      fail-fast 带诊断命令
- [x] 测试：新增 19 把 + 既有 12 把全绿（31 passed）
- [x] 评审修正（`review: #1314` commit）：少-bar 拒绝文案自相矛盾、诊断件在数组不齐/
      `dates` 过短上崩、`read_gate_header` 全量读盘（docstring 称不读列体）、
      `align_report` docstring 的 `sorted+去重` 出处与 `_dump_tape_rust.py` 不符、
      多出 bar 的时刻字段名 `utc` 实为本地时刻
- [ ] 要做③：DX 门控回测结果 + 6+1 全表入档（**卡点**：外部数据依赖，§4）
