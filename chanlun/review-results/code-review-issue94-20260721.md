# code-review：#94（#75 评审硬违规修复）双轴独立评审

- 日期：2026-07-21
- 评审对象：#94（SPEC #73 A 线，#75 交付后双轴评审发现的 2 项硬违规——①交付报告声明不属实、②cross_agree 统计灌水——的修复票；已完工未提交，关票前评审欠账本件补上）
- 方法：matt code-review 双轴（Standards / Spec），只读评审——未跑 cargo build/test（另一工位独占 crate），全部结论基于 git diff + 静态读码 + 文档交叉印证。测试未自跑处逐项标注「未自跑」。
- 工作区：`/tmp/kimi-nest-mainline/rust/`（分支 kimi-nest-mainline-20260717），基线口径 1807 passed / 0 failed（给定，未自跑）。

## 0. 证据基座（照实登记：三份原文均未落盘）

| 文档 | 状态 | 后果 |
|---|---|---|
| #75 交付报告 | **未落盘**——`chanlun/review-results/` 无 issue75-*.md；scorecard:33-34 以「#75 交付 §1」转引其内容 | 违规①的原文表述无法逐字引用 |
| #75 双轴评审报告（含 2 硬违规原文） | **未落盘**——仅 v1-e2e-scorecard-20260721.md:33 一行概括：「评审（2026-07-21 双轴）发现 2 硬违规（声明不属实/cross_agree 灌水）→ #94 修复票」 | 违规表述只能从概括 + #94 修复点反推 |
| #94 修复说明/交付记录 | **未落盘**——无 issue94-*.md；scene-ledger.md:70 仅记「#94…卡全备妥」 | 本评审直接以 git diff 中 #94 改动面为对象 |

反推的票范围（由代码内 #94 标记自证）：修复1 = 输出行两行分工（runner.rs:3149 自标「Spec A1 字面错位消除」）；修复2 = cross 对照差剔除自证成分（runner.rs:7060 测试自标「#94（评审修复2）」）。

## 1. #94 改动面（git diff 摘出，区别于 #74/#75/#78/#76）

全部在 `rust/src/theta_v0/backtest/runner.rs`，共 6 处，零判定路径改动：

| # | 位置 | 内容 |
|---|---|---|
| 1 | runner.rs:963-964（结构体 doc）、:988-990（字段） | `NestGateStats` 增 `cross_reuse` 字段：「复用通道单独记账（#94）…『一致』是自证成分——不计入 agree/对照差三列，单列呈现」 |
| 2 | runner.rs:1024-1031 | `observe()` 复用分支：`obs.reused_old_xzd` 时只 `cross_reuse += 1`，否则才入 agree/old_pass_new_rej/old_rej_new_pass |
| 3 | runner.rs:1140-1142（字段+doc）、:1485-1487（打标） | `NestGateObs.reused_old_xzd`：typed 无证 ∧ 旧臂已落 Xzd 通道时打标（admit≡old_admit by construction） |
| 4 | runner.rs:1449-1460（`admit()` 文档）、:1488 | 「#94 择 (b)：保留重走」设计理据——旧臂 nest 通道从未执行 Xzd 评估 ⟹ 无可复用读出，必须重走 `build_xzd_fallback`；纯文档，行为 #75 已有（见 §3-Spec-2） |
| 5 | runner.rs:2375-2376（接线注释）、:3149-3155（输出注释）、:3172/:3185（CHAIN 行 `reuse={}` 列） | 两行分工：STATS=准入判定分账、CHAIN=真链命中/链深构成/对照差（含 reuse 单列） |
| 6 | runner.rs:7060-7120 | 新测试 `nest_chain_gate_cross_reuse_excluded_from_agree`（#94 唯一新增测试） |

非 #94 的同文件改动（甄别依据 = 标记与测试命名）：#74（nest_index.rs，另文件）、#75 本体（NestChainGate :1103-1530、typed_lookup :1419-1445、接线 :2371-2414、#75-T1..T5 测试 :6897-7123 一带）、#76（ExitNestGateStats/:1536-1620、出场接线 :4131/:4502、#76-T1..T3）、#78（nest_lifecycle.rs，另文件）、W1 声部执行（voice_exec 段）。rust/src 内其余「#94」命中（runner.rs:75/:3300/:8203、mod.rs:74、p93_invalidseed_probe.rs）均为历史 task#94（闭环引擎/p93 探针），与本票无关。

## 2. Standards 轴（修复是否真消除两项硬违规）

### 2.1 违规② cross_agree 灌水 → **已消除（已核验）**

- 灌水机制：#75 中 typed 无证 ∧ 旧臂 Xzd 复用案例 admit≡old_admit **by construction**（判定直接复用旧臂读出），其「两路一致」是自证，掺入 agree 抬高对照一致性。
- 修复属实：observe() 复用分支（runner.rs:1024-1031）将该通道从 agree/对照差两列剔除、单列 `cross_reuse`；CHAIN 行尾增 `reuse={}`（:3172/:3185）使剔除量账面可见、可复算（agree + old_pass_new_rej + old_rej_new_pass + reuse == total，口径闭合）。
- 测试坐实（未自跑，静态核验逻辑成立）：:7109-7119 断言 `obs.reused_old_xzd` 打标、`admit == obs.old_admit`、`cross_reuse==1` 且 `cross_agree==0`、对照差两列和为 0。
- 复用正当性声明核验：「旧臂 Xzd 读出 = build_xzd_fallback 同一单一来源」——econ_positive.rs:1584-1598 证实 `build_gate_certificate` 的 Nest-None 分支即调 `build_xzd_fallback`，与 admit() 重走/复用同函数同入参（hist 仅 nest 证构建用，Xzd 分支不消费）。声明属实。

### 2.2 违规① 声明不属实 → **已消除（带边界）**

- 修复方式：两行分工（runner.rs:3149-3155 自标「Spec A1 字面错位消除」）——STATS 行（:3159）只打印 total/admitted/rejected + nest_pass/xzd_pass + 五路拒绝分账；真链命中（typed_found/typed_none）、链深构成（admit/rej rungs）、cross 对照全部归 CHAIN 行（:3172），注释明写「链深构成归属 CHAIN 行，不在 STATS 行」。
- 当前一致性（已核验）：注释 :3149-3155 与两条 eprintln! 实际打印字段逐字对应，名实相符。
- **边界（照实）**：#75 交付报告原文未落盘，违规①的原始失实表述无法逐字比对——只能验证修复后代码/注释/打印三者一致，不能对原文验证「那句假话现已改对」。

### 2.3 发现

- **低｜issue77-v4-runbook-20260721.md:15**：仍写「cross 口径待 #94 修」，且仍按 #94 前口径描述「NEST_GATE_STATS 行含真链命中/拒绝分账」——与 #94 两行分工后的事实（真链命中归 CHAIN）不符。#77 未开工，开工前须回填，否则对照表会取错行。
- **信息｜runner.rs:1024-1031 口径边界**：flat_dir/no_level 早退候选的 `NestGateObs::default()`（old_admit=false 系未实际评估旧臂），(false,false) 仍计 `cross_agree`——严格说同为 by-construction 一致（自证成分）。但两侧早退语义逐字相同（:1453「与旧臂同语义」），且 #94 票范围明确限定复用通道，照实登记为口径边界，不判违规。#76 出场侧 ExitNestGateStats 对 flat 候选同型（runner.rs:1610-1620 一带），属 #76 评审面。

## 3. Spec 轴（票范围 / 门关 bit-exact / 禁第二查法）

- **Spec-1 票范围（已核验无越界）**：6 处改动全部落在统计记账/注释/测试，未触碰判定路径（typed_lookup :1419-1445、身份桥、门关直通 :2413 均非 #94 手笔）。无新语义引入。
- **Spec-2 无行为变更混入（已核验）**：复用通道判定行为 #75 已有——#75-T4（:7008-7058）测试「旧臂 Xzd 判定直接复用」；择 (b) 重走亦为 #75 既有——#75-T3(c)（:6978-7005）已断言 `obs.xzd_fallback` 且「回退判定 = build_xzd_fallback 单一来源」。#94 只加 reused_old_xzd 标记、分账与文档。
- **Spec-3 门关 bit-exact（已核验）**：门关路径 `None => step_gamma_trade` 直通（runner.rs:2413）逐字节不变；NG-E④ 回归锁（:6793-6836，门关 == 基线 n_orders/trades/strat_return/equity 四断言）在；门开才构建 NestChainGate（:2017-2021），门关零工作。#94 未改该段。
- **Spec-4 禁第二查法（已核验）**：门判定唯一来源 = `cert.certificate().n_delta()`（runner.rs:1432，nest.rs 递归核）；Xzd 复用与重走同出 `build_xzd_fallback` 单一来源（econ_positive.rs:1606）；L2 旧臂 nest 读出仅对照双读、不进判定（:2375-2376 注释 + admit() 代码路径一致）。
- **Spec-5 测试计数佐证**：1799（#75）+ 2（#78 T14/T15）+ 1（#94 唯一新测试）= 1802，与「#94 完工 1802/0」口径吻合；给定基线 1807 = 1802 + #76-T1/T2/T3 + 2（末两项未逐一核对，未自跑）。

## 4. 总判

**带边界的通过**。

- 两项硬违规的修复均真实落在代码上且口径闭合（§2.1 已核验；§2.2 修复后名实一致已核验）。
- 无 Critical/High 发现；低 ×1（runbook:15 过期口径，#77 开工前回填）；信息级口径边界 ×1（flat/no_level 的 agree 成分，票范围外）。
- 边界三条（照实）：(a) #75 交付报告/评审报告/#94 修复说明三份原文均未落盘，违规①只能验证修复后一致性、不能对原文逐字验证；(b) 测试未自跑（crate 他工位独占），1802/0 与 1807/0 均为给定口径，本评审静态核验测试逻辑成立；(c) runbook:15 回填前，#77 取数口径存在取错行风险。
