你是 NewChanlun 仓库 PR #1663 的非作者独立审查者（只读，不改任何文件，不跑 cargo，不调用 gh）。工作目录是该 PR 分支 worktree，HEAD 两个提交（实装 703e3d1c7f + 评审工蜂 64af6270e8），base = 02b5b571eb。实装与评审工蜂都是 claude-opus-5-5，你是另一模型；分支自带的 `.chanlun/review-results/issue1604/REVIEW.md` 也是同模型子代理所做，不算非作者独审。

票 #1604：theta_v0 线段特征序列尾窗大小（原 `TAIL_WINDOW = 7`）并入 `second-seq-scan-budget` 下一版本字段，耗尽标 `INCOMPLETE_RESOURCE_BOUND` 且与正常确认段可区分，7 与更大窗读数差异写进报告；相切边界 `tangency ∈ {open, closed}` 全级同值（#804 对拍锁），默认 closed（现行 `<=`），Python P1 同读该字段。共同验收：运行路径只从当前生效的版本化记录读取，每份输出写出记录 id 与版本；生产/回放/准入路径不再有这些量的代码常数（Default 只作测试夹具）；缺记录/缺字段/非法值 ⟹ AwaitingPolicy 或显式报错，**不得回退到旧常数或任何备用值**（#799）；负控测试各至少一例，改回常数的变异体须红；等值对拍输出与改前逐位相同。不在本票：旧线 `rust/src/segment.rs:410/:413` 归 TB-10。

请审 `git diff 02b5b571eb...HEAD`，重点：
1. 是否还有任何生产/回放路径残留尾窗或相切的代码常数，或「记录读不到就用默认值」的回退（逐个看 `ParseConfig` 构造、`impl Default`、`from_active_records`、`unwrap_or`、Python dataclass 默认）；
2. 尾窗截断标记：`probe_tail_excluded` 的正确性（漏标/误标）、增量与全量一致、与正常确认段可区分、上层可见；
3. tangency：全级是否真的只读同一值，open/closed 两个夹具是否符合各自定义，Python v0 改口径是否在票范围内且已登记；
4. 等值对拍：签署值下输出是否逐位不变（尤其 tb02/tb1493/tb1581 夹具与 S 会话 TB-02-C 的 `tail_cache_window`、`resource_truncated` 新字段是否改变已有输出/夹具/序列化）；
5. 评审工蜂撤回旧线接线是否干净（无残留死代码、无半截改动），旧线是否与改前逐位相同；
6. 提交说明与报告的编号声明、注释票号；报告结论是否被测试支撑、有无夸大。
输出 Markdown：首行 `结论：PASS` 或 `结论：BLOCK`，列 HIGH/MEDIUM/LOW（附文件:行号），最后写明看过什么、未核实什么。不要写任何 IP 地址字面量。
