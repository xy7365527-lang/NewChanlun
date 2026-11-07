# #1505 D2 新轮远端独立验收

结论：**PASS_D2_COMPONENT_ONLY_CONFIRMED**，本轮审查发现 0 CRITICAL / 0 HIGH / 0 MEDIUM / 0 LOW。远端实际构建、单次 suite、执行后校验和监督清理均成功；独立逐原件复核通过。这个结论限定于 D2 组件原型，生产 v4 仍为 **BLOCK_NOT_MERGEABLE**，#1323/SPEC #1340 全图未完成，#1279 必须保持开放。

运行 [38773444490](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38773444490)，attempt 1，事件 `push`、分支 `proto/1505`，job `116554914228` 全步骤 success。HEAD `b9d3ecde1838e0db3eb67bfd01d364d624ec8619` / tree `daa492de47e80c261f173e9543542a6ac582536b`；父提交为已审源码 `e1d7e0b6bffb78f757e59cdfe3e35f3618e6790a` / tree `44d075dbd7eb99fbe388ee082acbca6beb9219cb`。两提交之间仅 CI runner 改动，Git blob 与本包已审 runner 逐字节相同；远端 API、SOURCE、BINARY、最终 RESULT 及 checkout 日志身份一致。

| 本轮验收项 | 独立实证 |
|---|---|
| 12 正控 | current/old/unknown 各 clean/cold/hot，original-longer、empty-journal、post-reader-kill 全部完成；各 48 行、完整查询 data 与虚拟 main 长度/哈希对拍相等；归档源文件重新逐字节哈希一致。old/unknown 仍不准入 |
| 11 负控 | callbacks、sql-denials、read-io、read-short、read-panic、late-exclusive、late-exclusive-clean、close-error、allocator-failure、arena-oom、path-swap 全部命中各自语义 |
| L01 | prepare=0、step=100、close=5；关闭拒绝后 active_files=2，原连接与 owner 保留；authorizer 恢复=0、finalize=0；最终 active_files=0、FD 关闭、注销，SQL 拒绝计数 7 |
| 10 super-journal | 4 项有效 footer 注册前拒绝，registrations/open_calls=0；6 项无效 footer 的 oracle/reader 边界对拍完成。外域 sentinel 哈希一致，非法边界不记作合法热恢复正控 |
| 2 tail | truncate-seven、bad-last-byte 完整边界对拍，明确 positive_hot_proof=false |
| 真实 kill | 3 次 writer SIGKILL/wait，READY 前实际 spill、main 改变、多段有效 journal；1 次 reader 在实际查询及 392 次 overlay write 后 SIGKILL/wait，随后 post-reader-kill 正控成功 |
| 关闭和分配故障 | close-error 实际 close rc=4106 且最终释放；malloc/realloc/hard-heap 拒绝命中并保留原 realloc 字节；arena OOM 在注册/open 前拒绝、自有分配归零 |
| 路径替换 | 查询仍来自 pinned 原件，binding_changed=true、accepted_rule=false，替换路径原样保留 external-sentinel |
| 字节与参数 | 独立 Python bytearray 重算全部 10 次操作长度/哈希，最终与普通文件 oracle 相等；六键的 missing/zero/overflow/negative 共 24 拒绝例及 unknown field 拒绝齐全 |

## 两条 late-exclusive 证明

两条证明均由本轮原 trace 重新核对，具名 proof 与重建 JSON 完全一致。它们各自保存原件，不靠“任意 SQLite 错误”放行。

- **hot**：main 仅成功读取头部 100 字节；journal 两次打开 flags=2049/2050，均返回 `SQLITE_CANTOPEN=14`。没有非头读取、没有 arm、没有 overlay 写/截断/虚拟删除。
- **clean**：只有 main，无 journal/open-denied；头部 100 字节读取成功，随后 offset=0、len=1024 的非头读取返回 `SQLITE_IOERR_READ=266`。没有 arm、没有 overlay 改写。
- 两者四份来源快照相同，归档文件长度/哈希相符；最终 active_files=0、FD 关闭、注销、SQLite heap 和 owned allocation 均归零。

旧 run `38767596239` 及 `1505-d2-remote-review-v1/` 的失败判定与原件完全保留。本报告所有运行证明均来自新 run，没有补入旧轮原件，也没有将旧失败改判为通过。

## 构建、执行后二进制校验与监督

macOS 26.6.2 arm64，Python 3.12。Cargo 实际构建 53.647401 秒，唯一目标产物 `fresh=false`，没有编译 error；原日志记录 lib 57 条、原型 bin 4 条 warnings。suite 用同一个 BINARY.path 执行，13.009330 秒 exit=0；verify-suite 0.527599 秒 exit=0。

二进制 SHA-256 为 `5f77d47d9c5a1c9c72975208b5d9b7439e3293d9a5667a84e416bc14588aa4f6`。执行后校验证据成立：已审 runner 的 `verify_suite` 在生成 SUITE-VERIFIED 前先重哈希 BINARY.path，与 BINARY.sha256 比较，再复核源码身份；本轮该阶段实际成功，生成的回执指向本轮 raw RESULT 哈希 `fe8da41db1d288d5fd97e7601d234dfd5600c8751b2427706fc776bbe84dd0b4`。回执保留 accepted=false，表示组件校验并非生产验收。二进制随后随 scratch 删除、未上传，本机没有重算已删除的二进制。

61 份正常子进程退出/回收收据、四份真实 kill 收据均已核，PGID 对齐 suite=3225，所有绝对时间落在本轮同一个 CLOCK_MONOTONIC 窗口。35 份已结束 reader 的限额释放报告通过：源 FD readonly、IO methods v1、payload/metadata/trace 各自容量不越界，SQLite xSize 观测峰值最高 717,840 bytes，自有 layout 观测峰值最高 13,631,488 bytes，最终二者 current 账归零。被 SIGKILL 的 reader 单独以真实 wait/reaped 和随后源字节不变、重读成功证明；不声称它执行了正常关闭路径。

callback 专用臂的 sqlite_worker_threads=-1 是源码 `driver.rs:525-532` 在 connect 前返回、`vfs.rs:1079` 初值 -1 的未连接标记；其余已连接报告均为 0。独审脚本首轮误把该额外字段统一要求为 0，已依据源码修正并重新核验；详细记录在 EXECUTION.json。未改变远端原件或产品代码。

identity/build/verify-build/suite/verify-suite/preserve/archive/cleanup/inventory 九阶段均 exit=0，未超时、未取消、组已回收。整轮监督器 71.200462 秒，unreaped_groups=[]，scratch_removed=true，无 LATE/CANCELLED。真实监督器 SIGTERM/SIGINT 和实际 OS EINTR 仍 NOT_RUN；容量证据不构成进程 RSS、allocator 私有开销或同步 I/O 硬截止的证明。

## 原件封存与可复用范围

唯一 artifact `11992078505`，2,451,897 bytes，ZIP SHA-256 `6161c4eb94e4fb82eb11f5769cd56e4a95f40472513decaad873c10ad62a88a3`，本机重算、GitHub API digest 和上传日志一致。443 个 ZIP 成员逐字节核对；FILES.json 封条 SHA `cdb780e0ecef80cdcce46b4fd20fc1927cba4dcd12b7838766576dbe60674b8e`，437 项逐长度/哈希核验，其余 6 件恰属终结排除项。405 件 tar 成员与 raw-evidence、归档清单三方字节一致。

runner SHA `0ef124bd379fe6dcc14284f43ace57e90ce4c4051c301bc027dd7e676c17bf15`；workflow SHA `53e05ad8392d1593ab9f3212ef5db3ffd32addc59d06fc74d2c90888d852650e`。本包包含原 ZIP、完整 job/build/suite 日志、全部 raw-evidence、API 元数据、已审 runner 副本、检查脚本、执行回执和 VERIFIED.json。`python3 -B verify_remote.py` 校验 exit=0；本机 ruff/mypy/pylint/black 均不在 PATH，没有宣称运行这些工具。

尚未完成的契约范围沿用原件明示：正式 producer/R4/CLI、完整 S store 基线、source RW final-open 连续性、同步 I/O 硬截止、49 GB 历史、64 GiB/store、148 GiB 双跑前提及全图验收。本包没有执行本机 Rust/cargo/SQLite/真实信号/产品/browser/性能测试，没有访问 AgentStorage，没有 commit、push、dispatch 或再次轮询旧运行。后续步骤由主控按组件边界决定。
