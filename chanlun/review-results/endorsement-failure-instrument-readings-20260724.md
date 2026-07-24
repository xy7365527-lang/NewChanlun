# #214 三窗重放读数报告（背书失败原因测量装置）

- 日期：2026-07-24 ｜ 票据：issue #214 ｜ 装置实装：`endorsement-failure-instrument-impl-20260724.md`
- 验收口径（编排者 2026-07-24 范围裁定）：**wf7 单窗必跑**；p3fold/wf8 已一并重放，
  本报告按增强读数全列（三窗基线/红线对照同样全做）。
- 重放命令（spec ID-7）：`THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=<dir>/opsem
  T5A_CHAIN_DUMP_DIR=<dir>/dump M8_WIN_FILTER=<tag> cargo test --release --lib
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture`
- 基线 = 本装置合入前同一未提交树（改码前二进制重放留档）；
  校验 = `endorsement-failure-instrument-verify-20260724.py`（72 项断言，退出码 0，
  输出留档 `endorsement-failure-instrument-verify-output-20260724.txt`）。
- 090 标注约定：【确证】= 全量计数直接读出；【推断】= 基于计数结构的解释，非直接测量。

## 1. 验收核心读数（wf7 窗）

### 1.1 C1：Trend 域窗内同向点 `center.start_index == b_center_start` 相等率【确证】

| 点类 | owner 等值 / 合计 | 相等率 |
|---|---|---|
| 一类 | 4 / 8 | 0.5000 |
| 二类 | 9 / 40 | 0.2250 |
| 三类 | 14 / 124 | 0.1129 |
| **二+三类（票体验收报数）** | **23 / 164** | **0.1402** |

口径：事件窗内逐窗各计（钉口径①），同一物理点落入多事件窗逐窗各计一次；
分母覆盖成功+失败全体 Trend 事件的窗内同向点（172 点次 = 8+40+124）。

### 1.2 三窗四桶计数表（Trend 域事件级，互斥完备）【确证】

| 窗 | trend 事件 | success | owner_start 不等 | 窗口外 | 异向 | 无合法点(missing/empty) | owner 点子计数(身份缺失/实不等) |
|---|---|---|---|---|---|---|---|
| **wf7（验收窗）** | 38 | 16 | 8 | 1 | 13 | 0 (0/0) | 0 / 145 |
| p3fold（增强） | 41 | 19 | 7 | 2 | 13 | 0 (0/0) | 0 / 67 |
| wf8（增强） | 42 | 14 | 4 | 3 | 21 | 0 (0/0) | 0 / 93 |

平账（校验 B 组断言全过）：trend = success + 四桶合计；三窗均成立。
wf7 失败事件 22 件的桶构成：异向 13（59.1%）> owner_start 不等 8（36.4%）> 窗口外 1（4.5%）。
身份缺失子计数三窗全 0——生产点 center 恒 Some（与 owner 载体补齐不变量一致，实不等主导）。

### 1.3 按级 × kind 产量分解表（US-03）【确证】

wf7（验收窗）：`base / assembled / indexed` 按 `[级别槽][Trend, Pan]`（indexed 与 assembled 逐格相同）：

| 级别槽 | base(Trend, Pan) | assembled(Trend, Pan) |
|---|---|---|
| L1 | (32, 393) | (14, 34) |
| L2 | (5, 227) | (2, 8) |
| L3 | (1, 157) | (0, 29) |
| L4 | (0, 63) | (0, 1) |
| 合计 | (38, 840) | (16, 72) |

p3fold：base = L1(33,337) L2(6,221) L3(2,157) L4(0,17)；assembled = L1(16,30) L2(2,10) L3(1,50) L4(0,0)。
wf8：base = L1(34,369) L2(7,131) L3(0,181) L4(1,1)；assembled = L1(11,26) L2(2,12) L3(0,11) L4(1,0)。

对账（校验 C 组断言全过）：Σbase_lk == INDEX.base_events（878/773/724）；
Σassembled_lk == INDEX.assembled（88/109/63）；Σindexed_lk == INDEX.indexed（同 assembled）。

### 1.4 base 事件 kind 分布（C2 分母，US-04）【确证】

| 窗 | Trend | Pan | Trend 占比 |
|---|---|---|---|
| wf7 | 38 | 840 | 4.33% |
| p3fold | 41 | 732 | 5.30% |
| wf8 | 42 | 682 | 5.80% |

Pan 域成功/失败：wf7 72/768（成功率 8.6%）；p3fold 90/642（12.3%）；wf8 49/633（7.2%）。
Trend 域成功率：wf7 16/38 = 42.1%；p3fold 19/41 = 46.3%；wf8 14/42 = 33.3%。

## 2. 对 C1-C4 的分账读数（供 #211 / map #126 裁定；行为变更不在本票）

- **C2（Trend 占比低）坐实**【确证】：m8 窗 Trend 事件占比仅 4.3%-5.8%（p115 语料
  47/465 ≈ 10.1% 的一半）——Trend 域绝对件数 38-42 件/窗，「放开二/三类」的
  供给基数天然单薄，是 wf7 typed_found +2 量级的第一解释项。
- **C1（owner=B 掐二/三类）点级坐实、事件级量级有限**【确证-计数】：wf7 二/三类点
  相等率仅 14.0%（三类 11.3%）——绝大多数二/三类点的 owner 中枢 ≠ 事件 B。
  但事件级 owner_start 不等桶仅 8 件（wf7），即使全部放开 owner 合取、上限也只
  +8 件 Trend 成功（+50% 域内）。【推断】点级低相等率（14%）与事件级桶占比（36%）
  的反差由「每事件窗含多点、最早性取点、一点命中即成功」结构摊薄——成功事件
  16 件的窗内点吸收了大量 owner≠B 点次。
- **C3（窗口掐点）量级小**【确证】：窗口外桶三窗仅 1/2/3 件（占 Trend 失败
  4.5%/13.3%/12.5%），宽口径下仍小——窗口不是主要掐点机制。异向桶（13/13/21）
  才是 Trend 失败最大桶（wf7 59.1%）。【推断】异向桶大 = 窗内点方向分布与事件
  方向错位，属事件-点方向结构事实，非 owner 判据问题。
- **C4（分母重设）实测分母落账**【确证】：m8 窗 Trend 事件 38/41/42 件（对比 465 案
  全量普查）；Trend 窗内同向点 172/105/115 点次。465/95 普查口径（4.6M bar）与
  m8 窗（~30 万 bar/窗）的换算第一次有实测锚（重设动作归 map #126 层裁定）。

## 3. 零干预红线对照（ID-6.3，三窗全做）【确证】

| 对照面 | p3fold | wf7 | wf8 |
|---|---|---|---|
| NEST_GATE_STATS 整行 | 逐字节一致 | 逐字节一致 | 逐字节一致 |
| NEST_GATE_CHAIN 整行 | 逐字节一致 | 逐字节一致 | 逐字节一致 |
| NEST_GATE_T3 整行 | 逐字节一致 | 逐字节一致 | 逐字节一致 |
| NEST_GATE_INDEX 整行（13 项） | 逐字节一致 | 逐字节一致 | 逐字节一致 |
| tower_events.jsonl MD5 | 一致 | 一致 | 一致 |
| trades.jsonl MD5 | 一致 | 一致 | 一致 |
| 候选流 t5a_chain_dump MD5 | 一致 | 一致 | 一致 |

baseline 三窗 stderr 无 NEST_GATE_FAIL/LEVEL 行（纯增量新行，校验 A 组断言）。
背书结果、索引内容、链闭合、trades、候选流一个 bit 未变。

## 4. 产物与留档清单

- 实装说明：`endorsement-failure-instrument-impl-20260724.md`
- 本报告：`endorsement-failure-instrument-readings-20260724.md`
- 校验脚本：`endorsement-failure-instrument-verify-20260724.py`（72 项断言，退出码 0）
- 校验输出：`endorsement-failure-instrument-verify-output-20260724.txt`
- 全量测试尾行：`endorsement-instrument-cargo-test-lib-tail-20260724.txt`
  （1789 passed / 1 failed = 既线 #110 `extract_signals_bit_exact_digest_guard`，未修未归因）
- 重放留档（本目录，20260724 日期戳）：baseline/after 各窗 stderr 日志、
  tower_events、trades、chain_dump jsonl、baseline MD5 清单
  （`endorsement-instrument-{baseline,after}-<tag>-*-20260724.*`）
