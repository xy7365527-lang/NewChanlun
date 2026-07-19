# 归档：/tmp/m6_btc_oos_r_decomposition.md（阶段3 跑批 (4)，C-5，2026-07-19）

§3.5 归档携带件（p126 runbook）：
- HEAD commit：`fe03fd5c28`（worktree /tmp/kimi-nest-mainline；工作树 δ 清单见 p126-stage3-c5-20260719.md）
- 数据：`analysis/data_cache/btc_1m_full.json` shasum256=`16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`
- 命令行全文：`unset KAPPA_BARRIER_NUM KAPPA_BARRIER_DEN THETA_DIR_PRESET && cargo test --release --lib theta_v0::backtest::wverify_run::m6_btc_oos_r_decomposition -- --ignored --nocapture`
- 墙钟：finished in 22.32s（test result: ok. 1 passed; 0 failed；RDECOMP_RC=0；现场 /tmp/c5_rdecomp.out）
- 关②终验产量构成锚（主仓只读，p124-s8-merge-recon-20260719.md:13 原样）：
  「4. 产量对照（vs `/tmp/p92_replay_v3.log`）：candidates=18297、trend=1831、pan=16466、divergence_confirmed=8606、trend_div=836、pan_div=7770 —— **全等**；唯一分歧 `terminal_confirmed` 855 vs 1213（−358，正是收紧维）。」
- p121 T-N10 注：关②后重跑版本；旧 R 表（2026-07-04 系列）只作机制证据存档，对照见 p126-stage3-c5-20260719.md。

以下为产物原文（原样粘贴，未转写）：

---

# M6 BTC OOS R 分解（TARGET_STRATEGY_MAXFULL.md M6 / 路线.pdf p16 第十一关）

R = Σ N_t ΔP_t − Commission − Slippage − Funding − Borrow − LiquidationLoss

口径：margin=CME-simple 单段近似；cost=参数化常费率（funding 1bp/8h、borrow 0.01bp/bar、liq 0.5%）。**有效域 L1**：机制真装 + 参数化费率，真实 funding/借贷历史是外部数据缺口（A10 waiver 豁免外部数据源，不豁免机制）。**照实：预期成本拖累 net_r<gross，成本真实化非 alpha 声明。**

**口径标签：[L1机制/费率未标定]**（A10 附则B 强制；TW桥列＝A10 C5 对账行 ⌊funding+borrow+liq⌋——TW 账本不经构造子见持盾成本，η=tw() 高估在险权益恰此量）

| 窗 | 臂 | ΣN_tΔP_t | Comm+Slip | Funding | Borrow | LiqLoss | net_r | 守恒残差 | TW桥 | n_orders |
|---|---|---|---|---|---|---|---|---|---|---|
| p3fold | M6 | +1523359.86 | 16755156.38 | 151666.02 | 11.07 | 0.00 | -15383473.61 | -1.77e-6 | 151677 | 32195 |
| wf7 | M6 | +158330.18 | 20993704.09 | 160097.56 | 8.63 | 0.00 | -20995480.10 | -2.31e-6 | 160106 | 29190 |
| wf8 | M6 | +1263299.84 | 18104175.56 | 293751.20 | 5.43 | 0.00 | -17134632.35 | -1.86e-6 | 293756 | 14244 |

**守恒断言**：各窗 |守恒残差| ≤ 容差（价格 PnL − 五项成本 = 账本净变动，无泄漏）。
