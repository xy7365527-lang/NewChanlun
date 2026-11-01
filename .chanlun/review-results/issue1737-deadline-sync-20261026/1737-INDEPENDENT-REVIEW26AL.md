# #1737 / PR1738 非作者独审

结论：**PASS**（测试同步修复本身）。冻结 head `a053ca2322ac9ab8d27d98e376f9c569024046bb`，工位 `/Volumes/AgentStorage/issue1737-deadline-sync-20261026`，基线 `b0bab458f04d2afea3282475c56e6134236c8a52`。审阅时间取本机 date，2026-10-26 15:46:46 CST。审阅者为 Opus 家族，作者为 GPT6.1-sol。

新测试用客户端 EOF 取代 sleep20ms，EOF 由生产代码的变量析构顺序保证发生在 reply receiver 销毁之后，旧竞态被消除。生产代码零改动，deadline 合同未被削弱。macOS run 40286381749 仍在运行，终态由主控消费；本 PASS 不含该读数。

## 差异范围

`git diff --stat b0bab458..a053ca23` 只有两项。`rust/src/session_transport.rs` 5 增 5 删，全部位于 `mod tests` 内 `:725-731`。`.chanlun/agent-roster-1737-20261026.md` 新增 8 行，登记工位。没有生产函数、字段或 TestOnly hook 变化。工作区干净，HEAD 与冻结 head 一致。

## 核心判断：EOF 能否证明 receiver 已断开

能。依据是 `rust/src/session_transport.rs` 在冻结 head 的 `io_connection_inner`（`:528-580`）。

1. `reply_rx` 在 `Ok(e)` 分支块内 `:548` 创建，属于该块的析构作用域。`stream` 是函数参数（`:529`），属于函数最外层作用域。
2. 测试持有 job，因此 `:550` 入队已成功；测试在读到 EOF 前不往 `job.job.result` 发送，sender 一直存活，`:554` 的 `recv_timeout` 只能以 Timeout 结束，走 `:557` `Err(_) => return`。
3. Rust 离开作用域时先析构内层块变量，再析构函数局部变量，最后析构参数。所以同一线程上 `reply_rx` 的 drop 先于 `stream` 的 drop。`stream` drop 关闭服务端 fd，客户端 `read` 才能返回 0。
4. 因此客户端观察到 EOF 时，`reply_rx` 已销毁；`:731` 的 `try_send` 必然返回 `Disconnected`。这是程序顺序加内核关闭可见性建立的 happens-before，不再依赖墙钟。

其他能让客户端读到东西的路径都会让测试显式失败而不是误判通过。成功路径 `:556` 需要有人发送回复，测试没有发送。`read_frame` 错误路径会写出错误 JSON，`read` 返回非零，`:729` 的 `assert_eq!(..., 0)` 失败；且测试已收到 job，说明 `read_frame` 成功。入队失败路径 `:551` 不会产生已入队的 job。

## `:730` 断言仍然成立

`:554` 传给 `recv_timeout` 的时长是 `response_deadline - t0`，std 在调用内取 `t1 ≥ t0` 加上该时长作为内部截止，超时返回时 `now ≥ t1 + (deadline - t0) ≥ deadline`。EOF 发生在其后，测试线程随后取 `Instant::now()`，单调时钟保证 `≥ job.response_deadline`。这条断言检查的是 I/O 线程没有早于队列项携带的原截止结束等待，属于真实可观测行为。

## 有无新死锁或时序问题

没有。`:725-727` 给客户端设了两秒读超时；服务端若永不关闭，`read` 返回 WouldBlock/TimedOut，`unwrap` 立即 panic，测试有界失败。`:732` 的 `join` 在 EOF 之后执行，此时服务线程只剩参数析构，不阻塞在任何通道上。客户端先前的 `shutdown(Write)`（`:722`）只关闭写半边，不影响读到 EOF。

两秒是测试等待上限，生产 `bounds.response` 仍为 10ms（`:711`），`io_connection_inner` 的期限计算 `:547` 未改。

## 删除 local_reset 是正确的

基线测试的 `local_reset = now + 10ms; assert!(local_reset > now)` 是恒真式，没有调用任何生产代码，删掉不损失覆盖。删除的 sleep 注释描述的正是被证伪的假设。

## 证据复用

- `1737-deadline26ai/controlled-channel.rs` 与 `20261026-152857-a053ca2322.log` 证明旧断言前提错误：receiver 存活时越过截止，`try_send` 仍可成功，旧臂 EXIT 101，同步臂 PASS。它演示的是通道语义，不是生产 I/O 的受控复现；生产路径的同步依据是上文的析构顺序，本审以源码核实。日志首行命令回显丢了引号和变量，属于回显渲染，实际输出 `REPRO_OLD_EXIT=101` 与 panic 行号 `:19:9` 对得上。
- `20261026-153154-a053ca2322.log`，同一 head，`queued_unix_job_keeps_ingress_response_deadline_in_actor` 与相邻 `lf_without_peer_eof_still_expires_without_accepting_a_frame` 2 passed；`trusted_ingress_deadline_is_not_restarted_by_economic_job` 1 passed。首个日志里 finance 筛选为 0 tests，作者已标明不计，本审同样不计。
- 未在本机运行 cargo/rustc，未触发任何 CI。

## 覆盖边界

这张票修的是一条传输层测试的同步。它覆盖两件事：I/O 线程等到原截止才结束回包等待，以及结束后回包通道确实断开。

它不覆盖 actor 出队时不重置截止。该合同由 `serve_unix_with_deadline`（`:595-603`）把队列项原 Instant 传给 handler，以及 `rust/src/economic_session/finance_tests.rs:935-963` 的 `trusted_ingress_deadline_is_not_restarted_by_economic_job` 锁定，后者只验证 `operation_deadline` 取原 ingress 并拒绝已到期。完整 `/2` 服务端到端无新增持久回执仍归 #1716 与整图真实消费，本票不能替代。

## 非阻塞建议

- 测试名 `..._keeps_ingress_response_deadline_in_actor` 比实际覆盖强，测试并不经过 actor。可在后续改名为描述 I/O 等待与回包断开的名字。不改不影响本票正确性。
- 新测试的正确性依赖 `reply_rx` 位于分支块内、先于 `stream` 析构。这是代码看不出的 why，按 poteto-mode Comments 规则值得留一行注释，例如说明 EOF 证明 reply receiver 已在关闭连接前销毁。将来若有人把 `reply_rx` 提到外层或在返回前显式 shutdown，测试会重新变成竞态。是否补注释由主控决定，不构成 BLOCK。

## 主控后续

1. 消费 macOS run 40286381749（job 121450922238）终态；实查 15:46 CST 时仍 in_progress，正在 `cargo test — economic_session / session_transport / session_protocol 库测试（#1394 G-1）` 步骤，head 与冻结 head 一致。
2. 终态成功后，PR1738 具备独审 PASS 与 macOS 读数，进入用户 main 批准门。

## 方法

读取 poteto-mode SKILL.md 全文及 goal1323 preferences.md、作者报告、分诊与任务原件。逐行核对精确 head 的 diff、`io_connection_inner` 全函数、测试全文、相关 finance 锁和既有日志。未修改作者树、store 或 tracker。
