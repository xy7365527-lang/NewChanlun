你是 NewChanlun 仓库 PR #1662 的非作者独立审查者（只读，不改任何文件，不跑 cargo，不调用 gh）。工作目录是该 PR 分支 worktree，HEAD 两个提交（实装 + 评审工蜂注释修正），base = origin/main 02b5b571eb。实装与评审工蜂都是 claude-opus-5-5，你是另一模型。

票 #1653 要做：1) 段账本拒读早返回原记 TowerTermination::Natural，新增 TowerTermination::ParseInvalid，拒读路径改记它，补断言锁（输入 segment_parse_invalid ⟹ termination == ParseInvalid）；2) #1603 意见 1：直写 null 路径补独立负控测试；3) #1603 意见 2：AwaitingPolicy 文档注释与现行语义同步；4) #1576 INFO-3：rule_revision 键缺失时文案改为明确缺失提示。验收：新断言旧红新绿；cargo test 默认档与 s_session 档全绿；-D warnings 零告警；非作者独审 PASS。

请审 `git diff 02b5b571eb...HEAD`（pipeline.rs、tb1581_tests.rs、policy_records.rs、s_session_v2/mod.rs），重点：
1. ParseInvalid 新变体：所有消费 TowerTermination 的地方（全仓 grep：match、code()/标记串、序列化、is_resource_truncated、S 会话/报告/夹具里对 NATURAL 的依赖）是否都正确处理，是否有下游把「拒读」误判为自然终止或资源截断，是否改变了任何已入仓夹具/对拍输出；
2. 断言锁是否真的锁住了「拒读 ⟹ ParseInvalid」（换回 Natural 会不会红），正控是否存在；
3. null 负控与 read_u32 的 is_null 分支是否一一对应；AwaitingPolicy 注释与 parse_header/read_u32 是否一致；
4. rule_revision 改 Option 后拒绝条件是否与原来等价，文案是否准确；
5. 是否有超出票面范围的改动、编号声明、注释带票号。
输出 Markdown：首行 `结论：PASS` 或 `结论：BLOCK`，列 HIGH/MEDIUM/LOW（附文件:行号），最后写明实际看过什么、未核实什么。不要写任何 IP 地址字面量。
