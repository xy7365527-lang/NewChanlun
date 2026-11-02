# 收据 workers/review-0070/REVIEW.md

- 原件：`workers/review-0070/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`dadeb76980e87a4be17aaa7ae3bec797d21fe0f96bf6819557cd4fc468f8a487`
- 字节数：9012；行数：90

## 关键行（行号: 原文）

```
1: # #1488 review-0070：0069 降级判据收紧的独立评审
7: ## 结论：**FAIL（阻塞）**
13: ## 1. 生产 release 不变（本审亲自构建）
15: | 提交 | 原始 sha256 | 去 build-id + strip-all | 仅 strip-all |
22: - 代码层面：`publish_attempt`、`publication_degrade_verdict`、`is_scratch_resource_bridge` 都在 `s_capture_native_testonly` 门内；`mod effects` 也在门内（audit_owner.rs:31）；生产 `CodecWork::run` 未改动。
24: ## 2. 锁表与宽对拍
29: ## 3. 红证与新锁（本审复跑）
40: ## 4. R1（高）：真实 S86/MIN12 的放弃路径被 fail-closed
44: | 运行 | base `44d312d7cf`（testonly `a6fb47ed…`） | HEAD `5aff2d1b41`（testonly `f2b8667b…`） |
49: 日志：HEAD `runs/20261021-043237-5aff2d1b41.log`，base `runs/20261021-043720-44d312d7cf.log`。
51: **根因**：`prepared.rs:1232` 的 `err()` 用 `S3PrepareInvalid：` 包装 codec 错误，prepared 的业务 store 的 `codec_work()` 委托给 inner store（`prepared.rs:84`）。实测表明，它在候选 publication 作用域上记录了资源失败：HEAD 回复的错误就是闭包原样返回的 `S3PrepareInvalid：<桥>`，而 base 在同一请求上降级成功。作用域里记录的是 Budget/Allocation，闭包返回的错误是 `S3PrepareInvalid：<桥文本>`。`is_
53: **作者证据为什么没发现**：报告说“真实放弃路径由 0065 的 S86/MIN12 目标运行覆盖”，但那两次运行用的是 `1660d9d32d`，在 0067 合并之前，整程只放弃过 1 次（message-20 / op-0015，走的都是 effects 追加路径）。合并 0067 之后，base 上放弃次数变多，而且出现了经 prepare 路径的放弃，0069 没有在 HEAD 上重跑目标运行。红证 b 和锁 2 只证明了 `S3DriverInvalid` 这一条链。
57: ## 5. R2（中）：F1 在 Ledger 类文本这一子类上没有闭合
63:   - `verdict(Resource(Allocation), Some(invalid(ledger))) = Ok(())`
72: ## 6. 建议（交作者，本审不改）
78: - 修复后**必须在 HEAD 上重跑 S86/MIN12 testonly 目标运行**。base 上 S86 有 3 次放弃、MIN12 有 5 次，HEAD 应与之一致并全量 Committed。
80: ## 7. 附带说明（信息）
83: - I2：本审 HEAD 的 MIN12 compare_arms（对 0065 的 nat2）为 FAIL，原因只是 HEAD 跑到 25 代就中断了，不是独立发现。
86: ## 留痕
```
