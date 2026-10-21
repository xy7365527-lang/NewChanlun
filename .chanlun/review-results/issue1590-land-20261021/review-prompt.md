你是 NewChanlun PR #1652 的非作者独立评审（只读，不得修改任何文件）。工作目录是该 PR 的 worktree（分支 issue1590-land-20261021，基于 origin/main 02b5b571eb）。

材料：
- 被批草稿：/Volumes/AgentStorage/decisions/1590/draft.md（用户已批准，批复见 #1590 评论 6309984967）
- 草稿独审：/Volumes/AgentStorage/decisions/1590/review.attempt4.json（notes 里 6 条非阻断建议，本 PR 声称已处理）
- PR diff：.chanlun/review-results/issue1590-land-20261021/pr1652.diff
- 验收实跑输出：.chanlun/review-results/issue1590-land-20261021/acceptance-run.txt
- 仓库规则：AGENTS.md（教义正本三行头、#799 收敛通则、引文核验）、docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md

逐条审，并亲自打开文件核实（不要只读 diff）：
R1 忠实度：ADR 0043 与 sanjieduan.md/CONTEXT.md/ADR 指针是否逐项落实草稿「裁定」一至六与「正本落点」，有无偷加或漏掉的裁定内容、改变语义的措辞。
R2 6 条非阻断建议是否都已处理且处理正确。
R3 引用准确：ADR 0043 里引用的 ADR 裁定编号（如「ADR 0034 裁定一／三／四／五」「ADR 0013 裁定四」「ADR 0040 裁定一／四」「ADR 0031 裁定一／二／三」「ADR 0017 裁定一／二」）与 Lean/Rust 行号（TotalWealth.lean:121/:125/:178/:256/:260、ledger.rs:97/:146/:989、voice.rs:169、book.rs:442）、缠师原文行号是否真实对得上（打开文件核）。
R4 ADR 0041 合规：两字段有默认值、可选值保留并写代价、无备用值路径、缺值/非法值统一 AwaitingPolicy；#799 无宽严两档兜底。
R5 订正指针形态：原文不删、带票号；sanjieduan.md 三行头格式保持。
R6 验收清单：草稿九项是否都在 acceptance-run.txt 里实跑，结果是否与判据一致（第 9 项 CLOSED 在合入前按规定为 OPEN，不算失败）。
R7 新实施票 #1651 与依赖边（可用 `cd /tmp && gh issue view 1651 --repo xy7365527-lang/NewChanlun`、`gh api repos/xy7365527-lang/NewChanlun/issues/1590/dependencies/blocking`）是否与草稿一致。

只输出一个 JSON 对象（不要 markdown 围栏）：{"pr":1652,"reviewer":"<你的模型名>","verdict":"PASS"|"BLOCK","items":[{"rule":"R1","verdict":"PASS"|"BLOCK","basis":"...","problems":["..."]}, ...],"notes":"..."}。只有会让落地与草稿不符、引用错误或违反仓库规则的问题才判 BLOCK；措辞偏好写进 notes。
