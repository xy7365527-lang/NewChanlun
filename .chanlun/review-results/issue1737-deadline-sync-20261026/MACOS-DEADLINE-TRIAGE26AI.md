# macOS ingress deadline 测试失败分诊

冻结源码 `f7755d322c2019815bc4d8f7d589254a4610391d`。分诊时间取本机 date，2026-10-26 15:21:48 CST。只读调查，未运行 Rust 或重跑 CI。

## 结论与下一步

`queued_unix_job_keeps_ingress_response_deadline_in_actor` 有可由源码证明的测试同步缺陷。测试把“墙钟已过截止”当成“另一个线程已处理超时并销毁 receiver”，二者没有 happens-before 关系。当前失败恰好落在这个错误断言，不证明生产 actor 重置了 deadline。

这不是 #1527 新引入的问题。已有 main `df6e68093052c105600c7905ea5de371196dd5c1` 的同名 macOS 失败原日志，以及随后另一个 macOS 成功 run。本轮已用 gh 回查二者。无需等待当前候选再败一次才登记。

已有交棒在 #1406 评论 [6302451174](https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6302451174)，明确写“归 #1588”。但 #1588 现已关闭，最终关票清单未列这项。当前没有核到专门承接该测试的开放票。建议主控按关闭后遗漏工程债安排后继 task，引用 #1588 与该评论，不重开 #1394，不并入当前 #1527 schema 修复。

优先修测试的同步关系，而不是延长生产期限、增加 sleep、删测试或盲重跑。修复 owner 需在独立验证环境加入受控调度负控，再取精确 head 的 macOS CI 证据。

## 测试究竟在检查什么

以下位置属于冻结源码的 `rust/src/session_transport.rs`。

- `:75-78,95-103` 的 `TimedJob` 携带 I/O 线程生成的绝对 response_deadline。
- `:547-554` 在入队前生成一次截止，同一截止用于入队和等待回包。`:553` 明确回包超期不撤销已入 actor 的业务工作。
- `:595-603` 的正式 `serve_unix_with_deadline` 把队列项中的同一个 Instant 传给 handler。
- `:706-733` 的失败测试没有调用生产 actor 或正式 `serve_unix_with_deadline`，而是直接取出 TimedJob，模拟延迟 handler。
- `:723-728` 收到队列项后 sleep 20ms，确认当前时刻不早于队列项截止。
- `:731` 随即要求 `SyncSender::try_send` 报错。这个断言实际检查的是 reply_rx 已销毁，不是 actor 收到的 deadline 是否被重置。
- `:732` 才 join I/O 线程。若把线程完成同步放在断言之前，才能证明 receiver 的生命周期已经结束。

## 缺失的 happens-before

源码允许如下调度，且不需要任何生产逻辑错误。

1. I/O 线程在 `:547` 生成截止 d，`:548` 创建容量为 1 的 reply channel，在 `:550` 把 TimedJob 放入请求队列。
2. I/O 线程在入队之后、reply_rx 超时返回并 drop 之前暂停。它可能尚未执行 `:554`，也可能已经进入等待但未获调度继续。
3. 测试线程在 `:723` 取出请求，睡眠 20ms。`:728` 的 now ≥ d 可以成立。
4. receiver 此时仍存活，reply channel 又是空的，因此 `:731` 的 try_send 可以合法成功，断言失败。

“入队完成”只建立请求内容对测试线程可见的关系，不建立未来超时处理完成的关系。sleep 不等待另一个线程的程序点。CI 的 960.45s 总耗时不证明上述具体调度发生；失败点和源码已足以证明断言不成立，具体调度仍缺线程事件证据。

此外，`:729-730` 本地生成一个新的 now+10ms，再断言它位于未来，并没有检查生产 handler 不重置截止。测试名字比实际覆盖更强。修复时应把两项行为分清：原截止在队列中保持不变，以及超时回包连接最终结束。

## 生产合同与故障范围

冻结 `rust/src/economic_session/service.rs:1544-1559` 在新 `/2` 域把 I/O 截止传入 `handle_with_deadline`。`:1424-1454` 对新工作及财务写入继续传递或检查该截止。`rust/src/economic_session/store.rs:839-866` 将可信 ingress 截止与内容 Job 的首次读截止取最早值，并拒绝已到期操作。

这些调用链支持“不可在 actor 出队时重置新域截止”的合同。它们不能由一个发送结果的 channel 是否仍存活来判定。本轮没有证明所有生产路径无缺陷，也没有因测试竞态而豁免正式运行验收。旧 `/1` 仍按 `service.rs:1560-1562` 保持原语义，测试修复不能顺手给它加入新政策。

## CI 原件改变了旧判断

- [run 40278590515](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40278590515) 本轮实查 completed/failure，head 为 `6fe84d477b0772bda381782c1e32153b100f9263`。job 121426626798 的名称是 `macos-session / macos-session`，completed/failure，06:13:53Z 开始，06:41:49Z 结束。
- 原件 `1527-CI40278590515-MACOS-SESSION26AA.log:659,667-668` 记录同名测试在 `session_transport.rs:731` 的 try_send 断言失败。`:675` 附近汇总是 230 passed、1 failed、9 ignored、3190 filtered out、960.45s。前置 financial_session 为 26 passed、0 failed、3 ignored。这不是全 job 唯一一个测试套件。
- [main run 39448001873](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39448001873) 本轮实查 completed/failure，head `df6e68093052c105600c7905ea5de371196dd5c1`。`gh run view 39448001873 --log-failed` 真实记录同名测试在当时 `:730` 同一 try_send 断言失败。该库套件为 77 passed、1 failed、7 ignored、3100 filtered out，19.44s。它直接反驳“只有当前慢 runner 才出现”的推断。
- [run 39485175986](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39485175986) 本轮实查 completed/success，head `673726b813955f0981fea4f3d3830b9f91196d4c`。其中 220/0/9 和 Linux 各5/5读数来自 #1406 上述评论，本轮未把这些历史计数升级为重新执行。
- 对 `6fe84d47`→`f7755d32` 和 main `b0bab458`→`f7755d32` 的 `git diff --exit-code -- rust/src/session_transport.rs` 都返回 0。本文件不属于 #1527 吸收 main 的变化面。

## 现有票与仓内双搜

Tracker 搜索均为 `gh issue list --repo xy7365527-lang/NewChanlun --search <词> --state all`，包含已关票。本轮检索词为精确测试名、`response deadline`、`session_transport`、`计时竞态`、`macos-session deadline`、`ingress`、`回包期限`。精确测试名、deadline及计时竞态命中 #1406。回包期限命中 #1673，但它承担正式会话财务链运行，不是这个传输测试的独立修复归属。

仓内执行 `rg -n --glob '*.md' 'queued_unix_job_keeps_ingress_response_deadline_in_actor|ingress_response_deadline|session_transport|回包期限' analysis/ docs/ .chanlun/`。`.chanlun/review-results/eng-debt-20261020/sandcastle/REPORT.md:130-132` 留有该测试被 #1406 工蜂执行的历史。该报告主要解决宿主 cargo 违规，不是测试根因修复。旧 roster 的 #1394 CI分诊是更早记录，也不构成当前开放 owner。

直接回读 #1588 票面与评论，实查 CLOSED，closedAt=2026-10-21T07:16:34Z。其关票评论 [6310348710](https://github.com/xy7365527-lang/NewChanlun/issues/1588#issuecomment-6310348710) 的修复与派生票清单没有该测试。结论限于本轮关键词和这些承接原件，没有宣称搜索穷尽全仓每种同义说法。

## 后续最小验证与修复范围

实施 owner 的起点是 `rust/src/session_transport.rs:706-733`，不改10ms生产语义。建议把原截止传递检查与回包生命周期检查分开。回包负控等待 I/O 完成信号后再断言 sender 已断开，并用测试级有界等待防止自身挂死。保留 sleep 只能用于推进到 deadline，不能用作线程完成证明。

最小确定性复现可以在测试专用 seam 中，让 I/O 在 enqueue 后、reply等待前停在 barrier。测试线程拿到 job 并越过原截止，此时原 try_send 仍合法成功。随后放行 I/O，观察它结束，再判 sender 断开。这区分“receiver还活着”与“deadline被重置”，不靠打满CPU赌调度。该 seam 仅限 TestOnly，不改公共信封或生产字段。

同时必须保留独立的 actor 截止负控。把队列任务延后到原截止之后，确认正式新域不会因出队时重新 now+timeout 而准入新写，且无新增持久回执。若现有 `finance_tests` 有等价锁，优先复用而不增加同义矩阵。

本轮未执行上述动态复现。因此已证明的是测试断言缺少同步及历史main同败，不是已取得受控调度实验结果。修复 owner 的验收输入应包括该实验、靶向测试及精确head的macOS读数。不要把一次重跑绿当作根因已解决。

当前 `40282501409@f7755d32` 实查 macos-session 为 skipped。`.github/workflows/ci.yml:796-801` 只在 workflow_dispatch 调用该job，`.github/workflows/ci-macos.yml:73-78` 执行上述库套件。因此该run即使其他项绿，也不能提供新的macOS通过证据。

## 方法与边界

已读 poteto-mode、Investigation、how、Fix Root Causes、unslop和technical-writing。how按父任务只读叶约束直接完成，没有派孙代理。Fix Root Causes使本轮把“20ms后另一线程必定drop”作为待检验的第一错误前提，而不是用960s总耗时解释失败。throughput checkpoint: n/a, read-only investigation。

代码发现先查图项目 `Users-silencehan-Projects-NewChanlun-rust` 的 queued_unix_job/serve_unix/handle_connection，返回0项，故回退冻结git源码。只写本报告，未更改代码、主控store、票面或CI。
