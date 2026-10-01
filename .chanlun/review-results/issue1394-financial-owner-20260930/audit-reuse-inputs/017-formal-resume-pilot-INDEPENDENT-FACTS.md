# #1394 S16 无输入 CLI ABBA 独立事实核验

状态：**PASS，仅限本次现成正式二进制的 S16 无输入 CLI 对照**。独立只读重算四个克隆 DB；未重跑 CLI、构建、服务或 full9011。观察脚本 SHA256 `7b3a888adc66239bc041be6f4024cfb31f0f754a6835db2130531719ef31bb9f`，与 `BINDING-BEFORE.json` 一致。

| 顺序 | 二进制 | CLI exit | wall 秒 | RSS 50 ms 采样最大 B | 采样数 |
|---|---|---:|---:|---:|---:|
| A | 旧正式版 | 0 | 7.157963834 | 74,022,912 | 119 |
| B | 候选正式版 | 0 | 6.926671875 | 84,787,200 | 114 |
| C | 候选正式版 | 0 | 6.231108375 | 84,869,120 | 103 |
| D | 旧正式版 | 0 | 7.276960042 | 76,578,816 | 120 |

四份克隆均以 `mode=ro` 重新打开，独立读取全部 schema、`PRAGMA table_xinfo`、24 张用户表的 48,403 行及 96,397 个 BLOB 单元；按类型敏感的规范 JSON 对正式 A `final.fullDB.json` 逐字段比较，全部相等。schema 为 43 条，DB 属性、表名和行数亦相等。每份 `raw_events=16`、`s_input_messages=16`、`catalog=116`，`AcceptedPending=0`；没有新增原始输入、消息或目录行。各轮独立克隆 inode 不等于旧 owned source，复制前 SHA256 相同。

四份原始 `stdout.json` 字节完全相同，解析字段与每轮 `RESULT.json` 一致：`ok=true`、`command=advance`、`generation="16"`、`accepted_pending="0"`、`structure_cut="cut-16"`、`writer_epoch="1"`；每轮 stderr 为空。命令均无 `--input`。每轮 `RESULT.json` 与总 `RESULT.json` 对应记录一致。原始 owned source DB/WAL/SHM 以及二进制、profile、clock、正式 A 预期 JSON 的路径/字节数/inode/mtime_ns/SHA256 均与运行前绑定相同。观察脚本自身身份也相同。

四个 PID 在核验时均不再存在；四份克隆及旧 source 的 DB/WAL/SHM 共 15 个文件的 `lsof` 检查无打开者。全部样本时间单调且位于记录 wall 区间，重新取最大 RSS 与四轮结果逐一相符。二进制 SHA256：旧 `b4c81c9468be1d1911eecbd48deb1be9e9a2f3a0d082fd8f7474cf9e30efc7f5`，候选 `c098bf76bb7870443b7564714ba2f9110f2c5593a3180ca13f9888c01ee16d17`。旧构建绑定 HEAD `ed1f184bf7513d960d87b6687c279a657c8fa481`，当前旧工作树 HEAD 已是 `818e024fdcfacb94564bd97d0be92890f7ab649b`，但 Rust tree 仍为绑定的 `5a2acd1e72c3846be38cc29689a82aeb57c68dc6`，15 个绑定源码 SHA 均相同。候选 HEAD `c6255d308fc69d94178916a1dc5534546fb77591`、Rust tree `7196a77fef56cf87cbe91dc4b135326a142e2395` 与绑定相同；五个候选变更源码 SHA 均未漂移。

旧版两轮 wall 均值 7.217461938 秒，候选两轮 6.578890125 秒；这是本次 ABBA 的观测值，未作冷缓存控制或统计推断。wall 包含子进程启动与轮询发现延迟，轮询目标 50 ms 且 `ps`/磁盘采样有额外开销。RSS 是采样下界，不是精确峰值；heap peak、F/SQL/阶段计数均未测。静态审查中“单轮无外层超时”的 MEDIUM 边界仍在，本次自然退出不证明未来不会挂住。此 PASS 不适用于正式服务资源门、instrumented probe、full9011 或整图。
