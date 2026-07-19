# p126：阶段 3 runbook——⑧ M7 treasury witness 复跑 / ⑨ M8 Π_max-full 四层报告（设计定稿，2026-07-18）

**状态**：runbook 设计定稿（纯文档，零 cargo、零 git mutation、主仓 `/Users/silencehan/Projects/NewChanlun` 只读）
**工位**：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`）；执行属后续任务，由编排者派发
**任务定位**：`chanlun/plans/mainline-merged-roadmap-20260717.md:22-24,47-52`——阶段 3 资金层验收 = 3a（⑧ M7 treasury 重验，带 A10 成本的可行集 K 内，退路裁定）+ 3b（⑨ M8 Π_max-full 端到端四层报告，主目标验收）。照实否定是合格结果（161 号）。
**认识论等级**：witness/跑批 = **L2**（真实 BTC 数据可达性/假设检验，可否证，231 号强制标注）；机制断言（守恒/桥接/bit-exact 回归锁）= L1。

**输入清单（全部已读）**：
- 旧 witness/验收报告（主仓 `.chanlun/review-results/` 只读）：`m7treasury-c3-20260704.md`（L2 witness 照实否定 + κ=0 冻结裁定）、`kappa-d1-20260704.md`（κ 有理定点 + {0,½,1,2} 网格全卡 Stage I）、`e2e-d2-20260704.md`（M8 四层实测三窗 + 12 窗 Reach 分布）、`maxfull-e2e-round1-20260704.md`（四层合成报告，INCONCLUSIVE 终判）。**登记：主仓不存在 `m7treasury-c2` 文件**（grep 仅命中 c3），任务书提到的 c2 系列未见，可能未落盘或在他处——如实标注，不虚构引用。
- 693 验收列：`TARGET_STRATEGY.md:38`（Π_treasury-full：Reach(StageIII)>0、Q_T>Q_0、W_T≥I_0、η_T≥η_*）、:44-54（边界互斥）；`TARGET_STRATEGY_MAXFULL.md:147-168`（M7/M8 关判据）、:259-270（M 段↔693 映射）。
- 新口径裁定（worktree `chanlun/escalate/`，2026-07-18 生效）：`a10-cost-model-ruling-20260718.md`（C1–C5+附则A/B）；`pan-terminal-endorsement-ruling-20260718.md`（P1–P4 背书语义收紧）。
- 实装点（worktree 直读）：`runner.rs:1289-1400`（cum_holding_cost shadow + 桥接断言 + F4 同源 η_bucket）、`risk.rs:920-961`（R 分解 `tw_holding_cost_bridge` 对账行）、`risk.rs:754`（`RATE_UNCALIBRATED_LABEL`）、`wverify_run.rs:1066-1339`（m6 R 分解 + m8_e2e 四层跑批，均带标签）、`runner.rs:5146/5232/5301`（三个 #[ignore] witness/网格/多窗测试）、`config.rs:308/314/328`（margin/cost_model/risk_policy 三 Option 字段，derive Default 全 None——**旧 witness 跑在零成本口径的实证**）。

---

## 1. 新口径总账（相对 2026-07-04 旧报告系列的差异）

| # | 维度 | 旧口径（0704 系列） | 新口径（0718 裁定+实装） | 锚 |
|---|---|---|---|---|
| 1 | 成本可行集 K | witness 用 `ThetaConfig::default()` ⟹ margin=None/cost_model=None，**零成本口径** | 阶段 3a 定义 = **带 A10 成本**：margin=Some(CME-simple) + cost_model=Some(三常费率) | 路线图:51；config.rs:293 derive Default |
| 2 | η 口径 | η_T = tw() 未修正（G1 缺口：TW 账本不见 funding/borrow/liq，η 高估恰 ⌊funding+borrow+liq⌋） | **η_corrected = tw() − cum_holding_cost**（C5 裁定 (b) 已实装：i64 shadow + 桥接断言 + F4 η_bucket 同源一笔改动） | A10 裁定 C5；runner.rs:1289-1400；risk.rs:937 |
| 3 | κ 注入面 | 仅 env 诊断 knob（kappa_policy_from_env） | `ThetaConfig.risk_policy: Option<RiskPolicy>`；**优先序 env>config>baseline 写死**；None ⟹ baseline bit-exact | A10 附则A；runner.rs:982-986；config.rs:328 |
| 4 | 费率标签 | m6margin-c2 自陈「费率待外部标定」（文字级） | **`[L1机制/费率未标定]` 常量强制**（T-N9 测试锁存在性）；常费率数值禁作 alpha 论据/策略择优 | A10 附则B；risk.rs:754；wverify_run.rs:1107/1224 |
| 5 | borrow | 0.01bp/bar 常费率有微量值（旧三窗 Borrow=8~10） | **v0 perp 口径 borrow≡0**＋报告强制「非市场借贷利率」声明（机制/字段保留） | A10 C3 |
| 6 | ADL | 未建模（同） | OUT_OF_SCOPE 声明＋venue adapter 登记；**禁称「强平模型完备」**；引用强平语义须随文注记 | A10 C1 |
| 7 | 强平判据 | — | OQ-5(d) 定理：cost_basis<0 禁进强平判据；T-N5 守卫升强制；v0 全程逐仓（阶段三全仓不接线） | A10 C4 |
| 8 | BSP 集合 | v2/v3 时代分类器口径 | 关②（S0/S1a/S2S4/037:20 口径修复）＋关③背书收紧（P1 趋势族=破 B 一类∧owner=B；P2 盘背族=同向一/二/三类全合法）⟹ **v3' BSP 集合 ≠ 旧集合 ⟹ 订单流/fill 序列变 ⟹ 全部旧数值基线作废存档**（只作机制证据，不作验收基线） | p121 H2:65；pan 裁定 P1/P2/P4 |
| 9 | funding 方向 | 无向保守 `|N|×rate` | 有向 datum＋`FundingScheduleBook` additive 接口预冻结（datum 未注入期间无向保底不动） | A10 C2；risk.rs:711 |

---

## 2. ⑧ M7 treasury witness 复跑协议

### 2.1 输入（两件，缺一不可）

- **输入 A：A10 成本可行集 K**。margin=Some(`q4_margin_model(nav)`)；cost_model=Some(`m6_cost_model()` 三常费率：funding 1bp/480bar、borrow 0.01bp/bar（C3 后 perp 口径≡0，字段保留）、liq 0.5%——`wverify_run.rs:1066-1074` 冻结近似值）。费率全部带 `[L1机制/费率未标定]`。
- **输入 B：终验后的 v3' dump**。关② S=8 分片重放收口（p123:21 候选 B，~5×，在跑）＋关③ P3 收紧后重放验收（terminal_confirmed 硬界 [748, 1,213]、Pan=748 精确、877=110/122/31 精确、CERT 三栏，pan 裁定 P3 裁决4/P4）通过后，**同一 commit 的生产分类器即 v3' BSP 集合的生产者**。witness 走 `IncrementalClassifier` 生产路径实时分类（不读 dump 文件）；dump 的作用是 provenance 交叉锚——witness 报告头部必须引用该 dump 的 terminal_confirmed 总量与族分层构成行，把「本 witness 跑在哪个 BSP 集合上」钉死。

**前置实装登记（additive 一件，执行前必须落地）**：现有 `m7_l2_witness_treasury_reach_real_btc`（`runner.rs:5146`）硬编码 `ThetaConfig::default()`（margin/cost_model=None），**不支持成本注入**。须按 `apply_theta_dir_preset_from_env`（wverify_run.rs:1257）/`kappa_policy_resolved`（runner.rs:986）先例加 env gate：`M7_WITNESS_A10=1` ⟹ 测试内 cfg.margin/cost_model=Some（与 m8_e2e 同函数同源，禁第二查法）；env 未设 ⟹ 零成本旧路径 bit-exact 不动（C5 不回滚条款：κ=0 ∧ cost=None 回归锁）。同一 gate 同步接入 `m7_kappa_sensitivity_grid_real_btc`（:5301）与 `m8_treasury_reach_distribution_real_btc`（:5232）。witness 输出增打两行：`cum_holding_cost`（shadow 值）与 `η_corrected = tw() − cum_holding_cost`（判读改用值）——additive eprintln，不动既有行。

### 2.2 命令序列（bin/env/数据路径全写死）

```bash
cd /tmp/kimi-nest-mainline/rust
# 数据：analysis/data_cache/btc_1m_full.json（worktree symlink → 主仓 329MB，只读消费；
# 执行时先 shasum -a 256 记数据版本进报告头）
unset KAPPA_BARRIER_NUM KAPPA_BARRIER_DEN   # ★强制：env 优先序最高，残留会静默覆盖 κ=0 基线

# (1) κ=0 基线 witness（A10 成本可行集内，10 万 bar 足量窗口）
M7_WITNESS_BARS=100000 M7_WITNESS_A10=1 cargo test --release --lib \
  theta_v0::backtest::runner::tests::m7_l2_witness_treasury_reach_real_btc -- --ignored --nocapture
# 全量 4,613,599 bar：去掉 M7_WITNESS_BARS（O(n²) 前缀重分类，小时级；墙钟不承诺）

# (2) κ 敏感性网格 {0,½,1,2}（L2 诊断，非择优；env 由测试内部 set/remove 自管理）
M7_WITNESS_BARS=100000 M7_WITNESS_A10=1 cargo test --release --lib \
  -- --ignored --nocapture m7_kappa_sensitivity_grid_real_btc

# (3) 多窗 Reach 分布（全部 12 个 anchored wf 窗，含 2020-2021 牛市段 wf1/wf2/wf3）
M7_WITNESS_A10=1 cargo test --release --lib \
  -- --ignored --nocapture m8_treasury_reach_distribution_real_btc
# 落盘 /tmp/m8_treasury_reach_distribution.md（须归档，见 §3.5）

# (4) R 分解（p121 T-N10 联动：关②后重跑，新旧 R 表对照入档）
cargo test --release --lib \
  theta_v0::backtest::wverify_run::m6_btc_oos_r_decomposition -- --ignored --nocapture
# 落盘 /tmp/m6_btc_oos_r_decomposition.md
```

env 纪律：`THETA_DIR_PRESET` 不设（Neutral 默认，witness 不消费该 knob）；`M7_WITNESS_BARS` 只截前 N bar（窗口口径沿用 c3「足量窗口即 L2 合规」）；κ 网格行内 env 自设自清，外部禁止预设（会覆盖网格逐行）。

### 2.3 输出字段与判读式（逐项）

witness eprintln 块（:5183-5208）逐行读法；**新口径增打的两行插入 η 判读**：

| 字段 | 判读式 | 新口径注 |
|---|---|---|
| `is_l2` | `n_orders > 0`（真实订单流） | L2 合规门槛 |
| `Reach(Stage=II)>0` / `Reach(Stage=III)>0` | `tw.stage.rank() ≥ CapitalRecovered/EarningShares.rank()`——**stage 单向不可逆 ⟹ 终态 rank = 全程最高 ⟹ Reach 由终态单读**（c3 §2 方法论根基，沿用） | 不变 |
| `Q_T > Q_0` | Q_T = 终态 `notional_in`；Q_0 = 开局注资 ⌊nav0⌋；未进 Stage III ⟹ 无 BuyCore ⟹ Q_T=Q_0 ⟹ false | **各窗 Q_0 不同**（e2e-d2 口径订正：nav=窗首价×1000，非恒 1e6） |
| `W_T ≥ I_0` | W_T=`withdrawn`；I_0=`notional_in`（同源） | 断言 `W_T ≤ notional_in` 常驻（wverify_run.rs:1324 先例） |
| `η_T ≥ η_*` | **η_T := η_corrected = tw() − cum_holding_cost**（C5 后唯一合法口径；修正只降不升，T-N4 语义）；η_* = `policy.eta_star(&tw)`（κ=0 ⟹ η_*=L^wc） | 报告须并置 `tw() 原值 / cum_holding_cost / η_corrected` 三行＋桥接断言「\|f64 真值 − shadow\| < 1」；c3 的「η_* 当前态」注（:110）沿用但以修正后 η_T 重读 |
| 归因三分支 | reach_iii ⟹ 验收四合取逐项；reach_ii ⟹ EnterReady 五合取分项（stage/W/legs/RiskNormal/η）；stage I ⟹ P3 RecoverCapital 门双距（holding−notional_in、free−recover_target）＋Σ已实现PnL | 结构不变；归因报告须注明「binding 约束位置」 |
| 账本恒等 | `tw() == q0 + ⌊Σ已实现PnL⌋`（GAP3 A' 清单⑥，断言常驻） | ＋C5 桥接断言常驻；reach_iii ⟹ `open_legacy_legs==0`（OQ-9 入口证书） |

### 2.4 κ=0 基线与正 κ 敏感性（各自口径声明）

- **κ=0 = 生产冻结基线**（A10 附则A 裁决4；T-keep 回归锁：None/env 未设 ⟹ 与现路径逐字节一致）。M7 验收判据只在 κ=0 行读数。
- **正 κ {1/2,1,2} = L2 敏感性诊断，非选择**（codex `.kappa-ruling` 裁定3 ＋ A10 附则A：网格只作敏感性陈述不作择优；正 κ 生产取值留编排者选择类＝M8 后 L3）。κ>0 行强制标注：「κ＝operator 声明式风险政策（不可识别性定理2），非价格导出、非回测择优」（附则A 报告闸）。
- 读表纪律（kappa-d1 §1(c) 沿用＋重验）：① 终 stage 分布；② n_orders 跨 κ 是否仍恒（κ 只门控 II→III 谓词——**新订单流下须重验**，见 §5.1）；③ barrier 距离随 κ 的线性性（有理定点精确性物证，旧读数每 0.5 步 −0.5·Q）；④ binding 约束位置（RecoverCapital 门 vs barrier η_*）。

### 2.5 照实否定的合格形态（161 号先例，c3 §3 沿用）

否定性结果照实合格，不改实现凑 PASS。合格形态五要件：
1. `is_l2=true`（真实订单流，非 fixture）；
2. 归因精确到「差多少、卡在哪」（双门距离＋Σ已实现PnL＋binding 约束位置）；
3. 区分 **L1 机制可达**（合成 witness `pi_loop_realized_profit_reaches_earning_shares` 已证机制在足额利润下可达）vs **L2 数据层燃料缺失**——禁止把数据层否定写成架构不可达（memory `project_gap3_l2_unreachable_architecture` 订正先例）；
4. 边界条件写明（结论翻转的充要条件）；
5. 禁外推（负结论限本对象、本窗口、本 Θ、本费率口径——693 §1.4 边界互斥）。
报告头部强制三件套：`[L1机制/费率未标定]` 标签（带成本数值行）；「ADL 未建模（venue 适配项）」注记（引用 LiqLoss/强平语义时）；borrow 行「非市场借贷利率」声明。

---

## 3. ⑨ M8 Π_max-full 四层报告骨架

### 3.1 跑批命令（写死）

```bash
cd /tmp/kimi-nest-mainline/rust
unset KAPPA_BARRIER_NUM KAPPA_BARRIER_DEN THETA_DIR_PRESET   # κ=0 + Neutral 基线
cargo test --release --lib \
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
# 落盘 /tmp/m8_e2e_all_systems_oos.md（缓冲，须归档，见 §3.5）
```

三系统同开接线（e2e-d2 §1 证据沿用）：`run_theta_v0_pi_overlay` 一次调用 = M5 overlay 声部账本（hook）＋ M6 margin+cost_model（cfg 字段）＋ M7 TW 账本（loop 内建），无独立组合层。窗口：p3fold（2023-01-01..2023-06-30）＋ 前两个 anchored wf 窗（`OOS_START` 起，与 m6 跑批同窗可差分）；`THETA_DIR_PRESET=follow/adversary` 是 execution 层敏感项，只作增量诊断另列，不进基线报告。**前置：agent-42 注记——需 BTC 数据（symlink 可达性先验）且 η 修正后数值口径待确认（见 §5.2，执行前须先裁定 (i)/(ii)）。**

### 3.2 四层骨架（判据列 / 数据来源 / 合格读法 / 边界声明）

| 层 | 判据（693/MAXFULL 冻结列） | 数据来源（哪份 dump/哪个 bin 的哪行） | 合格读法 | 边界声明 |
|---|---|---|---|---|
| **(1) signal** | `LCB_OOS(μ)>0 ∧ LCB_OOS(μ_R)>0`（MAXFULL M8:163；693 §1.1 三态） | **转引不重算**：关②终验＋关③收紧后的 signal 层终判报告（未出前留 `⟨PENDING-G2⟩` 占位，§4.1）；m8_e2e 代码内转引段 `wverify_run.rs:1170-1172,1230-1236` | 三态 VALIDATED/FALSIFIED/INCONCLUSIVE；功效门 n≥271~1083 | signal 结果不外推 max-full（措辞 §5.3）——但若本轮 Π_tested=Π_max-full 成立则按 round1 §4 口径合法性注记复核引用前提（§6-6） |
| **(2) execution** | `E[R(Π_exec)]>0`（MAXFULL M8:163；M6 式 R=ΣN_tΔP_t−Comm−Slip−Funding−Borrow−LiqLoss） | m8_e2e 跑批：`net_result.r_decomp` 六行（:1265,1308）＋`metrics.max_drawdown`（:1266）＋`n_orders`＋overlay by-role A/S/F 计数（:1267-1274）；**TW 桥列 `tw_holding_cost_bridge`**（risk.rs:937，C5 对账行）；守恒硬校验（:1317-1322） | 机制验证口径：守恒残差≤tol ⟹ 无泄漏；net_r 数值 = 成本真实化机制证据，**非盈利判定** | `[L1机制/费率未标定]`；数值禁作 alpha 论据/策略择优（附则B v3 硬禁令） |
| **(3) treasury** | `Reach(StageIII)>0 ∧ Q_T>Q_0 ∧ W_T≥I_0 ∧ η_T≥η_*`（693:38；MAXFULL M7:150-158） | m8_e2e：`OverlayRunResult.tw_final`（stage/notional_in/withdrawn/tw()，:1277-1284）＋`RiskPolicy::eta_star`；多窗分布补 `m8_treasury_reach_distribution_real_btc` 落盘表 | stage 单向不可逆 ⟹ 终态=Reach 单读；**η_T 列口径 = η_corrected = tw() − tw_holding_cost_bridge**（与 §2.3 witness 同源；当前代码打的是未修正 tw()，口径缝见 §5.2） | 负结论限本 Θ 本窗口；「数据层燃料缺失 ≠ 架构不可达」注记强制 |
| **(4) total** | `R(Π_max-full)>0 ∧ LCB_OOS(R)>0`（LCB 才 confirmed；R>0∧LCB≤0 ⟹ INCONCLUSIVE，M8:164-168） | m8_e2e：`trade_pnls_with_forced.sum()`（R 含浮盈，:1287）＋`significance.boot_ci95_lo`（block bootstrap 2.5 分位，:1288-1296）＋三态（:1297-1304） | 三态读法照 M8:168 | LCB 口径有效域：bootstrap 输入是已实现口径 ⟹ **否定保守正确**（LCB(已实现)≤0 ⟹ LCB(R)≤0 更强）；confirmed 须逐笔 R 分解序列（不存在，M8+ 精化项）——e2e-d2 §3.1 沿用；INCONCLUSIVE≠无 alpha（§5.6） |

### 3.3 合格判据列（693/MAXFULL 冻结列照录，禁改写）

- treasury：`Reach(Stage=III) > 0, Q_T > Q_0, W_T ≥ I_0, η_T ≥ L^{wc}+κQ_T`（MAXFULL M7 boxed，:156-159）。
- M8 四层：signal 双门 / execution `E[R(Π_exec)]>0` / treasury 四联 / total `R>0 ∧ LCB_OOS(R)>0`（M8:163-168）。
- 边界互斥（693 §1.4＋MAXFULL §6）：**任一对象（任一 M 段/任一层）的负结论不得外推到另一对象（另一段/层）**；有效域定理（`docs/formal-chain/有效域定理-20260704.md` 定理1）；禁「终局」措辞（M0 验收）。

### 3.4 费率标签与声明的强制位置（文档闸，缺 = 文档缺陷）

1. 报告头第二行：`**口径标签：[L1机制/费率未标定]**`（m8_e2e :1224-1228 / m6 :1107-1112 已实装，沿用；datum 注入后升 `[L2费率标定: datum 版本哈希]`，不得跳级）。
2. 层2 表头行注：TW 桥列注记（「TW 账本不经构造子见持盾成本，η=tw() 高估在险权益恰此量」，m6 :1108 先例）。
3. borrow 列/行：「非市场借贷利率」声明（C3 报告闸；perp 口径值≡0）。
4. 引用 LiqLoss/强平语义处：「ADL 未建模（venue 适配项）」随文注记（C1 文档闸；禁称「强平模型完备」）。
5. κ>0 行：「κ＝operator 声明式风险政策（不可识别性定理2），非价格导出、非回测择优」（附则A 报告闸）。
6. A10 全件 `[L0域外]` 标注（TS3:290；引用阶段×保证金耦合时另带「v0 全程逐仓口径（保守超集）」，C4 报告闸）。

### 3.5 产出物归档

`/tmp` 是缓冲不是档案：witness eprintln 全文进 witness 报告（c3 §7 先例）；`/tmp/m8_e2e_all_systems_oos.md`、`/tmp/m8_treasury_reach_distribution.md`、`/tmp/m6_btc_oos_r_decomposition.md` 跑批后移入 worktree `chanlun/review-results/` 归档，携带：HEAD commit、数据文件 shasum、命令行全文、墙钟、关②终验 dump 的产量构成行（provenance 交叉锚，§2.1-B）。

---

## 4. 依赖与前置清单

### 4.1 关② 终验结论未出前的占位清单（`⟨PENDING-G2⟩` 标记）

关② 终验重放（S=8 进程级 run 分片，p123:21 候选 B，在跑）＋关③ P3 收紧后重放验收未收口前，以下数字**全部留占位**，禁止从旧报告搬数充数：

- witness 全部数值行：n_orders、Σ已实现PnL、终 stage、双门距离、η 三行（原值/修正量/修正值）、W_T、Q_T；
- κ 网格全部行（新订单流下 n_orders 恒等性、线性步长都须重验）；
- 12 窗 Reach 分布全表；
- m8 四层报告层2/3/4 全部数值行与三态；
- 层1 signal 转引对象（终验后的 signal 层终判报告名与结论）。

**可先冻结的非数值件**（本 runbook 已冻）：判据列、口径标签与声明位置、归因分支结构、账本恒等/桥接/守卫断言、命令序列与 env 纪律、照实否定合格形态。终验通过后：witness/网格/多窗/m8 全量重跑，新旧对照入档（p121 H2:65＋T-N10:177——旧数值只作机制证据存档，「口径变更须重跑」m6margin-c2 §4 先例）。

### 4.2 费率 datum 未到位时各层声明口径（机制验证 vs alpha 结论）

| 层 | datum 未到位口径 | 禁 |
|---|---|---|
| (1) signal | 不消费费率 ⟹ 无标签义务；转引口径随关②终验 | — |
| (2) execution | **机制验证**：机制真实装＋守恒残差≈0（无泄漏物证）＋`[L1机制/费率未标定]`；net_r 是成本真实化证据 | 数值作 alpha 论据、作策略择优输入（附则B） |
| (3) treasury | η_corrected 是机制修正（C5）；修正量的费率成分未标定 ⟹ η 数值带同一标签；可达性是存在性命题（witness 合格形态 §2.5） | 「成本吃掉利润使三阶段更远」的方向陈述超机制陈述外推 |
| (4) total | R/LCB 数值同标签；三态照 M8:168 | INCONCLUSIVE 写成「无 alpha」（措辞 §5.6） |

datum 到位路径（附则B 裁决1）：四类 datum 用户供（带符号 funding 历史版本化/借贷曲线/清算费率表/ADL 规则史），注入后标签升级带 datum 版本哈希，不得跳级。

### 4.3 硬前置清单（执行前逐项核对，缺一不开跑）

1. 关② S=8 重放收口＋终验结论落盘（v3' dump＋验收行：terminal_confirmed ∈ 硬界 [748,1,213]、Pan=748 精确、877=110/122/31 精确）。
2. 关③ P3 收紧实装落地＋CERT 三栏（存续/失证/新增）验收（pan 裁定 P3 裁决4、P4-2）。
3. §2.1 前置实装：`M7_WITNESS_A10` env gate 接入三个 #[ignore] 测试＋witness η 三行增打（additive 一件）；T-N4 回归锁绿（κ=0 ∧ cost=None bit-exact）。
4. §5.2 m8_e2e η 列口径裁定 (i)/(ii)（agent-42 注记的待确认项）。
5. BTC 数据可达＋shasum 记录。
6. 零 cargo 纪律的解除时点：本 runbook 是纯文档；执行属后续任务，由编排者串行派发（p121 §6-8：实装关开跑前须确认重放收口）。

---

## 5. 风险与已知雷区

### 5.1 旧 witness 全卡 Stage I 的归因在新口径下的变化方向（**只作假设登记，不作预测**）

旧归因（c3/kappa-d1/e2e-d2 一致）：binding = **P3 RecoverCapital 门**（holding≥notional_in ∧ free≥退本目标），根因 = Σ已实现PnL < 0（signal 无方向 alpha 的下游）；κ 网格 {0,½,1,2} 全卡 I（κ 只门控 II→III，未触发）；12 窗（含牛市 wf1/wf2/wf3）全 I。新口径下三个变化方向，登记为假设、方向不预测：

- **(H-a) 成本通道使燃料更薄**：带 A10 成本后 Commission/Slippage 进 fill 侵蚀已实现 PnL（旧三窗 Comm+Slip 1.8e7~2.5e7 主导）；TW 账本不见持盾成本（G1 已闭为 η 修正），但 RecoverCapital 门读的是 holding/free——若关②新 BSP 集不改善已实现 PnL 符号，门距离大概率更远。量级不测。
- **(H-b) BSP 集合变化可改订单流符号**：关②修复（S0 取段坐标桥 113 案、S1a τ 门 37 案、037:20 破极值项）＋背书收紧（Trend 370 二/三类失证）改信号集合 ⟹ 订单流/fill 序列变 ⟹ Σ已实现PnL 可变号。方向不可测，只能实测。
- **(H-c) η 修正使 η 判据更紧、但只在 II→III**：η_corrected = tw() − ⌊funding+borrow+liq⌋（旧窗 funding ~2e5 量级）；EnterReady 第五项更难满足——但 binding 若在 P3（I→II），η 修正对终 stage 判读无数值影响（门没到）。修正只降不升（单向性可在报告中用作保守性论证）。
- **(H-d) κ 网格旧读数失效待重验**：「n_orders 跨 κ 恒 14472、barrier 距离线性步 −0.5·Q」是旧订单流旧注入面（env knob）读数；新注入面（env>config>baseline）＋新订单流下须重验，不得沿用旧表作新口径证据。

### 5.2 m8_e2e 的 η 修正影响（agent-42 注记的待确认口径）

- **现状缝**：m8_e2e 层3 η_T 列打的是**未修正** `tw.tw()`（wverify_run.rs:1283-1284），而 R 分解已带 `tw_holding_cost_bridge`（risk.rs:937；m6 报告 :1144/:1150 已消费）。C5 后 witness 新口径 = η_corrected——两处 η 口径不一致。
- **处置登记（执行前须裁定其一，本 runbook 不代裁）**：**(i)** m8_e2e 层3 η_T 列改打 `η_corrected = tw() − d.tw_holding_cost_bridge` 并并列原值（additive 报告列变更，与 witness 同源更可读）；**(ii)** 维持未修正列＋桥列并置，文注「η_T 为未修正值，修正量=桥列」（零代码改动）。
- **影响定量（单向性，可先行登记）**：修正只降不升（T-N4 语义，`ledger.rs:1241` 注释）⟹ 未修正口径下 `η_T≥η_*` 为 false 的，修正后必 false；未修正为 true 的须以修正后重判。旧三窗 η_T≪η_*（1448271/16543670 等）⟹ 修正**不改变旧结论方向**；改变的是「差多少」的诚实账目（把 G1 高估量从隐性变显性）。

### 5.3 其他雷区

- **/tmp 落盘非档案**（机器擦除即失）——归档义务见 §3.5。
- **墙钟不承诺**：c3 的 2.30s（10 万 bar）是旧机器旧口径读数；新口径带成本＋新 BSP 集、全量 461 万 bar O(n²) 小时级——报告只写实测值。
- **env 残留静默覆盖**：KAPPA_BARRIER_* 优先序最高，外部残留会静默把基线抬成正 κ——命令序列首行 unset 是硬步骤（§2.2/§3.1）。
- **数据版本**：btc_1m_full.json 是 symlink 指向主仓（只读消费）；执行时 shasum 记版本（数据未版本化是已知缺口）。
- **措辞纪律**：禁「终局」；INCONCLUSIVE≠无 alpha；M7/M8 负结论不外推到其他对象/标的/Θ（693 §1.4）；v3 硬禁令（费率数据只验证代码正确性）。
- **关②在跑期间禁开跑**：本 runbook 零 cargo 已守；执行关开跑以重放收口为前置（§4.3-6）。

---

## 6. 与旧报告/裁定的出入登记（差分表）

| # | 旧口径出处 | 新口径 | 差异性质 |
|---|---|---|---|
| 1 | m7treasury-c3 witness：零成本可行集（ThetaConfig::default()） | 带 A10 成本可行集 K（阶段 3a 定义）；数值基线 −562943 等作废存档（p121 H2） | 口径变更（路线图任务定义）＋前置实装（env gate） |
| 2 | m7treasury-c3/kappa-d1/e2e-d2：η_T = tw() 未修正 | η_corrected = tw() − cum_holding_cost（C5 实装落地）；c3「η_* 当前态」注沿用但以修正后重读 | 裁定＋实装（G1 闭合） |
| 3 | kappa-d1：env knob 单源注入 | env>config>baseline 三优先序写死（附则A；runner.rs:986 kappa_policy_resolved）；网格读法不变，「正 κ 无必要」的单调性推论在新订单流下重验（H-d） | 实装升级（接口冻结） |
| 4 | e2e-d2/maxfull-round1 四层表数值 | 全部作废存档（H2）；骨架列沿用＋新增 TW 桥列＋η 修正口径＋强制标签 | 口径变更（重跑重冻结） |
| 5 | maxfull-round1 §5.3：12 窗全 I ⟹「窗口选择偏差已排除」 | **该封死论证随订单流变化失效**——旧 12 窗表是旧 BSP 集旧成本口径读数；多窗 Reach 分布须在新口径重跑后方可恢复引用「牛市窗已覆盖」的排除论证。**这是与旧报告最大的出入** | 证据失效（依赖订单流的归纳论据） |
| 6 | maxfull-round1 §4：「Π_tested=Π_max-full 首次成立 ⟹ §5.3 不再适用」 | 终验后若 signal 层对象声明变化（关②改了 BSP/证书集），该注记的引用前提须复核后再沿用 | 口径合法性前提待复核 |
| 7 | 任务书提及 m7treasury-c2 | **主仓不存在该文件**（grep 仅 c3）——如实登记，不虚构引用 | 输入勘误 |
| 8 | 旧报告无强制标签 | `[L1机制/费率未标定]` 等六处强制位置（§3.4），T-N9 测试锁 | 裁定新增（附则B/C1/C3） |
| 9 | m8_e2e 层3 η_T=tw() | 口径缝（§5.2），执行前裁定 (i)/(ii) | 已知待确认项（agent-42） |

---

## 7. 本 runbook 验收线与纪律自检

- **文档闸（执行后适用）**：产出报告缺 §3.4 任一标签/注记/声明 = 文档缺陷（A10 各文档闸沿用）；出现「强平模型完备」「终局」「完整缠论已回测」= 090 违例。
- **停止规则（任一触发即停并如实报告）**：既有测试变红且无法在界内修复／塔路径 bit-exact 被破坏／桥接断言或账本恒等断言失败／关②终验与关③验收行出界（pan 裁定 P3-4 硬报警界 [748,1,213]）／数据源缺失。
- **本 runbook 自身**：零 cargo 调用；零 git mutation；主仓零写入（旧报告只读核对）；生产源码零改动（§2.1 前置实装与 §5.2 裁定为登记执行项，由编排者派发）；新增文件仅本文（worktree 内）。
- **谱系引用**：路线图 `mainline-merged-roadmap-20260717.md:47-52`（阶段 3 定义）；693 冻结两件（TS:38/44-54；MF:147-168/259-270）；A10 裁定 C1–C5＋附则A/B（G1 闭合、κ 接口、费率标签、T-N5）；pan 裁定 P1–P4（背书收紧与重放验收线）；p121 H2:65/T-N10:177（关②联动重跑）；p123:21（S=8 分片通道）；旧报告四件（c3/kappa-d1/e2e-d2/round1，作废存档口径）；161 号（照实否定合格）；231 号（L2 标注＋有效域）；674/#90（R/TW 账本不同构，桥接=对账不变量非统一账本）。
- **影响声明**：新增 `chanlun/review-results/p126-stage3-runbook-20260718.md`（本文）；未 commit（按约束）；引用未修改：上述全部文件。
