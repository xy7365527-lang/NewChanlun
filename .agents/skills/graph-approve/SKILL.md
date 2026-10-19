---
name: graph-approve
description: 批 P 类决策：逐张呈现挂 needs-approval 的代理草稿，一次一问，批了就落正本并关票。用法 /graph-approve [票号]。
disable-model-invocation: true
---

P 类决策（资金参数、风险偏好、真实账户与权限、关图去向、taste、无先例的新取舍）由代理起草和独审，用户拍板。流程正本 `.sandcastle/docs/graph-engineering.md`。这是 HITL 会话：用户的回答就是裁定，你只呈现和落地。

## 步骤

1. **取队列**。`gh issue list --label needs-approval --state open --json number,title`；用户点了票号就只看那张。完成标准：告诉用户共几张、各是什么。

2. **呈现一张**。读该票的草稿与审查（票上评论，或 `/Volumes/AgentStorage/decisions/<票号>/draft.md` 与 `review.json`）。分三段讲：要定的是什么、代理建议什么及依据、审查挂了哪些 BLOCK。然后只问一个问题：批、改成什么、还是退回补材料。完成标准：用户对这一张给出明确回答。

3. **落地**。
   - 批或改：按用户最终口径写正本（草稿的正本落点：`.chanlun/definitions/`、`docs/adr/` 或 `CONTEXT.md`），提交到 `decision/<票号>` 分支并开 PR；票上发 resolution 评论，首行「用户 <日期> 批复」并引用户原话；摘 `needs-approval`。PR 合入后关票，图的 Decisions so far 加一行。
   - 退回：把要补的材料写成评论，摘 `needs-approval`，票回 decision-claimer 队列重新起草。
   完成标准：PR 链接或退回评论链接。

4. **下一张**。回到第 2 步，直到队列空或用户喊停。结束时报：批了几张、退回几张、待合入的 PR。
