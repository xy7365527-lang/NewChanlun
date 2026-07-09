# STRICT-NEST-P1P2-RESULT（严格区间套 P1 谓词闭包 + P2 证书组装）

- 工作目录：`/tmp/strict-nest-work`（分支 `strict-nesting-20260708`）
- 校验 bin：`rust/src/bin/strict_nest_check.rs`；报告与本文数字一致来源：bin stdout（`STRICT-NEST-CHECK.md` 同步写盘）
- 数据：`/tmp/codex-work-p7/analysis/data_cache/btc_1m_full.json`，4613599 bar（2017-08-17 04:00:00 .. 2026-05-31 23:59:00）；交易：`/tmp/codex-work-p7/p7_inputs/trades.jsonl`，40001 笔
- 参数：`ThetaConfig::default()`（l_max=6, min_parts_per_level=3，未调参）；全量因果重放 1420.3s
- 冻结依据：strict-nesting-divergence-plan-20260708.md（P1/P2 规格与验收门）、strict-nesting-rulings-20260708.md（Cand^δ≔背驰段谓词、盘整背驰不入链、确认时点=完成时）、strict-nesting-divergence-survey-20260708.md（缺口盘点）

## 总判：P1 PASS / P2 PASS

## 0. 基线 sanity（每次必带）

raw seen = 29088（期望 29088），conf 键 = 27152（期望 27152），type3 键 = 15165（期望 15165），entry==close = 40001/40001（期望 40001/40001），零长度 = 2955（期望 2955）→ **一致**

## 1. P1 逐 bit 校验结果（硬门）

谓词输出 ≟ `extract_signals` buy1/sell1 背驰确认支：

- 校验 bar 数 = 4613599；checkpoint 逐 bit 全比对 = 4613 次；(bar/终态, 级) 比较次数 = 26618；**不一致数 = 0**
- 因果诊断（非门）：确认撤回 = 459，因果漏捕获 = 0（上游结构修订的两侧锁步行为，由 checkpoint 逐 bit 比对覆盖）；确认支首见滞后 max = 2829 bar
- 每级不一致：无

末 bar 快照（每级：buy1/sell1 bit 数 | 谓词事件数 | cand_delta=true | pan_div_diag=true）：

| 级别 ℓ | buy1/sell1 bits | Cand 事件 | cand_delta=true | pan_div_diag |
|---:|---:|---:|---:|---:|
| 0 | 452 | 579 | 452 | 0 |
| 1 | 7 | 20 | 7 | 0 |
| 2 | 13 | 20 | 13 | 0 |
| 3 | 9 | 16 | 9 | 0 |
| 4 | 2 | 4 | 2 | 0 |
| 5 | 0 | 0 | 0 | 0 |

**P1 硬门：PASS（逐 bit 一致）**

## 2. P2 探针复算比对表（E1 三元组）

| 级别 ℓ | t1 首见键（buy1+sell1） | 期望 |
|---:|---:|---:|
| 0 | 533 | 533 |
| 1 | 37 | 37 |
| 2 | 95 | 95 |
| 3 | 142 | 142 |
| 4 | 135 | 135 |

t1 首见键合计 = 942（期望 942）；主名单 n = 1278（期望 1278）；n_B = 1（期望 1）；n_C = 0（期望 0）→ **逐项一致**

## 3. 证书产量与样例

P2 装配层（`nest.rs`，N^δ_{ℓ↓0}：自高向低递降 0027:7,11 + 相邻级 Sub 包含复用 `is_sub` + 每级背驰段必要门 0027:15，终态全局口径）：

| 目标级 ℓ | 证书数（ℓ↓0 完整链） |
|---:|---:|
| 1 | 0 |
| 2 | 0 |
| 3 | 0 |
| 4 | 0 |
| 5 | 0 |

- 基例（ℓ0 终态 cand_delta=true）= 452；terminal 查无 = 0（须 0，P1 一致性推论）；证书合计 = **0**
- 产量口径比对：名单内 n_C = 0（期望 0）；全局证书产量 0 ∈ 预期「0 或个位数」→ **吻合**（稀是原文严格性的经验事实，未放宽任何判据凑产量）
- 证书样例：无（产量 0）

**P2 硬门：PASS**

## 4. 改动文件清单（git diff --stat）

P1 提交 `b48e8d9b11`：

```
 STRICT-NEST-CHECK.md                            |   41 +
 rust/src/bin/strict_nest_check.rs               | 1168 +++++++++++++++++++++++
 rust/src/theta_v0/classifier/mod.rs             |  136 +++
 rust/src/theta_v0/classifier/recursive_tower.rs |  242 +++++
 rust/src/theta_v0/classifier/signal.rs          |    6 +-
 5 files changed, 1590 insertions(+), 3 deletions(-)
```

P2 提交（本笔）：

```
 STRICT-NEST-CHECK.md                 |  22 ++-
 rust/src/bin/strict_nest_check.rs    | 102 +++++++++++-
 rust/src/theta_v0/classifier/nest.rs | 306 +++++++++++++++++++++++++++++++++++
 3 files changed, 422 insertions(+), 8 deletions(-)
```

铁律核对：`signal.rs` 仅 3 处 `pub(crate)` 可见性（零行为改动）；`divergence.rs` / `bsp.rs` 零改动；其余全部增量。

## 5. 偏差说明（操作化选择）

1. **Cand^δ 定位层**：A/C 段按该级 tower 段结构定位，跨中枢趋势配对（0016:62）；gauge 复用 `divergence.rs` `MacdArea` 默认路径；严格 `curr < prev`。
2. **盘整背驰**：单独诊断标志位 `pan_div_diag`，不入谓词/证书链（裁决冻结项；全程计数为 0）。
3. **确认时点=完成时**：「进入时」仅诊断对照，不参与硬门比对。
4. **P1 比对口径**：每 1000 bar checkpoint 逐 bit 全比对（4613 次）+ 终态单次重算定判；确认撤回（459 次）属上游结构修订的两侧锁步行为，由 checkpoint 覆盖，非漏捕获（漏捕获 = 0）。
5. **P2 证书口径**：终态全局口径装配（递降链每级都要求 Cand^δ 成立 + Sub 包含 + terminal 存在）；ℓ≥1 各级末 bar cand_delta 虽非零（7/13/9/2），但与下级配对段的 `is_sub` 包含 + 逐级递降联立后无完整链，故产量 0——与 n_C=0 口径吻合，未调判据。
6. **性能**：修复 bin 内跨 bar 持有 `ParseLayer` 致 parser `Rc::make_mut` 全量深拷贝的 O(n²)（bin 内部修复，不涉库行为）；全量重放 1420.3s（高于 314s 预算系 P1 checkpoint 逐 bit 比对与终态重算所致，属校验 bin 自身开销）。
7. **数据路径**：bin 内固定 `/tmp/codex-work-p7` 绝对路径（与 E1 探针 `e1_tri_anchor.rs` 对齐）。

## 6. cargo test 通过记录

- `cargo build --release --bin strict_nest_check`：通过（仅既有 `T1Emission` dead_code 警告）
- `cargo test --lib`（P2 改动后）：**1528 passed / 0 failed / 125 ignored**
- 偶发说明：一轮并行全量中 `theta_v0::backtest::runner::tests::opsem_dump_env_gated_bit_exact` 失败一次，单独重跑与整轮复跑均通过（并行测试 env 竞争偶发，与本任务改动无关）
- doctest：`ledger/separate.rs` 3 个失败为分支既有（引入提交 `89c23de3ac`，系 doc 注释缩进数学伪代码块被当作 doctest；`git merge-base --is-ancestor` 证实早于 P1 提交，本任务未触碰该文件）

## 7. 提交记录

- P1：`b48e8d9b11` feat(theta): P1 谓词闭包——per-level Cand^δ 背驰段谓词 + strict_nest_check 逐 bit 硬门 PASS
- P2：本笔（见 git log），feat(theta): P2 证书组装——N^δ_{ℓ↓0} 递降装配层 + 探针复算硬门 PASS
