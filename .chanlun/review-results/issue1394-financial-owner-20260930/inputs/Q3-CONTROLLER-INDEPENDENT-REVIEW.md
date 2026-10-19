# #1394 Q-only Controller 独立事实核验

结论为 PASS_BOUNDED。36 项定向核验全部通过，manifest 的 98 个具名原件均重新计算了字节数与 SHA256，没有差异。本次未重跑服务或小 Q3 HTTP 内容案例，仅新增本报告及同名 JSON。

核验对象为 `/Users/silencehan/Projects/NewChanlun-1394-content-v3`，HEAD 为 `c7e9827cd0b861cb051008b4d7967d22fc5ee48c`。最终 `ACTUAL-CONTROLLER-MANIFEST.json` SHA256 为 `1c5a1018b76dc4fbdb1b896862f837ea09217f6acdfbe946daa7cd4d74c8ba2d`；原报告实际文件名是 `RESULTS.md`，SHA256 为 `9d8a45983e51f4daf869f34541c3b312e395060dd1a1a3dd6f720c69156643f7`。

两次成功都来自 attempt2。SIGTERM 实例 PID 90778、端口 50221，nonce 为 `f08e23f7-f7f1-49bb-9aef-91b0bfd8dcaa`；SIGINT 实例 PID 90812、端口 50242，nonce 为 `6c8c7e85-e839-4101-aa33-05a6e1f3cb34`。二者使用不同 DB 副本及 state 目录，启动记录、Rust argv、UID 501、session、query epoch 1、Ready nonce、PID/启动时间/命令身份一致。原始 HTTP 为 HTTP/1.0 200，完整 response 的 body 与独立 body 文件逐字节一致；解析后的 Ready body 与 Controller 返回 reply 全字段一致。

命名 harness 的成功路径直接调用现有 `Controller.start_process("q", argv, env)`、`wait_ready("q", query_epoch="1")`、`status`、`stop` 和直接子进程 `proc.wait`，成功路径没有 mock。Controller 的 `start_process` 生成 nonce 并写入子进程环境；`wait_ready` 检查 session/generation/query epoch/nonce，并由 `confirm_ready_process` 再核 PID 和 process_identity；`stop` 复核后只发信号给该 PID。源码定位是 `s_session/s_service_control.py:314`、`:382`、`:434`、`:481`，逐函数实际行号见 JSON。

历史 RESULT/events 记载两次 stop 后 exit0、reaped、status stopped、无 LISTEN、可重绑。独立 fresh 检查确认 90778/90812 当前 ps 无记录，50221/50242 当前 lsof 无 LISTEN，各端口可立即 bind 后 close；原 DB 三文件 lsof 无持有者。fresh ps 没有相关 Rust 或具名 Python 服务，hdiutil 没有相关挂载。这里的 exit0 依据是保存的运行原件与 harness，当前 ps 只证明当前 PID 不存在。

原单 bar DB 的 main/WAL/SHM 当前字节数、SHA256、mtime_ns 与 source-before/source-after 全部相同。main SHA256 是 `08c9d59456cc3eeede74650fad8597ad2743b5acbdf198c7b0b43b64537ec4ad`，WAL 为 0 字节，SHM SHA256 是 `fd4c9fda9cd3f9ae7c962b0ddf37232294d55580e1aa165aa06129b8549389eb`。复制前原件 lsof 无持有者且两次采样稳定，harness 只 byte-copy main，没有连接原 SQLite。两副本 main mode 0444 且 main SHA 未变；其运行时创建了 WAL/SHM，WAL 为空。这只支持本次副本域无结构写与原件未变。

Q21 仍是原 `50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1` 文件，21 个 limit 与两份真实 Q 启动日志全部一致。具名 input-1 是一条 TEST.OHLC bar；input/clock 指纹与 SMALL-INPUTS 相符，catalog/profile/clock 冻结件与具名原件相同。两个 cfg 与 resolved-config 相同，session/source/producer/writer 身份与原输入一致；schema 为 s-launcher/2。七个 S 资源值与原 S-RESOURCE-PLAN 相同，本次没有执行 S 资源门。

`Controller.runtime_environment` 在真实成功路径执行具名 Python 探针；保存 probe 与当前 Python 可执行文件 SHA 相符，所声明共享库存在。环境原件只记录配置键名、具名 Python 和 keys-only 指纹，没有导出继承环境值，不能据此声称完整环境值已冻结。

二进制 SHA256 为 `388288c875f7b3ccfe7411943182602f78d7d3a437d34d9f2df283e2ae629275`，整体保持 OLD-BINDING 的 `909ccff13abaed83be3c939ab7459d52ff4afee5` 来源。独立 git show 与工作文件 SHA 对拍确认 `rust/src/bin/s_session_v3/q_serve.rs`、`rust/src/bin/s_session_v3/q_runtime.rs`、`s_session/s_service_control.py` 在旧绑定 commit、当前 HEAD、工作文件三者完全相同。兼容结论只覆盖这三个精确路径和实测 Q 控制域，没有把整个旧二进制认作当前 HEAD 重建。

attempt1 原件保留，二进制 mode 0644 导致 Popen PermissionError，RESULT 无 Q PID。attempt2 copy2 保留 0755，二进制字节与 attempt1 完全相同。失败尝试不计为成功实例。负控仅在纯合成 PID/HTTP observation 下调用现有 Controller，os.kill 被 mock，kill_calls 与 ready_confirm_calls 为 0；不代表真实外来服务或攻击场景验收。

执行时 events 的 complete 记录初始 manifest SHA `55681bf032906db0ed98a57bcc21d37a489b3f80eaf495b69d97f5fc5aa85340`。它与最终 manifest SHA 不同，最终汇总追加了失败尝试、source revision 和 terminal 等字段。因此 complete 事件不直接绑定最终汇总；执行原件与最终汇总按各自指纹区分。

未验证正式 `start_q /3`。现有 `start_q` 仍启动 Python `s_readonly_server.py`，cfg 仍是 /2。本结论也不覆盖 S3 serve/FIFO/七字段执法、Browser 完整 Q3 消费者、9011、全部资源或生产容量、M1/#1323/整图、main 合入或 push。历史 terminal handle 状态未由本代理重新调用，manifest/events 记载 terminal exit0。没有产品代码、资源、政策、main、评论、其他 PID 或服务状态变更。

核验时间为 `2026-09-30T16:18:55.309044+00:00`。完整逐文件指纹、检查条件、实时观测命令与源方法行号保存在 `INDEPENDENT-CONTROLLER-REVIEW.json`；mtime_ns 使用十进制字符串避免跨语言数值精度损失。
