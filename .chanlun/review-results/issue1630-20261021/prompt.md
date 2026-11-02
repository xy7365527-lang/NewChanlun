你是 NewChanlun 仓库 PR #1659 的非作者独立审查者（只读，不改任何文件，不跑 cargo）。工作目录是该 PR 分支的 worktree，HEAD = b34edde595，base = origin/main 02b5b571eb。

票 #1630 验收：定位 replay-drop 模式累计分配轻微超线性的来源；修复或给出有界证明与读数；对拍逐位相同；非作者独审。

请审：
1. `git diff 02b5b571eb...HEAD`（rust/src/bin/issue1588_parse_mem_probe.rs 与 .chanlun/review-results/issue1630/REPORT.md）。
2. 对照库源码（rust/src/theta_v0/parser/segment.rs 中 IncrSegments::append、resume_seg_start、confirmed_len、find_overlap_start 等）核实 rescan_span 计算是否与库实际续扫起点一致、是否可能越界/panic、是否影响原有摘要输出。
3. REPORT.md 的定位链（旧分支 1bc3534014 复现 → 二分 → #1582 消除）在逻辑上是否自洽、结论是否被读数支撑、是否有夸大；可用 git log/git show 核对报告引用的提交（如 24c2fa068f、439be77560、1bc3534014 若可达）。
4. 交付纪律：提交说明不得含 fix/close/resolve #号；编号声明区分 Issue/PR；注释带票号。

输出 Markdown：首行 `结论：PASS` 或 `结论：BLOCK`，然后列 HIGH/MEDIUM/LOW 发现（附文件:行号），最后写明你实际看了哪些文件与命令、哪些没核实。不要写任何 IP 地址字面量。
