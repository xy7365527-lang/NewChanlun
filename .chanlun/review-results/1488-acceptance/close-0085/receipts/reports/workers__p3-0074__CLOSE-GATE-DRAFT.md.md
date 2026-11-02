# 收据 workers/p3-0074/CLOSE-GATE-DRAFT.md

- 原件：`workers/p3-0074/CLOSE-GATE-DRAFT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`65b0db7680a9fd2aa36dffea385671fa195af6f1e4fda5fd1c19207d1435e009`
- 字节数：24582；行数：135

## 关键行（行号: 原文）

```
1: # #1488 关票门材料草稿（0074）
3: > **只陈述现状，不勾选验收框，不关票，不发布。** 截至 2026-10-21，候选源头是 `3ee05a2713`（分支 `codex/1488-budget-full-chain-fix-v2-20261018`，0072；非作者独审 0073 PASS，全锁 92/92）。
7: ## 0. 结论摘要
12:   3. `3ee05a2713` 上没有 CI run（最近一次 CI 39481122836 在 `44d312d7cf`）；
16: - 按 #1612 第三稿（独审 PASS，待用户批复）的拆分：**A② 整条、A③ 整条、A④ 后半句、R2 后半句、外部行为句**转出到 #1612 切片 3/5。**A①、A④ 前半句（对拍锁）、A⑤、R1、R2 前半句（testonly 增量与锁）、R3（或随 R2 转出）**留在本票（§7）。
18: ## 1. 关票门逐条现状
24: | G1 resolution 引用落地 commit | resolution 要写实际落地的 hash | 候选源头 `3ee05a2713`；它的祖先已覆盖冻结源 `fff166540e`、`62bad42a1b`，merge `57f9680c4e`（并入 main `6badaa6049`），r14 `c2bdf9b662`，S3 `bba547492d`，0059 `460f7a6cfd`/`998ce0218b`，0067 `7765c998f8`/`44d312d7cf`（本工位已用 `git merge-base --is-ancestor` 核过） | **D9 squas
25: | G2 已并入 main | 合不进 main 不关票 | 只做过「main→本票」方向的 merge（`57f9680c4e`，父 `fff166540e` + `6badaa6049`；0052 RESOLUTION_APPROVED_MERGE_BLOCKED；r14 阻断由 `c2bdf9b662` 解除，0054 APPROVED_WITH_FINDINGS） | **本票未进 main**。D8：合入时点在 S7，走用户合入闸；合入前要和当时最新的 main 再合一次，新冲突的解法要非作者独审 |
26: | G3 关票评论各声明此刻为真 | 不能预写尚未执行的评审、测试或结论 | — | 关票评论要等 S5/S6/S7、CI、#1612 关闭都完成后才能写；本草稿里「待…」的项一律不能进关票评论 |
27: | G4 多 commit 逐 commit 可编译 | 多 commit 交付要逐个声明 | D9 选 squash 成一个可编译提交 → 单 commit 交付，**不触发**逐 commit 列举 | 声明里写「单 commit（squash）交付」，并附 squash 面的构建/CI 结论 |
29: | G6 报告入 git，仓内相对路径 | /tmp 和绝对路径视同未入仓 | 已入仓：`.chanlun/review-results/1488-acceptance/min12/`（S2，`b6a5a3e46c`/`0803b2a71e`）；`.chanlun/review-results/1488-acceptance/degrade-0065/`（0065 输入、工具、`results/s86-nat2-vs-r8a.json`、`results/min12-arms-*.json`、`min12-runs-summary.json`，`78523300ad`）；锁表 `.github
31: | G+ CI 把关面 | 改了 `rust/`、`.github/`、`scripts/` 等，要附 `origin/main` SHA 和 CI run 链接，红灯照实写 | 历次 CI：39364508567（`fff166540e`，80/80）、39370285846（`57f9680c4e`，79/80）、39373166910（`c2bdf9b662`，80/80）、39384343438（`bba547492d`，84/84）、39481122836（`44d312d7cf`）；runner 39368457887（六 driver 6/6） | **`3ee05a2713` 
33: ## 2. 票面五个验收框：D1–D11 裁定后的口径与现状
35: 裁定出处：D1–D11 在 6283355919；A③ 在 6283982007、6298134855。
37: | 框 | 票面原文要点 | 裁定后口径 | 现有证据 | 缺口 |
39: | A① 段 A 逐位 | S86 全 86 条与 r6/r7 逐位相同（`compare_r7_r6.py` 口径） | **D1**：合并面 #1488 二进制对 main `6badaa6049` 完整路径二进制，在 S86 上逐位对拍；基线 r8 双跑一致后冻结哈希；对拍脚本重写，并由非作者独审 | S1（`codex/1488-accept-s1-20261020` `fbddd3584b`）：r8 双跑 PASS（INVARIANT 31/31，CONDITIONAL 5/5），基线二进制 sha `c944bd6b…`，单跑约 4372s。0065：testonly-`1660d
41: | A③ MIN12 | 52 根经正式会话入口，10s 帽内，第 51 条起 FORMED/EXTENDING，读数与 oracle 一致 | 6283982007：改为「52 根全程 10s 帽内、读数与 new-bi/2 oracle 逐位一致」，**FORMED seed 条件转 #1419**（等中枢合入会成环）。6298134855：**oracle 改用 main `6badaa6049` v3 臂**（同一二进制 `c944bd6b`，已双跑一致） | 0065：MIN12 testonly native 52/52 Committed；`compare_arms.py` 的 
42: | A④ 增量≡全量锁 | #804 对拍锁 + 变异体；旧完整路径只能作离线对照臂，不参与准入（#799） | 前半句（对拍锁）留本票；后半句（#799 退场）第三稿转切片 5 | S4 锁 `s4_generation_incremental_matches_cold_full_six_stages`（`a89996a487` PASS，28 代）；S3 四条锁加 N1–N5 负控；0067/0072 的锁表登记；全锁 92/92（0072 远端）。#799 审定：生产唯一生成路径是 delta=None 完整重算，增量只在 testonly 下 | ① 锁要在终面的 CI 上跑（CI 只
43: | A⑤ 正式入口+查询+浏览器 | 正式会话入口 + 公共查询 + 浏览器核对 | **D3**：用测试客户端 tb01c / tb03a / s-content-v3；产品浏览器归 #1460；#1497 是否阻塞由负责人核实后写明 | 0065 MIN12 Q3 查询走 `/api/v3/snapshot`（`b647f2d29f`），查询资源档在 `.chanlun/review-results/1488-acceptance/degrade-0065/min12/tools/query-resources-*.json` | **没有找到 tb01c/tb03a/s-content-
45: ## 3. 与 #1612 的阻塞关系
52: ## 4. 十七处裁改断言清单
54: | 处 | 测试（锁） | 提交 | 裁定 |
59: | 4 | `audit_owner_write_budget_retains_candidate_within_retention_cap_and_parent_limit`（原 write_budget 锁改名）；**最终形态：`handoff_peak − entry ≤ candidate.limit`**（0045 P3-B / R-21 要求登记，P18） | `f49db3d7e7`、`cc5e324736` | 6281614540 (i)；派发 6282365750；runner 锁名 6283069810 |
60: | 5 | `r14_source_keeper_seal_and_next_request_share_explicit_parent`（①来源合同额与 P2-A 前封存基准）：期望值改由 tb02b 合同字符串长度推导，保留 803 节点交叉核；负控：还原两字符串后回到 111,334，推导式只改一处即变红（这个读法待主控确认，P28） | `b8208e722d`（头 `c2bdf9b662`） | 6283672661 |
77: ## 5. 冻结与合入形态（D9 / D8 / D10）
79: - **D9**：squash 成一个可编译提交合入 main。冻结分支 `codex/1488-recover-journal-frozen-20261020`（`fff166540e`，CI 39364508567，80/80，0045 终审 P1=0/P2=0）、`codex/1488-unified-ownership-v4-frozen-20261020`（`62bad42a1b`）和合并面分支**保留不删**，在关票声明里列出，并要和 G5 白名单登记一致（§1 G5）。
80: - 合并面分支：merge `57f9680c4e` 所在的分支；当前推进分支 `codex/1488-budget-full-chain-fix-v2-20261018`（头 `3ee05a2713`）。关票声明要写明，以哪个分支哪个 hash 作为 squash 源。
84:   `关票声明：resolution=<squash hash>，已并入 main；本评论各项在此刻均已为真；逐 commit 可编译性=单 commit（squash）交付；工位=<已删清单>，白名单保留 codex/1488-recover-journal-frozen-20261020@fff166540e、codex/1488-unified-ownership-v4-frozen-20261020@62bad42a1b、<合并面分支>@<hash>（D9）；报告=.chanlun/review-results/1488-acceptance/…；评审面==关票面（尾部 <hash> 
86: ## 6. 切片计划进度（MAP 0048 S1–S7）
92: | S3 | 残留 append、stage_indexes/assemble | 完成（`bba547492d`，CI 39384343438 84/84） |
94: | S5 | 终面 S86 双跑 + 对 r8 对拍 + g=10..86 曲线 + 不可达统计（+ 8MiB 峰值读数） | **未做**。0065 有 testonly 版的部分读数（S86 vs r8a PASS，不可达 38.5% 对 56.1%），不能替代终面 |
98: ## 7. 验收归属映射（按 #1612 第三稿切片；第三稿独审 PASS，**待用户批复**）
109: | **A①** 段 A 逐位（D1 口径） | **本票可满足** | 见 §2 A①。差距：终面 S5 重跑、对拍脚本非作者独审、CONDITIONAL 3 项由主控确认 |
111: | **A③** MIN12（6283982007 + 6298134855 口径） | **随升格转出 → #1612 切片 5**（验收字面：「MIN12 52 根全程在 10s 帽内、读数与 new-bi/2 oracle 逐位一致」）；**FORMED seed 条件 → #1419**（6283982007；第三稿 :589 说 #1419 不加边，FORMED seed 可在 testonly 构建上验，由 #804 锁与完整路径同值）；切片 1 先做「MIN12@256MiB 52/52」和正式入口两路对拍 | 本票现有的 0065 MIN12 三臂逐位 PASS 作为测量证据留
112: | **A④ 前半**：增量≡全量对拍锁 + 变异体 | **本票可满足（testonly）** | 证据：S4 锁、S3 锁与负控、锁表、全锁 92/92。差距：终面 CI；prepare_source_audit 的 N3a/N3b 变异仍绿（P42，转切片 0/2），本票声明里要照实写「该阶段变异体验证在本票不成立，随切片 0/2 承接」 |
114: | **A⑤** 正式入口 + 公共查询 + 浏览器（D3 口径） | **本票可满足**（第三稿：第 29 行留本票） | 差距：tb01c/tb03a/s-content-v3 测试客户端核对收据缺失；#1497 是否阻塞没有书面结论。产品浏览器归 #1460 |
115: | 资源侧（8MiB、×depth、候选保留、P2/P5） | **#1612 切片 2**（×depth 已由主控登记；8MiB 按 D7 随 S5 读数，≥90% 才转切片 2） | 见 P3-REGISTER P01–P03、P43、P47、P48 |
118: ### 7.1 转出后 #1488 关票所需的最小证据集
123: 2. **A①**：终面（squash 源 `3ee05a2713` 或之后）二进制对 main `6badaa6049` r8 冻结哈希，S86 86 条逐位对拍：INVARIANT 全等，CONDITIONAL 差异逐项说明并经主控确认；对拍脚本有非作者独审记录；收据入仓（D10）。
125: 4. **A④ 前半**：终面 CI run（`s_session,s_capture_native_testonly` 组合）锁表全绿，含 `s4_generation_incremental_matches_cold_full_six_stages` 与 S3 锁；负控提交 hash 列表；照实写 N3a/N3b 的例外。
126: 5. **A⑤**：tb01c / tb03a / s-content-v3 测试客户端对终面的正式会话入口加公共查询的核对收据；#1497 是否阻塞的书面结论。
127: 6. **P3 处置**：P3-REGISTER 中 44 条「需主控处置」都有三选一结论；建议本票修的项要么已落地（附 hash 与评审），要么改判为接受或转票。「8MiB」按 D7 用 S5 峰值读数结案或转切片 2。
128: 7. **关票门**：D9 squash 提交（resolution）；S7 用户合入闸后进入 `origin/main`，附 CI run；合入前新冲突的独审（D8）；评审区间首尾相接到 squash 面（G7）；报告和收据入仓（G6，含 N2 日志收据）；工位清理，冻结分支登记白名单（G5/D9）。
133: ## 证据引用迁移（0077 P30，2026-10-21 追加）
135: 本文原引用的 1 份 `/tmp/nc-remote/<run-id>.log` 已改为仓内收据 `.chanlun/review-results/1488-acceptance/receipts/<run-id>.md`（相对工位 checkout 根；关键行 + 整文件 sha256，原日志名与 run-id 一一对应）。索引见 `.chanlun/review-results/1488-acceptance/receipts/INDEX.md`。
```
