# #1621：X 执行域时钟义务勘察与承接建议

日期：2026-10-25。只读勘察，未修改代码、Issue 或 blocking 边，未运行 cargo。

## 结论

**当前候选没有可调用、可测试的 R2 X 执行域。** X 不是 B 内转移，也不是旧 Nautilus order adapter 或 #1523 的离线 launcher。按已批准 SPEC，X 是独立持久进程，唯一持有场所写凭据并执行受控网络调用。

X 的实现与对应时钟验收已有真实承接票，不应在 #1621 另造一套：

- **#1407**：`E.ReleaseExact` 与 `X.AcceptExact`，负责精确交付和 X 接纳，接纳本身不授调用权。
- **#1416**：`S.RequestConditionalExecution` 的稳定点核验及 `ExecuteCommitted`。
- **#1422**：`X.OwnAttemptAndCall`，负责 Ready→MayHaveCalled 的原子占有和一次受控调用。

建议把 #1621 的 X 部分显式移到上述现有票，#1621 只保留 S/E/B。**义务是迁移，不是删除或降级。** 先在 #1407/#1416/#1422 写清下述验收，再修订 #1621 的指向；不要给 #1621 新增 `blocked by #1422`。

## 1. 合同与当前实现事实

### 1.1 X 的准确职责

`INTERFACE-CONTRACTS.md` 在候选 `78dc0449500d5ad82ef42b5304527beade24fc8e` 中规定：

- 共同消息必须分别保存原始 received、真实 first_known 与持久提交序号（`:9`）。
- `E.ReleaseExact` 先持久 ReleasedExact，之后才交付 X（`:31`）。
- `X.AcceptExact` 只接纳同身份同内容命令，**不是网络调用权**（`:32`）。
- `S.RequestConditionalExecution` 在稳定点核验后才有 `ExecuteCommitted`（`:33`）。
- `X.OwnAttemptAndCall` 仅允许同 command 的首次 CAS 赢家尝试一次受控调用；MayHaveCalled 以后不得因重启或 NoRecord 补发（`:34`）。
- X 的持久原子组是命令身份/内容、Ready→MayHaveCalled 及真实调用/回报观测，不与 E/S 伪装成跨库事务（`:60`）。

SPEC #1340 还规定：S、E、X、各 B 是独立进程（正文 `:326`）；X 唯一持有场所写凭据，SDK 隐式写重试也受一次调用合同约束（`:332`）；完整会话是 S→操作→B→E→X→事实→B→观察（`:340`）。

### 1.2 当前没有 X 生产入口或时间源

对候选全部 `*.rs/*.py/*.ts/*.js/*.sh/*.toml` 搜索 `AcceptExact`、`OwnAttemptAndCall`、`ReleasedExact`、`RequestConditionalExecution`、`ExecuteCommitted`、`MayHaveCalled`，结果为零。`rust/src/bin` 只有 `s_structure_session`、`e_economic_session`、`b_chong_session` 三个 R2 相关入口，没有 X 二进制。

现有 `rust/src/economic_session/finance.rs:946,1058,1171,1877` 仅写 `execution_eligibility:"None"`，不是 X 实现。旧 `theta_v0/nautilus/order_adapter.rs` 只映射订单意图，不满足上述独立 X 持久域合同。#1523/PR #1526 仍是离线 adapter/launcher 候选，其票面明确不得绕过 S/E/B/X，也没有提供 X 时钟证据。

因此，**今天不存在可指出为“现役 X”的文件、函数或 received/first_known/commit 来源。** 这些字段需要由 #1407/#1416/#1422 在对应生产路径建立，不能用测试名或 B 转移冒充。

## 2. 两阶段时钟合同，不能合并成一条“最新性闸”

#1430 明确保留两个在途窗口：E-Release 后的新经济风险不保证撤销首次调用，S-Execute 到实际调用也有窗口；不得新增跨域最新性闸改变已批语义（票面 `:12`）。

所以应验证的是**因果提交顺序**，不是“X 必须看到全局最新状态”：

1. X 接纳不得早于它引用的 E.ReleaseExact 持久提交。
2. X 实际调用不得早于它引用的 E.ReleaseExact 与 S.ExecuteCommitted 两个持久提交。
3. X 的阶段内顺序仍须满足 `received_ns <= first_known_ns <= commit_ns`。
4. 比较对象来自 `causal_refs` 指向的具体提交，不是另一个域的当前时间、最新 cut 或命令自报 `known_at`。

X.AcceptExact 可以合法地早于 S.ExecuteCommitted，因此不能错误要求“X 的全部状态都必须晚于 S 执行”。只有 OwnAttemptAndCall 同时消费两张收据。

## 3. 各票应增加的正例与提前 1ns 负控

### 3.1 #1407：E.ReleaseExact / X.AcceptExact

**正例**：E 在 `release_commit_ns` 持久 ReleasedExact；X 的接纳观察满足 `x_accept.first_known_ns >= release_commit_ns`，随后 `x_accept.commit_ns >= x_accept.first_known_ns`。X 返回同身份接纳结果，场所适配器调用数仍为 0。

**提前获知负控**：令 `x_accept.first_known_ns = release_commit_ns - 1`。在 X 持久写之前返回 typed InvalidDomain，并逐项断言：

- 没有 X 接纳状态变化或 Ready→MayHaveCalled；
- 场所调用数为 0；
- E 的 ReleasedExact 与最大责任原样保留；
- 同输入合法重试仍可被接受，不把一次负控变成永久拒绝。

生产写入与恢复审计必须复用同一判据，不能只在测试 helper 中检查。

### 3.2 #1416：S.ExecuteCommitted 的时间载体

保留稳定点核验产生的实际 S 持久 `execute_commit_ns` 及对应 causal_refs，供 X 引用。它不能把 Request 的 received 时间或后来的查询时间冒充提交时间，也不能因为恢复重放而回填旧 known。

### 3.3 #1422：X.OwnAttemptAndCall

**正例**：给定有效 ReleasedExact 和 ExecuteCommitted，满足：

```text
x_call.first_known_ns >= max(release_commit_ns, execute_commit_ns)
received_ns <= x_call.first_known_ns <= x_call.commit_ns
```

同 command 唯一 CAS 在适配器调用之前持久 Ready→MayHaveCalled，随后仅触达受控场所一次。重复消息返回原结果；重启/NoRecord 不补发。

**两个独立提前负控**：

- `x_call.first_known_ns = execute_commit_ns - 1`；
- `x_call.first_known_ns = release_commit_ns - 1`。

每个都必须证明：CAS 未发生、Ready 原值不变、没有 MayHaveCalled、场所调用数为 0、E/S 收据与责任不变。合法后续调用不被这个测试永久封死。恢复审计也走同一生产判据。

这些负控验证“不能凭未来前件取得调用权”，并不撤回两个已批准的在途窗口。

## 4. 原生依赖与票据归属

当前原生 blocking 边：

- #1422 blocked by #1416、#1361；
- #1416 blocked by #1407、#1361；
- #1407 blocked by #1395、#1361；
- #1621 blocks #1406；#1621 当前无 open blocker。

#1400/#1412 负责后续具名场所只读与真实外效验收，不是 #1422 受控一次调用及其时钟锁的替代。#1523/PR #1526 是 adapter 准备，不是 X 已完成证据。

### 不建议的图修改

不要添加 `#1621 blocked by #1422`：#1422 依赖 TB-03 汇总 #1361，而 #1621 又阻塞 TB-03-C #1406。若 #1621 必须等 #1422 才能关闭，TB-03 与 TB-04 会形成逻辑等待环。即使当前原生 #1361 未把 #1406 列进 blockedBy，#1361 的验收仍要求全部内部子片交付，不能利用索引缺边制造假解锁。

### 推荐的图修改顺序

1. 在 #1407 的验收中写明 X.AcceptExact 的因果时钟正负控。
2. 在 #1416 写明 ExecuteCommitted 的实际持久 commit_ns/causal_refs。
3. 在 #1422 的验收中写明 OwnAttemptAndCall 对两张收据的因果时钟正负控。
4. 再把 #1621 的范围文字改为 S/E/B，并链接上述三项作为 X 的完整承接，不用 blocking 边，不开新票。

这样 #1621 可在当前 S/E/B 候选满足交付门后独立验收，X 不丢项，也不让 #1406 与 TB-04 相互等待。

## 5. 证据来源

- 候选源码与文档：`/Volumes/AgentStorage/issue1621-1622-clock-integration-20261024` @ `78dc0449500d5ad82ef42b5304527beade24fc8e`。
- `INTERFACE-CONTRACTS.md:9,31-34,57-62`；SPEC #1340 正文 `:319,326,332,340`；#1430 票面 `:12`。
- #1407、#1416、#1422、#1361、#1362、#1365、#1400、#1412、#1523 的当前票面及 GitHub 原生 blocking/subIssues 查询。
- #1621 依赖方向说明：[comment 6324234340](https://github.com/xy7365527-lang/NewChanlun/issues/1621#issuecomment-6324234340)——它证明 #1406 需要修复自报 known_at，并未证明 #1621 必须拥有 X 实现。
- #1688 独审：`/Volumes/AgentStorage/issue1688-78dc044-independent-review-20261025/REVIEW.md`，仅将 X 判为 NOT_VERIFIED，与本报告一致。
