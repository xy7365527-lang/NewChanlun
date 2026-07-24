# 裁定 #65：χ 门 X_γ 量纲——÷入场名义（方案③）

- 日期：2026-07-21
- 性质：**编排者直接拍板**（非代理签署）。票据：wayfinder #65。
- 证据基座：`chanlun/review-results/chi-dimension-evidence-20260720.md`（教义 + 机制 + wf7/wf8 静态数据三层）；前置材料 `chanlun/review-results/gap1-chi-dimension-ruling-material-20260719.md`。
- 纪律：本文纯文档裁定，零代码改动；零 git mutation；主仓零写入。

## 1. 裁定

**χ 门的 X_γ 量纲 = 方案③：费扣后持仓期相对收益 `X = (δ(P_exit−P_entry) − fee) / P_entry`。**

- 方案②（%/bar）**淘汰**：wf7/wf8/合并三键 100% REJ 无一 PASS，退化为全拒门；D-a 失真实测坐实（wf8 L3 mean 被 ÷bars 抹成 −1.56e-6，盈利引擎符号被毁）。机制批判从方向推导升级为实测坐实。
- 方案①（绝对额，现状 ledger 口径）**废止为 χ 喂入口径**：level 键 100% REJ 含 L3 盈利引擎（mean +458.34 → LCB −59.92），χ 实为方向门——维持① = 维持声明=能力不符（090）。① 可保留为 dump/对账口径（pnl_raw_unlevered），不得再作 χ 喂入。

## 2. 裁定理由

1. **教义层**：两份 PDF 均未规定 X_γ 量纲（alpha2 §12 的 X_γ=δΔP−C 是每单位价格差；严格alpha §1 的 ΔR 是货币额）——量纲为 extra-教义设计决策。③ 是 §12 字面距离最近的可比化推进，θ=0 语义与严格alpha §1 净值比例解释对齐。
2. **机制层**：③ 移出价格水平源、估计对象不变（仍是 per-certificate 全程持有收益）、无新增失真；qty 留在 Θ_risk 层（层级净化，严格alpha §4 μ(z,a) 的 a 不含仓位）。
3. **稳健层**：③ 对 qty 喂入免疫——① 一旦切到 pi_bsp_timing 式 qty 口径实测立刻全拒（S1q：wf8 dir Long LCB +10.25 → −4516.26），③ 的逐笔 ÷(qty·entry_px) 天然吸收该方差源。

## 3. 照实边界（声明=能力，禁止夸大）

- ③ 在本静态样本（wf7 n=504 / wf8 n=518，θ=0、z=1.645）的桶判定与①（S1f）**完全相同**（dir 1/2、level 全 REJ）——③ 未展示判定改善。
- ③ 的 CV 缩减效应**本样本未观测到**（wf8 L3：S1f CV 3.30 → S3 3.43 微升）——裁定③的预期收益落在「θ=0 语义跨期可比 + qty 免疫 + 层级净化」，**不得**落在「已证过滤率下降/CV 缩减」上。
- ③ 下唯一放行桶（dir Long）跨窗口不稳定（wf8 PASS / wf7 REJ），照实登记。
- 量纲裁定生效后的首批门生效观测属【待实测】（chi=Some 的 walk-forward 运行）。

## 4. 执行后果（实装指引，非本轮）

- χ 喂入口径切换点：`rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:266-269`（ledger 主口径，现状 qty=1 费扣绝对额）→ 费扣后 ÷ entry_px；`rust/src/bin/pi_bsp_timing.rs:370/:463`（bin 口径，现状含 voice.qty）同步切换为 ÷(qty·entry_px)。
- `mu_estimator.rs` 的 LCB 代数（:468-472）不动——量纲在喂入侧，不在判定侧。
- dump/对账口径（runner.rs:2940-2943 pnl_raw_unlevered 费前绝对额）不动，与 χ 喂入口径分离登记。
- 本裁定是 L0 χ 接入（wayfinder #71）的前置；接入实装另行授权。

## 5. 复议通道

实测出现③的逐笔 ÷entry_px 引入新失真（带桶统计与机制归因的具体案例），或 walk-forward 门生效观测推翻本裁定预期，可发起复议；推翻以新 escalate 文档显式 SUPERSEDE 本文为准，推翻前本文有效。

## 签字位

- [x] 方案③ ÷入场名义 = χ 喂入唯一合法量纲——编排者拍板（2026-07-21）
- [x] 方案② 淘汰（实测全拒门）/ 方案① 废止为 χ 喂入口径（保留 dump 对账用）
- [x] 照实边界 §3 全条登记（判定未改善、CV 缩减未观测、放行桶跨窗不稳）
- [ ] 编排者复议（空位）
