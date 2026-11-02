# #1516 前提核验

**结论：否定“必须先把 make_token 改成 frontier/generation 哈希，才能推进 #1304”的前提。不要按现票方案实施。** 当前 v2 和 S3 已使用另一份具名、确定性的 Begin 配方；`make_token()` 只服务无 v2 协议标记/控制表的旧入口。当前有限 replay comparator 没有比较 Begin token，但它也没有实现 #1411 要求的完整比较，不能据此把 token 永久排除在验收之外。

采样为 2026-10-26。本报告绑定 #1516，只读调查，无实现、票面或依赖修改。源码固定为 `origin/main@99df2ed762201236b89f0e81c0276cb3aa847f0a`，不是存在未跟踪 WIP 的主仓工作树。#1516 与 #1411 的 GitHub assignees 均为空；#1516 唯一评论为释放旧认领及前提待核记录。

## 1. 实际 Begin 有两条路径

|路径|固定 main 来源|实际行为|
|---|---|---|
|旧无协议入口|`rust/src/bin/s_structure_session.rs:123–130`；`s_session_v2/mod.rs:119–125,413–425`|`active` 仅在 protocol_revision 缺失且五张控制表均缺失时返回 false，随后才调用墙钟/PID `make_token()`。标记或控制表损坏返回错误，不降级。|
|v2 入口|`s_session_v2/mod.rs:430–488`|读取已持久的 attempt_count、next_attempt_ordinal 和 session_generation，记录本次 begin clock，推进 attempt/ordinal，返回完整 SHA256 token。|
|S3 入口|`s_session_v3/control.rs:254–281`|核实际 writer_epoch，在事务内直接调用上述 `v2::record_begin_context`，然后持久 Begin。没有绕到墙钟/PID函数。|

当前确定性配方为规范 JSON 数组：

```text
[session_id, session_generation, writer_epoch,
 attempt_ordinal, base_cut, generation, input_frontier]
```

返回 `begin-` 加完整 SHA256。`s_session_v2/content_publication.rs:958–988` 从真实 begin clock 行读 ordinal，并按相同七项重建 expected_token。这不是只存在于生成函数里的约定，出版核验也消费该配方。

因此，固定身份、输入与故障/调度轨迹时，这条配方不读取当前 PID 或墙钟。若两个独立运行使用不同 session 身份，不能只修改这个 token 让表面相等，必须按 #1411 的预声明一一重命名规则处理整个身份引用关系。

## 2. token 是 attempt 所有权，不是只由业务 frontier 决定的名字

`persist_begin_state` 将 token、generation、frontier 写入同一持久 Begin，见 `s_structure_session.rs:3011–3026`。`verify_begin_owner` 同时核配置 writer_epoch、可达根、完整 `begun:{token}:{gen}:{frontier}`、begin_token 和已发布 generation，见 `3029–3044`。

`cancel_own_begin` 也要求这些所有权条件仍匹配才清理自身占用，见 `2964–3008`。它不依赖另一个独立回放进程产生相同 token。票面“回放 token 与原运行不同，所以 CAS 必败”的推导漏掉了同一 attempt 内部的持久 token 和 epoch 比较边界。

现票提议仅用 `(input_frontier,generation)`，会丢掉 session、session_generation、writer_epoch、attempt_ordinal 和 base_cut。恢复后未提交的同一输入可再次 Begin，业务 generation/frontier 可以相同，attempt 与 writer_epoch 却必须区分。将两字段哈希拿来替换现七字段配方不是保持语义；将它只用于旧入口也不能证明 #1411 整体确定性。此处指出身份区分被删除，不宣称已经复现 CAS 攻击。

已有测试源码支持这一合同：

- `s_session_v2/mod.rs:3630–3659` 的 `v2_pending_recovery_preserves_identity_and_uses_next_attempt` 实际 Begin、recover、再推进，断言两个 begin 事件，原 canonical envelope 不变且最终 Committed。
- 同文件 `3662–3670` 的 `v2_exhausted_attempt_plan_has_zero_new_begin_or_write` 验证恢复后的计划不足不产生新写入。
- `s_session_v3/control.rs:502–528` 验证错误 writer_epoch 在 attempt/ordinal 写入前拒绝。
- 同文件 `533–660` 真实写入 accept/begin/commit，检查 ordinal、三相位时间和回执幂等，不把返回值构造当成持久成功。

本轮阅读这些测试源码，没有重跑 Rust；不把本次静态核验称为新的测试 PASS。

## 3. #1411 要比较的面与当前实现缺口

[票 #1411](https://github.com/xy7365527-lang/NewChanlun/issues/1411) 要求固定完整输入修订、政策/模型、时钟、随机种子、数值、故障与调度轨迹，硬比较结构关系、逐重状态/毛账、责任、命令/回报分配及 AsKnown，并真实中断 S/E/B/X 的持久边界。只允许预先声明的非语义 run 身份一一重命名。

已批 SPEC 的 `view/spec/SPEC.md:336–344`，尤其 C10.4 第341行，同样明确这些比较面。`view/spec/inputs/TEST-SEAMS.md:188` 还明确发生、获知、确认、接纳、应用时刻及前沿必须硬比较，不能将语义时间塞入墙钟白名单。这里的路径均在 `.chanlun/review-results/issue1339-r2-spec-20260909/`，票引用的签署提交为 `efc1ddc4d6c015f4f8c6d44f904d704a88308b46`。

当前已有 `rust/src/trading/replay_comparator.rs:115–153` 实际只比较序列长度和 timestamp_ms；业务字段保留 TODO。`rust/src/bin/replay_compare.rs:92–102` 构造的是 MinimalEngine。对这两个文件及 `rust/src/trading/replay.rs`、`rust/tests/replay_compare_test.rs` 检索 `make_token|begin_token` 没有命中。该组件不包含 Begin token 比较，也不是 #1411 完整验收器。

本次按 `#1411|TB-07-C|tb07` 搜固定 main 的 Rust/Python/shell 源（排除 fixtures），未找到具名 #1411 实现入口；GitHub PR 搜索 `1411` 也未返回候选。这个检索只说明未找到已具名的交付，不能断言仓内没有任何复用组件。#1411 仍 OPEN、无 owner，其实际完整字段清单/归一映射及真进程验收仍待实施。

不能做两个错误推论：

1. “当前 comparator 不比较 token，所以可以忽略全部身份和控制状态”。这会缩小已批合同。
2. “现有 make_token 含墙钟，所以已证明它阻断正式回放”。正式 v2/S3 Begin 根本不走该函数。

## 4. 来源核认与下游去向

[#1516 释放评论 6310562774](https://github.com/xy7365527-lang/NewChanlun/issues/1516#issuecomment-6310562774) 已指出 #1514 两份报告不支持票面的 R1/R2 归因，并明确先按 #1340 TB-07 核定。当前源码进一步否定将旧入口函数当作正式 S3 Begin 的做法。

[#1304 现行订正 6245981746](https://github.com/xy7365527-lang/NewChanlun/issues/1304#issuecomment-6245981746) 与[后续 triage 6390152512](https://github.com/xy7365527-lang/NewChanlun/issues/1304#issuecomment-6390152512) 将工作交给 #1381、#1399、#1411、#1418 及 FU-19/TB-07。#1516 没有被确认是其前置，本次不新增这条阻塞边。

对 Destination ④/⑤ 的动作是保留正式协议的身份/attempt合同，继续完整回放与同核接线，不能用改旧 token 代替。Destination ③ 的旧入口终态仍由 TB-10 实际入口清单承接。旧 `make_token` 是否随对应入口收编或退出，要按该入口最终处置判断，不另造一条简化确定性生产臂。

## 5. 可直接落票的处理建议

主控可将以下内容追加到 #1516，撤销票面“P0 必须先修”的当前采用口径；原历史正文不必伪装成当时已知。

> 经固定 main `99df2ed762201236b89f0e81c0276cb3aa847f0a` 核验，本票提出的 make_token 墙钟/PID 路径仅用于无 v2 标记/控制表的旧入口。v2 与 S3 已共用七字段确定性 Begin 配方，包含持久 attempt ordinal、session generation 和 writer epoch；出版核验也重算同一配方。CAS 核的是本 attempt 持久所有权，不要求另一独立运行复用原 token。因此撤销“本票是 #1304 P0 前置”的假定，不实施 frontier/generation 两字段替换。完整比较与预声明身份重命名仍由 #1411 按已批 SPEC 实施；旧入口终态随 TB-10 对应入口收编。该结论不代表 #1411 或六项 Destination 验收完成。

如按流程关闭本问题票，应先将本报告纳入可引用实体并完成必要独审，再在 #1411 核实登记上述身份/attempt验收提示，给旧入口挂已有终态清单的精确承接点。本次只交建议，没有改票、关票或把待实施义务宣布完成。
