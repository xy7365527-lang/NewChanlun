# #879 S1 复测证据：「重」一等实体落地后，重内跨级反向持仓 = 0

**日期**：2026-08-17 ｜ **分支**：`sandcastle/issue-879` ｜ **实施票**：[impl] S1「重」成为一等实体 ⟨标的, 操作级别, 专属筹码⟩
**裁定锚**：ADR 0010（§一 重的三要件）／ ADR 0013（裁定一 各重的钱独立·不等式上限；裁定二 键=(标的,操作级别)）／ ADR 0014（裁定一 重内单向；裁定二 重间不仲裁）
**AC 对照物**：#837 探针与口径（`rust/src/theta_v0/backtest/issue837_probe.rs`，五窗 BTC 1m，`ThetaConfig::default()`）

---

## AC1：实体与键进类型层 + 逐仓计提单测

- `rust/src/theta_v0/strategy/chong.rs`（新增）：`ChongKey = (symbol, op_level)`（成本状态不进键）、
  `Chong = ⟨quota_usd, position_lots, cost_basis_usd, realized_pnl_usd⟩`、`ChongBook`
  （N 重 + 全局名义上限**不等式**；`posted_margin_usd = Σₖ |nₖ|·pxₖ/L_maxₖ` 逐仓全额计提；
  `deployable_notional_usd` 净额省下的差额**不回流**）。
- 单测 5 件（`chong::tests`）：对称多空两腿各自全额计提（反向两重 posted=两腿之和≠净额 0）／
  净额差不折名义（可部署 = 上限 − Σ 全额计提）／成本状态不进键／重复开户与零筹码 fail-loud／
  单重退化 = 旧「全账户净 lot」同值（旧行语义刻录）。
- 单向门单测 6 件（`coverage::leg::tests::chong_uni_*`）：反向腿零化折减仓／不穿零钳 0／
  空仓多数侧定方向／平局两侧全零化／Flat 腿不动·持空对称／无反向腿逐位不变。
- 逐仓计提接进风控门：`backtest/admission.rs` `k_theta_risk_gate` 的保证金基数由
  「全账户净 lot × mark」（`fill.rs` 旧行，`p_t.abs()*px`）改为 `ChongBook::posted_notional_usd`
  （Σₖ |nₖ|·pxₖ；单重时与旧行同值、语义锚定逐仓极性全额——#834 定落点随本票一并改）。

## 重内单向的合成层实装（ADR 0014 裁定一）

`coverage/leg.rs::enforce_chong_unidirectional`，挂在 `coverage/step.rs` 净额塌缩
（`net_target_units`，下单前唯一路径）上游：

- 反向腿不建持仓（量零化、不产生声部），其总量 R 折成同向腿减仓：f = max(0,(L−R)/L)；
- 方向权威 = 该重当前持仓符号（现状单重 = 账户净持仓）；空仓 ⟹ 当步合成多数侧，严格平局 ⟹ 两侧全零化；
- R > L ⟹ 钳 0（翻向必经空仓，不直接穿零）；
- L ≥ R 时变换后净目标 = L−R，与旧净额**逐位相同**（差异只在旧口径会穿零的 bar）。

⚠️ 退场条件（总缝规则举证门②）：「空仓时多数侧定方向」是**代理判据**——重自己的方向本应由其
操作级别读法给出，而塔目前没有操作级别概念（#826）。该代理随 SPEC #847 S3 落地后退役。

## AC2：同款探针复测——重内跨级反向持仓 = 0（实测，非声明）

### 落地后（本分支，五窗全跑）

| 窗 | bar 数 | D1 跨级反向信号 | D2 跨级反向动作 | **D3 跨级反向持仓重叠** | 对冲在场 bar |
|---|---|---|---|---|---|
| p3fold 2023-01-01..06-30 | 260,560 | 91（11.29%） | 4（0.443%） | **0 对** | **0（0.000%）** |
| w2023H2 2023-07-01..12-31 | 264,960 | 91（11.14%） | 10（0.956%） | **0 对** | **0（0.000%）** |
| w2024H1 2024-01-01..06-30 | 262,080 | 80（10.04%） | 9（0.918%） | **0 对** | **0（0.000%）** |
| w2021bull 2021-01-01..06-30 | 260,037 | 86（11.21%） | 12（1.319%） | **0 对** | **0（0.000%）** |
| y2023 全年 | 525,520 | 212（12.70%） | 14（0.667%） | **0 对** | **0（0.000%）** |

### 同代码基对照（前置 commit `09b32f722e`，独立 worktree 同款探针）

| 窗 | D3 跨级反向重叠对数 | 对冲在场 bar |
|---|---|---|
| p3fold | 1,694 对 | 171,030（**65.64%**） |
| w2023H2 | 1,510 对 | 180,884（**68.27%**） |
| w2024H1 | 1,546 对 | 166,116（**63.38%**） |
| w2021bull | 1,394 对 | 162,218（**62.38%**） |

对照组复现 #837 的 62.6%–72.0% 违规带（探针在当前代码上仍能测出违规）⟹ 「归零」是
本票改动的效果，不是探针失灵。两组的 D1（信号层）逐值相同（91/91/80/86）⟹ 结构判定层
不受影响（该层不碰钱），与票面声明一致。y2023 窗对照臂未跑（照实明写；四窗已入带）。

### 结构性归零论证（机制层）

1. 单向门在 `net_target_units` 上游把反向腿量零化；
2. LEE 镜像目标构造 `build_level_targets` 对 q≤0 的腿一致剔除 ⟹ 反向声部**不进入**镜像；
3. 翻向必经空仓：持仓符号定方向 ⟹ 只有账户归零后反向腿才可能携量；
4. 调试插桩旁证（复测中临时加入、已撤）：全窗镜像目标**从未**出现同 bar 双向（zero mixed targets）。

### 复测牵出的两条连带修复（均为「不穿零」的执行/簿记层破口，随本票一并落地）

1. **声部簿跨向原位改写**（`level_ledger.rs` / `overlay_state.rs`）：「σ_v 入场固定」前提在
   重内单向后失效——同一 carrier 可跨 bar 换向（先平后开经空仓过渡同 carrier 反向重开），
   原位改写 side 使 entry_bar 沿用旧方向入场点，派生跨向**幻影重叠**（实测 13 bar 假重叠）。
   修复：换向 = 旧声部出场 + 新声部同 bar 入场（声部一次性：出场即终结）。
   回归锁：`level_ledger::tests::side_flip_closes_old_voice_and_opens_new_same_bar`（两簿同式）。
2. **空头加深单方向碰撞**（`coverage/sizing.rs::schedule_order`）：旧口径「更空=Add」，而执行
   三后端（`fill.rs::apply_order` / `nautilus/order_adapter.rs` / `dual_ledger.rs`）一律把 Add
   当**买入** ⟹ 空头加深被执行成买入穿零（实证：w2023H2 bar198663，p_t=−3、p_star=−31 的
   加空 28 被执行成买 28，持仓翻 +25；错向持仓回馈为方向权威 ⟹ 派生幻影反向声部）。
   修复：同号增持按持仓符号分发——持多 Add / 持空 Sell。该缺陷先于本票存在（fill.rs 创世
   即如此），但它是 ADR 0014「不穿零」在**账户层**的破口，不修则 AC2 残余不为 0。
   回归锁：`schedule_add_reduce`（更空=Sell）+ `schedule_short_deepening_executes_as_sell_in_apply_order`
   （端到端：加空单经 apply_order 真卖出，持仓 −600→−900 不穿零）。

## AC3：回测基线作废声明（关票 comment 口径）

**既有回测基线在「保证金档 / 强平触发」两维作废**（票面口径）——保证金基数已从全账户净名义
改为逐仓全额计提名义（当前单重下同值，语义已锚定；多重落地后分叉）。**照实扩记**：连带修复 ②
改变了空头加深区间的**成交方向** ⟹ 凡发生过空头加深的窗口，成交序/PnL/权益曲线维同样作废
（对照组四窗 net_r ∈ [−38.9M, −16.0M]，落地后同窗 n_orders 与 net_r 均变——具体读数以重跑为准，
本报告不把旧基线任何一维当作仍有效）。结构判定层（信号/塔/分类）不受影响：两臂 D1 逐值相同为证。

## 既有测试的口径更新（5 件，均为「重内反向持仓」旧态的断言）

- `type2_open_short_channel_no_parent_lands_short_account` / `..._active_parent_lands_reverse_open_account`
  （#200 通道见证）：通道路由/证书/互斥断言不动；反向腿**身份落账、量零化**（qty −0）。
- `account_view_witness_three_identities_in_pi_loop`（#197）：三身份对账不动；短差账 realized
  归零（经济效果改经同向父仓减/补兑现进净额账户，视图对 resize 不归因——#837 已登记 resize 欠计）。
- `pi_tw_wiring_conserves_with_parent_and_reverse_open_in_flight` / `pi_four_step_cycle_keeps_parent_while_reverse_open_round_trips`
  （#526 VOICE_EXEC）：决策层往返（typed/verdicts/open_legacy_legs）不动；执行簿只剩父 campaign。
- `compose_tests_1::t1_target_zero_assertion_sell_batch_checks_long_side_only`：p_t 符号对齐存活侧
  （遗留合成混向态，非生产新可达态）。
- `held_tests_2` 两件（#315/#350 时序孔）：反向腿 depth→q 直接数量见证被单向门遮蔽，
  判别力改由 role_v + 同向腿折减因子 f 携带（f=(L−R)/L 含反向原料量 R）。

## 验证

- `cargo check --all-targets` 绿；`cargo test --lib` 2729 过 0 挂（含新增 13 件）；
- `cargo clippy --all-targets` 零新增 warning（WIP 引入的 6 件已修：4×too_many_arguments 加
  allow（同仓 118 处惯例）+ redundant field shorthand + `bsp.rs` 白名单行号位移重登记 553→557/775→779）；
- `cargo fmt` 已跑；
- 探针与生产 `classify()` 末帧对拍五窗 PASS 后取数（#837 同款纪律）。

## 090 照实（未测/限度）

- **多重未实例化**：当前全账户 = 一个重（legacy 占位键 op_level=0，`ChongBook::LEGACY_SYMBOL`），
  多重键的真操作级别维度随 SPEC #847 S3 落地；簿的 N 重语义（逐仓计提 + 全局不等式）在类型层
  定型并经单测，生产消费面当前单重。
- 代理判据「空仓多数侧定方向」的退场条件见上（S3 挂载点后由操作级别读法取代）。
- D2 残余 4–14 例/窗（0.44%–1.32%）：口径为「同 bar 净买/净卖开平事件」——同向持仓的
  减/补与他级开仓同 bar 发生仍会计数（动作方向 ≠ 持仓方向）；未逐例核查（照实明写）。
  持仓口径（D3，AC2 目标）已结构性归零。
- D1 与 #837 绝对值不可比（其间主线信号集有变，whitelist 登记链为证）；对照取同代码基 pre/post。
- 保证金档/强平触发的实测对照表未跑（作废声明已含）；y2023 对照臂未跑。
- 交易所侧逐仓（重间物理隔离）不在本票——ADR 0014 有效域注记：记账先分开、出口仍走净额
  （#178 档 A 中间态），funding/borrow 毛暴露口径解锁归 #836。
