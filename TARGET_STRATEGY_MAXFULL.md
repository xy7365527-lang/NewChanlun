# TARGET_STRATEGY_MAXFULL.md — 最终目标对象与 M0→M8 主线冻结表

> **权威来源**：`docs/formal-chain/路线.pdf`（23 页《推导完全分类》，编排者 2026-07-04 钦定）。本文件逐页忠实转写其 M0 目标定义（p9-10）、M1-M13 各关判据（p10-17）、M0-M8 最终主线（p18-21）、§15 开跑条件（p21）、AncOK waiver 措辞（p4/p15）。
> **与 693 号 `TARGET_STRATEGY.md` 的关系**：本文件是 693 三分冻结表（Π_signal-full / Π_exec-full / Π_treasury-full）的**执行序整合与扩展**，不是推翻。三分表回答「三个互不混淆的对象各是什么」；本文件回答「把三个对象按 M0→M8 一条主线闭合到 Π_max-full 的顺序与逐关验收」。对应见 §6。
> **措辞纪律**：见 §5，全项目强制。**禁用「终局」描述当前已测对象**（PDF M0 原文要求）；signal-full 结果**不得**当成 max-full 结论。

---

## 1. 最终目标对象 Π_max-full（PDF p9-10 逐符号转写）

PDF p9 boxed 定义：

\[
\boxed{\Pi_{\max\text{-}full} = (T,\ K,\ \Gamma,\ Z,\ \mathcal{I},\ A,\ P^{sep},\ N,\ E,\ Q,\ \mathcal{K},\ TW,\ \pi,\ Eval)}
\]

> **元数说明**：任务书称「十三元组」，PDF p9 符号表实列 **14 个位置**——含正体 \(K\)（操作 carrier forest）与花体 \(\mathcal{K}\)（风险/保证金/强平可行集）两个**不同**符号。本表忠实 PDF 转写 14 位；若上游以 \(N=Net(P^{sep})\) 为 \(P^{sep}\) 的导出量而不单列，则退化为 13 位。此差异不改变任何验收判据。

| 符号 | 含义（PDF p9-10 原表） |
|---|---|
| \(T\) | bit-exact 多级走势塔 |
| \(K\) | 操作 carrier forest |
| \(\Gamma\) | 区间套买卖点证书集合 |
| \(Z\) | 全互斥状态分类 |
| \(\mathcal{I}\) | 固定优先级解释器 |
| \(A\) | 活动声部集合 |
| \(P^{sep}\) | 分账本多空声部头寸 |
| \(N = Net(P^{sep})\) | 执行层净头寸 |
| \(E\) | typed exit 出场规则 |
| \(Q\) | 仓位函数 |
| \(\mathcal{K}\) | 风险/保证金/强平可行集 |
| \(TW\) | 三阶段资金账本 |
| \(\pi\) | 最终订单策略 |
| \(Eval\) | OOS 经济评价系统 |

### 1.1 最终订单策略公式（PDF p10 原式）

\[
\pi^{full}_\Theta(x_t) = Schedule_\Theta\!\left[\ \operatorname{LexArgmin}_{p\,\in\,\mathcal{K}_\Theta(x_t)} J_\Theta(p,\ \tilde{p}_{t+1}) - p_t\ \right]
\]

其中：

\[
\tilde{p}_{t+1} = \sum_{v\,\in\,A_{t+1}} q_\Theta(z_v,\ x_t)\, e^{\sigma_v}
\]

\[
A_{t+1} = AncOK\big[(A_t \setminus D_t) \cup O_t\big]
\]

解释器输出：

\[
(D_t,\ O_t,\ L_t,\ TWEvent_t) = \mathcal{I}_\Theta(A_t,\ \Gamma_t,\ TW_t,\ Risk_t)
\]

PDF 原文：「这才是最终完整策略对象。」

---

## 2. M0 → M8 主线（PDF p18-21，每关目标 / 必须包含 / 验收判据逐条转写）

PDF §14 原则：「下面是完整主线，不再拆 A/B/C。」完整路线图为
\(M0 \to M1 \to M2 \to M3 \to M4 \to M5 \to M6 \to M7 \to M8\)。

### M0：命名和目标冻结

- **目标**：输出 `TARGET_STRATEGY_MAXFULL.md`（即本文件），写明最终目标是 \(\Pi_{\max\text{-}full}\)。
- **必须包含**：十四元组定义（§1）+ 最终订单策略公式（§1.1）。
- **验收**：**禁止再用「终局」描述当前已测对象**（PDF M0 原文）。

### M1：结构塔最终锁定

- **必须包含**（PDF p10 / p18）：中枢延伸（center extension）；走势类型分解（move decomposition）；局部趋势门（local trend gate）；A/C 次级别走势类型化（typed A/C）；盘整背驰承接（PanDiv）；bit-exact 长历史塔；mutable frontier 重算（不把未确认窗口当 sealed prefix）。
- **验收**：
  \[
  \boxed{T^{inc}_t = T^{full}_t \quad \forall t}
  \]
  即增量塔与全量重算塔逐 bar bit-exact。附加：`long-history tower parity = 0 diff`、`type1 > 0 on full history`、`center extension / move decomposition / local trend gate parity` 全绿。
- **PDF 附注**：「一类买卖点零触达」的核心结构错误（趋势门全历史锁死、中枢延伸缺失）已确认为**定义级实现错误**；「一类=0 是市场事实」列入**作废口径**。高级别塔若未 bit-exact，则任何 \(\mu(L2+,\cdot)\) 都不可采信。

### M2：区间套和 Cand 最终锁定

- **必须包含**（PDF p11 / p19）：\(N^\delta_{\ell\downarrow e}\) 真递归；跨级 rung 为 \(J_{child} \subseteq J_{parent}\)（**不是** \(source\_index = end(parent)\)——后者列入作废口径）；ForceState 真入候选；MACD **不**作为唯一硬门；LiveCand / SettledCand 口径明确。
- **递归定义**（PDF p11）：
  \[
  N^\delta_{e\downarrow e} = Conf^\delta_e,\qquad
  N^\delta_{\ell\downarrow e} = Cand^\delta_\ell \wedge [J^\delta_{\ell-1} \subseteq J^\delta_\ell] \wedge N^\delta_{\ell-1\downarrow e}
  \]
  其中 \(J_e \subseteq J_{e+1} \subseteq \cdots \subseteq J_\ell\)。
- **验收**（PDF p19）：
  \[
  \boxed{\forall \gamma:\ N=1,\qquad J_e \subseteq \cdots \subseteq J_\ell}
  \]
  并输出每笔证书 \((\ell, e, \delta, J_e, \dots, J_\ell, Ndepth)\)，检查 \(\forall k:\ J_k \subseteq J_{k+1}\)，输出 \(depth = 0,1,2,3,\dots\) 的真实分布。
- **PDF 附注**：若大部分退化为 base case（\(depth=0\)）可接受，但必须说明「这是真实数据几何结果，不是代码未调用」。

### M3：分类状态最终锁定

- **必须包含**（PDF p13 / p19）：六类买卖点；side-free \(bsp\_class\)；δ-free 主裁决；\(parent\_dir\)；ForceState；ExitType；\(d\) 或 \(\mu_R\) 字段；\(higher\_dir / lower\_exec\) 统计字段。
- **六类保留**（PDF p13）：\(I_\gamma \in \{B1,B2,B3,S1,S2,S3,StructBreak\}\)，或 side-free 类 \(bsp\_class \in \{1,2,3,StructBreak\}, \delta \in \{+1,-1\}\)。**禁止压扁**（压扁会使某一类正 alpha 被其他类稀释：\(\mu(\ell,\delta)=\sum_I \mu(\ell,\delta,I)\Pr(I\mid\ell,\delta)\)）。
- **δ-free 主裁决桶**（PDF p13 关卡5）：
  \[
  \boxed{Z_{decision} = (level,\ bsp\_class,\ parent\_dir,\ ForceState,\ ExitType,\ \dots)}
  \]
  **而非** \((level, bsp\_class, \delta, parent\_dir)\)——\(i\_class\) 与 \(\delta\) 共线，含 \(\delta\) 桶会自毁置换检验。逐笔保存 \(entry, exit, X, d, \delta, Z_{\delta\text{-}free}, ForceState, ExitType\)，**不再依赖离线聚合补救**。
- **验收**（PDF p19）：
  \[
  \boxed{\mathcal{X} = \bigsqcup_z C_z,\qquad \text{运行时 } \sum_z \mathbf{1}_{C_z} = 1}
  \]

### M4：统计准入最终锁定

- **必须包含**（PDF p14 / p19）：\(X\) 与 \(X/d\) 双收益；OOS；LCB；block/bootstrap；δ-free permutation；高级别定方向低级别统计。
- **双收益定义**（PDF p14 关卡6）：\(X_i = \delta_i(P_{\tau_i} - P_{t_i}) - C_i\)，\(X^R_i = X_i / d_i\)（\(d_i > 0\)），对应 \(\mu(z)=\mathbb{E}[X\mid Z=z]\) 与 \(\mu_R(z)=\mathbb{E}[X/d\mid Z=z]\)。必须**双主判据** \(LCB(\mu)>0\) 且 \(LCB(\mu_R)>0\)。四态解释：\((+,+)\) 强信号；\((+,-/0)\) 只在大波动上赚钱风险调整失败；\((-/0,+)\) 小风险高质量信号；\((-/0,-/0)\) 无证据。
- **高级别定方向低级别统计**（PDF p14-15 关卡7）：高级别 \(H_t=(\ell_H,\delta_H,Cand_H,\sigma_H)\) 提供方向与容器，低级别 \(g_e=(e,\delta,I_\gamma,Conf_e)\) 提供执行与样本，完整状态
  \[
  Z_{HL} = (higher\_dir, higher\_level, lower\_level, lower\_bsp\_class, parent\_dir, ForceState, ExitType, \dots)
  \]
  然后 \(\mu(Z_{HL})=\mathbb{E}[X\mid Z_{HL}]\)。**不要再独立估** \(L4, L5\) 这种必然欠功效的桶（高级别一类信号 L1-L4 合计约 31 个 / 8.8 年，功效远远不足）。
- **验收**（PDF p19）：
  \[
  \boxed{\text{VALIDATED / FALSIFIED / INCONCLUSIVE}}
  \]
  三态输出，**且无离线补主裁决**。

### M5：声部执行账本接入

- **必须包含**（PDF p16 / p19-20）：\(P^{sep} \to N=Net(P^{sep}) \to Order\)；多空双开和短差必须真正影响订单；声部账本保留 \(entry_v, exit_v, parent(v), role(v), pnl_v\)。
- **订单**（PDF p16 关卡10）：\(P^{sep}_t = \sum_{v\in A_t} q_v\sigma_v e_v\)，\(N_t = Net(P^{sep}_t)\)，\(Order_t = N_t - N_{t-1}\)。否则「多重赋格 / 多空双开 / 短差」只是诊断，不是交易策略。
- **验收**（PDF p20）：
  \[
  \boxed{\Delta N = Net(P^{sep}_{t+1}) - Net(P^{sep}_t)}
  \]
  并逐声部归因 \(pnl_v\)。

### M6：保证金与强平接入

- **必须包含**（PDF p16 / p20）：margin；funding；borrow；liquidation；ADL 或等价强平；RiskExit 优先级。
- **验收**（PDF p16 关卡11 / p20）：
  \[
  \boxed{R = \sum_t N_t \Delta P_t - Commission_t - Slippage_t - Funding_t - Borrow_t - LiquidationLoss_t}
  \]
  （p20 简式：\(R = N\Delta P - C - Funding - Borrow - LiquidationLoss\)。）

### M7：三阶段资金接入

- **必须包含**（PDF p16-17 / p20）：R/TW 双账本；RecoverCapital；Withdraw；EnterEarning；BuyCore；\(\eta_* = L^{wc} + \kappa Q\)；\(\kappa\) 风险政策冻结；openLegacyLegs 守卫。
- **三阶段状态**（PDF p16）：\(TStage \in \{CostReduction, CapitalRecovered, EarningShares\}\)，负成本缓冲 \(\eta_t\)，风险 barrier \(\eta_*(x_t)=L^{wc}_{t+1}+\kappa Q_t\)（\(\kappa\) 价格不可导出）。
- **触发**（PDF p17）：
  \[
  EnterReady_t = S_t=II \wedge W_t \ge I_0 \wedge openLegacyLegs_t=0 \wedge RiskNormal_t \wedge \eta_t \ge L^{wc}_{t+1}+\kappa Q_t
  \]
  **不能只看** \(W_t \ge I_0\)。
- **验收**（PDF p17 / p20）：
  \[
  \boxed{Reach(Stage=III) > 0,\qquad Q_T > Q_0,\ W_T \ge I_0,\ \eta_T \ge L^{wc}+\kappa Q_T}
  \]

### M8：端到端全策略 OOS

- **必须包含**（PDF p17 / p20-21）：跑 \(\Pi_{\max\text{-}full}\)，输出四层报告：(1) signal alpha；(2) execution alpha；(3) treasury effect；(4) total PnL / drawdown / risk。四层验收分别为 \(LCB_{OOS}(\mu)>0 \wedge LCB_{OOS}(\mu_R)>0\)（信号层）、\(\mathbb{E}[R(\Pi_{exec})]>0\)（执行层）、\(Reach(StageIII)>0 \wedge Q_T>Q_0 \wedge W_T\ge I_0 \wedge \eta_T\ge\eta_*\)（资金层）、\(R(\Pi_{\max\text{-}full})>0 \wedge LCB_{OOS}(R)>0\)（完整策略层）。
- **最终判据**（PDF p21 / p17）：
  \[
  \boxed{LCB_{OOS}(R_{\Pi_{\max\text{-}full}}) > 0}
  \]
  才叫完整策略 confirmed alpha。若只有 \(R>0\) 但 \(LCB\) 不过 → **INCONCLUSIVE**，不能宣称 alpha。

---

## 3. 当前位置标定：M3/M4 交界（PDF §15 / p21）

PDF §15 原文：现在可以开跑的是 \(M1 \to M4\)（完整信号层收口）；但若要最终完整实装，须按 \(M0 \to M8\) 走完，不能在 M4 就停。**当前大概在 M3/M4 交界处**，还没进入 M5 声部执行 / M6 保证金 / M7 三阶段资金 / M8 完整 OOS。严格答案：「能搞，但不是立即跑总回测；先按 M0-M8 顺序把对象闭合。」

### 3.1 逐项完成状态（✅ 已完成 / ⏳ 进行中 / ⬜ 未进入）

| 关 | 子项 | 状态 | 锚点 / goal 工位 |
|---|---|---|---|
| **M1** 结构塔 bit-exact | 长历史塔 parity = 0 diff | ✅ | `T^inc = T^full`，type1>0 全历史 |
| | 中枢延伸 / 走势分解 / 局部趋势门 parity | ✅ | 全绿 |
| | typed A/C、PanDiv、frontier 重算 | ✅ | |
| **M2** 区间套 + Cand | \(N^\delta\) 真递归、\(J_{child}\subseteq J_{parent}\) | ✅ | `theta_v0/classifier/nest.rs`、`descend.rs`，max_depth=3 |
| | LiveCand / SettledCand 口径 | ✅ | |
| | *真实分布：95% 退化 base-case（\(depth=0\)）* | 观测 | 记忆 `project_interval_nesting_not_called_in_backtest`——真实几何结果非未调用 |
| **M3** 分类状态 | 六类买卖点、side-free bsp_class | ✅ | `classifier/bsp.rs`、`signal.rs` |
| | parent_dir、ExitType | ✅ | `strategy/exit.rs` → `mu_estimator.rs` 桶 |
| | δ-free 主裁决（主程序直落逐笔序列） | ⏳ | acc-route-a-closure(iii)：废除「含δ报告桶→离线合并」补丁路径 |
| | ForceState 真入主裁决桶（默认使用） | ⏳ | acc-route-a-closure(ii)：`divergence.rs:353 ForceStateA5` 原语已装，主桶默认接入进行中 |
| | \(d\) 或 \(\mu_R\) 字段 | ⏳ | ws-prereg2（a1+a6）\(q(z)\propto 1/d(z)\)、\(\mu_R\) 与裸 \(\mu\) 并列 prereg |
| | higher_dir / lower_exec 统计字段 | ✅ | 高低配（acc-highlow-power 已 CHECK_PASS） |
| **M4** 统计准入 | δ-free 在线（主裁决路径 δ-free） | ✅ | goal 当前标定 |
| | 高级别定方向低级别统计（高低配 \(Z_{HL}\)） | ✅ | acc-highlow-power |
| | \(X\) 与 \(X/d\) 双收益 μ_R prereg | ⏳ | ws-prereg2 进行中（a1+a6） |
| | OOS / LCB / block-bootstrap / δ-free permutation | ⬜ | M1→M4 收口后开跑 signal-full 首轮 OOS 前置 |
| **M5** 声部执行账本 | \(P^{sep}\to N\to Order\)、逐声部 pnl_v | ⬜ | 未进入（仅诊断层：`overlay_net_delta`、`strategy/mutex.rs`/`coverage.rs` 代数骨架） |
| **M6** 保证金强平 | margin/funding/borrow/liquidation/ADL | ⬜ | 未进入（未实装） |
| **M7** 三阶段资金 | RecoverCapital/EnterEarning/BuyCore、\(\eta_*\) | ⬜ | 未进入（L1 可达已证，L2 真实数据 witness + κ 政策待裁；`closed_loop/transition.rs`、`strategy/ledger.rs`） |
| **M8** 端到端 OOS | 四层报告、\(LCB_{OOS}(R)>0\) | ⬜ | 未进入 |

**goal 现状**（本文件产出时）：a2/a3/a4/a5 已 CHECK_PASS，a1+a6 进行中（ws-prereg2）。

### 3.2 M1→M4 剩余闭合项（PDF §15 列举的前置）

M1-M4 里尚未闭合、构成 signal-full 首轮 OOS 前置的四项（PDF p7 boxed）：

1. δ-free 在线主裁决（acc-route-a-closure(iii)——主程序直落逐笔序列，废除离线合并）；
2. ForceState / ThetaScoreBin 真进主桶（acc-route-a-closure(ii)——ResidualTrade 完成 β_bin 装配并默认桶键含 ForceState）；
3. \(\mu\) vs \(\mu_R\) 的 prereg 裁决（ws-prereg2，双主判据 co-primary）；
4. 高级别定方向 + 低级别统计的 prereg（已由高低配 acc-highlow-power CHECK_PASS 承接）；

外加 **AncOK ceiling** 本轮 waiver（见 §4，不再悬着）。

---

## 4. AncOK ceiling waiver 声明（PDF p4 / p15 原文措辞）

PDF p4 原文（若路线只做 signal-full）：

\[
\boxed{AncOK\ ceiling = WAIVED\ FOR\ SIGNAL\ ALPHA}
\]

**必须写明**（PDF p4 boxed）：

> **它会阻断 exec-full，不阻断 signal-full。**

PDF p15（关卡9 路线图裁决）等价标注：\(AncOK\ ceiling = not\ blocking\ signal\ alpha\)；但**完整声部执行必须闭合**——这一步在 exec-full（M5）**不能 waiver**，只能排在执行层之前完成。

### 4.1 数据依据（`.chanlun/review-results/ancok-a5-20260704.md`，L2 验证）

- **认识论等级 L2**：BTC 全历史 4,613,599 bar 单标的假设检验。
- **暴露面 = 0**：`restore_ancestor_chain_from_registry` 的 42,823 次调用中 `restore_break_registry_lost = 0`，祖先链恢复成功率 = 1.000000。声部树严格性在 BTC 数据上**从未被触发违反**。
- 697 号 ceiling 由「生成态」降级为「**已验证休眠缺口（BTC/L2，暴露面=0）**」——机制在 BTC 上无暴露，休眠但保留可见性（探针常驻）。

### 4.2 触发翻转条件（有效域边界，`formalization-validity-domain` 规则）

waiver 有效域 = **BTC / L2**，**不得膨胀到全标的**。以下任一成立则 waiver 翻转为「实装严格修复」（路径1）：

1. **跨标的（L3）**：CL/ES/金油等其他标的若在 LiveDetached 分布或 **registry 作废时序**上与 BTC 不同，`restore_break_registry_lost` 可能转正 → 翻转。
2. **数据窗口扩展**：BTC 追加历史后出现当前 4.6M bar 未覆盖的 registry 作废模式 → 须复测。
3. **彻底修复实装后**：增量 extract_elements / confirmed prefix immutable 被实装，overlay 语义改变 → 本 L2 结果作废（但届时 ceiling 本身消失）。
4. **探针被移除**：coverage.rs 探针（`ancok_probe_*`）被删 → 休眠缺口失去可观测性，697 回退为不可验证 ceiling。

---

## 5. 措辞纪律（全项目强制，PDF M0 / §5 转写）

1. **禁用词**：「终局」——执行层（M5-M6）/资金层（M7）/风险层未闭合前**不存在**合法使用场景。禁止用「终局」描述当前已测对象。
2. **禁用句式**：「完整缠论已回测」「完整缠论已实装并否证」「完整缠论无 alpha」「完整缠论终局 alpha 检验」。
3. **signal-full 结果不得当成 max-full 结论**：当前测的是 \(\Pi_{signal\text{-}full}\)（M1-M4），其任何结论**不得**外推为 \(\Pi_{\max\text{-}full}\)（PDF §2 严格证明：\(R(\Pi_t)\le 0 \not\Rightarrow R(\Pi_{\max})\le 0\)）。
4. **合法表述**（PDF §1 钦定）：
   > 当前已修复信号层没有 confirmed direction alpha；完整执行层和资金层尚未闭合。
5. **报告命名口径**：「信号层修复后第一轮 OOS 报告」「五步结构修复后的 alpha 初检」——**不用**「终局报告」。
6. **INCONCLUSIVE ≠ 无 alpha**：\(\forall z\in Z_{tested}, LCB_{OOS}(\mu(z))\le 0\)（not validated）与 \(\forall z\in Z_{full}, \mu(z)\le 0\)（no alpha exists）是两个命题，当前资料只支持前者。高级别桶 \(n_{eff}\ll n_{min}\)（功效门 \(n\ge 271\sim 1083\)）时只准 INCONCLUSIVE。

---

## 6. 与 693 号 TARGET_STRATEGY.md 三分冻结表的关系

693 三分冻结表将策略对象分为三个互不混淆的检验对象；本文件的 M0-M8 是这三个对象的**执行序整合**——把「三条平行路线」收敛为「一条主线」（PDF §14：「不再拆 A/B/C」）：

| M 段 | 693 对应对象 | 检验的量 | 合格判据 |
|---|---|---|---|
| **M1-M4** | \(\Pi_{signal\text{-}full}\)（693 §1.1，本轮 Π_target） | 方向层 \(\mu(z)\)、\(\mu_R(z)=\mathbb{E}[X/d\mid z]\) | 三态 VALIDATED/FALSIFIED/INCONCLUSIVE + 双主判据 |
| **M5-M6** | \(\Pi_{exec\text{-}full}\)（693 §1.2，A11 #1 / A10 #2 MUST） | 净值 \(R=\sum_t N_t\Delta P_t - C_t - Funding_t - \dots\) | \(\mathbb{E}[R(\Pi_{exec})]>0\) |
| **M7** | \(\Pi_{treasury\text{-}full}\)（693 §1.3，GAP3） | 可达性 \(Reach(\cdot)\) 与终态不等式 | \(Reach(StageIII)>0\)，\(Q_T>Q_0, W_T\ge I_0, \eta_T\ge\eta_*\) |
| **M8** | \(\Pi_{\max\text{-}full}\) 端到端（三对象合成） | 四层：signal / execution / treasury / total | \(LCB_{OOS}(R_{\Pi_{\max\text{-}full}})>0\) |

**边界互斥继承 693 §1.4**：任一对象（任一 M 段）的负结论**不得外推**到另一对象（另一 M 段）（严格证明见 `docs/formal-chain/有效域定理-20260704.md` 定理1）。693 §3.1 的 A11/A10 硬裁决在本主线中的体现：A11 对 M1-M4（signal-full）= OUT_OF_SCOPE，对 M5（exec-full）= MUST；A10 对 M1-M4 = OUT_OF_SCOPE（无杠杆现金结算，定义外），对 M6（exec/treasury-full 带杠杆）= MUST。

---

## 结果包（简化版，PDF §纯文档转写 / audit_exempt 行动类）

- **结论**：产出 `TARGET_STRATEGY_MAXFULL.md`——忠实转写 PDF 路线图的 M0 十四元组目标定义、最终订单策略公式、M0-M8 逐关目标/验收判据、当前位置 M3/M4 交界逐项标定、AncOK ceiling signal-alpha waiver 声明（含 L2 数据依据与四项翻转条件）、措辞纪律、与 693 三分表的执行序整合关系。
- **边界条件**（结论翻转）：
  1. 若 PDF `路线.pdf` 被上游修订（M 段拆分、判据改写），本文件须同步重写；
  2. AncOK waiver 的四项翻转条件（§4.2）任一触发 → waiver 段作废，退回严格修复；
  3. 当前位置标定（§3.1）随 goal 推进变化——ws-prereg2 完成后 M3/M4 的 ⏳ 项转 ✅，标定须刷新；
  4. 十四元组 vs 十三元组（§1 元数说明）：若上游明确 \(N=Net(P^{sep})\) 不单列，元数退化为 13，不改判据。
- **影响声明**：
  - 新增文件 `TARGET_STRATEGY_MAXFULL.md`（仓库根，与 `TARGET_STRATEGY.md` 并列），无代码改动，无既有文档改动。
  - 本文件是 M0 关的产出物本身（PDF M0：「输出 TARGET_STRATEGY_MAXFULL.md」），M0 关的验收（禁用「终局」描述已测对象）由 §5 措辞纪律承载。
  - 未 commit（按任务约束，供 Lead 联合真封）。
  - 引用但未修改：`docs/formal-chain/路线.pdf`（权威源）、`TARGET_STRATEGY.md`（693）、`.chanlun/review-results/ancok-a5-20260704.md`（waiver 数据依据）、`docs/formal-chain/有效域定理-20260704.md`（边界互斥证明）。
