你是 NewChanlun 仓库 PR #1661 的非作者独立审查者（只读，不改任何文件，不跑 cargo，不调用 gh）。工作目录是该 PR 分支 worktree，base = origin/main 02b5b571eb。

票 #1660 验收：harvest.sh 不再出现关票、合入、推 main、本机 cargo；对宿主克隆的工蜂分支能推分支并开 PR（Refs #n，不含关闭关键词），已有 PR 时不重复开、不重复评论；提交说明含关闭关键词的分支 fail-loud 不推；评审未完成或有 open blocker 的分支跳过；有离线测试；launchd 定义入仓（不安装）。硬约束：绝不自动关票、绝不自动合入。

请审 `git diff 02b5b571eb...HEAD`（.sandcastle/harvest.sh、harvest.test.sh、harvest.test.mts、install-sandcastle-harvest.sh、launchd plist、README、graph-engineering.md），重点：
1. harvest.sh 是否存在任何路径会关票、合入、推 main、force push、删分支、跑 cargo；
2. 关闭关键词正则是否漏 GitHub 实际支持的写法（close/closes/closed/fix/fixes/fixed/resolve/resolves/resolved，冒号、owner/repo#n、大小写），是否误伤常见 `perf(#1629):`、`chore(probe): #1630` 写法；生成的 PR 正文与登记评论能否意外包含关闭关键词；
3. set -euo pipefail 下的陷阱（SIGPIPE、空变量、子 shell 退出码）、bash 3.2 兼容、分支名/票号解析、远端分叉判定、reviewer_done 判据（workers.jsonl 语义：implementer started → reviewer completed）；
4. 幂等性：重复运行是否会重复开 PR 或重复评论；PR 曾关闭的处理；
5. 测试是否真能抓住回退（不是只镜像实现）；你可以运行 `bash .sandcastle/harvest.test.sh`。
输出 Markdown：首行 `结论：PASS` 或 `结论：BLOCK`，列 HIGH/MEDIUM/LOW（附文件:行号），最后写明实际看过/跑过什么、未核实什么。不要写任何 IP 地址字面量。
