# TARGET_STRATEGY.md — 策略对象冻结表

> **权威来源**：`docs/formal-chain/问题1.pdf`（16页《推导完全分类》，编排者 2026-07-04 钦定本轮最高权威）。
> **goal**：`g-20260704T1756Z-strategy-object-freeze-cbc36d`，base_head = `cbc36d070e`。
> **本文件的效力**：没有本文件时，所有 alpha 结论都不应被接受（PDF §5 第1条）。任何回测报告必须首先声明其被检验对象是本文件三对象中的哪一个，否则该报告的有效域视为未定义。

---

## 1. 三个互不混淆的策略对象（PDF §1 三分）

策略对象统一记为五元组 \(\Pi = (\Gamma, Z, E, M, K)\)：

- \(\Gamma\)：证书集合（买卖点/背驰/区间套等结构证书）
- \(Z\)：状态分类（分桶键、estimand 所在的状态空间）
- \(E\)：出场规则（typed exit）
- \(M\)：资金管理（仓位、声部账本、三阶段资金）
- \(K\)：风险可行集（保证金、强平、资金费约束下的可行域）

### 1.1 Π_signal-full — 信号层完整检验对象

**检验问题**：「完整信号层缠论（买卖点 + 区间套 + 非 MACD 唯一背驰力度 + 正规出场 + 风险归一化状态）在方向层是否有 alpha？」

- \(\Gamma\)：全部六类买卖点证书 + 区间套下沉证书 + PanDiv/XZD 承接证书 + ForceState 力度证书。
- \(Z\)：主裁决桶键 \(Z_{decision} = (\ell, \text{bsp\_class}, \text{parent\_dir}, \text{ForceState}, d\text{-bin})\)，δ-free、逐笔直接输出。
- \(E\)：typed exit（正规出场），ExitType 逐笔携带。
- \(M\)：**仅**风险归一化仓位函数 \(q(z) \propto 1/d(z)\)；无声部执行账本、无三阶段资金。
- \(K\)：无杠杆现金结算（cash-settled no-margin netting）；不含保证金/强平/资金费/ADL。

### 1.2 Π_exec-full — 真实声部执行层检验对象

**检验问题**：「多重赋格 / 多空双开 / 短差的完整声部执行策略，在真实开平仓账本下是否有净值 alpha？」

- 在 Π_signal-full 全部构件之上，**必须**追加：声部树真实驱动订单 \(P_t^{sep} = \sum_{v \in A_t} q_v \sigma_v e_v\)、净头寸 \(N_t = Net(P_t^{sep})\)、声部级 ledger \((G_v, entry_v, exit_v, parent(v), stage(v))\)。
- \(K\)：含杠杆时必须含保证金/强平/资金费（A10 联动为 MUST）。

### 1.3 Π_treasury-full — 三阶段资金管理检验对象

**检验问题**：「三阶段负成本挣股数（GAP3）是否可达？」即 \(Reach(Stage=II) > 0\)、\(Reach(Stage=III) > 0\)、终态 \(Q_T > Q_0,\ W_T \ge I_0,\ \eta_T \ge L_{wc} + \kappa Q_T\)。

- 独立于普通 alpha 检验（PDF 路线C：「GAP3 必须单独验，不要混进普通 alpha」）。
- \(M\)：三阶段资金状态机 + EnterEarning 独立触发 + \(cost\_basis=0\) 与 \(openLegacyLegs\) 分离口径。
- \(K\)：\(free_t \ge 0\)、\(RiskPolicy(\kappa) \ge 0\) 在 release 模式成立。

### 1.4 边界互斥声明

三对象的检验问题、estimand 与合格判据**两两不同、互不蕴含**：

| | 检验的量 | 合格判据 | 不回答的问题 |
|---|---|---|---|
| Π_signal-full | 方向层 \(\mu(z)\)、\(\mu_R(z)=E[X/d\,|\,z]\) | 三态 VALIDATED/FALSIFIED/INCONCLUSIVE + 功效门 | 执行可行性、净值、资金可达性 |
| Π_exec-full | 净值 \(R = \sum_t N_t \Delta P_t - C_t - Funding_t - \dots\) | 净值 alpha 三态 | 信号层逐桶方向、资金三阶段 |
| Π_treasury-full | 可达性 \(Reach(\cdot)\) 与终态不等式 | 可达性证人（witness）存在 | 方向 alpha、净值 alpha |

**任一对象的负结论不得外推到另一对象**（严格证明见 `docs/formal-chain/有效域定理-20260704.md` 定理1）。

---

## 2. 本轮被检验对象声明

\[
\boxed{\Pi_{target} = \Pi_{signal\text{-}full}}\quad\text{（PDF §4 路线A）}
\]

本轮必须闭合（路线A 清单）：\(P1 + P2 + P4 + P7 + d + ForceState + \delta\text{-free 逐笔主裁决}\)，外加高级别定方向+低级别执行统计（PDF §5 第8条）。

本轮**不**检验：声部执行层（A11）、保证金/强平/资金费（A10）、三阶段资金（GAP3）。这不是遗漏，是有效域裁剪，见 §3 硬裁决。

---

## 3. 模块冻结表（PDF §2 表格为准，逐项三态硬裁决）

三态语义：
- **MUST**：该模块属于对应策略对象的定义构件，缺失/未真接入则该对象**未实装**，禁止以该对象名义下 alpha 结论。
- **WAIVED**：显式豁免——模块属于对象定义但本轮以书面理由豁免，结论须标注「豁免下有效」。
- **OUT_OF_SCOPE**：不属于该对象的定义构件，其缺失不削弱该对象结论的有效域，但结论**禁止**冒用含该模块的更大对象之名。

| 模块 | 对 Π_signal-full（本轮约束列） | 对 Π_exec-full | 对 Π_treasury-full | 当前处理 / 代码锚点 |
|---|---|---|---|---|
| P1 区间套 | **MUST** | MUST（承接） | OUT_OF_SCOPE | 已实装（`rust/src/theta_v0/classifier/nest.rs`、`descend.rs`；有锚 750，max_depth=3）；本轮继续验深度与触达 |
| P2 非 MACD 唯一背驰（ForceState） | **MUST** | MUST（承接） | OUT_OF_SCOPE | 原语已装（`divergence.rs:353 ForceStateA5`、`ThetaScoreBin`）；**分层键真接入主裁决桶 = 本轮 acc-route-a-closure(ii)** |
| P4 六类买卖点类型 | **MUST** | MUST（承接） | OUT_OF_SCOPE | 已接（`classifier/bsp.rs`、`signal.rs`；type1 全级别>0 已修复）；统计口径经 δ-free 逐笔收口 |
| P7 正规出场（typed exit） | **MUST** | MUST（承接） | OUT_OF_SCOPE | 已接（`strategy/exit.rs`，ExitType 进 `mu_estimator.rs` 桶）；持续对拍 |
| A11 声部执行层（真实开平仓） | **OUT_OF_SCOPE**（硬裁决，见 §3.1） | **MUST**（PDF §7 优先级 #1） | OUT_OF_SCOPE | 仅诊断层完成（`overlay_net_delta`、`delta_n_l1`、`strategy/mutex.rs`/`coverage.rs` separate 代数骨架）；未接真实订单 |
| A10 保证金/强平/资金费/ADL | **OUT_OF_SCOPE**（硬裁决，见 §3.1；旧 WAIVED 状态废止） | **MUST**（PDF §7 优先级 #2） | **MUST**（若带杠杆） | 未实装；Π_signal-full 定义为无杠杆现金结算，故不需要 |
| GAP3 三阶段资金 | OUT_OF_SCOPE | OUT_OF_SCOPE | **MUST** | EnterEarning 可达性待证（`closed_loop/transition.rs`、`strategy/ledger.rs`）；路线C 单独验 |
| 结构止损距离 d | **MUST** | MUST（承接） | OUT_OF_SCOPE | **当前缺失 z 维——本轮 acc-route-a-closure(i)**：d 进状态与仓位 \(q(z)\propto 1/d(z)\)，\(\mu_R\) 与裸 \(\mu\) 并列 |
| AncOK parent 稳定性 | **MUST** | MUST（承接） | OUT_OF_SCOPE | stale 分支存在伪造 parent=None 边界（`strategy/persistent.rs` overlay 只是缓解）——**本轮 acc-route-a-closure(iv) 三选一收口** |
| δ-free 主裁决口径逐笔输出 | **MUST** | MUST（承接） | OUT_OF_SCOPE | 当前为「含δ报告桶→离线合并」补丁路径——**本轮 acc-route-a-closure(iii) 废除，主程序直接落逐笔序列** |

冻结表检验：10 个模块 × 3 对象 = 30 格，**每格均为三态之一，无模糊态、无空格**。

### 3.1 A11 / A10 硬裁决（二选一，禁模糊）

PDF §5 第2/3条要求 A11、A10 各自处于 \(\{MUST, OUT\_OF\_SCOPE\}\) 之一，不得处于「诊断层完成但完整策略又暗含它」的模糊状态。本轮裁决：

- **A11**：对 \(\Pi_{signal\text{-}full}\)（= 本轮 Π_target）→ **OUT_OF_SCOPE**；对 \(\Pi_{exec\text{-}full}\)、\(\Pi_{treasury\text{-}full}\) → **MUST**，按 PDF §7 优先级 **#1** 排入后续 goal。
- **A10**：对 \(\Pi_{signal\text{-}full}\) → **OUT_OF_SCOPE**（对象定义为无杠杆现金结算，非豁免而是定义外；旧「waiver」表述废止）；对 \(\Pi_{exec\text{-}full}\)、\(\Pi_{treasury\text{-}full}\)（带杠杆/多空双开/期货/加密环境）→ **MUST**，按 PDF §7 优先级 **#2** 排入后续 goal。

**推论（措辞约束）**：在 A11=OUT_OF_SCOPE 下，任何本轮产出**不得**声称「多重赋格/多空双开策略已回测」，只能说「多声部结构已诊断，未接真实执行」；在 A10=OUT_OF_SCOPE 下，**不得**称「杠杆版完整策略」，本轮对象名为「无杠杆净额版信号层缠论」。

**escalate 登记**：本 §3.1 整体裁决属概念层对象边界裁定，已登记为待编排者 `/ritual` 追认的 escalate 点（谱系 `.chanlun/genealogy/pending/693-strategy-object-triple-freeze-a11-a10-hard-ruling.md`）。追认前本表即为工作口径（本轮标注无模糊态）；若编排者翻转，则按 /ritual 基底刷新流程退回生成态重裁。

---

## 4. 措辞纪律（全项目强制）

1. **禁用词**：「终局」（执行层/资金层/风险层未闭合前不存在合法使用场景）。
2. **禁用句式**：「完整缠论已回测」「完整缠论已实装并否证」「完整缠论无 alpha」。
3. **合法表述**（PDF §1 钦定）：
   > 当前已修复信号层没有 confirmed direction alpha；完整执行层和资金层尚未闭合。
4. 报告命名口径：「信号层修复后第一轮 OOS 报告」「五步结构修复后的 alpha 初检」。
5. INCONCLUSIVE **不是**「无 alpha」：\(\forall z \in Z_{tested},\ LCB_{OOS}(\mu(z)) \le 0\)（not validated）与 \(\forall z \in Z_{full},\ \mu(z) \le 0\)（no alpha exists）是两个命题，当前资料只支持前者。
6. 高级别桶 \(n_{eff} \ll n_{min}\)（功效门 \(n \ge 271 \sim 1083\)）时只准 INCONCLUSIVE，禁二值结论。

---

## 5. 后续 goal 排序（PDF §7，按「是否改变策略对象」）

1. A11 声部执行层接真实开平仓（Π_exec-full 的 MUST #1）
2. A10 保证金/强平/资金费（Π_exec-full / Π_treasury-full 的 MUST #2）
3. ~~结构止损距离 d~~（本轮 acc-route-a-closure(i) 承接）
4. ~~ForceState 分层键主接入~~（本轮 (ii) 承接）
5. ~~AncOK parent 稳定性~~（本轮 (iv) 承接）
6. ~~逐笔 δ-free 主裁决~~（本轮 (iii) 承接）
7. ~~高级别方向 + 低级别执行统计~~（本轮 acc-highlow-power 承接）
8. 概念 pending 清理（settle-sweep 约 13 条概念裁定，尤其中枢延伸 vs 高级别升级边界、cand_channel vs i_class 轴正交性——后者已由谱系 692 号裁定）
