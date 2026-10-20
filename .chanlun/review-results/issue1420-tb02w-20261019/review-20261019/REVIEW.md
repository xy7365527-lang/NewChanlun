# #1420 / PR #1569 独立审查（本轮新增部分）

> 审查者：`/root/review_1420_pr1569`（非作者，只读）。分支 `codex/1420-scan-boundary-onmain-20261019`，HEAD `b87c45fea3`，基点 main `5baeb030a6`。范围只含本轮新增：规则合成提交 `c37dd813b2`/`3b55b45f6e`、验收脚本 `b64424c392`…`8a6aea67b8`、rebase 两处冲突、交付报告；2026-10-14 已评审的 11 个实施提交不重审。

## 结论：PASS（附一项交付报告必须改的措辞）

四项审查点都过。唯一需要改的是交付报告对 `m3_partition_synthetic_holds` 转绿的表述：它是 078:52 失效语义改变段划分后的偶然结果，不是修好了线段续扫或 #1487 的吸收态。实测在该测试的种子上，本分支段数从 24 降到 14，末候选在第 160 笔起以 `FAILED_GEOMETRY / top_gt_bottom` 卡到输入末端（第 292 笔），正是 #1493 要 fail-closed 的段端门吸收态。报告应改为「转绿属偶然，#1563 第一项仍归 #1493，#1493 落地后重跑」。这条是文字订正，不挡合入。

## 1. 规则合成：一致，旧串处理正确

Rust `RULE_REVISION`（`rust/src/bin/s_structure_session.rs:66`）、Python `RULE_CONTRACTS`（`s_session/s_tb02_contract.py:24-31`）、JS `RULE_CONTRACTS` 与目录轴表（`s_session/browser/tb01c-client.js:498-505`、`:673-676`）、`s_query_integrity._published_axes`（`:346-351`）四处都是 `s2-axis-quantifiers+new-bi/2+segment-second/2` → (`s-new-bi/2`, `s-segment/3`, 知识域 V2)，页面 `index.html:413` 把 `s-segment/3` 的 slot 认作逻辑身份。

旧串按「同一规则串不混两代」处理：

- Rust 侧 accept（`:1333`）与 advance（`:2051`）对非当前规则一律 `VersionMismatch`，旧会话只能读；
- Python/JS 侧旧串 `…+new-bi/2+segment-second/1` 仍绑 `s-segment/2`，所以旧规则 cut 里出现 `s-segment/3` 对象会被「段合同与cut规则版本不符」拒收，反之亦然（`test_tb02w_contract.py:338-342`、`.cjs:160-167` 两向都锁了）；
- 原实施分支上的中间串 `new-bi/1+segment-second/2` 从未进 main，合成时整体丢弃，全仓无残留（`rg` 零命中），没有孤儿规则串。

本地复跑 `test_tb02w_contract`/`test_tb02b_contract`/`test_tb02c_verifier`（31 过）与三份 `.cjs`（16 过）。PR CI `test`/`rust-check`/`s-observer`/`fixture-drift` 全绿。

## 2. 验收脚本：真测到了验收 3、4，期望不是自产自比

**期望来源独立。** `tb02w_runtime.py` 的 `EXPECTED` 是按笔手写的判定点（n=6/7/9），价格与锚点经 tb02d 固定生成器的 `transform`/`anchor` 由端点算出，不导入生产 reducer；cut 映射 `4i+7` 与生成器一致。`check_case` 对每个判定点取首个达到该笔数的 cut 比对 result/semantic_state/killed_by/geometric_end，并核「失效后不复活」与「判定点全覆盖」。三份引文（`078:52`、`065:28`、`067:38`）在博文正本逐字核过，078:52 在 `↑正文`（`:64`）之前、行首无注。

**逐 cut 合同。** 每臂调 `tb02b_verify.verify(db, None, node)`：Q 完整性审计后对全部 40 个 cut 跑前端合同（`validateTB02CutSources`/`validateTB02Axes`）。收据 8 份 EVAL 全为 `contract_all_cuts.status=passed, cuts=40`，无 `evaluation_error`。

**浏览器三时点。** `tb02w_browser.cjs` 用真 Chromium 读既有页面，在 open（27，AsKnown）、decided（31 或 39，AsKnown）、final（40）三个 cut 等到「已验证提交」后按轴读 `#objects` 文本找关键字；失效 cut 另断言 CC-012 不出现 CONFIRMED，页面错误须为空。四个 A 臂收据均 `passed`。

**新进程重放。** B 与 A 逐 cut 全字段哈希 40/40 相同（killed_up、confirmed_up），这是最强的一层。

**SIGKILL 恢复。** 远端原件证实故障真的发生：`run-C/after_batch.marker` 记 S pid 25287，events 里 `--signal KILL` 回执 `SIGKILL`，收尾停的是新 pid 25343。脚本自身的判据只比「对象 kind+payload」层，不含 relations/witnesses，偏宽；我在 windows-5090 上对 killed_up 的 A/C 原件另做了全量对拍（去掉 `batch_id`、`semantic_commit_ns`，object_id 按内容换名后比 objects/withdrawn/relations/witnesses 多重集），cut 1/20/21/22/30/40 全部 0 差异。所以「仅提交时间戳（及由时钟计划决定的批次身份）不同」成立。

**脚本的两处弱点（不挡合入）：**

- W-a：`session()` 的总判 `ok` 只看手算 oracle 与 A/B、A/C 对拍，**不看** `contract_all_cuts` 与 `evaluation_error`；合同失败会被吞进 `evaluation_error` 而总判仍 PASS。本次收据实测都过，不影响结论；建议后续把两者并进 `ok`。
- W-b：A/C 判据只比 objects 的 payload 层；本审查已用全量归一化对拍补上，建议把该归一化并进脚本。

## 3. rebase 冲突取舍：正确

- Lean 引文（`1570f74209`，原 `4ef0fa236e`）：两处保留 main 由 #1495（`9e1b22aa8b`）订正的 071 正本行号（`:26/:28/:32/:42`），只加 078:52 的指路行和失效出口段；没有把旧的 `067:122/126` 引法带回来。
- 会话规则表（`c37dd813b2`，原 `903369bb4d`）：取 #1405 的 `new-bi/2` 加本票的 `segment-second/2`，旧的单代分支（`new-bi/1+segment-second/2`）丢弃；原提交里见证排序的测试改动在 main 已由 #1419 以同样方式（集合核对）落地，丢掉该 hunk 正确。

## 4. `m3_partition_synthetic_holds` 转绿：偶然，不是修好了线段续扫

在 windows-5090 上对五个提交注入只读探针（seed 20260704，见同目录 `probe1420*.sh` 与日志）：

| 提交 | n=3000 段数 | 末段终点（bar） | ledger | m3 | n=20000 段数 / ledger |
|---|---|---|---|---|---|
| main `5baeb030a6` | 24 | 2865 | 0 | 红 | 75 / 6 |
| `8badf07694`（红测骨架） | 24 | 2865 | 0 | 红 | 75 / 6 |
| `279b7c8708`（078:52 失效语义） | 14 | 1708 | 1 | 绿 | 79 / 6 |
| `b824d302ca`（K1 游标） | 14 | 1708 | 1 | 绿 | 79 / 6 |
| HEAD `b87c45fea3` | 14 | 1708 | 1 | 绿 | 79 / 6 |

转绿精确落在 `279b7c8708`，与续扫游标/预算（`b824d302ca` 起）无关。段事实（`divide_segments_with_facts`）：

- main：候选 CASE_ONE 19、CASE_TWO 5、末候选 `NO_FIRST`（第 278 笔起，正常未决）；
- HEAD：CASE_ONE 13、CASE_TWO 1、末候选 **第 160 笔起 `FAILED_GEOMETRY`，waiting=`top_gt_bottom`，observed_through=292**。

机理：main 上若干第二种候选在越过极值后仍被晚到的分型确认；078:52 把它们判失效、原段延续，之后该段的终结试探撞上「顶>底」门，现行实现只会跳过续扫，于是从第 160 笔卡到末端——这就是 #1487 查到、#1493（TB-02-X 义务：段端门 fail-closed）要修的吸收态，只是在这个种子上提前暴露。ledger 从 0 到 1 是塔结构改变后的副产物；该测试只断言 ledger 非空，#1564 已指出它「本就踩在边上」。

所以：本票没有修线段续扫边界，也没有缓解 #1487/#1493；在这个种子上反而让吸收态更早出现（但教义上 078:52 的判法是对的，错在后续的段端门处理，归 #1493）。与 #1487 票面记录一致（叠加 #1420 后 BTC 全量段数 773→735，停滞点不变）。#1563 第一项不能据本分支关闭；#1493 落地后应在 main 上重跑 m3 并看本种子 n=3000 的段数是否回到连续产出。

## 其他

DevSkim 在 main 上也是红（零发现门 + 步骤摘要超 1 MB），本 PR 新增的两条命中（`tb02w_runtime.py:185` 哈希、`:282` 同 `tb02c_runtime.py:38` 的同族）是既有误报类，不另计。

## 要求作者处理

1. （必改，文字）`DELIVERY.md` 的 CI 段把「本分支上 `m3_partition_synthetic_holds` 转绿（#1563 的第一项）」改为：转绿始于 `279b7c8708`，属 078:52 改变段划分后的偶然结果；该种子 n=3000 上段数 24→14、第 160 笔起 `FAILED_GEOMETRY/top_gt_bottom` 吸收态，归 #1493；m3 待 #1493 落地后重判。
2. （建议）脚本 W-a、W-b 两处并进总判。
