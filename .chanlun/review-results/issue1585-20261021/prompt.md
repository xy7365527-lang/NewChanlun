你是 NewChanlun 仓库 PR #1664 的非作者独立审查者（只读，不改任何文件，不跑 cargo，不调用 gh）。工作目录是该 PR 分支 worktree，两个提交（实装 + 评审工蜂修正），base = origin/main d07fc20438。实装与评审工蜂都是 claude-opus-5-5，你是另一模型。

票 #1585：#1581 在 classifier 入口对 segment_parse_invalid 整层拒读，但以下直读段账本的路径不经该入口：backtest/capture_oos.rs（按段确认开平仓）、backtest/wverify_run.rs（复算中枢级联）、classifier/diag/cp_recall_audit.rs、classifier/diag/cand_delta.rs、backtest/segment_gn.rs。验收：这些路径遇标记时显式产出「段解析无效」状态（拒读或在报告中标出受影响区间），不得静默使用该区间结构；对照臂不得据此产生任何交易记录写入生产账；每处一条负控（注入标记 → 输出带标记/拒读）；非作者独审。

请审 `git diff d07fc20438...HEAD`，重点：
1. 五处是否都处理且没有遗漏的其他直读段账本路径（全仓 grep `.segments`、`segments_confirmed_len`、`ParseLayer` 直读点，排除经 classifier 入口的）；
2. `segments_valid_prefix` 的语义是否与 #1581 标记一致（前缀边界是否可能包含失败段或丢掉有效段）；
3. capture_oos 逐 bar 增量：标记 bar 不开平仓、持仓受污染不入统计、标记区间上报，是否有路径仍会用失败区间的结构做决策；评审工蜂登记的「标记消失后第一 bar 一次性处理积压确认」是否构成静默使用失败区间结构（这是票面验收的关键判断）；
4. 负控是否真的锁住（把处理去掉会红）；正控存在；
5. 范围、编号声明、注释票号。
输出 Markdown：首行 `结论：PASS` 或 `结论：BLOCK`，列 HIGH/MEDIUM/LOW（附文件:行号），最后写明看过什么、未核实什么。不要写任何 IP 地址字面量。
