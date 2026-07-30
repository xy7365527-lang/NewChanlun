# #644 影子评审（两轴）——LEE 归因只读层接线 + wverify 死文件清理 + 余项

- 日期：2026-07-29
- 票据：GitHub Issue #758（影子评审）；被评对象 = merge `bcf83cf386` 相对第一父的 diff
  （`f601bea180` 接线 + `51b956fa6a` 死文件清理 + `e26b9827df` 报告）
- 工位：`/private/tmp/wt-758`，分支 `shadow/644-review`（main 尖端含被评 diff）；参照面
  `/private/tmp/kimi-nest-mainline`（全程只读，仅 `sed`/`grep`，未 build/未 test）
- 执行：本会话前台单线程，禁子代理；本报告为**独立评审**，评审器不是 #644 交付方
- 测试复跑：`cd rust && cargo test --lib` = **2595 passed / 0 failed / 138 ignored**
  ⟹ 与 #644 报告申报的「2595/0/138」**逐位吻合**
- 附加复验：`cargo check --all-targets --features backtest_bin` = **0 error**（Finished）
  ⟹ #644 报告「意外发现」节的 CI 腿转绿申报**属实**

---

## 判词

| 轴 | 判词 | 一句话理由 |
|---|---|---|
| **Standards（代码/工程）** | **PWC** | 只读边界在代码层**真成立**（M2/M4 零触碰经全仓 grep 坐实、bit-exact 有机器锁、M1 段与 kimi 逐字同构）；但死文件四判的依据与实际不符、只读层进入了不消费其产物的净额路径、非平凡性未进回归锁。 |
| **Spec（票面/裁定）** | **FAIL** | 删除的 `m8.rs` / `issue71_chi_gamma.rs` 是 #71 χ 线撤销裁定登记在册的 A6/A8「诊断件保留（**禁删**，删须**编排者终审**）」两件，两文件自身头部明载该纪律；#644 未识别冲突、未上浮终审即删，且报告对文件头做了**选择性引用**。另有 #693 裁定①「与 kimi 读数对拍」被单方降级、裁定②冲突清单「0 条」依据不足。 |

发现计数：**HIGH 1 / MED 6 / LOW 3**。

---

## 抽核六点逐点结论

| # | 抽核点 | 结论 | 相关发现 |
|---|---|---|---|
| 1 | 只读边界（新锁覆盖面/强度、接线点隐性回写、M2/M4 零触碰） | **成立（强度有保留）** | MED-3 / MED-5 |
| 2 | 语义重放保真（三段 ↔ kimi 原位点等价论证） | **两段 PASS，一处取数源不等价未登记** | MED-4 |
| 3 | 死文件四判 | **判词依据不成立 + 越权删除** | HIGH-1 / MED-1 / MED-2 |
| 4 | 对拍口径（三锁替代逐值对拍的充分性、降级声明是否如实） | **降级理由如实，但证据不可复现、未回裁** | MED-5 |
| 5 | 附带 CI 修复（theta_overlay.rs）+ #289 LOW-1 移植正确性 | **PASS（逐字核对通过，0 error 复验通过）** | — |
| 6 | Spec 轴（票面四件 + #693 两裁） | **item 2 越权、裁定①降级、裁定②清单依据不足** | HIGH-1 / MED-5 / MED-6 / LOW-1 |

### 抽核点 1 展开：只读边界确实成立

正面核实（非采信报告 prose）：

- **M2/M4 零触碰**：`grep -rn "level_order\|level_risk\|plan_level_gated\|LevelOrderLedger\|LevelRisk"
  src/theta_v0/backtest/ src/bin/` 全部 7 处命中**均在注释里**，零代码引用。M2/M4 真零触碰。
- **归因产物全部读点**（`fill.rs` grep `level_ledger|level_clock|level_attrib|clock_ticks|pan_levels|lstep|standard_p_star|forced_flat_anchor`）：
  写侧只有三个 `u64` 计数器自增（`fill.rs:4243-4248`）、`level_clock_stats.observe`（`fill.rs:4230`）、
  `ll.step`/`ll.force_flat`（`fill.rs:4273`/`4840`）；读侧 `standard_p_star` 只读消费（`fill.rs:4240`），
  `attribute_total` 的 `_out` 丢弃。**无一处回写 `order`/`cash`/`units`/`p_t`**。确认。
- **bit-exact 机器锁**：`runner.rs:4236` `lee_readonly_layer_does_not_perturb_net_result_644` 在 60-bar
  合成数据上比 `n_orders`/`trades`（逐字段 `PartialEq`）/`strat_return`，全绿。
- **M1 段等价性**：main `fill.rs:4269-4290` 与 kimi `fill.rs:1645-1666` **逐字同构**（仅注释差异）；
  `forced_flat_anchor` 段 main `fill.rs:4826-4844` 与 kimi `fill.rs:1948-1968` **逐字同构** ⟹
  #289 LOW-1 移植正确性 **PASS**。
- **M3 collect_ticks 段**：main `fill.rs:4201-4229` 与 kimi `fill.rs:1571-1599` 逐字同构，
  仅两处适配：`step_trace.closed` 三元组 vs kimi 二元组（纯类型适配）、`pan_levels` 取数源（见 MED-4）。

---

## 发现清单（分级 + 文件锚）

### ⚠ HIGH-1｜删除 `m8.rs` / `issue71_chi_gamma.rs` 违反 #71 χ 线撤销裁定的「禁删 + 删须编排者终审」

**锚**：`rust/src/theta_v0/backtest/wverify_run/m8.rs`（已删，见 `bcf83cf386^1`）、
`rust/src/theta_v0/backtest/wverify_run/issue71_chi_gamma.rs`（已删）、
`chanlun/review-results/prob-inference-disposition-registry-20260728.md:13/26/27`、
`chanlun/escalate/chi-line-falsification-ruling-20260728.md:38`、
`chanlun/review-results/issue644-lee-attr-wiring-20260729.md:82-84`

处置登记册与裁定原文逐字：

- 登记册 `:13`：「**禁删**：所有 15 件均不删除，仅在文件头/函数头落档标注释（裁定链接 + 日期 +
  一句理由），零行为变化」；`:6`：「本票纯机械登记与标注，不重裁……禁 git mutation，禁删（#225 先例）」
- 登记册 `:26` **A6** = `wverify_run/m8.rs` 的 `run_q4_fullpi_policy` 函数头，处置「诊断件保留」
- 登记册 `:27` **A8** = `wverify_run/issue71_chi_gamma.rs` 模块头，处置「诊断件**永久**保留」
- 裁定 `chi-line-falsification-ruling-20260728.md:38`：「**禁删**，**删须编排者终审**，#225 先例」

两件的标注**已经落在被删的这两个文件里**——`m8.rs:52` 函数头原文「诊断件保留（禁删，历史裁决
q4 π^full INCONCLUSIVE 照旧有效）」；`issue71_chi_gamma.rs:5-7` 模块头原文「诊断件**永久保留**
（**禁删**，历史证据不因仪器撤销而失效）」。#644 直接删除，无编排者终审记录。

**加重情节（引用完整性）**：#644 报告 `:84` 对该文件头的引用是「其验证对象 χ 门本身已被
`chi-line-falsification-ruling-20260728.md` 正式撤销（**文件自己头部承认**）」——文件头承认
「χ 门已撤销」的**同一句话紧接着**就是「诊断件永久保留（禁删）」。报告只取前半句作为删除依据，
略去后半句这条正面禁令。报告随后的辩护（「纪律由 git 历史 + kimi 树满足」）是执行车对一条明文
「删须编排者终审」的**自行重裁**，正是登记册 `:6`「不重裁」所禁止的动作。

**票面 item 2 的泛授权不能覆盖它**：#644 票面 item 2 写「随本票清理或声明启用」，但 #71 裁定
（2026-07-28）在其后且更具体，并明文把删除权保留给编排者终审。两条纪律相撞时，本票的正确动作
是上浮裁决而非自行选择。

**建议处置（评审器不代裁）**：回滚 A6/A8 两文件的删除（`git checkout bcf83cf386^1 -- <两路径>`，
按登记册在文件头落档标注释即可），或补一次编排者终审并把终审结论写进 #644 留痕；`report.rs`/
`tests.rs` 不在 15 件清单内，不受本条约束（但受 MED-1）。

---

### MED-1｜`report.rs` / `m8.rs` 的「逐字重复、零净增能力」判词只对各 2 个函数成立

**锚**：`chanlun/review-results/issue644-lee-attr-wiring-20260729.md:81-82`

报告表格给 `report.rs`（660 行）的全部理由是「main `wverify_run.rs:35/46` 已含逐字同名的
`force_state_code`/`force_state_label` — **纯重复，零净增能力**」；给 `m8.rs`（716 行）的理由是
「`wverify_run.rs:918/927` 已含逐字同名的 `q4_shift_back_6m`/`q4_prev_day`」。

实测（对 `bcf83cf386^1` 的两文件顶层符号逐个 `grep` main `wverify_run.rs`）：**20 个符号在 main 侧
零命中**——

- `report.rs`：`layer4_cells` / `layer4_random_control_cells` / `layer4_scalar_caliber_notice` /
  `layer4_verdict_line` / `SCALAR_UNDEFINED_UNAVAILABLE` / `m8_execution_projection_label` /
  `m8_fee_audit_headings` / `format_m8_fee_audit_row` / `m8_layer23_settlement` / `lee_row_cells`
- `m8.rs`：`parse_fee_datum_spec` / `M8_LEVEL_CAP_WEIGHTS_310` / `apply_m8_level_cap` /
  `resolve_m8_symbol` / `m8_epistemology` / `assert_m8_audit_coverage` / `m8_windows` /
  `resolve_m8_report_path` / `run_q4_fullpi_policy`

`m8.rs` 一行还给了第二个（更强的）理由「能力已内联在 main `m8_e2e_all_systems_oos`」——那条对
M8 e2e 报表主体成立，但对 `m8_windows`（窗口表）/`m8_epistemology`/`apply_m8_level_cap` 等参数化
入口不成立（main 内联版是硬编码路径）。`report.rs` 一行**没有**给出任何「能力已内联」的论证。

删除本身对编译/测试**零影响**（四文件确为未声明模块，`backtest/mod.rs:63` 只 `mod wverify_run;`
指向单文件 `wverify_run.rs`）——本条不是行为缺陷，是**判词依据失实**：以 2 个同名函数为据给一个
25 符号文件盖「纯重复」章，会让后续读者误以为该处置已被逐符号核过。

### MED-2｜删 `report.rs` 连带删掉 `lee_row_cells`，抬高第二档（#755）接线成本，未作跨票登记

**锚**：`rust/src/theta_v0/backtest/wverify_run/report.rs:378`（已删的 `lee_row_cells`，消费
`strategy::level_order::LevelOrderStats`）、`rust/src/theta_v0/strategy/coverage/sizing.rs:248-256`
（本次 diff 把「四层均已在场」改成「三层均已在场」）、#693 备料补充评论

#693 的备料补充（编排者贴，2026-07-29）明确把 M4 接线的已在场资产盘为四层：「字段本身、其统计
`n_cap_narrowed`、逐级 sparsity 判据、**m8 报表列**四层均已在场——缺的只是唯一填入者」，并据此判
「M4 级别帽接线的实际成本比 #642 执行器申报的**小**」。

#644 删掉的 `report.rs::lee_row_cells` 正是那第四层（LEE 行的 m8 报表单元格生成器）。#644 的处置
是把 `sizing.rs` 的 doc 从「四层」改写为「三层……本条不再指向它们」——即**用改文档的方式吸收了
资产减损**，但没有把「第二档接线成本因本票删除而回升」作为跨票影响上报 #755 / #693。

本条与 HIGH-1 独立：即便 A6/A8 回滚，`report.rs` 不在 15 件清单内，仍会被删；`lee_row_cells` 的
去留应由 #755 而非 #644 决定。

### MED-3｜`level_clock` / `level_attrib` 无门控地进入了不消费其产物的净额路径

**锚**：`rust/src/theta_v0/backtest/fill.rs:3568`（`level_clock_stats` 无 `Option`）、
`fill.rs:4201-4249`（collect_ticks + attribute_total 逐 bar 无条件执行）、
`fill.rs:730`（`pi_theta_fill_loop` wrapper，`level_ledger=None`）、
`rust/src/theta_v0/backtest/runner.rs:115`（`RunResult` **不含** `level_clock`/`level_attrib_*`）

`level_ledger` 有 `Option` 门控（`None ⟹ 整段跳过`，bit-exact 锁），但 `level_clock` 与
`level_attrib` 两段**无门控**。后果：纯净额臂 `run_theta_v0_pi` → `pi_theta_fill_loop`
（`overlay=None`、`level_ledger=None`）每个可交易 bar 仍会执行

- 7 个 `Vec<u32>` 收集 + `collect_ticks` 的 `BTreeSet` 插入（`level_clock.rs:279-311`）
- `level_nets(sep_legs)` 一次 + `attribute_total` 一次（含 `Vec::with_capacity` ×2、
  最坏情况一次 `sort_by`，`level_attrib.rs:60-102`）

而这些读数在 `RunResult` 里**没有出口**，全部被丢弃。数值上 bit-exact 无碍（已由回归锁坐实），
但这是**纯浪费**，且量级不小：BTC 全量 ~460 万 bar 尺度上每 bar 约 8 次堆分配。

#644 报告 `:58-60` 为「无门控」给的理由是「kimi 的 `level_clock_stats` 同样无门控，纯 O(1) 累加，
不构成成本/风险」——但 kimi 侧 `pi_theta_fill_loop_overlay` 是其**唯一**主循环且读数被消费；main
侧存在三个入口（净额 wrapper / voice wrapper / overlay），其中两个不消费。「逐字重放 kimi」在这
一点上把 kimi 的前提条件一并丢了。**建议**：给两段加 `if level_ledger.is_some()` 或独立 `Option`
门控（也会让「三读数同一决策点集合」的口径断言更干净）。

### MED-4｜`pan_levels` 取数源与 kimi 不等价（kimi 多一层 `Candidate` 过滤），报告称「语义等价」

**锚**：`rust/src/theta_v0/backtest/fill.rs:4076-4077`（main：`prepare` 之后**无条件** push）、
kimi `rust/src/theta_v0/backtest/fill.rs:1424-1431`（kimi：`match prepare { Candidate ⟹ push,
Record(_) ⟹ 不 push }`）、`chanlun/review-results/issue644-lee-attr-wiring-20260729.md:41`

kimi 的 `pan_candidates` 只收 `PreparedPanDiv::Candidate` 分支；`Record(_reason)` 分支（kimi 注释
原文：「无合法 parent/lot identity 时不伪造候选」）**不入**该 Vec。main 侧 `prepare` 在 #282 收缩后
已无 `Candidate`/`Record` 二分，`pan_levels.push` 无条件执行 ⟹ main 的 PanDivCert 通道是 kimi 的
**真超集**（多收「首见过门但无合法 parent/lot」的那部分）。

#644 报告 `:41` 的表述是「语义等价（"首见并过门"判据相同）」——「首见并过门」这半句成立，但
kimi 侧还有第三个条件，报告未登记。公允地说 main 版**更贴合** `level_clock.rs` 模块头对该通道的
文字定义（「E6 结构：该级盘整背驰证书首见并过门」），所以这**不是 bug**；问题是它被当作「等价」
而不是「本票选择的口径偏离，理由是 X」上报——而 #693 裁定②恰恰要求「与 kimi 的出入点逐点列
清单（点位/两边形状/取谁的理由）随票上报」。这一点位应进清单（见 MED-6）。

实测影响面待定：默认 `ThetaConfig` 下 pan_div 惰性（`level_attrib.rs:41` 原话），该通道很可能实测
恒空——但 §件4 的 BTC 跑批只报了「106 结构钟」总数，**未按 7 个通道拆分**，无法确认 PanDivCert
通道是否响过。

### MED-5｜对拍证据不可复现；回归锁未固化非平凡性；#693 裁定①的「对拍」被单方降级未回裁

**锚**：`chanlun/review-results/issue644-lee-attr-wiring-20260729.md:136-174`（§件 4 全节）、
`rust/src/theta_v0/backtest/runner.rs:4262-4275`（回归锁的三条断言）、#693 Resolution ①

三层问题：

1. **证据不可复现**。§件4 的三条不变量读数（`obs=14386 max residual=0 max|N|=15603`、
   `14386/106/11`、`14386/0/2533`）全部来自一次性手跑，本次 merge 的 9 个改动文件里**没有**任何
   载体——无脚本、无 `#[ignore]` 测试、无命令行留痕。报告也没写出复跑命令。这三条数字目前
   **只能采信，不能复核**，而它们是本票「结构不变量复现」这一整套替代论证的**全部经验基座**。
2. **回归锁的非平凡性断言被削弱**。§件4 自己定的判据是 `identity_witnessed()`（残差 0 **且**
   `max_abs_net > 0`）与 `sparsity_witnessed()`（`0 < n_structural < n_decisions`），但入仓的
   `lee_readonly_layer_does_not_perturb_net_result_644` 只断言 `w.n_observations > 0` 与
   `ov.level_clock.n_decisions > 0`（`runner.rs:4266-4268`）——这两条**只要 loop 跑过就必然成立**，
   即使 60-bar 窗口内从未开过仓、结构钟一次没响。两个 witness 方法都已存在且零成本，未被调用。
3. **裁定条款被单方降级**。#693 Resolution ① 原文对第一档的要求是「只读不动决策，**与 kimi 读数
   对拍证等价**」。#644 把它换成「结构不变量复现」。降级的**技术理由站得住**（两树在「接不接
   M2/M3 决策门控」处已分叉，逐值比较分母不同源，报告 `:144-149` 论证清晰且如实），但替换一条
   编排者裁定写死的验收手段应当回裁确认，而不是在实施报告里自行改口径。**这条与 HIGH-1 是同一
   类问题的两次发生**：遇到上级纪律与本票便利冲突时，选择自裁而非上浮。

### MED-6｜「接口冲突清单 = 0 条」的核查方法不足，且与件 1 表格自认的出入点口径矛盾

**锚**：`chanlun/review-results/issue644-lee-attr-wiring-20260729.md:177-183`

报告的核查方法是 `git log --oneline --all | grep "#<n>"`（**只看 commit 标题**），据此断言
#355/#363/#369/#376「全部集中在 `level_order.rs`」。实测 `git log --all --grep "#<n>" --name-only`：

| 票 | 实际触及的 rust 文件 |
|---|---|
| #355 | `backtest/fill.rs`、`strategy/level_order.rs`、`strategy/coverage/sizing.rs`、`sizing_tests_2.rs` |
| #363/#369/#376 | 同上 **＋ `backtest/runner.rs`** |

即四票**都触及 `backtest/fill.rs`，其中三票触及 `backtest/runner.rs`**——而这两个文件正是 #644
的主改动面。终点结论（「零一处触及 `level_ledger.rs`/`level_clock.rs`/`level_attrib.rs`」）经复核
**仍然成立**，实际冲突大概率确为 0；但「全部集中在 level_order.rs」这个中间陈述不属实，用它推出
的「与本票零交叠」低估了需要核的面。

口径矛盾：报告件 1 表格 `:41` 自己把 pan 取数源标为「**接口形状对齐但取数源不同**」（即一处两边
出入点），而 `:183` 的清单结论是「条数 = 0」。按 #693 裁定②的清单定义（点位/两边形状/取谁的理由），
这一条应当在清单里（见 MED-4），清单不应为空。

### LOW-1｜h12 判词「移植目标函数在两侧都不存在」失实

**锚**：`chanlun/review-results/issue644-lee-attr-wiring-20260729.md:126-130`

报告称「移植目标函数在**两侧都不存在**，不是"取舍"问题，是移植对象已消失」。实测 kimi 侧
`plan_and_fill_mtm` 在 `rust/src/theta_v0/backtest/fill.rs:2278` **存在**，h12 测试
`short_leg_end_to_end_trade_marked_short` 在 `rust/src/theta_v0/backtest/runner.rs:6504` 也存在；
只有 main 侧因 #499 v1 引擎退役而零命中（已复核确认）。

结论（放弃移植）与理由主干（main 无宿主函数）**不受影响**，仅「两侧都不存在」这句需订正为
「main 侧已独立退役，移植对象在 main 无宿主」。

### LOW-2｜`n_rescaled` ⊇ `n_residual`，两个计数并列呈现未说明包含关系

**锚**：`rust/src/theta_v0/strategy/level_attrib.rs:66-71`、`rust/src/theta_v0/backtest/fill.rs:4243-4249`、
`rust/src/bin/theta_overlay.rs:122-125`

`attribute_total` 的残差桶分支 `return (out, true, true)` —— residual 与 rescaled **同时**为 true。
故 `level_attrib_n_rescaled_bars` 恒 ⊇ `level_attrib_n_residual_bars`。CLI 打印「决策点/残差桶/缩放」
三数并列、报告 §件4 写「0 落残差桶 / 2533 经比例缩放」，均未说明第三个数包含第二个。实测
residual=0 时无歧义；一旦残差桶非零，读者会把同一批 bar 数两次。属上游算子既有返回约定，本票
新增了消费点即应在文档里点明。

### LOW-3｜`forced_flat_anchor` 无条件计算，纯净额路径新增一次 O(n) 反向扫描

**锚**：`rust/src/theta_v0/backtest/fill.rs:4826-4830`

单源化重构把 `(0..n).rev().find(...)` 提到两个 `if let Some(...)` 之外，overlay 与 level_ledger
**均为 None** 时（净额臂/voice 臂）也会扫一遍 `bars`。与 kimi 原版一致（kimi `fill.rs:1952` 同样
无条件），数值零影响，最坏 O(n) 一次；登记备查，不建议为此偏离 kimi 形状。

---

## 未发现问题的部分（正面登记）

- **只读边界的核心主张成立**：M2/M4 在 `backtest/`+`bin/` 下零代码引用；三段接线体内无一处赋值
  到 `order`/`cash`/`units`/`p_t`；`attribute_total` 是无状态纯函数，`_out` 丢弃。
- **M1 段 + `forced_flat_anchor` 段与 kimi 逐字同构**，`#289 LOW-1` 移植正确。
- **`theta_overlay.rs` 的 CI 修复属实**：`cargo check --all-targets --features backtest_bin` 复验
  0 error；删除的 M2 打印段消费的 `r.level_order` 字段在 main `OverlayRunResult` 确实从未存在
  （非本车引入的能力削减，报告如实登记）。
- **`cargo test --lib` 计数申报属实**：2595/0/138 复跑逐位吻合；基线 2444→2594 的「移动靶」解释
  与 #614 先例一致。
- **#571 放弃移植的论证充分**：`fill.rs:3590-3594` 的 `codex a9-posnode 裁定 C` + `gen_hiwater`
  高水位表确为 main 已裁决的替代设计，kimi 的 `drain_carrier` 重写属决策路径复议，越出只读边界，
  判独立跟进项恰当。

---

## 关票前建议处置（按优先级）

1. **HIGH-1 必须处置**：回滚 `m8.rs`/`issue71_chi_gamma.rs`（登记册 A6/A8）的删除，或补编排者
   终审并把终审结论写入 #644 留痕。在此之前 Spec 轴不应判过。
2. **MED-5 第 2 点零成本**：把回归锁的两条弱断言换成 `w.identity_witnessed()` 与
   `ov.level_clock.sparsity_witnessed()`（方法已存在），并在 §件4 补上 BTC 跑批的可复现命令
   或落一个 `#[ignore]` harness。
3. **MED-3 一行门控**：给 `level_clock`/`level_attrib` 两段加 `level_ledger.is_some()` 或独立
   `Option` 门控，把只读层从净额/voice 两臂摘出去。
4. **MED-1/MED-4/MED-6/LOW-1 为留痕订正**：不改代码，订正 #644 报告对应段落的依据表述，并把
   pan 取数源那一处补进 #693 裁定②要求的接口清单（清单条数从 0 改 1）。
5. **MED-2 上报即可**：把 `lee_row_cells` 的删除作为「第二档接线资产减损」登记到 #755。
