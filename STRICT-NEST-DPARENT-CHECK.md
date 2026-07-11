# STRICT-NEST-CHECK（P1 逐 bit 校验 + 基线 sanity + E1 三元组复算 + P2 证书装配）

数据：`/tmp/codex-work-p7/analysis/data_cache/btc_1m_full.json`，4613599 bar（2017-08-17 04:00:00 .. 2026-05-31 23:59:00）；交易：`/tmp/codex-work-p7/p7_inputs/trades.jsonl`，40001 笔。全量因果重放 1483.8s（P1 确认支逐 bar 因果累积，谓词层终态单次重算定判）。
参数：`ThetaConfig::default()`（l_max=6, min_parts_per_level=3，未调参）。

## 基线 sanity（每次必带）

- raw seen = 29088（期望 29088），conf 键 = 27152（期望 27152），type3 键 = 15165（期望 15165），entry==close = 40001/40001（期望 40001/40001），零长度 = 2955（期望 2955）→ **一致**

## P1 硬门：谓词输出 ≟ extract_signals buy1/sell1 背驰确认支（checkpoint 逐 bit 全比对 + 终态定判）

- 校验 bar 数 = 4613599；checkpoint 全比对 = 4613 次；(bar/终态,级) 比较次数 = 26618；不一致数 = **0**。
- 因果诊断（非门）：确认撤回 = 459，因果漏捕获 = 0（上游结构修订的两侧锁步行为，由 checkpoint 逐 bit 比对覆盖）；确认支首见滞后 max = 2829 bar。
- 每级不一致：无。

末 bar 快照（每级：buy1/sell1 bit 数 | 谓词事件数 | cand_delta=true | pan_div_diag=true）：

| 级别 ℓ | buy1/sell1 bits | Cand 事件 | cand_delta=true | pan_div_diag |
|---:|---:|---:|---:|---:|
| 0 | 452 | 1874 | 452 | 1295 |
| 1 | 7 | 302 | 7 | 282 |
| 2 | 13 | 113 | 13 | 93 |
| 3 | 9 | 26 | 9 | 10 |
| 4 | 2 | 4 | 2 | 0 |
| 5 | 0 | 0 | 0 | 0 |

**P1 硬门：PASS（逐 bit 一致）**

## E1 三元组复算（P2 验收比对表输入）

| 级别 ℓ | t1 首见键（buy1+sell1） | 期望 |
|---:|---:|---:|
| 0 | 533 | 533 |
| 1 | 37 | 37 |
| 2 | 95 | 95 |
| 3 | 142 | 142 |
| 4 | 135 | 135 |

- t1 首见键合计 = 942（期望 942）；主名单 n = 1278（期望 1278）；n_B = 1（期望 1）；n_C = 0（期望 0）→ **逐项一致**

## P2：D_parent 证书生产装配（N^δ_{ℓ↓0}，终态 cand_delta=true）

| 目标级 ℓ | 证书数（ℓ↓0 完整链） |
|---:|---:|
| 1 | 3 |
| 2 | 0 |
| 3 | 0 |
| 4 | 0 |
| 5 | 0 |

- 基例（ℓ0 终态 cand_delta=true）= 452；terminal 查无（唯一基例数）= 0（须 0，P1 一致性推论）；证书合计 = **3**。
- 历史 E1 名单 n_C = 0（冻结期望 0，仅 sanity）；全局 D_parent 证书产量 = 3。L0→L1 证书 = 3，回归预期 ≥3 → **达到（非硬闸门）**。
- 证书样例（前 3）：
  - ℓ=1 side=Short base(confirm=345578 I(A)=[345043,345348]) rungs(高→低)=confirm=345518 I(A_child)=[345043,345348]⊆D_parent=[344830,345518]
  - ℓ=1 side=Short base(confirm=2168532 I(A)=[2167960,2168349]) rungs(高→低)=confirm=2168349 I(A_child)=[2167960,2168349]⊆D_parent=[2167920,2168349]
  - ℓ=1 side=Long base(confirm=2365281 I(A)=[2364887,2365194]) rungs(高→低)=confirm=2365194 I(A_child)=[2364887,2365194]⊆D_parent=[2364887,2365194]

**P2 硬门：PASS（terminal 对账；产量预期不作硬闸门）**

## 总判：**PASS**（sanity ✓ / P1 ✓ / E1 三元组 ✓ / P2 证书 ✓）
