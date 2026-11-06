# #1371 合入与 DevSkim 尾报告

本目录是 #1371 的工作草稿收尾包。产品已由 PR [#1449](https://github.com/xy7365527-lang/NewChanlun/pull/1449) 合入，实际主线提交为 `d2219e8e014a5d55c091f8eda62ffb9dd8154953`，双亲分别是原主线 `2212eba31e0b7419907e622e79f02f7c2cc37706` 与批准的 `256cd822e0d872f1fb65d433ca0b15b01fa621a2`。合并树 `6fd745ce161d8c4e4cc341483435bef24b810831` 与批准 head 完全一致。

## 用户授权及未解决项

用户先要求“能不能修复这个东西”，随后明确“如果不行就批准”。范围内核对未发现扫描器偏离固定规则或 gate 差集计算错误：9,233 个基线外键中，9,232 个具备本次分诊反证，1 个旧限价 entry 映射待办继续 needs_review。隔离试算将全部已核 not_actionable 键加入临时、未采用的 baseline 后，原 gate 仍以 1 个旧待办返回失败。真正处理该待办会改变订单价格及 entry 归属，归 #1385 TB-10-B，现有前置尚未闭合。本次不删 TODO、不修改扫描器、规则、baseline、ignore、工作流或冻结证据字节。

据此，用户后备授权用于上述固定 head 的一次例外合入。DevSkim 始终按实际结果记为 FAIL；not_actionable 是具名扫描告警分诊结论，不是全仓安全审计，实际风险不声明为零。新尾文档提交不是该固定 head，不能把旧授权或旧扫描写成新提交已获得批准或已通过检查。

## 版本与主线验证

- 固定 head 的[常规 CI](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34709866744)通过；[DevSkim](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34709866745)扫描/上传成功，baseline gate 失败。
- 合入后[主线 CI](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34714772462)已完成：test、rust-check、fixture-drift 全部通过。[主线 DevSkim](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34714772442)扫描及报告上传成功，baseline gate 仍为 FAIL。原始结果仍为 17,484 条，基线外 9,233 个键与合入前逐键完全相同；这是实际 gate 日志键集对比，不声称两次 SARIF 原字节相同。完整运行状态和比较摘要见 `MAIN-CHECKS.json`。
- 本目录分诊报告及 SARIF 固定对应 `256cd822` 和其具名 PR 检查。合入 d2219e8e 的同树事实可用于源码身份核对；报告并不宣称本目录自身已被旧扫描覆盖。后续文档检查的 run/结论在 PR、票的实际回执中保留，不因每个 CI 回执再建立一份全量递归报告。

## 逐提交与评审范围

原主线到固定 head 共 25 个提交：前 21 项见 `../native-codex-r7-r12/COMMIT-DECLARATIONS.json`；750f、b9、6189 三项见 `../native-codex-r13-r14/COMMIT-DECLARATIONS.json`。第 25 项 `256cd822e0d872f1fb65d433ca0b15b01fa621a2` 仅新增该 R13/R14 包的 23 个文档文件，产品与 6189 同字节，明确为纯文档编译豁免。`R14-FINAL-DELIVERY.json` 保留当时生成的原件，不将其中历史 pending 状态改写。

R14 产品及 8 AC / 17 个 B 子义务的有界独立复核见 `../native-codex-r13-r14/r14/REVIEW.md` 和根验收。R13 失败、R7 失败及修复历程保持原件。合并提交无额外产品变化；本尾包只有报告、证据和工位登记，按交付纪律纯文档豁免编译与追加产品评审，仍须核原件恢复完整性及提交差异。

## 工位与关票边界

已重新核对 14 个旧 #1371 工位：HEAD 和未提交面与保全快照一致，HEAD 均已在 d2219e8e 主线祖先链内；9 份未提交登记/重复旧报告与仓外保全逐字相等。用户主仓仍有 21 处跟踪更改和 333 条未跟踪记录，未改动；父 #1323 工位、其他票工位及整个仓外证据目录受保护。

这 14 个工位及当前尾文档工位尚未删除，#1371 仍 OPEN。尾报告正式进入 main、剩余收尾满足后，再以当时 HEAD/未提交面重新核对并按精确清单清理；若出现新在制品，先保全处置，不覆盖用户工作。当前不是关票 resolution，不声称清理完成。

本票成功域仍仅为 CC-006/local_shape 的具名 TestOnly tick 1:1 OHLC、严格无包含、固定初始方向、无同价竞争。#1372 持续负载、#1359 完整 TB-01、#1448 同 ID 恢复、#1385 旧限价映射及 #1323 六 Destination 均不因本次例外合入或报告归档自动关闭。#1372 仍以 #1371 正式完成作为原生前置。
