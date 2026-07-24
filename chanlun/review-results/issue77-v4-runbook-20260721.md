# #77 实装卡：V4 三窗三臂复验 runbook（goal 判据④ 主体）

- 日期：2026-07-21
- 来源：issue #77（SPEC #73 A 线第四票）；goal 判据④；前置 #75（进场门）+ #76（出场门）落地。
- 性质：验收票——照实判定不伪造（090：照实否定也是合格判定，伪造=失败）。

## 三臂定义（#75 交付后定稿，2026-07-21）

| 臂 | 含义 | env |
|---|---|---|
| 臂A 门关 | 默认路径（直通锁 :2179） | （无 nest gate env） |
| 臂B v0 基例门 | L2 点包含/v0 基例判定 | **不可经 env 复现**——#75 起门开 = typed 真链判定，v0 降为对照读出（无判定面模式位）。**臂B 数据 = V4 旧跑（20260720，wf7 Long +5598→-644 / Short -5599→-13178）作对照基线，报告中注明「旧跑口径」** |
| 臂C typed 真链门 | #75/#76 真链判定（N^δ 跨级链） | `THETA_NEST_CERT_GATE=1` |

另：#94 已修（2026-07-21 code-review 通过）后两行分工——NEST_GATE_STATS 行 = 准入判定分账（admitted/rejected 等）；NEST_GATE_CHAIN 行 = 真链命中 typed_found/链深构成/cross 对照，行尾 `reuse=` 列为复用通道剔除量（复用 admit≡old_admit by construction，已剔出 agree/对照差，账面可复算）。#77 对照表取数：真链命中/链深/cross 取 CHAIN 行，准入分账取 STATS 行，臂B 用旧跑读数。出场侧对应 NEST_GATE_EXIT 行（#76，v1/dual 各一）。

## 三窗

`p3fold` / `wf7`（震荡窗，硬验收线所在）/ `wf8`（趋势窗）——沿用 m8_e2e 既有窗口定义（wverify_run.rs）。

## 执行矩阵（9 跑，可并行 3 窗 × 逐臂）

```bash
cd /tmp/kimi-nest-mainline/rust
# 每跑：OPSEM_DUMP_DIR=/tmp/v4_<arm>_<win>；VOICE_EXEC=1（沿用现行执行臂口径）
THETA_NEST_CERT_GATE=<arm> VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/v4_<arm> \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture \
  > /tmp/v4_<arm>.out 2>&1
```

（OPSEM dump 内含三窗 trades.jsonl；按既有先例一次跑产三窗。）

## 对照表（已回填 2026-07-21；口径见表下注）

| 窗 | 臂 | trades | execR（Long/Short/Total） | nest 三态分布 | 拒绝率 | 链深构成（0/1/2+ rung） |
|---|---|---|---|---|---|---|
| wf7 | A 门关 | 504 | +5597.59(229) / -5598.74(275) / **-1414690** | 门关无统计（verdict 无(R≤0)） | 门关无统计 | 162 / 160 / 181（32.1/31.7/36.0%） |
| wf7 | B v0门（旧跑） | 467 | -644.44(212) / -13177.99(255) / **-2563498** | 旧跑无 CHAIN 行（STATS: nest_pass=1157 xzd_pass=276） | 291/1724 = 16.9% | 158 / 137 / 172（33.8/29.3/36.8%） |
| wf7 | C typed门 | 213 | -4521.68(108) / -13986.31(105) / **-1847334** | typed_found=71 / typed_none=1653 / xzd_fallback=1118 | 1044/1724 = 60.6% | 13 / 80 / 120（6.1/37.6/56.3%） |
| wf8 | A 门关 | 518 | +23826.69(284) / -16464.99(234) / **+2508870** | 门关无统计（verdict INCONCLUSIVE） | 门关无统计 | 207 / 146 / 165（40.0/28.2/31.9%） |
| wf8 | B v0门（旧跑） | 449 | +20668.46(250) / -8269.28(199) / **+3465366** | 旧跑无 CHAIN 行（STATS: nest_pass=1136 xzd_pass=89） | 293/1518 = 19.3% | 187 / 114 / 148（41.6/25.4/33.0%） |
| wf8 | C typed门 | 174 | +23415.97(92) / -8662.46(82) / **+4724613** | typed_found=51 / typed_none=1467 / xzd_fallback=1122 | 1050/1518 = 69.2% | 15 / 56 / 103（8.6/32.2/59.2%） |
| p3fold | A 门关 | 479 | +13737.94(229) / -11684.58(250) / **+923705** | 门关无统计（verdict INCONCLUSIVE） | 门关无统计 | 153 / 159 / 167（31.9/33.2/34.9%） |
| p3fold | B v0门（旧跑） | 431 | +3626.09(211) / -10178.65(220) / **-36842** | 旧跑无 CHAIN 行（STATS: nest_pass=1183 xzd_pass=141） | 309/1633 = 18.9% | 148 / 131 / 152（34.3/30.4/35.3%） |
| p3fold | C typed门 | 164 | +5066.77(80) / -14410.94(84) / **+468096** | typed_found=80 / typed_none=1553 / xzd_fallback=1153 | 1079/1633 = 66.1% | 1 / 63 / 100（0.6/38.4/61.0%） |

口径注记：
- execR 列 Long/Short = trades.jsonl `pnl_raw_unlevered` 按 `certificate.dir` 分侧求和（笔数在括号内）；Total = `[m8]` 行 execR（net_r，成本真实化口径）。两口径不同源，并列展示，与旧报告 §2.1/§2.2 同法。
- nest 三态分布 = NEST_GATE_CHAIN 行 typed_found / typed_none / xzd_fallback（#94 后口径）；臂B 旧跑（20260720）无 CHAIN 行——「旧跑读数未落盘」，以 STATS 行 nest_pass/xzd_pass 代注；臂A 门关恒无门统计。
- 链深构成 = trades.jsonl `certificate.nest_depth` 分布归并为 0 / 1 / ≥2（trades 侧口径，与旧报告 §2.4 同源归并）。候选侧对照（NEST_GATE_CHAIN admit_rungs）：臂C wf7 r0=69 r1=2 r2+=0；wf8 r0=51 r1=0 r2+=0；p3fold r0=75 r1=5 r2+=0——准入真链几乎全为单级。
- 链深构成对照 72.7% 单级基线（goal 判据②「不再 95.36% 退化」同口径）：本批 NEST_GATE_INDEX single_level_share = p3fold 0.9579 / wf7 0.9605 / wf8 1.0000——装配侧单级占比不降反升，退化未消除（详见验收报告 §5）。
- 出场侧 NEST_GATE_EXIT 行：本测试臂（`run_theta_v0_pi_overlay` 净额主 loop）**三窗均无输出**——#76 出场门统计只接进 `plan_and_fill_mtm`（v1）与 `plan_and_fill_mtm_dual`（dual）两 runner（runner.rs:4307/:4748），overlay 主 loop 未接出场侧统计。非跑批缺失，是接线面事实，照实落账。

## 验收线（goal 判据④，照实判定）

**判定结果（2026-07-21 回填，详见验收报告）**：① wf7 硬线 **未过 ✗**（臂C Long +5597.59→-4521.68 翻负；Short -5598.74→-13986.31 = 2.50× 超翻倍阈值，且双臂均劣于臂B）；② wf8 线 **过 ✓**（臂C execR +4724613 ≥ 臂B +3465366，Long 更优，Short 略劣 393.18 但总盘/execR 不劣）；③ 臂C 在 wf7 仍有害且 Long 侧害于臂B，已归因（验收报告 §5），不伪造通过。

1. **wf7 硬线**：臂C（真链门）不再有害——Long 不翻负（对照旧跑 +5598→-644）、Short 不亏翻倍（对照 -5599→-13178，2.35×）。
2. **wf8 线**：臂C 不劣于臂B（v0 门）。
3. 若臂C 仍有害：**如实落账 + 归因**（链深构成/拒绝率/逐笔对照定位有害机制），不伪造通过。
4. 产出物：本 runbook 回填 → `chanlun/review-results/v4-three-window-typed-chain-acceptance-<date>.md` + v1-e2e-scorecard-20260721.md 逐段回填 ✓/◐/✗ → goal 判据③④ 对账。

## 成本注记

三窗三臂 = 9 个 OPSEM 重放（单跑约 30-60s 量级，按既有任务 bash-9vudv9a8/bash-p7lkb3x1 先例）；无需 4.6M 全量重跑。
