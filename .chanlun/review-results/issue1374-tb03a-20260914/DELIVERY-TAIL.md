# #1374 扫描报告入仓补件

> 名分：绑定 #1374 的验收工作草稿，随票保留与归档。本文件不是关票声明。

用户已批准 PR #1454 的具体版本及已披露扫描状态，该 PR 已合入 `origin/main`，实际提交为 `abf5c0f8491fc066a0f2bc47ab4defdb120035c1`。两父为原主线 `65298686cf3e7c9e94e0b9ceeb83aebad2b5530b` 与获批 head `5d5d6236a71b6726078d180fd09b2b32c5de0371`；合入树与获批 head 完全相同。

本补件补齐交付纪律第 6 条要求的最终安全分诊实体。它只加入已有报告原文、主线扫描对照原文、本索引及工位登记，适用纯文档编译豁免。产品源码、扫描规则、基线、ignore 和既有审查原件均不变。

## 报告原文与来源

| 入仓文件 | 原始位置与范围 |
|---|---|
| [devskim_pr_review.json](devskim_pr_review.json)、[devskim_pr_review.md](devskim_pr_review.md) | 仓外证据根的 `evidence/pr1454-devskim-triage/REVIEW.json` 与 `REVIEW.md`；基线、首轮和获批 PR 三轮真实 SARIF 的独立分诊。 |
| [devskim_main_review.json](devskim_main_review.json)、[devskim_main_review.md](devskim_main_review.md) | 仓外证据根的 `evidence/merge-closure-20260914/MAIN-DEVSKIM-REVIEW.json` 与 `.md`；实际主线运行与获批 PR 的独立对拍。 |

四份报告逐字复制，未把其中历史时点的 `merge_approved: false` 或“未合入”改写。原文中的“本目录”和相对证据路径仍相对于各自上表所列仓外原目录；完整仓外证据根由既有 [EVIDENCE.json](EVIDENCE.json) 的 `archive_root` 给出。大体积 SARIF、日志、归档元数据及比较工具保留在该根下，原报告绑定它们的身份与摘要；它们未被混作仓内新文件。

## 已执行检查

[主线常规 CI](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34782751175) 绑定实际合入提交，最终 `test`、`rust-check`、`s-observer`、`fixture-drift` 四项全部 SUCCESS，已在提交本补件前核实。Rust 检查包括两档构建、格式检查、默认测试、正式结构会话测试及 backtest feature 测试。

[主线 DevSkim](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34782751153) 如实为 FAIL：35,168 条原始结果、35,094 个唯一位置键、26,907 个基线外位置键，与[获批 PR 扫描](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34781472772) 相同。完整结果多重集和规则定义对拍相等，无新增或消失内容；SARIF 原始文件摘要不同，未声称文件逐字相同。3,155 KiB 摘要超过 GitHub 的 1,024 KiB 限制仍如实保留。

上述对拍只确认实际主线没有超出获批扫描内容，既有 `strategy.rs:314` 待核事项仍由 [#1385](https://github.com/xy7365527-lang/NewChanlun/issues/1385#issuecomment-5648592872) 承接。没有把所有历史告警重新审计或清零；补件自身的扫描结果应读取它的实际 PR 检查，不能借原批准冒充新合入批准。

## 交付边界

本叶五项验收的运行、来源、恢复和审查范围沿用 [REPORT.md](REPORT.md) 与原索引。已完成的原始运行不因纯报告补件重复执行。#1374 在补件合入与工位收尾完成前保持 OPEN；TB-03 父票、完整 SPEC 与 #1323 六个 Destination 仍保留各自义务。PR #1453、未决资金或语义规则及真实外部交易均不由本补件取得批准。
