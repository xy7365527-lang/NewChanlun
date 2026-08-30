# #1315 DX 门列比驱动 closes 少 2 根：口径核对定论 + 裁定 + 对齐落地（2026-08-30）

> **上游**：#1312 第二轮状态门控实验的 DX 缺口；#1314 对齐修复后的唯一残项。
> **状态**：要做①（核对）**定论**；要做②（裁定 + 落地 + 测试）**已交付**；
> 要做③（重跑 DX + 7/7 全表入档）**卡在外部数据依赖**（沙盒无 1min 数据/无磁带/无门列/
> 无 Rust 扩展），复现证据与续跑命令见 §4。

## 0. 结论摘要

1. **少 2 根不是错，是两侧清洗口径差的正常形态**。#1314 已在代码层证得
   「同一份 json 上 dump 的保留集 ⊆ 驱动的保留集」；本票观测的方向（门列 **少于** 驱动）
   与该结论**一致**——而 #1314 观测的「多 6 根」与之矛盾（那是磁带 vintage 问题，重落
   磁带后消失）。⇒ 少的这 2 根的精确定义：**驱动保留、dump 因「任一 OHLC ≤0」额外剔除
   的 bar**（§1 给出该定义在同源前提下的唯一性论证）。
2. **裁定（§2）：以驱动侧 bar 空间为坐标系，门列映射过去；不改任何一侧的清洗口径。**
   两侧清洗在各自谱系内都正当（dump 复刻 `fugue_v2_full_backtest.load_ohlc` 的
   nan+≤0 口径，理由写在该函数 docstring：≤0 价 bar 污染 PH/MACD 下游并令价格阈值止损
   误触发；驱动侧 `m1_e_futures_backtest.load_ohlc` 只剔 nan，且被 #1282 预注册与 #1309
   第一轮归档冻结）。改驱动 = 动预注册冻结输入并让第一轮/第二轮不可配对；改 dump =
   把已知的坏价 bar 灌进信号层且波及全部 v2r 磁带消费者。故**两侧都不动**，只在 join
   处做精确映射。
3. **落地**：`gate_state_columns.load_state_gate` 增「口径差重建臂」——门列第 k 根 =
   驱动侧第 k 个「四价全正」的 bar；被 dump 剔掉的 bar **无门读数 ⇒ fail-closed**
   （多空腿一律拒进场），并登记进 `no_reading_bars` 单列记档，不拿邻 bar 状态代理
   （沿 #1312「列不可用即 fail-fast，不降级代理」同一条纪律）。
4. **重建必须自证**：①驱动侧四价全正的 bar 数 == 门列 n；②这些 bar 的首/末 close ==
   门列头的 first/last close。任一不成立即 fail-fast 转诊断——差不止 ≤0 一条时（磁带
   vintage/数组不齐），重建无据，不得拿假设掩盖真错。
5. **诊断件同步**：`_diag_gate_bar_alignment.py` 把 `DRIVER_SUPERSET` 一分为二——
   逐根 ts 核到「驱动独有的正是被 ≤0 剔掉的那几根」⇒ 判口径差（走重建臂）；核不上
   ⇒ 仍判门列过期/不同源（重跑 dump）。旧文案一律说「门列过期，重跑 dump」，在本票
   的形态上会把人引去白重落一遍磁带。

## 1. 要做①：2 根差的精确定义（代码层）

| 侧 | 入口 | 清洗 | 遍历 |
|----|------|------|------|
| 驱动 | `analysis/m1_e_futures_backtest.py:70` `load_ohlc`（88 行 nan 分支） | 只剔任一 OHLC 为 **nan** 的整根 bar | `zip(o,h,l,c)`（**截到最短数组**） |
| dump（门列） | `analysis/_dump_tape_rust.py:56` `load_ohlc_with_ts`（72/74 行两分支） | 剔 nan **另剔任一 OHLC ≤0** 的 bar | `range(len(closes))` |

门列 bar 数 = v2r 磁带 bar 数（`rust/src/trading/gate_state_dump.rs:209` `tape.bars.len()`）
= dump 清洗后的 closes 数，无第三口径插入（#1314 已核）。

**同源前提下「门列少 k 根」的唯一解释**（逐条穷尽两侧差异）：

- nan 剔除：两侧逐字相同 ⇒ 不产生差；
- ≤0 剔除：**只有 dump 有** ⇒ dump 少的正是这些 bar；
- 遍历范围：驱动 `zip` 截到最短数组、dump 按 `len(closes)` 遍历 ⇒ 若 closes 是最短列，
  两侧覆盖同一下标区间，无差；若 closes 更长，**dump 会 IndexError 崩**（跑不出磁带，
  故已落盘的门列必非出自该 json——这属 vintage 情形，不属本形态）；
- 排序/去重：两侧都没有 ⇒ 不产生差。

⇒ 门列比驱动少 2 根 ⟺ **恰有 2 根 bar 四价含 ≤0 且不含 nan**（DX 实测 2,058,416 vs
2,058,418）。这一定义可被 §2 的两道核验（数目 + 两端 close）就地证否，不是只能信的推断。

> #1314 观测的是「门列多 6 根」，与 dump ⊆ 驱动矛盾，当时判为磁带 vintage 与 json 不同；
> 本票观测已翻到「少 2 根」，方向恢复一致 ⇒ 那 6 根多出的问题在重落磁带后已消失，本票
> 处理的是**剩下的、真正由口径差产生的** 2 根。

## 2. 要做②：裁定与落地

### 2.1 裁定

**以哪侧为准 = 以驱动侧 bar 空间为坐标系；两侧清洗口径都不改。** 理由：

| 选项 | 代价 | 判 |
|------|------|----|
| 驱动改成也剔 ≤0 | 改的是 #1282 预注册冻结的输入；#1309 第一轮归档（及全部 `_m1e_fut_*.json`）在旧口径上算的，两轮不可配对，报告里「本跑无门重算应与第一轮一致」的复现校验必然失配 | ✗ |
| dump 去掉 ≤0 剔除 | 把已知坏价 bar 灌进信号层（`fugue_v2_full_backtest.load_ohlc` docstring 写明其污染 PH/MACD 与价格阈值止损）；且 v2r 磁带是共享件，波及全部消费者 | ✗ |
| 两侧各自保留，join 处精确映射 | 需在 join 处显式登记「无门读数」的 bar 并 fail-closed；对齐的 6 品种零影响 | ✓ |

**残差如实记档**：门状态由生产函数在 **dump 的 bar 序列**上算出（该序列缺这 2 根坏价
bar），读数被搬到驱动坐标使用。即门读数并非「在驱动那条序列上重算一遍」的结果——两条
序列差 2 根 / 2,058,418 根。本票不消除这一残差（消除它就得统一口径 = 上表两个被否的
选项），只把它写明并把无读数的 bar 拒死。

### 2.2 落地（`analysis/gate_state_columns.py`）

```text
n == len(closes)                      → 原路径（#1312/#1314 行为不变，6 品种零影响）
n >  len(closes)                      → #1314 的截头/截尾锚定臂，锚不上 fail-fast
n <  len(closes) ∧ 传了 opens/highs/lows → 口径差重建臂（本票）：
      门列第 k 根 = 驱动侧第 k 个四价全正 bar；被剔的 bar 三列填 fail-closed
      （up/down_unexhausted = True、fatigue = UNAVAILABLE）+ 记入 no_reading_bars
      自证两道：四价全正 bar 数 == n；其首/末 close == 门列头 first/last close
n <  len(closes) ∧ 未传 OHLC          → fail-fast（closes 单列判不出某根是否因
      open/high/low ≤0 被剔），错误信息指路本臂
```

- fail-closed 的语义：`long_entry_allowed` / `short_entry_allowed` 在这些 bar 上一律
  False——**无读数 ⇒ 不准入**，与 #1312「fatigue 列不可用时开臂即 fail-fast、不拿 close
  代理 run_high」同一条纪律；不做「沿用上一根状态」的代理（那是给无读数的 bar 造读数）。
- `gate_open_rates` 增 `no_reading_bars` 单列读数：填充值计在 up/down 占比分子里，占比
  须按此扣除，不藏进总数。
- driver（`m1_e_futures_dual_gated_backtest.run_symbol`）把 opens/highs/lows 一并传入；
  报告 §3 增一行无读数 bar 记档（7 标的全 0 时也显式打「全为 0」）。

### 2.3 诊断件同步（`analysis/_diag_gate_bar_alignment.py`）

新增读数 `driver_excess_is_nonpositive_only`：门列 n == dump 口径保留数 ∧ 逐 bar ts 判
`DRIVER_SUPERSET` ∧ 驱动独有的 ts 集合 == 被 ≤0 剔掉的那几根的 ts 集合。三条全中才判
「口径差」（处置 = 走重建臂）；缺 ts 边车/无 dates 一律 False（不拿数目相等冒充逐根相等），
处置回落到「门列过期/不同源，重跑 dump」。

## 3. 验证（沙盒实跑）

```text
$ uv run --with pytest pytest tests/test_gate_bar_alignment.py tests/test_state_gate_columns.py -q
39 passed
```

本票新增 8 把（在 #1314 的 19 + #1312 的 12 之上）：

- 重建臂逐 bar 值正确（读数落回 0/2/4，被剔的 1/3 两向 fail-closed 且多空腿都拒）；
- `no_reading_bars` 计入门开率单列读数；
- 数目核不上（一根 ≤0 都没有却少 bar）⇒ 拒绝 + 点名诊断命令；
- 两端 close 不锚（数目对得上）⇒ 拒绝；
- 未传 OHLC 的少-bar ⇒ 仍 fail-fast 且指路重建臂；
- bar 数一致时传不传 OHLC 结果逐位相同、`no_reading_bars == ()`（6 品种零影响锁）；
- 诊断件：口径差形态判 `driver_excess_is_nonpositive_only=True` 且处置不说「重跑 dump」；
  非口径差的少-bar 仍判过期。

Rust 侧本票零改动（门列生产函数与格式一字未动）⇒ 无需 `cargo check`。

### 3.1 评审修正（同分支追加，语义不变）

```text
$ uv run --with pytest pytest tests/test_gate_bar_alignment.py \
      tests/test_state_gate_columns.py tests/test_dual_gated_report_note.py -q
44 passed
```

- `_nonpositive_map`：驱动侧**一根四价全正 bar 都没有**时（门列必空）原走「数目核不上」
  文案，打出的理由是「门列 bar 数 0 ≠ 四价全正的 bar 数 0」——自相矛盾（#1314 同款文案
  病）。改为单列一条准确理由，拒绝行为不变（仍 ValueError + 指路诊断）。新增 1 把锁。
- 报告 §3 的无门读数记档：`gate_open_rates` **缺** `no_reading_bars` 键（归档 JSON 出自
  #1315 之前的跑批，resume 直接复用不重算——正是 §4.3 第 6 步「只重算 DX」的常规路径）
  时，原实现按 0 记并打「7 标的全为 0（门列与驱动 bar 集逐根一致）」，把「没有这条读数」
  报成「有证据的 0」。改为缺键标的单独点名、不并进全 0 断言（与 `gate_open_rates` 把填充
  值单列同一条纪律）；行文抽成 `_no_reading_note` 并配 4 把锁
  （`tests/test_dual_gated_report_note.py`）。
- `_align_start` docstring 补一句：少-bar 分支在同源时由调用方**先**分流到重建臂，未传
  OHLC 才落到本函数拒掉——只读 docstring 会以为少-bar 一律致命。
- `gate_sanity` docstring 补记：`round1_*_blocked` 拦截计数里含无读数 bar 的 fail-closed
  拒入（拒的理由是「无读数」而非门读数，根数从 `no_reading_bars` 读）。

## 4. 要做③：卡点报告（外部依赖，与 #1314 同一处）

### 4.1 复现证据（本沙盒实跑，2026-08-30）

```text
$ ls analysis/data_cache/          → 仅 venue_fee_* 5 个文件；无 *_1m_databento_10y.json、
                                     无 _tape_v2r_DX.bin / _tape_v2r_DX_ts.i64 / _tape_v3_gates_DX.bin
$ env | grep -ciE databento        → 0
$ python3 -c "import newchan_rust" → ModuleNotFoundError（Rust 扩展未构建）
$ PYTHONPATH=src:analysis uv run python analysis/_diag_gate_bar_alignment.py DX      → exit=2（卡点报告）
$ PYTHONPATH=src:analysis uv run python analysis/m1_e_futures_dual_gated_backtest.py → exit=2（7 缺数据 + 7 缺门列）
```

⇒ 「DX 恰有 2 根 OHLC ≤0 的 bar」这一条**实测确认**、DX 门控回测数值、7/7 全表，均无法
在本沙盒产出。本票交付的是定义、裁定与对齐实现；数据一就位即可一次跑出，且重建臂自带
两道核验——若 DX 的 2 根差**不是** ≤0 口径差，跑批会 fail-fast 并转诊断，不会静默错位。

### 4.2 解除条件

1. 7 个 1min 数据文件就位（`analysis/data_cache/`，gitignored）或注入 `DATABENTO_API_KEY`；
2. 构建 Rust 扩展 `cd rust && uv run maturin develop --release`；
3. `_tape_v2r_DX.bin` / `_tape_v2r_DX_ts.i64` / `_tape_v3_gates_DX.bin` 就位（下方 3-4 步）。

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
# 5) ★ 先跑对齐诊断：确认 driver_excess_is_nonpositive_only=True（口径差）并把那 2 根的
#    时间戳原样入档；若为 False ⇒ 是磁带过期/不同源，先重落磁带再继续，不得强行对齐
PYTHONPATH=src:analysis uv run python analysis/_diag_gate_bar_alignment.py DX
# 6) 重跑第二轮 driver（另 6 品种自动 resume 缓存，只算 DX；write_report 出 7/7 全表）
PYTHONPATH=src:analysis uv run python analysis/m1_e_futures_dual_gated_backtest.py
# 7) 产物
#    .chanlun/review-results/issue1312-state-gated-backtest.json / .md
```

第 6 步跑起来时 driver 会打印一行对齐记档：
`[DX] 门列 2058416 根 < 驱动 2058418：口径差重建对齐——驱动侧 2 根 OHLC ≤0 的 bar 被
dump 剔除…首/末 close … 双锚成立`，该行与报告 §3 的无读数 bar 记档即本票的入档证据。

## 5. 交付物清单

- [x] 要做①：2 根差的精确定义（同源前提下唯一解释 = dump 的 ≤0 额外剔除）+ 穷尽论证
- [x] 要做②：裁定（驱动 bar 空间为坐标系，两侧口径均不改）+ 重建臂落地 + 自证两道
      + fail-closed 无读数 bar + 门开率单列记档 + 诊断件 `DRIVER_SUPERSET` 一分为二
- [x] 口径差异成文（本文件 §1/§2，含残差如实记档）
- [x] 测试：新增 8 把（`tests/test_gate_bar_alignment.py` + `tests/test_state_gate_columns.py`）
      39 passed；评审修正再加 5 把（含 `tests/test_dual_gated_report_note.py`）⇒ 44 passed（§3.1）
- [ ] 要做③：DX 门控回测入档 + 7/7 全表更新（**卡点**：外部数据依赖，§4）
