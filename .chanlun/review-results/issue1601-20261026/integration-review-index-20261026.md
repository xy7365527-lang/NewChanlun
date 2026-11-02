# #1601 / PR #1680：集成评审索引

代码候选：`06a095112fb6fbf94f90d55c19e027ad76c63830`。以下两份外盘原件逐字归档，不回写历史结论与 CI 时态：

- [集成及验证记录](1680-STROKE-INTEGRATION26.md)：保留 Stroke 已否证对照臂名分及 q_total 三参资金接口。
- [集成非作者独审](1680-STROKE-INDEPENDENT-REVIEW26.md)：本次集成代码与合同 PASS。

本次只归档文书。精确候选及归档后 head 的 CI、合入与关票仍由主控按门验收；本归档不代表合入或关票。原始运行日志保留在 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1680-integration-validation26/`。stream 筛选为 0 tests，未计作测试通过。

## 完整资金接线审查及首次 main 集成审查

以下两份原件在 2026-10-26 补入本目录，逐字保留历史结论与当时的 CI 状态。

| 历史阶段 | 审查候选 | 原件与范围 |
|---|---|---|
| 2026-10-25 最终非作者复审 | `107336f3d4ae0e4b2b9b52d3916d62311685829c` | [完整资金接线独审](1680-FINAL-INDEPENDENT-REVIEW25.md)，覆盖 funding-record、回放账本、NT 策略与 venue、起始操作级别、旧线 policy、Python FFI、测试及调用方。代码与合同 PASS，报告当时 CI 仍待终态。 |
| 2026-10-26 首次 main 集成尾审 | `5eb523f6a7b1e8ce8997addeda74470bb15f952a` | [首次主线整合独审](1680-INTEGRATION-REVIEW26.md)，核对与主线 `7cacd14135d12e2339fce58cd1c87cd0ee132ee0` 的合并差异，复用前一份完整审查。代码与合同 PASS，报告当时新候选 CI 仍待终态。 |

前两阶段先于上节 `06a095112fb6fbf94f90d55c19e027ad76c63830` 的 Stroke 集成。原件分别来自 `/Volumes/AgentStorage/final-review-wave-20261025/1680/REVIEW.md` 与 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1680-INTEGRATION-REVIEW.md`。原件未列审查时刻，索引不以文件修改时间代填。

完整独审中的 18 项 Python 测试使用 NT、数据与 Rust 协作者替身，证明 venue 装配合同，不证明完整 NT 历史行情回放。生效 L0 与旧 L1 不等值的行为差异沿用原报告。

本次归档不更新历史报告的 CI 状态，也不新增代码审查或运行验收结论。#1601 的实际 main 交付、CI 终态、工位处置及关票由主控另行核实。
