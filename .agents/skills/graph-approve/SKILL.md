---
name: graph-approve
description: 批 P 类决策：逐张呈现挂 needs-approval 的代理草稿，一次一问，批了就落正本并关票。用法 /graph-approve [票号]。
disable-model-invocation: true
---

P 类决策（资金参数、风险偏好、真实账户与权限、关图去向、taste、无先例的新取舍）由代理起草和独审，用户拍板。流程正本 `.sandcastle/docs/graph-engineering.md`。这是 HITL 会话：用户的回答就是裁定，你只呈现和落地。

## 步骤

1. **取队列**。`gh issue list --label needs-approval --state open --json number,title`；用户点了票号就只看那张。完成标准：告诉用户共几张、各是什么。

2. **呈现一张**。读该票的草稿、审查与考古（票上评论，或 `/Volumes/AgentStorage/decisions/<票号>/` 下的 `draft.md`、`review.json`、`archaeology.md`）。按 `/grilling` 的分工呈现：事实已由考古查完，只把决定交给用户。分三段讲：A 类已裁沿用了什么（一句带过）、代理建议什么及依据、审查挂了哪些 BLOCK。然后逐个问考古里的 B 类与 C 类 `decision` 点，一次一问，每问附出处与推荐值：B 类问「默认值 <值>（出处）——确认还是否决」，C 类问「推荐 <值>（推导依据）——批、改成什么」。C 类 `research`/`probe` 是事实缺口，不问用户：按 `/wayfinder` 在同一张图下开 `wayfinder:research` 票（交 `/research` 子代理）或探针票，接 blocking 边挡住本票。票上没有 `archaeology.md` 时先补考古：`node --import tsx .sandcastle/decision-claimer.mts --archaeology-only --ticket <号> --out /Volumes/AgentStorage/decisions`，再按问表呈现。完成标准：这一张的每个 B/C 决定点都有用户的明确回答，事实缺口都已开票，或整张退回。

3. **落地**。
   - 批或改：按用户最终口径写正本（草稿的正本落点：`.chanlun/definitions/`、`docs/adr/` 或 `CONTEXT.md`），提交到 `decision/<票号>` 分支并开 PR；票上发 resolution 评论，首行「用户 <日期> 批复」并引用户原话；摘 `needs-approval`。PR 合入后关票，图的 Decisions so far 加一行。
   - 退回：把要补的材料写成评论，摘 `needs-approval`，票回 decision-claimer 队列重新起草。
   完成标准：PR 链接或退回评论链接。

4. **下一张**。回到第 2 步，直到队列空或用户喊停。结束时报：批了几张、退回几张、待合入的 PR。
