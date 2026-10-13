# #1394 S3/Q3 同源联调 revision1：独立事实复核

**PASS_BOUNDED：9 个检查组通过，新增事实异常 0。** 射程仅为自然可写 S 源库与独立只读 Q 的一个原始单 bar 联调域。作者 FACT-REVIEW 作为待核材料，本报告独立读原件、重算 hash 与内容节点，没有启动服务、build、unit、新矩阵或打开 SQLite。

| 检查组 | 独立结果 |
|---|---|
| 新旧包完整性 | revision1 manifest 全 132 条、旧包全 131 条的 bytes/SHA 实读一致；旧 MANIFEST 仍为 e358…58a98，未被覆盖。 |
| 当前源与 binary | 根 HEAD ed1、Rust tree 5a2 与包绑定相同；6 份 product 文件当前完整身份一致，原候选 6 个变更文件逐字等于 46d。冻结 binary 当前完整身份、18,609,984 bytes、SHA 5437…18e91 一致，不重扫 574 源文件。 |
| 旧原件 | 102 项 BEFORE/AFTER 一致，当前 live bytes、SHA、mtime、inode、device、UID、mode 全核一致。未碰其他运行工位的 DB。 |
| 权限与 Q 无写入 | 当前 harness 无 chmod，原包仍含额外 db.chmod(0444)。新源 DB/WAL/SHM 自然 0644；Q startup/读取前后完整物理字典、四次 observer 前后物理记录及 gen0/gen1 语义快照不变。活 WAL 实有 350,232 bytes，未用 immutable 或 closing 偷换观察。 |
| 正式进程与资源 | raw Controller init 11178、S 11179、Q 11194 的 argv/UID501/同 DB 实核；原始 ps/lsof 与登记身份相同。两个 nonce 不同，私有 S_CONTROL_RESOURCE_SHA256 仅 S，Q 为未声明；S 的 writer.lock 实持 fd3u，Q 主 DB 为 fd5r。capture 正本对应两段 READ_ONLY｜NO_MUTEX、query_only 与校验行逐字同当前源。S/Q 原资源 d6…53dad/50ea…d4eb1 保持。 |
| 原始公共回复 | 独立核 4 个 S canonical+LF 信封及 7 个 Q native public 信封，exact 11 字段、payload SHA、原请求 causal、reply ID、session/generation、S/Q producer/source namespace/epoch 都一致。Q Ready 原始 HTTP 状态与本次 Q nonce 相同。 |
| 同源发布与查询 | Q 先 gen0 Snapshot/content 空；S 原 message1 实际 Committed、receipt 全字段相同、clock10/11/12；随后 gen1 Snapshot 7 行，Q fixed cut/index_frontier 与 S publication 及持久 publication/delta 根一致，Watch0→1 恰一 delta。旧 cut0 content 与显式 as_of_generation=0 请求仍返回空代，不冒充当前 gen1。 |
| 内容与业务根 | 独立重算 1845 个持久源节点 framing hash、三份 HTTP content 共 109 个返回节点 framing hash。gen1 的 7 个 item root 按实际 row metadata 重算一致，含 49 个局部 proof 节点。raw_history 所选根从同一持久节点集独立解码，OHLC、源坐标、namespace/epoch、事件、receipt 与源账及原 message1 一致；objects/observations 行根对拍持久 birth_wire_root。 |
| 终态、清理与失败保留 | S SIGTERM 实际 -15，Q SIGINT 实际 0，原 Popen wait/reaped 记录一致；当前两个 PID 均不存在，自有 socket 目录不存在，原包有终止后 HTTP58882 rebind 成功记录。本轴未重新 bind。首包额外 chmod 与 FAILED-VERIFY-1 的 missing-node KeyError 仍保留。 |

Q 的只读性质由连接的 READ_ONLY 与 query_only 及主 DB fd5r 支撑，不要求源文件整体只读。SQLite WAL/SHM 协调 fd 可以显示 u；当前完整物理/语义前后证据均未变化。本次接受的是正常可写 S shared source，新包没有以 chmod 制造这种结果。

1845 个源节点来自持久 DB 观察，不能算成 HTTP 已下载的完整 closure。49 个 proof 节点只覆盖 path/header；本报告验证 7 个 item ref 与 metadata 的根绑定，没有把 fragment 递归解码成 Client 完整安装。旧 FAILED-VERIFY-1 正是把局部 proof 当完整递归闭包而 missing-node 的验证器失败；它不被覆盖或算入本轮成功次数。

未证明 Browser、Client 实际分配、全部 source 覆盖、生产容量、资金权限、条件 FIFO、9011、整图或 main 集成。没有把未测 owned SQL/mount/Client 分配填成 0。exec74446 terminal exit0 来自父任务运行时证据；本轴独立核进程终态原件和当前 PID。此通过不关闭 #1394/#1323。

固定证据：

- ROOT HEAD：`ed1f184bf7513d960d87b6687c279a657c8fa481`
- 原候选：`46d183a20afe2c68e01e26555531663a7810f065`
- Rust tree：`5a2acd1e72c3846be38cc29689a82aeb57c68dc6`
- binary SHA256：`5437fbb5d581d1b48f6a1d3405cae67327113e5a2c7fedd7ce3a1d9ce1e18e91`
- RESULT SHA256：`da9abe58385530b7d875555659490e84c379ca3989aac5d9d5b18c4418614b18`
- MANIFEST SHA256：`f39b80bbcc7244245725eb770a27976903640d4d5277e0f5cc7f4116deab1aa6`
- 旧 MANIFEST SHA256：`e3581d6f189bf446b8952f9487336da6f9b8d9f6ddac321e5405d81e26f58a98`
- raw_history 选中根：`sc1-146ef9e11171c44c5c4e1c6223305e9b0c2445ab43582b41ec7dbcd23d7d399b`

完整机器检查与原件计数见同目录 INDEPENDENT-FACT-REVIEW.json。本轴仅新增这两个文件，未改产品、旧包、其他工位、原件、main、push 或 tracker。
