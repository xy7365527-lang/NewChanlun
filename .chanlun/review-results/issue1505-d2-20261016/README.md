# #1505：D2 普通 hot-journal 私有回滚原型结论

本目录是 #1505 的工作草稿，结论回喂 #1405/#1420，归 #1323/SPEC #1340。只归档已完成原型的报告和证据索引；不收编原型代码、不解除生产恢复 v4 的 BLOCK_NOT_MERGEABLE、不关闭 #1279。

## 实际结果

固定 O_RDONLY main/journal FD 与受限非默认 SQLite VFS、私有字节 overlay 的机制，在具名 D2-SMALL-01 测试域中取得非空恢复成功。SQLite 自己解释日志；12 正控、11 负控、10 super-journal 边界和 2 tail 边界完成。3 次 writer 和 1 次 reader 的真实 SIGKILL/wait 已核，随后重读成功。旧/未知规则拒绝且来源零写；hot journal 的更早 CANTOPEN=14 与 clean main 的 IOERR_READ=266 分别验证。

新轮唯一运行：[38773444490](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38773444490)，attempt 1，job 116554914228，success。精确候选 `b9d3ecde1838e0db3eb67bfd01d364d624ec8619`，源码 `e1d7e0b6bffb78f757e59cdfe3e35f3618e6790a`；原型永久留存于 `proto/1505`。独立审查 0C/0H/0M/0L，根再核 455 件清单，结论为 **ACCEPT_D2_COMPONENT_ONLY**。旧失败运行 38767596239 不改判，也未混入本轮证据。

## 原件与归档范围

- [独立报告](REVIEW.md)、[结构化结果](RESULT.json)、[根验收](ROOT-ACCEPTANCE.json)分别记录核验者范围。
- [源码身份](GIT-IDENTITY.json)、[独立逐项核验结果](VERIFIED.json)、[完整远端证据清单](REMOTE-EVIDENCE-MANIFEST.json)保留原字节。
- 完整原件位于本轮 [artifact 11992078505](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38773444490/artifacts/11992078505)。ZIP 为 2,451,897 bytes，SHA256 `6161c4eb94e4fb82eb11f5769cd56e4a95f40472513decaad873c10ad62a88a3`。443 ZIP 成员、437 FILES 条目、405 tar/raw 原件已独立逐字节核验。
- 本目录仅纳入上述六份报告/索引及 README/INDEX；完整 raw、ZIP、日志和验证脚本不随此文档提交入仓。REMOTE-EVIDENCE-MANIFEST 中的相对路径指原证据包，不指本目录。远端 artifact 遵 GitHub 保留期，本机冻结原包继续保留；此处没有永久远端保存全部原件的声明。
- 原件的根验收记录当时 `git_report=PENDING_ARCHIVE`，本次归档保持该历史字节；是否已合入以本目录的实际 Git 提交及 main 状态为准。

## 未完成项与下游

D2 真实监督器 SIGTERM/SIGINT 取消及实际 OS EINTR 仍 **NOT_RUN**；fixture SIGKILL 不替代监督器取消。组件容量账不证明进程 RSS、allocator 私有开销或同步 I/O 硬返回期限。

正式 producer/R4/CLI、完整 S store baseline、源 RW 最后打开的身份连续性、同步 I/O 硬期限、49 GB 历史、64 GiB/store 和 148 GiB 双跑条件继续归 #1405/#1420 及既有下游。全图资金、隔离、恢复、旧入口和历史/paper/live 验收未完成；产品前端归 #1460，K4 排除。

本次为纯文档归档，编译豁免；没有执行本机 Rust/SQLite/信号/产品运行，没有访问外盘。报告入分支不等于 main 已交付或本票已关闭。
