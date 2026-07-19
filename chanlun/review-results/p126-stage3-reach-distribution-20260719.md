# 归档：/tmp/m8_treasury_reach_distribution.md（阶段3 跑批 (3)，C-5，2026-07-19）

§3.5 归档携带件（p126 runbook）：
- HEAD commit：`fe03fd5c28`（worktree /tmp/kimi-nest-mainline；工作树 δ 清单见 p126-stage3-c5-20260719.md）
- 数据：`analysis/data_cache/btc_1m_full.json` shasum256=`16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`
- 命令行全文：`unset KAPPA_BARRIER_NUM KAPPA_BARRIER_DEN THETA_DIR_PRESET && M7_WITNESS_A10=1 cargo test --release --lib -- --ignored --nocapture m8_treasury_reach_distribution_real_btc`
- 墙钟：finished in 82.12s（test result: ok. 1 passed; 0 failed；REACH_RC=0；现场 /tmp/c5_reach_dist.out）
- 关②终验产量构成锚（主仓只读，p124-s8-merge-recon-20260719.md:13 原样）：
  「4. 产量对照（vs `/tmp/p92_replay_v3.log`）：candidates=18297、trend=1831、pan=16466、divergence_confirmed=8606、trend_div=836、pan_div=7770 —— **全等**；唯一分歧 `terminal_confirmed` 855 vs 1213（−358，正是收紧维）。」

以下为产物原文（原样粘贴，未转写）：

---

# M8 treasury 多窗 Reach 分布（BTC anchored walk-forward，含 2020-2021 牛市段）

口径：κ=0 baseline；nav=窗首价×1000；生产 π 路径 tw_final 单读（stage 单向不可逆 ⟹ 终态=Reach）。

**成本口径：A10 注入（M7_WITNESS_A10=1）——margin=CME-simple + cost=三常费率（[L1机制/费率未标定]）；TW桥列=r_decomp.tw_holding_cost_bridge（⌊funding+borrow+liq⌋）。**

| 窗 | 期间 | bar | n_orders | 终Stage | Σ已实现PnL | holding | Q(notional_in) | holding−Q(门距离) | free | 退本金target | Reach≥II |
|---|---|---|---|---|---|---|---|---|---|---|---|
| wf0 | 2019-08-17..2020-02-16 | 264637 | 37856 | I | -9688013 | 0 | 10352240 | -10352240 | 664227 | 10352240 | false |
| wf1 | 2020-02-17..2020-08-16 | 261238 | 37171 | I | -9245801 | 592744 | 9909920 | -9317176 | 71375 | 9909920 | false |
| wf2 | 2020-08-17..2021-02-16 | 264531 | 23022 | I | -7957376 | 3778551 | 11920000 | -8141449 | 184074 | 11920000 | false |
| wf3 | 2021-02-17..2021-08-16 | 259846 | 34311 | I | -46653661 | 0 | 49269780 | -49269780 | 2616119 | 49269780 | false |
| wf4 | 2021-08-17..2022-02-16 | 264840 | 36690 | I | -42353468 | 308218 | 45927190 | -45618972 | 3265505 | 45927190 | false |
| wf5 | 2022-02-17..2022-08-16 | 260640 | 39835 | I | -40734256 | 2418296 | 43874510 | -41456214 | 721958 | 43874510 | false |
| wf6 | 2022-08-17..2023-02-16 | 264960 | 39445 | I | -22566920 | 0 | 23858180 | -23858180 | 1291261 | 23858180 | false |
| wf7 | 2023-02-17..2023-08-16 | 260560 | 29190 | I | -20845533 | 757152 | 23471260 | -22714108 | 1868575 | 23471260 | false |
| wf8 | 2023-08-17..2024-02-16 | 264960 | 14244 | I | -16842272 | 11570156 | 28721770 | -17151614 | 309343 | 28721770 | false |
| wf9 | 2024-02-17..2024-08-16 | 262080 | 36058 | I | -47916275 | 0 | 52133490 | -52133490 | 4217215 | 52133490 | false |
| wf10 | 2024-08-17..2025-02-16 | 264960 | 25969 | I | -46099202 | 3683514 | 58864020 | -55180506 | 9081305 | 58864020 | false |
| wf11 | 2025-02-17..2025-06-30 | 192960 | 20494 | I | -74441140 | 0 | 96066180 | -96066180 | 21625040 | 96066180 | false |

**结论**：任一窗 Reach≥Stage II = **false**。全窗终 Stage I（CostReduction）——退本金门（holding≥Q ∧ free≥target）无窗满足。门距离列（holding−Q<0）= 持仓市值从未累积过名义基线，与 signal 层无方向 alpha ⟹ 已实现 PnL 无正累积一致。
