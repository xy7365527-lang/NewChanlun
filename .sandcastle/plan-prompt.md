# ISSUES

以下是本仓可拾取的 open issue（已按专属 label `sandcastle` 过滤）：

<issues-json>

!`gh issue list --label sandcastle --state open --limit 50 --json number,title,body,labels,assignees --jq '[.[] | select(.assignees | length == 0) | {number, title, labels: [.labels[].name]}]'`

</issues-json>

# TASK

分析这些 issue，建依赖图：B 被 A 阻挡 = B 需要 A 引入的代码/基建 / 两者改重叠文件易冲突 / B 依赖 A 定的决策或 API 形状。

只放行 unblocked 的 issue。分支名严格用 `sandcastle/issue-{id}`（确定性，无后缀）。

另须遵守本仓 frontier 纪律（与 prompt 层判定叠加）：跳过人读的 issue 体里写明 blocked 关系的票。

# OUTPUT

把计划放进 <plan> 标签输出 JSON：

<plan>
{"issues": [{"id": "42", "title": "Fix auth bug", "branch": "sandcastle/issue-42"}]}
</plan>

只含 unblocked issue。全部受阻时含一张候选最弱的。无事可做时输出 <plan>{"issues": []}</plan>。
