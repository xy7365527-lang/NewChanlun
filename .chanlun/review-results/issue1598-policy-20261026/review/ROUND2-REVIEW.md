# #1598 第二轮非作者独审

结论：**PASS，可提交 P 类批准；不可据此关票或实施生效。**

范围：只复审 ROUND2.diff 中 F1–F5 返修及直接依赖；沿用第一轮未受影响的检查。主线来源基准为 origin/main `13ed4d45c8ff85175819ca987690328ab2a247a4`，不以落后的工作树 ADR 0022 替代。第一轮 INDEPENDENT-REVIEW.md 和 review.json 保留。

| 项 | 复审结论 | 依据 |
|---|---|---|
| F1 | PASS | 固定 r=√0.32，w(δ)=0.32^(0.5/δ)；δ=.4/.7 对应 .2406786/.4431346、−24.7879%/+38.4796%。与第一轮独立计算一致；明确仅为敏感性示例，非生产量档。 |
| F2 | PASS | 三支持文献已对齐 ADR 0017；#907 指向评论 5194145006，真实三步及否证口与原件一致。后续变更流程与原话分开。 |
| F3 | PASS | affected-code.md:35 已区分 FULL 汇总与 H1/H2 两互斥半窗；明确不满足三窗门，#1444 仍须补窗。 |
| F4 | PASS | 比较量唯一为逐层逐窗单边下界；分子/分母、缺失处理、块抽样、分位、样本/块/窗/覆盖门及不足/失败状态均已定义。θ₂ 未签仍 AwaitingPolicy；方法新增部分明确供 P 确认，未冒称已裁。 |
| F5 | PASS | archaeology.json Q2.reasoning 与 MD 同步为约化未算、原 ADR 仅推测不约；未再把推测当确定推导。 |

## Q6 的审批含义

既有 #1677/evidence-gate/v1 六字段与新判据方法是两件事。沿用已授权六字段不需要重签 v1；本次 P 确认包含下钻事件统计域、分层、bootstrap 算法及参数的新增推荐。ADR 0022/0036 的 `(重,S稳定点)` 操作样本属于既有操作统计合同，不能因本稿改成下钻事件。本稿已将事件分母限定于新增 share_threshold，并明确默认 order_relation 不改，因而未构成对既有统计合同的静默替换。

审批展示应保留完整 Q6 方法段，不能只呈现“多一个 θ₂”的选项摘要；批准政策形式不代签 θ₂，也不代替 #1441 的方法实现与独立 oracle/同 seed 双跑验收。当前是文档方法推荐复审，不把尚未实施当作草稿阻断。

## 交棒

无遗留阻断项。交主控提交 P 批准；随后按原流程核下游实际接单、正本落地及 main 批准门。该报告不改变 needs-approval、GitHub 状态或 #1279 状态。

本轮读取：ROUND2-HANDOFF.md、ROUND2.diff、VALIDATION-round2.txt、draft.md、affected-code.md、archaeology.json、evidence/0022-main.txt、evidence/delta-calculation.json 及 #907 原件；关键修复与第一轮独立证据对照。未改草稿/代码/正本，未运行 macOS Rust 或无关测试。
