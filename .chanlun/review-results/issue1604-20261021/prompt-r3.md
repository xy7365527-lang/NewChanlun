你是 NewChanlun 仓库 PR #1663 的非作者第三轮独立审查者（只读，不改文件，不跑 cargo/pytest/node，不调用 gh）。工作目录是 PR 分支 worktree。作者为另一模型（gpt 系），非你。本轮范围 = `git diff 89b0d7ca89..HEAD`（提交见 `git log 89b0d7ca89..HEAD`）。

背景：第二轮独审留新 MEDIUM-1——`s-segment/4` 在不升 schema/rule_revision 的情况下把预算 token 必填值从 `second-seq-scan-budget/1` 改成 `/2`。主控裁定 N-2：升到 `s-segment/5`。四条要求：
1. `s-segment/5` 要求 `second-seq-scan-budget/2`；
2. `s-segment/4` 恢复为只接受 v1（回到 #1604 改动之前的 /4 合同），保留为历史合同；读路径照常接受 /3、/4、/5；
3. 写路径只写 /5（S 写者 Rust 侧 schema 常量与写出点、K1 游标所在对象等）；凡按 schema 分派的读路径（Rust 读回、Python `s_session/s_tb02_contract.py`、`s_session/s_query_integrity.py`、浏览器 `s_session/browser/q3-typed.js`、`q3-consumer.js`、`index.html` 及打包产物 `tb01c-client.js` 与其 SHA 注释）都认 /5；
4. 测试（Python 与 JS 各至少覆盖）：旧 /4+v1 通过；/5+v1 被拒；/5+v2 通过；/4+v2 被拒。另：rule_revision 与其他合同登记（docs、教义正本、实施报告 §7）随之登记；DevSkim 基线按行号登记，不得让已有发现整体漂移。

请逐条核对四条要求，给出各自落点（文件:行号）与是否满足；并查：
- rule_revision 新 token `+second-seq-scan-budget/2` 的历史规则 `…+segment-turn/1` 是否在所有按规则精确分派的 Rust 表中登记为只读（tb02a_facts、tb02b、tb1405 旧规则清单、source_pending 等）、有无遗漏点（grep 全仓规则字面量与 `crate::RULE_REVISION` 的 match 臂）；
- Python 与 JS 的 v4/v5 分档是否对称（`_budget_policy(v5)` / `budgetPolicy(v5)`、`scanCheckpoints` 的 v4/v5 两参、`expected_version`、`validate_segment_links`/q3-consumer 的段引用闭合与段端门版本筛选）；是否构成 AGENTS.md #799「宽严两档」违规；
- 测试 `s_session/tests/test_tb02c_turn_contract.py` 新增各条是否真能抓住变异（例如 v5 退回 v4 逻辑、/4 误收 v2），`judge` 是否同时跑 Python 合同与浏览器捆绑；
- `q3-typed.js`、`q3-consumer.js`、`tb01c-client.js` 行数是否与 `89b0d7ca89` 相同（用 `git show 89b0d7ca89:<path>` 与工作树比较行数，可用 `grep -c ''`）；
- 文档登记（`.chanlun/definitions/xianduan.md` §7.1 第 7 条末、`docs/reference-theta-v0.md`、`.chanlun/review-results/issue1604/IMPLEMENTATION.md` §7）是否如实、未改语义；
- 提交说明是否带编号声明、不写 fix/close/resolve。

输出 Markdown：首行必须是 `结论：PASS` 或 `结论：BLOCK`；四条要求逐条「满足/不满足」及落点；新发现按 HIGH/MEDIUM/LOW 列（附文件:行号）；最后写看过什么、未核实什么。不要写任何 IP 地址字面量。
