# #1394 正式 start_q 实测原件独立核验

结论：**PASS_BOUNDED**。固定 Controller `71e473bd54f46c31159a742a2b3ac706ac04a7da` 的两个 fresh 单 bar Q21 实例，正式 `start_q → Ready → Snapshot → stop → reap/rebind` 证据链完整。本次只读核验，不重开工程审查，未启动服务、发送 HTTP 请求或信号。

最终清单 `FORMAL-ACTUAL-MANIFEST.json` SHA-256 为 `c54ed3301e1c141c17d640deccea5cf2e63442303f02b56d5e2333f5d9f60506`。逐文件读取全部 67 份原件，bytes/SHA 全部匹配。36 项独立检查全部通过，逐项结果及完整文件检查索引见 `INDEPENDENT-ACTUAL-CHECKS.json`。

`run_formal_actual.py:156-166` 的 spy 记录实参后原样调用已保存的原 `start_process`，随后真实调用 `Controller.start_q`；未替换 Ready、argv 或身份判断。`:173-201` 通过 socket 保存真实 GET Ready 与 POST `/api/v3/snapshot` 原文。独立核验 HTTP 200、Content-Length、body 原件一致性，并离线使用当前 `validate_reply` 核实两个 Snapshot 的 schema 3、session/generation、Q producer/epoch、payload_hash 及因果回引。两次响应均为 done、next_token=null、7 条 rows，cut_generation=1。未把 ContentRef 响应解释成完整客户端内容恢复。

| 实例 | PID | 端口 | control_instance_id | 退出信号 | 结果 |
|---|---:|---:|---|---|---|
| sigterm | 26584 | 50299 | 01222dcd-9abe-4934-90f7-18f48219b35c | SIGTERM | exit 0，reap，rebind |
| sigint | 26648 | 50341 | 881b73cf-2bdf-41c2-9b51-2f13528095b4 | SIGINT | exit 0，reap，rebind |

两实例 PID、nonce、端口彼此不同。Ready 原文、Controller 返回、进程记录、运行状态、完整命令和配置中的 session/epoch 一致。`:205-229` 调用原 stop、等待本次 Popen、核 stopped、lsof 无监听并成功重新 bind。原 stop/reap 记录与事件相符；本次独立 `ps -p 26584,26648` 返回 1、stdout/stderr 为空，没有存活 PID。rebind 依据为原运行脚本及结果，本次没有再占用端口。

源码与 binary 分开绑定。Controller SHA 为 `bac0956929d861b8a359ac187e2fd1d2dbd6adb96f2271701f9eaca860c6f1f9`。binary 为 07 工位 debug 输出，SHA `991dd18dd0239b4d234a1f607cf72b405b79ec2a75ee7a3350c33d7a7cfca5b4`，与保存的实测 binary 逐字节一致。07 工位 HEAD 为 `08241aa12fa429245ba5132e6b6c2480f40029e4`，tracked clean；该提交和 Controller 提交的完整 Rust tree OID 均为 `a1743b5626629d7dfe9ef2ebf2ef3281b8190ad8`。另核三份保存的 Rust 源码与固定提交 blob、07 工作文件相同，现存 build 日志含 dev profile 完成。`OLD-BINDING.json` 保留旧 binary 信息，只用于旧资源来源，未将其旧 binary SHA 当成本次产物。未独立重建 binary。

Q21 文件及两次启动日志冻结的配置一致，SHA 仍为 `50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1`；21 项数值未改。源库 main/WAL/SHM 的运行前两次采样、运行后记录和本次当前 bytes/SHA/mtime 完全一致。两个 clone main 的 bytes/SHA/mtime 均不变，原验收记录中 WAL 为零字节；clone 曾产生 SQLite 侧文件，不将其描述为目录从未变化。

清单冻结顺序已核实。事件最后的 complete 引用初始清单 SHA `585015ca5ef54be08fe0d0710ee2e80d1563539cec46a3443aba70bce906ec13`，与保留的 `FORMAL-ACTUAL-MANIFEST-initial.json` 相同。初始清单的 events 索引恰好对应 complete 追加前的原文字节；最终清单仅更新该 events 索引并新增 receipt_finalization，其他原件条目和结果均不变。因此可复核当前最终索引，不能称初始索引也匹配最终 events 文件。

本结论仅覆盖现有正式 Controller 入口与具名小源实例。不代表完整 Consumer、Browser、S3/S16 全面验收、全部资源、9011、整图完成或合入批准。
