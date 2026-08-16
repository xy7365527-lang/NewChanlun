# Agent Roster 2026-08-16

| 类型 | 模型 | 任务 | 状态 |
|---|---|---|---|
| rlm 子代理 `sandcastle-research` (sub-414617b9) | kimi-coding/k3 | #999 sandcastle 官方用法调研（GitHub + AI Hero），产出 chanlun/review-results/issue999-sandcastle-usage.md | 在飞 |
- [!] 2026-08-16 22:40 | ⚠ 并行会话交接提示：afk3 的 in-flight 改动（cli.py/server.py/test_cli/test_server）因 stash 交叉被收进 stash@{0} 或 stash@{1}（均标 "parking-after-922"），恢复：git stash list 核对后 pop 对应条目
- [x] 2026-08-16 22:30 | 本体（GLM-5.3）| BCD 线收官：#919 代码 cherry 至 ticket-919-final（main+1，2828/0 全绿待并）；#922 由并行版 a1d09cf44a 落地（我的两臂版弃）；#913 盘点报告落票（215 条/66 文件三类分法）；#924/#912/#903 已并
- [!] 2026-08-16 23:15 | 交接：#919（Bar.volume f64）分支 `ticket-919-final`（1d8a68d082）rebase 至 main 2d9f18e084 完毕、lib 测试 2695/0 全绿、FF-ready——afk3 释放 main 后 `git checkout main && git merge ticket-919-final` 即收；#1022（TwStepCtx 编译红）是 afk3 #648 抽离中间态遗留，非 #919 阻塞（default/lib 均绿）
