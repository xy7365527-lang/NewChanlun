# #1495 Lean 第一种情况交付证据

> 名分：工作草稿，活期绑定 [#1495](https://github.com/xy7365527-lang/NewChanlun/issues/1495)；归档按世代宪法 §5。属于 #1323 / SPEC #1340，落实 #1490 X-31，不挡 #1493。

第一种情况的先破判据、转折点窗三态、上下两向等价引理及两文件引文订正已形成可审候选。源码提交为 `9e1b22aa8b50de9097f4ce6d0247281707f8506c`，父提交为已含 #1490 裁定的 `dedf11f047b9c809c167b35e728e4a2ee8bf3494`。本目录仅追加证据，不修改源码；[PR #1501](https://github.com/xy7365527-lang/NewChanlun/pull/1501) 指向 `main`。合入与关票仍由主控裁决，本报告不宣称已经完成。

## 实际变化

- [SegmentFeatureSeq.lean](../../../formal/Origin/SegmentFeatureSeq.lean) 定义 `FirstBreakConfirmsUp/Down`、转折点窗三态与按时间顺序的确认函数；证明包含元素跳过、OPEN 等价于全部窗成员被 b 包含、已裁状态在追加数据后不回退。`firstOutsideWindow` 给出首个出窗元素，非包含前提由其定义推出；完整 `FirstKindConfirmsUp/Down` 显式携带同一无缺口条件，并证明无并列时与 067 严格分型上下等价。相等不是破，并列确认由先破直接裁决。`IsTopFractal` 与 `IsBottomFractal` 字节保持原样。
- [Claim10_SegmentV1.lean](../../../formal/Phase2/Claim10_SegmentV1.lean) 仅订正第 71 课的来源与行号；边界包含规则指正本 `docs/chanlun/text/blog/071-第71课.md`:26/:28/:32/:42。另三处边界包含错引从 `071:40` 改为 `071:42`，确属缺口定义的 `071:40` 保留。

具体源码 SHA-256 与提交补丁身份见 [SOURCE-MANIFEST.json](SOURCE-MANIFEST.json)。两文件提交补丁与独立评审冻结候选逐字一致，正常 Git hooks 已执行。

## 验证与证据

| 检查 | 结果 | 原件 |
|---|---|---|
| 内盘独立工位原 `lake build` | 158 jobs，exit 0；复用本工位已建依赖 | [完整构建日志](full-build.log) |
| 原 `python3 scripts/check_fixture_drift.py` | center、certificate_chain、parity 三组均 0 字段差异且字节一致，exit 0 | [fixture 日志](fixture-drift.log) |
| 152 份 tracked Lean 可执行 token 检查 | sorry/admit/axiom 为 0；严格分型谓词未改 | [扫描收据](TOKEN-AND-PREDICATE-CHECK.json) |
| 独立语义评审 | PASS_SCOPED_SEMANTIC_REVIEW，未解决必须修复项 0 | [独审原件](INDEPENDENT-REVIEW.md) |
| 精确源码提交的远端 CI | `workflow_dispatch`，head 和 checkout 均为 `9e1b22aa8b50de9097f4ce6d0247281707f8506c`；fixture-drift job 完整 Lean 构建 158 jobs 成功，三组 fixture 均 0 差异且字节一致 | [run 38681567438](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38681567438)、[job 116247066853](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38681567438/job/116247066853)、[job 原日志](ci-exact-head-fixture.log) |

结构化读数及查询时刻见 [VALIDATION.json](VALIDATION.json)，原件校验和见 [SHA256SUMS](SHA256SUMS)。远端原日志保留四行空输出时间戳后的空格，`git diff --check` 仅在该原日志报四处尾随空格；人工文档与源码无该问题。远端日志明确显示两份改动模块都被构建；日志中的 `git log -1 --format=%H` 给出源码提交，因此没有把 PR 的测试合并引用当作候选 head。

取证时整条 CI 的 `test`、`rust-check` 尚未结束，不能把 fixture-drift 单项成功写成全 CI 成功；`s-observer` 已成功。PR 的常规合并引用 CI 为 [38681519553](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38681519553)，其中 Lean/fixture 也已成功。最终 PR 文档追加提交会照现有规则自动触发一次 PR CI；这不改变本目录两份源码的身份，也不冒充该后续 run 已结束。远端状态随时间变化，以实际 run 为准。

## 必须保留的边界

`FeatureElem` 仅保存区间，输入的同向笔与时间顺序沿用 §1 调用方约束。新定理没有证明原始行情到这一输入域的构造连接。既有 `SegmentEndUp/Down`、`SegmentFeatureComplete`、`SegmentAutoConstruct` 历史消费者未迁移；本票不声称整个 Lean 构造链已经接入新判据，也不代表 Rust #1493 或 #1323 整体完成。

恢复来源是固定基线加内盘日志三个完整写入体，并与 18 份历史源码片段核对。**当前外盘 WIP 字节未访问、未核对、未处置，原外盘分支保留。** 本目录不放会话导出、整个恢复日志或构建缓存；源码身份与验证结论不依赖外盘当前内容。本报告不使用自动关闭 #1495 的指令。
