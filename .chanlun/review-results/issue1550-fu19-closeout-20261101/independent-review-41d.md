# FU-19 / PR #1821 增量独立审查

结论：**PASS**。冻结候选 `e92fb2afde0b26eb2dbc5038632b4ce9cf20ee08` 已修复 40X 的 F1/F2；Standards、Spec 两轴通过。合入和关票仍由主控完成下列真实动作。

- 审查者：GPT 家族、非作者独立工位；作者：claude-opus-5-5。
- 只读工位：`/Users/silencehan/.codex/worktrees/fu19-publish40q/NewChanlun`。
- 取证时间：2026-11-01T00:04:06Z 至 2026-11-01T00:04:44Z（工具实际 `date -u`）。
- 原审候选：`b3ce910cc47eab4297fa218ad3da5df704a1a65a`；旧审报告：`/Volumes/AgentStorage/fu19-review40x/REPORT.md`。
- 当前远端 main 与本地 origin/main 均为 `71df2ae749664714e6bff10f8f71d9e15cc4c815`；合前 merge-base：`457902e3c371d1f1c9611bb083415067f061f4d2`。
- 本轮只写本目录 REPORT.md、review.json；未改源码、票、分支、服务或共享台账，未触发或等待 CI，未运行 Rust。

## F1：保留原退场合同，PASS

实时读取 #1607，状态 OPEN，正文仍要求「随冻结影子删除，并在 #1304 记录去向」，完成后删除待办注释和 DevSkim 抑制。

候选 ADR 0058 的终态表（172）、裁定四（180–185）、验收（231）、下游运行验收（238）、受影响票（244–252）与代码清单（264）一致：#1304 登记只完成去向，#1607 继续 OPEN；#1413 负责过渡指路注释，不能代替删除。实际影子删除及 TODO/抑制清除后，才由删除那次交付关闭 #1607。#1550 映射交付不等 #1607 关闭。

完整 ADR 已回读，相邻 `chong-traceability.md` 全文 88 行已回读。持久 Chong、Voice、阶段史、毛账、恢复和 AsKnown 合同未被本次修复改写。

## F2：使用真实 BASE/CAND 验收，PASS

实际执行候选 ADR 的三条 diff 管线。所有 gh 调用均加 `env -u GH_TOKEN -u GITHUB_TOKEN -u GH_DEBUG`。候选来自实时 PR head，BASE 来自 `git merge-base origin/main "$CAND"`，对象本地已存在，没有 fetch 或改 refs。

- CAND = `e92fb2afde0b26eb2dbc5038632b4ce9cf20ee08`。
- BASE = `457902e3c371d1f1c9611bb083415067f061f4d2`。
- 非 #1550 行增删计数 = **0**；#1550 行增删计数 = **2**；`@@ -65 +65 @@` 以外 hunk = **0**。
- 原始 diff 逐字只有第 65 行追加 ADR 0058 链接；三行头不动。`git diff --check "$BASE" "$CAND"` 无输出，成功。
- 实测输入不依赖 `decision/1550`；不会把没有指针补丁的旧决策分支当发布候选。

## 差异与旧审查复用

`git diff b3ce910c e92fb2af` 实际仅 ADR 0058 一个文件，**+15/-10**，与作者 `repair.diff` 的内容一致。整个发布候选相对 BASE 仅 ADR 0058 与教义第 65 行两个文档文件。没有代码或代码注释变化；no-comments 不产生新代码整改项。

40X 的以下证据按未变差异复用：唯一 launcher 与冻结影子终态；六面输出、恢复/AsKnown；H1–H4 与 ADR 0052 (a)–(d) 合取；#1528 需求交 #1411 而不升级旧影子；#1769→#1381 的设计与不新增 #1523→#1381。原决策 attempt 2 的 Opus→Opus 非作者 R1–R8 PASS 仍按同家族记录，未改称异家族。

当前 main 相对旧审 main `9de18e606b2c0330162a83e2a030898b46c3690f` 只新增 `71df2ae749`（PR #1816）：6 个 `rust/src/bin/s_session_v3/` 文件，涉及 audit_ledger、audit_owner、history_clamp_tests、native、prepared、state。差异提供 H1 消费 P6 完整资格、身份/四根/水位核验和预算记账；未改变正式 launcher 外壳、evidence_class 或本 ADR 的对象/终态合同。

对 docs、教义、production_session.rs、x_execution_session.rs、ci.yml 的该段 main 差异为空。新增资格组件不等于 ADR 的四个全链具名测试已经运行，#1411/#1418 的义务保留。该影响核查不重审 #1816 的代码质量。

实时 PR head 与冻结候选相同，工作区干净。GitHub 返回 `isDraft=true`、`mergeable=MERGEABLE`；该快照的 `baseRefOid` 仍为旧 main，因此主控合前以最新 main 重新确认集成状态。CI 本轮未查询、触发或等待。

## 主控接续：合入与合后动作

1. 在精确 head 必要 CI 全绿、最新 main 集成无冲突后，转 ready 并 squash 合入。squash 正文不得含 close/fix/resolve #1550，避免提前自动关票。
2. #1528 先登记分支 `codex/issue1528-replay-compare-20261018` 的完整 head `72b1dbcc7344dc0532cf2cef76cd90a429f31e6d` 与 ADR 指针，需求明确交 #1411。**删除远端分支及工位属于破坏性清理，普通 main 合入常设授权不包含此项；须另取适用授权后实际处置在制品。** 不合旧分支，不把留 SHA 冒充已经删除。处置完成后撤 #1381→#1528，改题「不实施」再关闭。
3. #1769 接收 dry-run/evidence_class 冲突，#1381 添加 blocked_by #1769，保留原 blockers 1364/1353/1481/1443；不新增 #1523→#1381。
4. #1304 登记 #1607 退场去向，#1607 保持 OPEN。#1413 接收 `replay_comparator.rs:151` 的过渡指路注释义务。
5. #1304/#1411/#1413/#1418/#1769 实际接收 ADR 0058；#1418 同时接收 `at14-report/1`、实体路径和 CI 锁，#1381 接收输入合同，#1368 接收 Python/NT adapter 指针。旧草稿或关键词命中不等于新合同已消费。
6. #1550 发布两项实际冲突处置登记；#1323 Decisions so far 记录实际落地。完整动作和报告入仓证据齐备后，才发布 resolution 关闭 #1550。本目录外部报告须由主控按交付纪律归档入仓，不能只用绝对路径完成关票。
7. #1304/#1279 保持 OPEN，六 Destination、TB-01–11、#1320 重型义务保留。四个具名测试、实体 AT-14 报告、CI 真运行仍归 #1411/#1418；#1607 的实际退场关闭归删除那次交付。

## 原则怎样改变了审查选择

Prove It Works 使本轮直接运行修复后的 BASE/CAND 命令并核原始 diff，而不是采信作者的 0/2/0 陈述。技术写作与 unslop 检查聚焦 #1607 的正文、表格、验收、下游是否同义，未为措辞偏好重开无关修订。无新增阻断发现。
