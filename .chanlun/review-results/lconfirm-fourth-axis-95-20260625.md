# L_confirm 第四轴 instrumentation 设计+实装+L1 验证（task#95，W-lconfirm 工位）

**工位**：W-lconfirm（topo_address: codex-line/W-lconfirm）
**parent_callback**：codex-line-20260625 Lead
**承接**：codex #92 payoff 塔异质审计（判决 NO，最致命断点=缺 L_confirm 第四轴）+ #597 仓位上同调塔（生成态）+ #69 shortleg-alpha（L_confirm 实证根据）
**认识论等级**：instrumentation 管线正确性 = **L1**（合成数据/受控 LevelView 验证，验管线不验假设，信息增量零）；OFF bit-exact = **L1**（逐位回归）；第四轴成立/否证裁定 = **L2/L3 未决（数据缺失阻塞，照实报告）**。
**约束遵守**：read-only（observation-only，无决策消费者）+ env 门控（T_LCONFIRM_AUDIT）+ OFF bit-exact + no-workaround（数据缺失停下报告不硬塞）+ no-patch（机械穷尽守卫补 #92 缺的可结算底座，非补丁）+ formalization-validity-domain（不捏造 L3，否定性优先）。

---

## 1. 结论

**L_confirm 第四轴 instrumentation 已设计 + 实装 + L1 验证通过。第四轴成立/否证的 L2/L3 裁定因 8 标的真实 OHLCV 数据缺失而阻塞——照实报告，不捏造已验证。**

### 1.1 设计：确认深度 c 的可观测代理（严格非补丁）

`c` 不是新造代理，是 `g_pair` **既有门控信号**的读数（`ConfDepth` 三值枚举）：

| c 桶 | 缠论信号 | g_pair 既有用途 | 持仓尺度（#69:11/26） |
|------|---------|----------------|---------------------|
| **T1（深）** | type1 顶/底背驰 = 走势完成 = 全深度区间套链贯通 a0（`t1buy`/`t1sell`/`d_top`） | 核心多腿 churn 门控（`core_done=view.d_top[k]`） | 主力核心 = 全深度 `d_top` |
| **T2（中）** | 其余买卖点（非 type1 非 type3，`buy`/`sell`） | 多头开仓 `view.buy` / 一般卖点 | 中等 |
| **T3（浅）** | type3 = 突破中枢+回试不回 = 单层区间套转折（`t3sell`） | 次级别开空门控（`sub_break=view.t3sell[k]`） | 次级别短差 = 单层 `t3sell` |

这正是 #69 自相似原则「确认深度 ∝ 持仓尺度（主力 d_top 全深度 / 短差 t3sell 单层）」的**直接读数**——c 在代码里早已是开/平腿的门控信号，instrumentation 只是把它**记账**而非新造。

### 1.2 实装：per (k, leg, dir, c) 纤维会计（read-only，env 门控）

- **纤维数组** `pi_k_leg_dir_c[k][leg][dir][c]`（`[MAX_LEVEL][2][2][3]` f64）+ 计数 `pi_count_k_leg_dir_c`，索引：k=级别 / leg(0=H⁰核心/1=H¹短差) / dir(0=R+long/1=R−short) / c(0=T1/1=T2/2=T3)。
- **腿轴 leg** 由 `highest_active_long()==k`（开仓后重算）结构涌现决定（H⁰=骑最高/主走势=核心，H¹=次级别衬底/机动），零 `if level==N`。
- **捕获点**：3 个 g_pair 开腿点（核心多腿 / 核心翻空 / 次级别空腿），open 成功后锁 `(conf, is_core)` 到 LegPair；close_*_leg 据此把 realized pnl 累加进格子。**所有 realized P&L 必经 close_*_leg（含 finish 收尾 + NAV 强平）⇒ 无逃逸。**
- **env 门控** `T_LCONFIRM_AUDIT`：OFF ⇒ 新数组恒 0（无写入）= bit-exact。

### 1.3 机械穷尽守卫（补 #92 缺的可结算底座，codex 指出的关键）

`assert_lconfirm_exhaustive()` / `lconfirm_exhaustive_check()`：断言 **Σ_{所有cell} π(k,leg,dir,c) == Σ总 leg realized P&L（Σpair_long_pnl + Σpair_short_pnl）**，tol=1e-6。三来源交叉（纤维和 / pi_total_audited 累加镜像 / pair_*_pnl 独立来源）一致 ⇒ 每笔 realized 都进了且仅进了一个格子。**不等 ⇒ 有未分类 P&L 流 = 分类不完备的可验证信号。** 这正是 #92 不可结算的修复：payoff 塔此前无「Σ分量==总P&L」的可验证守卫，codex 据此判 NO。

### 1.4 L1 验证（管线正确，逐位回归）

- **OFF bit-exact**：`lconfirm_audit_off_bit_exact` —— 同一合成序列，audit ON vs OFF 在 **OFF 基线 + RB_PAIR 两路径**上 `final_nav` 逐位（`to_bits()`）一致 + 全 per-level long/short pnl + opens/closes/stops/churns + liq + leg_trades 长度 + max_gross 全一致。OFF 路径纤维格子恒 0。✅
- **四轴归格正确性 + 穷尽守卫**：`lconfirm_exhaustive_guard_holds` —— 受控驱动开 H⁰核心多腿(type1)→ (4,H⁰,R+,T1)=+6666.7/n1；H¹次级别空腿(type3)→ (1,H¹,R−,T3)=+41.2/n1；穷尽守卫 Σ纤维==Σ总leg 成立。✅
- **全套回归**：`cargo test --lib` 562 passed / 0 failed / 31 ignored（含新增 L3 占位）。✅

### 1.5 L2/L3 裁定：数据缺失阻塞（否定性如实报告）

8 标的真实 OHLCV（`analysis/data_cache/*.json`）**不存在于当前工作区**：
- 主仓 `analysis/data_cache` 是**自指断裂 symlink**（`-> .../analysis/data_cache` 指向自身）。
- 所有 worktree 副本 symlink 回主仓断裂目标。
- wip-orphans-20260625 分支树只有 result/report JSON（无 raw OHLCV）。
- `trading_system/data_cache/*.json` 全是回测**结果**文件（有 strat_pct/n_trades，无 OHLC bars）。
- L3 harness `lconfirm_4axis_l3`（已实装，production/Face A + audit ON）跑 8 标的全部输出「数据缺失，跳过」。

**裁定结论：第四轴成立 vs 否证 = L2/L3 未决（数据阻塞）。不捏造「已验证」（formalization-validity-domain：L0/L1 信息增量为零，合成数据不能验证假设）。**

### 1.6 #92 完备性最终判定

- **codex 第二断点（最致命）= 缺第四轴 c + 缺可结算底座**：本工位**补上可结算底座**（机械穷尽守卫，L1 成立）+ **补上第四轴 instrumentation**（c 可观测、可记账，L1 成立）。
- **但 #92「生成性完备」仍 = NO（不恢复）**：完备性恢复需 L3 裁定「所有 c 桶符号一致」（第四轴被否证 ⇒ 三轴足够）。该裁定数据阻塞未决 ⇒ #92 完备性**悬而未决**，不能宣布恢复。codex 另两断点（a0 底腿后补特判 / 核心全正是条件命题）本工位未触及，独立仍 FAIL。

### 1.7 #597 结算建议

- **597 保持生成态**（不结算）。597 的 `definitions_involved` 已列「第27课逐级区间套链 → L_confirm 内在来源」「α*_k 配额 = L_pullback × **L_confirm**」——597 早已把 L_confirm 作为配额因子之一预置。
- **建议**：597 在 `pending_verification` 增列「⑤ L_confirm 第四轴 L2/L3 裁定（W-lconfirm instrumentation 已就位，数据阻塞）：若符号分裂 ⇒ 仓位塔纤维须扩为 ⊕_k(H⁰_k⊕H¹_k) **× c 轴** = 四纤维而非三纤维」。数据恢复后跑 `lconfirm_4axis_l3` 得裁定再推进 597。

---

## 2. 定义依据

- **L_confirm = 区间套确认深度**：#597 `definitions_involved`「第27课逐级区间套链 → H¹_k 机动腿开-绕-平循环定位 + **L_confirm（区间套确认深度）的内在来源**」；codex-payoff-audit-92 §60-62「L_confirm 确实是漏轴（第四轴）」。
- **c 的可观测代理（持仓尺度分级）**：shortleg-alpha-69-20260623.md:11「主力=全深度 d_top / 短差=单层 t3sell」+ :26「区间套确认深度 ∝ 持仓尺度……角色由 highest_active_long 结构涌现，零 if level/regime」。代码满足：`g_pair` 既有 `core_done=view.d_top[k]`（核心 churn）+ `sub_break=view.t3sell[k]`（次级别开空）= c 已是门控信号。
- **腿轴 leg（H⁰/H¹）**：#80/#597「H⁰_k 骑本级走势核心 / H¹_k 操作次级机动」；代码 `highest_active_long()==k` = H⁰ 核心结构涌现判据。
- **机械穷尽守卫**：codex-payoff-audit-92 §69「核心结构性遗漏……payoff 纤维 K×L×D 不足以决定 P&L 符号」+ tower-sub8-payoff-92 §八#1「若所有 (k,leg,dir) 都可达=自由叉乘=笛卡尔积」——守卫给出「Σ分量==总P&L」的可验证完备性底座（#92 此前缺）。
- **可证伪裁定（H1 符号分裂）**：codex-payoff-audit-92 §54-58 最强漏 case「同一索引下符号不定 ⇒ 分类不完备」——本工位把它形式化为「同 (leg,dir) cell 内 c 桶符号比较」。

## 3. 边界条件（结论翻转）

1. **若 L3 裁定「所有 c 桶符号一致」** ⇒ 第四轴**被否证** ⇒ 三轴 K×L×D 足够 ⇒ #92 完备性恢复（对 #92 有利的否定性结果，照实报告）。当前数据阻塞，未裁定。
2. **若 L3 裁定「同 (k,leg,dir) cell 内 c 桶符号分裂」** ⇒ 第四轴**坐实** ⇒ 索引必须扩为 π(k,leg,dir,c) ⇒ #92/597 必须改四轴。
3. **若机械穷尽守卫在真实数据上 panic（Σ纤维≠Σ总leg）** ⇒ 存在未经 close_*_leg 的 realized P&L 流（如未来新增平仓路径绕过 close_*_leg）⇒ 第四轴会计本身不完备，须先补漏。当前 L1 守卫成立（合成数据）。
4. **若 c 的三值代理（T1/T2/T3）不能区分真实失血**（如同一 t3 桶内仍因更细确认深度符号分裂）⇒ c 轴需细化为连续轴（参 #601 连续轴），三值离散化是有效域边界。
5. **若腿轴 H⁰/H¹ 用 `highest_active_long` 涌现判据在真实数据上错配核心**（如核心腿被误判为 H¹）⇒ leg 轴归格错 ⇒ 裁定失效。当前 L1 受控验证归格正确。

## 4. 下游推论

- **#92 payoff 塔**：可结算底座（穷尽守卫）就位 ⇒ payoff 塔从「不可结算」（codex 断点）升级为「可结算待裁定」。但完备性仍 NO（裁定数据阻塞 + 另两断点未触及）。
- **#597 仓位塔（生成态）**：若第四轴坐实，597 的 ⊕_k(H⁰_k⊕H¹_k) 三纤维须扩为含 c 轴的四纤维；`pending_verification` 应增列第四轴裁定项（见 §1.7）。
- **子4（#83 α*_k 配额）**：597 已定义 α*_k = L_pullback × **L_confirm** ⇒ 本 instrumentation 给 L_confirm 的逐 cell 实测端（哪个 c 桶失血），是 α*_k 自适应的输入。
- **数据基础设施**：`analysis/data_cache` symlink 断裂是**全线阻塞**（所有 L3 测试 #[ignore] 跑空）——需 Lead 修复 symlink 或恢复 databento 数据，否则 codex-line 主线所有 L2/L3 裁定都无法落地。

## 5. 谱系引用

- **codex-payoff-audit-92-20260625.md**：本工位直接承接其判决 NO 的最致命断点（缺 L_confirm 第四轴）——补 instrumentation + 可结算底座。
- **#597（仓位上同调塔，生成态）**：本工位为其第四轴裁定提供 instrumentation；597 `definitions_involved` 已预置 L_confirm 作配额因子。
- **#80/#81/#82/#89（塔四兄弟）**：leg 轴 H⁰/H¹ 来源（#80 S¹ 上同调），dir 轴 R+/R− 来源（#81 Z₂ τ）。
- **shortleg-alpha-69-20260623.md（L3 已结算）**：c 的可观测代理实证根据（主力 d_top / 短差 t3sell，#69 L3 实装事实）。
- **#601（连续轴，597 children）**：若 c 三值离散不足，c 轴细化为连续轴的谱系出口（边界条件4）。
- **谱系分离声明**：本工位**未触发新概念分离**——L_confirm 第四轴的「概念分离」（payoff 三轴→四轴）若 L3 坐实，由 #92/#597 的后续裁定记录，本工位只交付 instrumentation + L1 验证 + 数据阻塞报告，不擅自宣布分离（数据未决）。**这与 codex-payoff-audit-92 已指出「payoff 纤维犯了与仓位分类 G 同一遗漏（三轴应为四轴）」同构——但坐实与否待 L3。**

## 6. 影响声明

- **改动文件**（无 git commit，Lead 统一提交）：
  - `rust/src/recursive_t/rec_engine.rs`（+167 行）：`ConfDepth` 枚举 + `EngineConfig.enable_lconfirm_audit`（env `T_LCONFIRM_AUDIT`）+ TRoot 字段 `enable_lconfirm_audit`/`pi_k_leg_dir_c`/`pi_count_k_leg_dir_c`/`pi_total_audited` + LegPair 字段 `long_conf`/`long_is_core`/`short_conf`/`short_is_core` + 3 个 g_pair 开腿点捕获 + close_long_leg/close_short_leg 归格 + 访问器 `lconfirm_cell`/`lconfirm_cell_sum`/`lconfirm_exhaustive_check`/`assert_lconfirm_exhaustive`。
  - `rust/src/recursive_t/rec_stream.rs`（+196 行）：测试 `lconfirm_audit_off_bit_exact`（OFF bit-exact L1）+ `lconfirm_exhaustive_guard_holds`（四轴归格 + 穷尽守卫 L1）+ `lconfirm_4axis_l3`（L3 裁定 harness，#[ignore]，数据阻塞）。
- **影响判断层面**：(1) 为 #92 payoff 塔补可结算底座（穷尽守卫）+ 第四轴 instrumentation；(2) 为 #597 第四轴裁定提供工具；(3) 暴露数据基础设施阻塞（symlink 断裂）。
- **不影响**：引擎交易行为（observation-only，OFF/ON 决策路径 bit-exact，562 测试全绿）；#92 完备性判定（仍 NO）；第四轴成立/否证裁定（L2/L3 数据阻塞未决，未声明膨胀）。
- **认识论诚实**：本工位交付 instrumentation（L1 管线正确）+ 可结算底座（L1 守卫成立）+ 数据阻塞报告。**不交付**：第四轴 L2/L3 裁定（数据缺失）、#92 完备性恢复（裁定未决 + 另两断点未触及）、597 结算（生成态保持）。

---

## 七、数据依赖（给 Lead，明确报告需求，no-workaround）

L_confirm 第四轴 L2/L3 裁定**唯一阻塞 = 真实 OHLCV 数据缺失**：

- 需要：`analysis/data_cache/{cl,brn,dx,gc,es}_1m_databento_10y.json` + `qqq_1m_databento_full.json` + `btc_1m_full.json` + `oklo_1m_databento.json`（8 标的，backtest_run.rs:332 SYMBOLS）。
- 当前状态：`analysis/data_cache` 是自指断裂 symlink（指向自身），全部 8 文件不可达；worktree 副本 symlink 回同一断裂目标；wip-orphans 分支无 raw OHLCV。
- **不强行合并 wip-orphans-20260625**（bar_spec 1秒源）——任务约束明确「不要强行合并」。
- 数据恢复后跑：`T_LCONFIRM_AUDIT=1 cargo test --release recursive_t::rec_stream::tests::lconfirm_4axis_l3 -- --ignored --nocapture` ⇒ 输出 8 标的 per (leg,dir,c) 纤维 + 穷尽守卫 + 符号分裂裁定 ⇒ 第四轴成立/否证 L3 结论。
