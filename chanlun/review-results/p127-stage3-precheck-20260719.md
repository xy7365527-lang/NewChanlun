# 阶段 3 执行前置核对单（p126 runbook §4.3 逐项，2026-07-19）

**性质**：执行前硬前置的落盘核对——runbook `chanlun/review-results/p126-stage3-runbook-20260718.md` §4.3 六项的当前状态登记。本单随前置完成度更新；全绿后才开跑⑧⑨。

## §4.3 硬前置逐项

| # | 前置 | 状态 | 证据/备注 |
|---|---|---|---|
| 1 | 关② S=8 重放收口＋终验结论落盘（v3' dump＋验收行：terminal_confirmed ∈ 硬界 [748,1,213]、Pan=748 精确、877=110/122/31 精确） | **在跑** | S=8 分片 7/8 完成；shard2（L1 独块片）跑至 3.5M/4.6M bar（2026-07-19 02:37 时点）；自动归并脚本 `/tmp/p124_s2_auto_merge.sh` 已挂（PID 57506），完成后自动执行 `/tmp/p124_r2_merge_and_check.sh` |
| 2 | 关③ P3 收紧实装落地＋CERT 三栏（存续/失证/新增）验收（pan 裁定 P3 裁决4、P4-2） | **实装已落，验收待①** | P3 实装（kind 分域谓词 + owner=B start_index 判同 + b_center_start 快照）已落，`cargo test --lib` 1732/0 全绿；CERT 三栏验收待①的收紧后重放对账 |
| 3 | §2.1 前置实装：`M7_WITNESS_A10` env gate 接入三个 #[ignore] 测试＋witness η 三行增打（additive 一件）；T-N4 回归锁绿（κ=0 ∧ cost=None bit-exact） | **已完成（2026-07-19）** | 实装：`wverify_run.rs` `q4_margin_model`/`m6_cost_model` 提 `pub(crate)`；`runner.rs` 三测试（m7_l2_witness :5146 / m8_treasury_reach_distribution :5232 / m7_kappa_sensitivity_grid :5335）env gate 接入，与 m8_e2e 同函数同源（禁第二查法）；witness 增打 `cum_holding_cost`（=r_decomp.tw_holding_cost_bridge）+ `η_corrected` 两行。验证：`cargo test --lib` **1732/0 全绿**（`/tmp/m7a10_test_verify2.txt`）；冒烟 `M7_WITNESS_BARS=1000 M7_WITNESS_A10=1` gate 识别正确、增打两行生效、无 panic；env 未设路径 = 零成本旧路径 bit-exact（C5 回归锁：三项恒 0 ⟹ shadow=0） |
| 4 | §5.2 m8_e2e η 列口径裁定 (i)/(ii)（agent-42 注记的待确认项） | **待编排者拍板** | 选项 (i)：m8_e2e 层3 η_T 列改打 η_corrected = tw() − tw_holding_cost_bridge 并并列原值（additive 报告列变更，与 witness 同源更可读）；选项 (ii)：维持未修正列＋桥列并置，文注「η_T 为未修正值，修正量=桥列」（零代码改动）。影响定量（单向性，已登记）：修正只降不升 ⟹ 未修正为 false 的修正后必 false；旧三窗 η_T≪η_* ⟹ 修正不改变旧结论方向，改变的是「差多少」的诚实账目。**本单不代裁** |
| 5 | BTC 数据可达＋shasum 记录 | **已完成（2026-07-19）** | `analysis/data_cache/btc_1m_full.json`（worktree symlink → 主仓 329MB，只读消费）；**SHA-256 = `16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`**；κ env 残留检查：`KAPPA_BARRIER_NUM/DEN`、`THETA_DIR_PRESET`、`M7_WITNESS_*` 均无残留（κ=0 + Neutral 基线干净） |
| 6 | 零 cargo 纪律的解除时点：执行属后续任务，由编排者串行派发 | **待①后派发** | 本单即派发依据档案；⑧⑨ 执行命令序列照 p126 §2.2/§3.1 逐字 |

## 当前阻塞链

```
② shard2 完成 → 自动归并对账 → 终验结论落盘（①绿）
   → (0) signal 层终判重跑（wverify_full，新 BSP 集——M8 层1 转引对象；0704 旧报告不作转引，p126 §6-6）
   → ⑧ M7 witness 重跑（M7_WITNESS_A10=1，10 万 bar 足量窗口先行）
   → ⑨ M8 四层报告（§5.2 裁定 (i)/(ii) 先行拍板）
```

**补记（2026-07-19）**：signal 层终判重跑为执行序列第 0 步——`wverify_full`（`wverify_run.rs:391`，#[ignore]）。原 stage3_execute.sh 遗漏，已补入 `run_signal`（`/tmp/stage3_execute.sh`，用法 `[signal|witness|kappa|reach|m8|rdecomp|all]`）。转引纪律：0704 旧报告（`final-alpha-20260704.md`，旧 BSP 集）不作转引对象——关②改 BSP/证书集 ⟹ 信号集/订单流变 ⟹ 引用前提须复核重跑（p126 §6-6 注记的引用前提复核义务）。

## 纪律声明

- 照实否定是合格结果（161 号）：⑧⑨ 任一判定为否定/INCONCLUSIVE 均如实落盘，不改实现凑 PASS。
- v3 硬禁令：不用回测验证策略；witness/m8 跑批只验证代码正确性与不变量（L2 witness 是可达性/假设检验，认识论等级照 p126 :6 标注）。
- 报告头部强制三件套（p126 §2.5/§3.4）：`[L1机制/费率未标定]` 标签（带成本数值行）、「ADL 未建模（venue 适配项）」注记（引用 LiqLoss/强平语义时）、borrow 行「非市场借贷利率」声明。
- 旧数值基线作废存档：关②新 BSP 集合 ≠ 旧集合 ⟹ 订单流/fill 序列变 ⟹ 0704 系列旧 witness/m8 数值只作机制证据存档，不作验收基线（p126 :27, H2:65）。
