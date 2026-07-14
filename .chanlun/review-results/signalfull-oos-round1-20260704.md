# signal-full 首轮 OOS 报告（Π_signal-full，M1-M4 信号层）

- **工位**：swarm/ws-b4report | **Task #11 联合合成**（b1 裁定甲 + b2 M3 断言 + b3 三态全跑 + a1/a6 prereg-rev2 + M0 框架）
- **合成性质**：纯文档合成，零新数据、零代码改动。下列数字全部转引自源结果包（sfoos-b3 / m3lock-b2 / codex-ruling-exittype / prereg-rev2-results / final-alpha），**不改一位**。
- **跑批 HEAD**：`bd01afc1b8`（a1-a6 收割后冻结口径，prereg-rev2 冻结基 `26ebe90b29` 之上）。
- **认识论等级**（231/formalization-validity-domain 标注）：主裁决 / μ_R / 高低配 = **L2**（BTC 单标的 461 万 bar walk-forward OOS，5 窗 test_start≥OOS_START，residuals=2256）；交叉验证 = **L3**（7 品种池化 σ̂-归一化，pooled_residuals=9030）。

---

## 0. 有效域声明（措辞纪律，TARGET_STRATEGY_MAXFULL.md §5 强制）

**本报告检验对象 = Π_signal-full（M1-M4 信号层），不是 Π_max-full（完整策略对象）。**

- **Π_signal-full ≠ Π_max-full**：本报告只覆盖 M1（结构塔）→M4（统计准入）四关。执行层（M5 声部执行 P^sep→N→Order）、保证金层（M6 margin/funding/liquidation）、资金层（M7 三阶段 TW 账本）、端到端（M8 四层报告）**均未闭合**——见 §4 后继序列标定。
- **禁用「终局」**（TARGET_STRATEGY_MAXFULL.md §5.1 / PDF M0 原文）：本报告标题、结论、任何段落**不使用**「终局」描述本对象。M5-M8 未闭合前「终局」无合法使用场景。
- **不外推**（PDF §2 严格证明 R(Π_t)≤0 ⇏ R(Π_max)≤0）：signal-full 的任何结论**不得**当成 max-full 结论。合法表述（PDF §1 钦定）：**「当前已修复信号层没有 confirmed direction alpha；完整执行层和资金层尚未闭合。」**
- **INCONCLUSIVE ≠ 无 alpha**（§5.6）：本报告的 INCONCLUSIVE 命题是 ∀z∈Z_tested, LCB_OOS(μ(z))≤0（not validated），**不是** ∀z∈Z_full, μ(z)≤0（no alpha exists）。高级别桶 n_eff≪n_min（功效门 n≥271~1083）时只准 INCONCLUSIVE。

---

## 1. 一行结论

**Π_signal-full 首轮 OOS = 无 confirmed 方向 alpha（INCONCLUSIVE 不翻）。** 无任一桶双门（raw μ ∧ μ_R）同时 VALIDATED；唯一 raw μ Validated 桶经四重 beta 确认链坐实为 beta 漂移伪结构，非可交易 alpha；L3 跨品种池化 V=0；高低配停机条款未触发（非 suspect Validated=无）。fail 条件未触发，不上浮。

---

## 2. 信号层验收节（Π_max-full 四层报告 M8 第一层的 signal-full 前置，路线.pdf §13.1 / M4 关卡6-7）

### 2.1 双门判据：LCB_OOS(μ) 与 LCB_OOS(μ_R) 双主判据

M4 关卡6 铁律（PDF p14）：双主判据 **LCB(μ)>0 且 LCB(μ_R)>0** 同时成立才算方向层 confirmed。四态解释：(+,+) 强信号 / (+,−/0) 只在大波动上赚钱、风险调整失败 / (−/0,+) 小风险高质量信号 / (−/0,−/0) 无证据。

| estimand | 桶数 | verdict | V/F/I | 唯一非-Inconclusive 桶 |
|---|---|---|---|---|
| **raw μ = E[X\|z]** | 25 | Pass | **1/0/24** | Validated：L0 bsp3 σ+1 force=None（n=248 n_eff=248.0 mean **+162.50** LCB **+89.90** perm_p **0.000**） |
| **μ_R = E[X/d\|z]** | 25 | Inconclusive | **0/1/24** | Falsified：L0 bsp2 σ−1 None（n=346 mean −1.749 UCB −0.133≤0 powered） |

**双门 acceptance（Holm 全族校正，假设族 = {raw μ, μ_R} × 25 桶 × {β 路径}）= INCONCLUSIVE。**

- **无双门 VALIDATED**：raw μ 的唯一 Validated（L0 bsp3 σ+1）在 μ_R 下退为 `mean +0.296 LCB −0.162 perm_p 0.085` = **Inconclusive**；μ_R 侧无任一 Validated 桶。校正前已无双门候选 ⟹ 校正后必无。
- μ_R 侧唯一 LCB>0 桶 `L2 bsp2 σ−1 None`（mean +0.775 LCB +0.016 n_eff=23 perm_p 0.800）——perm_p 不显著 ⟹ Inconclusive，非 Validated。

### 2.2 唯一 raw Validated 桶（L0 bsp3 σ+1）= beta 漂移伪结构（四重确认链）

唯一 raw μ Validated 桶 **不是可交易方向 alpha**，四条独立证据链一致坐实其为 BTC secular bull beta 混入：

| # | 确认轴 | 证据（转引） | 判定 |
|---|---|---|---|
| **1** | **δ 同号不抵消** | 同桶 δ+1（mean +162.25 n156 LCB+81.18 perm_p 0.000）与 δ−1（mean +162.91 n92 LCB+23.01 perm_p 0.000）**两方向同为正 +162**；δ-free 池化 mean +162.50。若为方向性 alpha，买/卖池化必**异号抵消**；同号不抵消 = 纯 beta（memory `oddeven_mu_identity`，657/oddeven 签名）。 | beta，非方向 alpha |
| **2** | **σ^H 载体** | 正残差绑定在特定波动格 σ+1（level×持有窗 beta 相位），非买卖边际——高低配 D1 该桶 `L0 bsp3 δ+1 σ+1`（n=156 mean +162.25 LCB +81.18 perm_p 0.000）标 **BETA-DRIFT-SUSPECT**（反 δ 格同号，beta 漂移探针命中）。 | 结构格 beta 暴露 |
| **3** | **μ_R 归零** | 风险归一化（除 d）后该桶 `LCB +89.90→−0.162`、`perm_p 0.000→0.085` = Inconclusive——beta 漂移的「超额」被止损距离摊平，风险调整视角下不成立。 | 风险调整失败 |
| **4** | **L3 池化 Falsified** | 跨品种 σ̂-归一化池化后翻转为 Falsified：主桶 (0,3,+1,σ0) BTC 单标的 n=428 mean −0.6419 LCB −2.5905 perm_p 0.360 → 7 品种池化 n=1856 mean **−1.9552** LCB **−2.6650** perm_p **0.055** = **Falsified**（翻转=是，INCONCLUSIVE→FALSIFIED，**非** VALIDATED）。正 μ 是 **BTC 独有** beta（memory `l3_cross_symbol_btc_idiosyncratic`），非跨品种可复现 alpha。 | 跨品种证伪 |

**四重链一致结论**：signal-full 无任何 raw Validated 转化为可交易方向 alpha。

### 2.3 方向不对称 co-primary（H2：β=μ_sell−μ_buy，block bootstrap 单边）

| L | n_buy | n_sell | β | boot_p(block=50，prereg §7 冻结值) |
|---|---|---|---|---|
| L0 | 1026 | 996 | −11.97 | 0.633 |
| L1 | 87 | 65 | −515.46 | 0.997 |
| L2 | 22 | 28 | −162.30 | 1.000 |
| L3 | 19 | 1 | +345.13 | 0.000（n_sell=1 退化，不可采） |

co-primary β 路径 L0/L1/L2 **全不显著**（boot_p≥0.633）；L3 唯一 p=0.0000 但 n_sell=1 单笔结构退化，无功效。**方向不对称路径不 PASS**（final-alpha R2 已用 block=20/50 两块长验证一致成立，块长只影响重采样非观测统计量）。

### 2.4 前置合法性：ForceState⊥δ 检验 PASS

A1（ForceState 进 δ-free 主裁决聚合基）前置检验 **PASS**：每态 δ 两向非空（Dom− 6/11、Inc 10/11），ForceState 由 A/C 段绝对量算 mirror-invariant，置换 δ 恒定 ⟹ **非 i_class×δ 共线**（避开 memory `iclass_delta_collinearity_perm_degeneracy` 的自毁陷阱）。合法进主裁决聚合基。δ-free 主裁决从 22 桶升 25 桶（force_state 细分一类买卖点桶，与旧表 #135 不可逐桶比较）。

### 2.5 在线↔离线 bit-exact

Z_decision 逐笔 dump（2256 笔，force_state+d 两列 round-trip）与在线主裁决表**逐字节一致**（25 桶同 verdict）。自检 `deltafree_dump_roundtrip_and_pooling` 绿。

---

## 3. M4 逐项验收对照表（路线.pdf M4 关卡6-7，逐项打勾附证据）

M4 关卡6 必须包含：X 与 X/d 双收益 / OOS / LCB / block-bootstrap / δ-free permutation / 高级别定方向低级别统计。逐项验收：

| M4 子项 | 验收要求（PDF p14-15/p19） | 本轮证据 | 状态 |
|---|---|---|---|
| **X 与 X/d 双收益** | X_i=δ_i(P_τ−P_t)−C_i，X^R_i=X_i/d_i；双主判据 LCB(μ)>0 ∧ LCB(μ_R)>0 | raw μ 25 桶 + μ_R 25 桶双 estimand 表（§2.1）；μ_R_n=2184（诚实剔除 d 不可得 NAN=72，D_MIN=tick_size 无触发） | ✅ 双门跑通，acceptance=INCONCLUSIVE |
| **OOS** | walk-forward，test_start≥OOS_START | BTC 5 窗 walk-forward OOS，residuals=2256（2023-02-17…2025-06-30） | ✅ |
| **LCB** | LCB_OOS 单侧下界，z_α=1.645 | 双门每桶 LCB 列（§2.1 表）；主判据判定基 | ✅ |
| **block bootstrap** | block/bootstrap 方向不对称 | H2 β block=50（prereg §7 冻结）+ block=20 对照两块长一致（final-alpha R2 §2） | ✅ 不显著 |
| **δ-free permutation** | δ-free 置换检验（i_class×δ 共线免疫，657） | 25 桶 perm_p 列（分层键恒 (ℓ,h,time,σ^H) δ-free）；ForceState⊥δ PASS（§2.4） | ✅ |
| **高级别定方向低级别统计** | Z_HL=(higher_dir,higher_level,lower_level,lower_bsp_class,parent_dir,ForceState,ExitType,…)，μ(Z_HL)=E[X\|Z_HL] | 高低配 highlow_full（residuals 全级别=2256，L0=2022，高级别 234 仅作条件源）；D1 σ^H_tower 切片唯一 Validated 格标 BETA-DRIFT-SUSPECT；停机条款（非 suspect Validated=无）**未触发**；acc-highlow-power 已 CHECK_PASS | ✅ alerts=0，无隐藏条件 alpha |

**M4 三态输出验收**（PDF p19 `VALIDATED / FALSIFIED / INCONCLUSIVE`，无离线补主裁决）：三态照实产出，主裁决路径 δ-free 在线直落逐笔序列（无离线聚合补救）。**M4 结论 = INCONCLUSIVE**。

### 3.1 M3 分类状态运行时锁定（M4 聚合的前置不变量）

M4 双收益聚合建立在 M3 分区不变量上。M3 验收判据 `𝒳 = ⊔_z C_z, 运行时 Σ_z 1_{C_z}=1` 已从「离线可算」升为**生产路径运行时不变量**（m3lock-b2 Task #10）：两级守恒断言（records→buckets 互斥穷尽 + 值域封闭；ledger→records 穷尽账本）落在生产纯函数 `ledger_disposition`/`assert_m3_partition`，BTC 全窗 Σ|ledger|=2300 / Σkept=Σ|records|=2256 / 44 笔落显式排除类，**零违例**。下游 μ/μ_R 无「未分类残差泄漏」。

### 3.2 ExitType 处置（codex 裁定甲，M3 桶键铁律）

M3 δ-free 主裁决桶键 `Z_decision = (level, bsp_class, parent_dir, ForceState, …)` 中 **ExitType 不进裁决聚合基**（codex-ruling-exittype 裁定甲，L0 定理类）：ExitType = 逐笔账本字段 + 收益计价来源 + 事后诊断切片，**不进** accept/reject 主裁决聚合基，也不进 χ_t 门控可查询键。理由两轴——因果可测性（live 入场时刻 t exit_type 未发生，keyed-on-exit_type = post-treatment 泄漏）+ 样本量代数（n≈2209 typed-exit × exit_type 5 层 ⟹ ≈29.5/桶，只能作诊断）。`路线.pdf` 公式中 ExitType 读作路线图层诊断扩展维简写。本轮按现行冻结口径（ExitType 不在裁决基）跑，**与裁定一致 ⟹ 无需补跑裁决维**。翻转条件唯二：exit_type 替换为 ex-ante 代理 + 新 prereg，或 Z/χ_t 语义移到出场点 τ（须 escalate）。

---

## 4. M1-M4 全线状态总表 + M5-M8 后继序列标定

### 4.1 M1-M4 全线状态（signal-full 本对象）

| 关 | 子项 | 状态 | 锚点 / 本报告证据 |
|---|---|---|---|
| **M1** 结构塔 bit-exact | 长历史塔 parity=0 diff、type1>0 全历史 | ✅ | T^inc=T^full；final-alpha F1 census type1>0 全级别（type1=0 前提已修复）；中枢延伸/走势分解/局部趋势门 parity 全绿 |
| **M2** 区间套 + Cand | N^δ 真递归、J_child⊆J_parent、LiveCand/SettledCand | ✅ | max_depth=3；final-alpha F3 真实分布 90.91% 小转大 / 有锚 750（d=1:716, d=2:33, d=3:1），95%+ 退化 base-case 是真实几何非未调用（memory `interval_nesting`） |
| **M3** 分类状态 | 六类 side-free bsp_class、parent_dir、ForceState、ExitType、d/μ_R、higher_dir/lower_exec | ✅ | 运行时 Σ_z 1_{C_z}=1 锁定（§3.1 m3lock-b2 零违例）；ForceState 真入主桶（§2.4 PASS）；ExitType 账本/诊断不进桶键（§3.2 裁定甲）；d/μ_R 字段落地（§2.1）；XZD C3 死门维持（final-alpha F4，gate_pass 不含 C3） |
| **M4** 统计准入 | X 与 X/d 双收益、OOS、LCB、block-bootstrap、δ-free permutation、高级别定方向低级别统计 | ✅（判据全跑，结论 INCONCLUSIVE） | §3 逐项对照表六项全绿；三态输出 INCONCLUSIVE，无离线补主裁决 |

**M1-M4 收口结论**：信号层四关全部闭合并跑通 OOS 判据，**结论 = 无 confirmed 方向 alpha（INCONCLUSIVE）**。

### 4.2 M5-M8 后继序列现状标定（各自当前缺口一行）

| 关 | 对象（693 三分表） | 当前状态 | 缺口一行 |
|---|---|---|---|
| **M5** 声部执行账本 | Π_exec-full（P^sep→N=Net→Order，逐声部 pnl_v） | ⬜ 未进入 | 仅诊断层代数骨架（overlay_net_delta、strategy/mutex.rs、coverage.rs）；多空双开/短差**未真正影响订单**（ΔN=Net(P^sep_{t+1})−Net(P^sep_t) 未接生产开平仓）——AncOK ceiling 对 M5 **不能 waiver**（PDF p15） |
| **M6** 保证金强平 | Π_exec-full（带杠杆） | ⬜ 未进入 | margin/funding/borrow/liquidation/ADL/RiskExit 优先级**未实装**；R=∑N_tΔP_t−C−Funding−Borrow−LiquidationLoss 未闭合；A10 对 M6 = MUST |
| **M7** 三阶段资金 | Π_treasury-full（GAP3） | ⬜ 未进入 | RecoverCapital/Withdraw/EnterEarning/BuyCore、η_*=L^wc+κQ、κ 政策冻结、openLegacyLegs 守卫；L1 可达已证，L2 真实数据 witness + κ 政策待裁（closed_loop/transition.rs、strategy/ledger.rs）；Reach(StageIII)>0 未验 |
| **M8** 端到端 OOS | Π_max-full（三对象合成） | ⬜ 未进入 | 四层报告（signal/execution/treasury/total）未合成；最终判据 LCB_OOS(R_{Π_max-full})>0 未跑——**本报告只完成第一层（signal）的 signal-full 前置** |

**AncOK ceiling waiver**（TARGET_STRATEGY_MAXFULL.md §4）：signal-full 层 = WAIVED FOR SIGNAL ALPHA（BTC/L2，暴露面=0，restore_break_registry_lost=0）。**它阻断 exec-full（M5），不阻断 signal-full。** waiver 有效域限 BTC/L2，四项翻转条件（跨标的/数据窗口扩展/彻底修复/探针移除）任一触发则退回严格修复。

---

## 5. 结果包六要素

1. **结论**：Π_signal-full（M1-M4 信号层）首轮 OOS = **无 confirmed 方向 alpha（INCONCLUSIVE 不翻）**。无双门 VALIDATED；唯一 raw Validated 桶经四重链（δ 同号 / σ^H 载体 / μ_R 归零 / L3 池化 Falsified）坐实为 beta 漂移；co-primary β 路径不显著；高低配停机未触发；L3 池化 V=0。有效域限 Π_signal-full，不外推 Π_max-full（M5-M8 未闭合，§4.2）。

2. **定义依据**：
   - 可交易方向 alpha 判据 = 双主判据 LCB_OOS(μ)>0 ∧ LCB_OOS(μ_R)>0（PDF M4 关卡6，663/§12）。
   - 主裁决桶键 = δ-free `(level, bsp_class, parent_dir, ForceState)`（prereg-rev2 §1；ExitType 不进，codex 裁定甲 §3.2）——含 δ 或 ExitType 会自毁置换检验 / post-treatment 泄漏（657/§3.1）。
   - μ_R = E[X/d\|z]（codex-ruling-696 选项 B 残差框架，d=|entry−structural_stop| ex-ante，D_MIN=tick_size 防护，risk.rs:74-105）。
   - 三态判据 = final-prereg §3.2（μ̂>0 ∧ powered n_eff≥(1.645·CV)² ∧ LCB>0 ∧ perm_p<0.05）。
   - beta 漂移判据 = 两 δ 方向同号残差（memory `oddeven_mu_identity`，若方向 alpha 则买/卖必异号抵消）。
   - L3 池化 σ̂-归一化 = pre-OOS 逐 bar $ 涨跌样本 std（因果无前视，prereg-l3norm §1）。
   - **输入特征满足定义的条件**：BTC 5 窗 OOS residuals=2256 逐笔进 δ-free κ 桶键（M3 运行时 Σ_z 1_{C_z}=1 锁定，§3.1）；ForceState⊥δ 每态 δ 两向非空实证（§2.4）满足「合法进主裁决聚合基」；唯一 raw Validated 桶 δ+1/δ−1 两向同为 +162（满足 beta 漂移定义的「同号不抵消」条件）。

3. **边界条件（结论翻转）**：
   - (a) 若某桶双门（raw μ ∧ μ_R）同时 VALIDATED 且 powered ⟹ 翻转 signal-full 结论、上浮（本轮无）。
   - (b) 若 L3 池化任一桶 VALIDATED ⟹ 跨品种 alpha 上浮（本轮 V=0）。
   - (c) 若高低配非 BETA-DRIFT-SUSPECT 的 Validated 格出现 ⟹ 条件 alpha 上浮（本轮 alerts=0）。
   - (d) 若 ForceState⊥δ FAIL（任一态 δ 单向）⟹ A1 退出主裁决基（本轮 PASS）。
   - (e) 若 ExitType 语义翻转（ex-ante 代理 + 新 prereg，或 Z/χ_t 移到出场点 τ）⟹ 补跑该维（裁定甲：现行不进裁决基，无需补跑）。
   - (f) 样本量升数量级（跨标的池化 + 657 归一化新 prereg）⟹ 提功效链可再检。

4. **下游推论**：
   - signal-full 首轮 OOS 确认 M1 里程碑维持「无 confirmed 正 alpha」——force_state 第 8 维细分未暴露隐藏 alpha，风险归一化抹掉唯一 raw Validated，跨品种池化进一步证伪为 BTC-独有 beta。
   - 生产化 / 收缩（B30②④）**无触发**（无 VALIDATED 候选 = 无消费者 / 无收缩对象），附理由后置。
   - **后继序列**：M5 声部执行（P^sep→N→Order 接生产开平仓）是 signal-full 之后的下一关；AncOK ceiling 对 M5 不能 waiver，须在执行层前闭合。M8 四层报告只完成第一层（signal）的 signal-full 前置，execution/treasury/total 三层待 M5-M7 闭合后合成。
   - ExitType 诊断切片实装（ResidualTrade 增 exit_type 列 + W-VERIFY 按 5 变体拆解占比，codex 裁定甲下游）是 W-VERIFY 增量，**不改本轮裁决基**。

5. **谱系引用**：
   - 冻结基：prereg-rev2-20260704（`26ebe90b29`）；跑批 HEAD `bd01afc1b8`。
   - 裁定：codex-ruling-696（μ_R 选项 B）；codex-ruling-exittype-20260704（ExitType 裁定甲）；final-prereg-20260704（终局 INCONCLUSIVE 不翻）。
   - prereg：prereg-l3-cross-symbol-20260702 + prereg-l3norm-20260703（L3 池化 + σ̂ 归一化）；highlow-a3-20260704（高低配口径）；ancok-a5-20260704（AncOK waiver L2 数据依据）。
   - memory：`iclass_delta_collinearity_perm_degeneracy`（ForceState⊥δ 防护对象）、`oddeven_mu_identity`（δ 同号=beta，唯一 Validated 性质）、`l3_cross_symbol_btc_idiosyncratic`（BTC-独有正号）、`q4_fullpi_no_alpha`、`type1_goal_closed_final_inconclusive`（终局无方向 alpha）、`interval_nesting`（区间套退化）。
   - 规则：231/formalization-validity-domain（L2/L3 标注 + 诚实剔除 + 有效域声明）；090/161（bit-exact + 否定性照实）；135（冻结先于跑数 + 不可比）；180/exit-μ-BUCKETING-FROZEN（ExitType 桶键铁律）；657/663/665/667（判据 / 共线免疫 / 方向不对称免疫 / 功效门 inconclusive≠证伪）。
   - **发生史无新概念分离**：本报告是既有 b1-b3 + a1/a6 产出的合成，未产生新谱系条目。

6. **影响声明**：
   - 新增文件 `.chanlun/review-results/signalfull-oos-round1-20260704.md`（本报告）。
   - **纯合成，零新数据、零代码改动**——所有数字转引自六份源结果包，不改一位。认识论等级 L2/L3 照实标注（§0）。
   - **不 commit**（任务约束，供 Lead 联合真封）。
   - 引用但未修改：sfoos-b3-20260704.md、m3lock-b2-20260704.md、codex-ruling-exittype-20260704.md、prereg-rev2-results-20260704.md、TARGET_STRATEGY_MAXFULL.md、final-alpha-20260704.md、docs/formal-chain/路线.pdf（权威源）。
