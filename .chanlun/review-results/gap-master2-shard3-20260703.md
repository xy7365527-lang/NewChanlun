# gap-master-2 分片3：alpha/检验族 PDF 对照（8份）

任务 #153 | 工位 ws-gap2s3 | 2026-07-03
方法：INDEX.md 定位 → 历史对照凭据引用（dlpdf-b/dlpdf-e 全读产出 + wverify/strict-alpha/perm 系列）→ 存疑点对当前代码 grep 验证（decontam.rs / selector.rs / perm_test.rs）。

## 覆盖声明（8/8）

全部 8 份均有历史全读对照在案：6 份在 `dlpdf-b-bsp-alpha-20260702.md`（任务 #71，B 组 8/8 全读，逐页页数在案），过拟合.pdf 在 `dlpdf-e-gap-verification-20260702.md` §6（任务 #74，E 组 10/10 全读），回测的问题.pdf 闭环凭据在 `docs/formal-chain/INDEX.md:50`。本分片未重读 PDF 正文，仅对三个存疑点做了代码级复核（见表内锚点）。

## 逐 PDF 对照表

| PDF | 核心可实装主张 | 分类 | 锚点/凭据 | 严重性 |
|---|---|---|---|---|
| alpha.pdf（33页） | Eat(e) 覆盖定理 + \|E\|=O(n) 复杂度；§5 端点定向前视禁止（ε_e=sign(P_ρ−P_λ) 非 F_λ-可测） | 已对照 | dlpdf-b §0/§4：全 L0/结构层（soundness 非 performance）；前视禁止已由 b1 差分验证（645 谱系：π^cov≠π^bsp 对象分离） | 低（L0 结构层，无待装项） |
| 严格alpha.pdf（27页） | V*=E[max(0,max_a μ)]>0；LCB 门控选择器 χ_t=1[LCB>θ]；no-trade a_0；鞅不可能定理 | 已对照 + 1 项 B | dlpdf-b §3 缺口9 [中]；本次复核：`backtest/selector.rs:1`（χ_t 阈值过滤，买卖点alpha2 §13/§17）**无 LCB 字样**——检验层 LCB 在 `decontam.rs`/`mu_estimator.rs` 已装，**生产选择器仍是裸阈值非 LCB 门控**。B 留白理由：μ̂ 门已降级为防灾难非 alpha（memory `★i_class×δ共线`），生产侧 LCB 门控在 alpha 未确认前无消费者 | 中→低（alpha 终局 INCONCLUSIVE 后暂无消费者） |
| alpha分离.pdf（18页） | alpha/beta 分离去污原文：**残差 Y_i=δ(H−B̂)−C 上做检验**（§1/§4.1 强制）；分层键 (ℓ, h bucket, time block, σ_higher)（§4.2）；跨标的（§9：仅 BTC 正=可能 BTC beta）；确认滞后 | B 登记留白（原 A 已被路径终局吸收） | dlpdf-b §3 缺口1/2（最高优先级：retest 无 B̂ᵢ 减法、分层缺 h/time block）。本次复核：`backtest/decontam.rs:1-48` 为**路径 A 替代**（分层内 δ 置换 + 三态判据 VALIDATED/INCONCLUSIVE/FALSIFIED，665 裁决），非原文"残差减法+残差上置换"双重。后续终局：双 goal 四口径全 INCONCLUSIVE（memory `★i_class×δ共线`）、跨标的已跑（`l3-cross-symbol-alpha-20260702.md`→BTC 独有、`l3norm-alpha-20260703.md`）。**留白记录：若未来重启 alpha 检验，B̂ᵢ 残差减法 + h/time-block 分层维仍是原文要求的未装前半** | 中（重启 alpha 检验时升 A） |
| alpha检验.pdf（20页） | 统计判定四定理：n_eff 修正/功效门/三态判据/block bootstrap；§6 删尾稳健性诊断；§7-§8 方向不对称回归 μ_sell−μ_buy>0 | 已装大半 + 存疑 1 项 | dlpdf-b §2.1：n_eff/三态/功效门已逐条溯源实装（`mu_estimator.rs`/`decontam.rs`/`perm_test.rs`/`prereg_windows.rs`）。存疑：删尾——grep `trim` 命中 `perm_test.rs`/`wverify_run.rs`/`econ_positive.rs`，**疑似已装但未逐行确认语义**（铁律：列为存疑）；方向不对称回归（block bootstrap/cluster SE）dlpdf-b 缺口4 记录缺失，未见后续实装凭据 | 中（方向不对称检验缺失；但 alpha 终局后无活跃消费者） |
| 买卖点alpha.pdf（17页） | π^cov vs π^bsp 对象分离；定理3 前视禁止 | 已对照/已闭环 | dlpdf-b §0；645 谱系（goal 否证=错对象）+ b1 差分（`codex-b1-audit-20260702.md`）；657 谱系（alpha-pdf-part1 能指冲突降级 M24 因果可测性） | 低 |
| 买卖点alpha2.pdf（38页） | 本组权威 alpha 判定链：§12 μ_{ℓ,δ,I}>0 桶键（I_γ⊆{1,2,3} 七子集集合语义）；§13 细分类 V(Z)≥V(Y)+χ_t 选择器；§14/§16 严格/鞅定理；§10-11 ΔN 增量 | C 在办→已终局 | dlpdf-b §2.2 缺口7 [中]（I_γ 集合被压成单值 bsp_class）。后续已在办并终局：`beta-bucket-design-20260703.md` + `codex-beta-ruling-20260703.md` + memory `★i_class×δ共线=置换自毁`——方向性维进桶键即毁 δ 置换，四口径全 INCONCLUSIVE。I_γ 扩维不再是缺口而是已否证路径 | 低（已终局，勿重开） |
| 过拟合.pdf（14页） | 结构分类器零参数无过拟合 vs 收益估计器必然过拟合（winner's curse）；标准解=shrinkage+LCB+严格 OOS+block/cluster；prereg+n_eff 修正；pooling 需部分可交换验证 | 已装 | dlpdf-e §6 明确裁决："W-VERIFY 过拟合防护线的方法论蓝本，已在代码层落地"——`prereg_windows.rs`/`l3_pi_falsify.rs`/`perm_test.rs`/`pooling_icc.rs`/`mu_estimator.rs`/`selector.rs`/`l3_fullwindow.rs`/`decontam.rs` 全管线（#51/#52 收敛）。shrinkage（层级收缩）一项 dlpdf-e 未单列——存疑：LCB 已装但显式 shrinkage 估计未见凭据，列存疑留白 | 低（存疑项：shrinkage 未单独实装） |
| 回测的问题.pdf | 7-02 晚 ChatGPT 双裁决：区间套 rung=区间包含（定义层）+ 增量塔未确认 frontier 必重算（实现层） | 已装/已闭环 | `INDEX.md:50`：#77 否证归因 / #84-#93 修复 bit-exact+O(n) / #97 高级别重测维持；memory `★bottom-up区间套≡point-contain descend`（parity 测试固化）、`★frontier resume b_t选太晚`（实现 bug 已修） | 低（已闭环） |

## 三分类汇总

- **A 必装缺口：0 项**（alpha分离.pdf 的残差减法原为最高优先级 A，但双 goal 终局四口径 INCONCLUSIVE 后无活跃消费者，降 B 留白；重启 alpha 检验时自动升回 A）。
- **B 登记留白：4 项**——① B̂ᵢ 残差减法 + h/time-block 分层维（alpha分离 §4.1/§4.2）；② 生产 selector LCB 门控（严格alpha §12）；③ 方向不对称回归 μ_sell−μ_buy（alpha检验 §7-§8）；④ shrinkage 层级收缩估计（过拟合.pdf）。共同触发条件：**未来任何重启 alpha 检验/生产化的 prereg 必须把这四项列入冻结判据**。
- **C 在办/已终局：1 项**——I_γ 集合桶键（买卖点alpha2 §12）已由 beta-bucket 线走到否证终局，勿重开。

## 六要素

1. **结论**：alpha/检验族 8 份 PDF 无 A 级必装缺口；4 项 B 留白全部挂在"alpha 终局 INCONCLUSIVE 后无消费者"这一共同前提上，触发条件明确（重启 alpha 检验）。
2. **定义依据**：各 PDF 核心主张摘自 dlpdf-b/dlpdf-e 全读产出（逐页读取在案），本分片对照的是"主张→实装凭据"映射，非重新定义。
3. **边界条件**：若"alpha 终局 INCONCLUSIVE"被推翻（如新标的/新桶键出现 VALIDATED），4 项 B 全部升 A，本分类翻转。删尾（trim）与 shrinkage 两处标注"疑似/存疑"——若逐行核实发现 trim 语义非删尾诊断，alpha检验.pdf 行的"已装大半"降级。
4. **下游推论**：#156 汇编时本分片贡献 0 A / 4 B / 1 C-终局；B 项应汇入统一的"alpha 重启 prereg 前置清单"。
5. **谱系引用**：645（对象分离）、657（能指冲突降级）、663（可交易性≠显著性）、665（路径 A 裁决）；memory：`★i_class×δ共线`、`★W-VERIFY alpha重测=PASS但脆弱`、`★S_Θ v1全窗8/8 L3否证`、`★跨标的L3=INCONCLUSIVE主桶BTC独有`。
6. **影响声明**：仅新增本文件，不改代码不改谱系；对 #156 汇编是输入。

## 认识论等级标注（formalization-validity-domain）

本对照本身是 L0 文档层作业（主张↔凭据映射）。所引实证结论各自等级：W-VERIFY retest=L2（单标的条件性 PASS，dlpdf-b 已裁定 ≠ confirmed structural alpha）；跨标的=L3 否证向（BTC 独有）；perm/置换终局=L2-L3（四口径 INCONCLUSIVE）。本分片未产生新的实证等级声明。
