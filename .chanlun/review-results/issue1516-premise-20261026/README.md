# #1516 技术前提证据

本目录是绑定 #1516 的工作草稿，按世代宪法 §5 随票保存。两份报告从外盘原件逐字收录，核验源码固定为 `99df2ed762201236b89f0e81c0276cb3aa847f0a`。

## 采用结论与报告

[前提调查](1516-PREMISE-AUDIT26.md)和[非作者独审](1516-PREMISE-INDEPENDENT-REVIEW26.md)均支持撤销“先改 make_token 才能推进 #1304”的 P0 前提。正式 v2/S3 已共用七字段确定性 Begin 配方，不能用 frontier/generation 两字段替换来删除 attempt、epoch 等身份区分。

两份报告核查源码、测试源码及已批合同，没有运行新的 Rust 测试。当前有限 replay comparator 不是 #1411 的完整验收器。

## 下游已登记的具体义务

主控已追加以下评论，本次通过 GitHub API 回读内容及所属票确认。

- [#1516 技术订正](https://github.com/xy7365527-lang/NewChanlun/issues/1516#issuecomment-6405087160)不再采用原 P0 归因及两字段替换方案。
- [#1411 完整身份与恢复比较](https://github.com/xy7365527-lang/NewChanlun/issues/1411#issuecomment-6405083806)保留身份引用、attempt、epoch、语义时刻及前沿的完整验收，只允许已批合同中的预声明非语义身份一一重命名。
- [#1413 旧入口终态](https://github.com/xy7365527-lang/NewChanlun/issues/1413#issuecomment-6405083410)具名接收 `record_begin` 的 `!active` 无协议分支及 `make_token`，逐命令核用途、可达性及写权限。实际旧责任仍经 #1402 crosswalk 承接，不删除整个正式 v2/S3 共用的二进制。

这些评论登记了后续义务，不代表 #1411、#1413 或整图验收完成。原报告中的“建议追加”段保留调查当时状态，实际登记以上述评论为准。

## 本次交付与关票条件

本候选仅含三份证据文档及[工位登记](../../agent-roster-1516-premise-doc-20261026.md)，属于纯文档编译豁免。两份原件 `cmp`、相对链接检查及 `git diff --check` 通过。

主控仍须核精确候选尾审、main 批准与实际合入，再按交付纪律处置本工位和分支、发布引用落地 commit 的关票结论。本候选不关闭 #1516，不修改依赖，也不关闭 #1279。
