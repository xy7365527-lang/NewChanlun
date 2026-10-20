# #1394 S full9011 A/B 首试实际证据复核

**实际状态：`FAIL_OR_PARTIAL_BOUNDARY_CONFIRMED`。** 仅 A fresh 域启动；客户端确认前 27 条，第 28 条在原 30 秒期限内返回 `DeliveryUnknown`。B 未创建或启动，未重试；`complete9011=false`。这不是服务通过、C 完成或整图完成。

前 27 条逐项核实：冻结请求与实际发送 wire 逐字一致；原回复 wire 的原生因果身份与 Hash、持久收据和三相 clock、进度 Hash、最终 raw_events/raw_ohlc 来源全字段一致。导出的全库快照通过 SQLite integrity/foreign-key 结果核验及 native 节点 Hash 检查（104,465 节点）；它不等于完整 9011 最终验证。

第 28 条请求 1,004 字节已完成 sendall，实际回复 wire 为零字节；exchange 用时 30,007,694,666 ns，记录 `DeliveryUnknown`。逐请求持久证据与停机后全库导出一致：`message-28` 为 `AcceptedPending`，`accepted_seq=27`、`attempt_count=1`、`first_published_generation=NULL`，只有 accept/begin，缺 commit；没有第 28 代 publication 或 Delta。最终库有 28 条 raw 与 message、27 条 publication/Delta、83 条 clock（28 accept、28 begin、27 commit），`next_attempt_ordinal=29`、clock frontier `185`。客户端 27 个确认不能替代第 28 条回执，更不能推成 9011 完成。

A 的 `CASE-RESULT` 为 `PARTIAL`，27 个客户端确认；全 9011 最终验证因条数不足失败，已保留 `final.fullDB.json`。清理六门全真；init PID 91598 返回 0，S PID 91605 经 SIGTERM 返回 -15，均为原 Popen reap，复核时两 PID 已不存在；socket 目录已移除。runner 自有 38 份原件首末一致；Root 外层 27 份原件 PRE/POST 逐条记录完全相同，原 SQLite 未打开。

S7 SHA256 `b3d8b67b723d881a98dd0f4116fdc9651cf05b207111b19dad26fa69c2c54e69` 未变。2,582 个 RSS 样本的单进程最大读数 222,101,504 字节只是样本下界；严格 RSS/heap/IO/内部阶段峰值 `UNMEASURED`，完整资源容量仍为 false。Q/HTTP/audit 未启动，C 与整图仍未完成。

关键原件 SHA256：

| 原件 | SHA256 |
|---|---|
| `RUN-RESULT.json` | `1bfde3d806c0a630339e460e5bb4e2ee12dcec4dd31a01c22c5ca22760fcdad6` |
| `A/CASE-RESULT.json` | `5cd834a782039174d4e303bbdb4c9f774ae9ef84e28a392742b4c4701a680486` |
| `A/final.fullDB.json` | `14f854f8abcf66626d506e231ad83e22bf4661e6d2cbf6072408ccd253736fe1` |
| `A/input-28.exchange.json` | `cc8d6850eb232263875404b6e859b9ce15852a12d581af1fadbaad85d4bc3920` |
| `A/input-28.persistent.json` | `8c889563664bce5d97e3c63f82b1b3789811168e276b109e76444ac803f70d49` |
| `A/input-28-0.request.wire` | `5aabb944eb76a57c5bdaf3125e0b82a984fbbcb5bb215842bab17742ffcc2a36` |
| `A/input-28-0.reply.wire` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

Root 27 原件前/后记录 SHA256：`3bca89e81f1bfcf00f1614abfecd344bd439f45c98e2a8cfb3808d5728af901b` / `f02feffb9cc279159710d427b6758adb1b70b3cf73fe03c688f9c5f76a6562ce`。本复核只读已导出的 JSON 与原 wire，未打开 SQLite 或重启任何服务。
