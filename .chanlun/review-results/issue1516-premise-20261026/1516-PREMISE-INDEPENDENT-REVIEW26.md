# #1516 技术前提独立复核

**PASS。** 可以撤销“先改 make_token 才能推进 #1304”的技术前提，不应实施票面两字段、截断摘要替换方案。该结论不豁免 #1411 完整双跑及恢复验收，也不直接关闭 #1516。

非作者复核者 `/root/review1516_premise26`。源码固定 `99df2ed762201236b89f0e81c0276cb3aa847f0a`，2026-10-26 现场读取 #1516、#1411、#1368 及 TB-10 子票。未修改源码、GitHub、依赖或运行 Rust。图工具查询目标符号无结果，改用固定提交的定点源码与检索。

## 已核决定性证据

1. `rust/src/bin/s_session_v2/mod.rs:119–125` 的 active 仅在无协议标记且五张控制表全部不存在时返回 false。标记/表不配套返回错误，不降级。`:413–425` 仅在 false 分支调用 `make_token()`。固定提交的 `git grep 'make_token(' -- rust/src/bin` 只出现该调用和 `s_structure_session.rs:124` 定义。墙钟/PID 路径不能代表正式 v2/S3 Begin。
2. `s_session_v2/mod.rs:430–488` 的实际实现读取持久 attempt_count、next_attempt_ordinal、session_generation，记录 Begin 时钟并推进 attempt，然后对规范七字段数组做完整 SHA256。字段是 session_id、session_generation、writer_epoch、attempt_ordinal、base_cut、generation、input_frontier。生成表达式没有读取 PID/当前墙钟。
3. `rust/src/bin/s_session_v3/control.rs:254–281` 在真实写事务和 writer_epoch 核验后直接调用 `v2::record_begin_context`，再 `persist_begin_state`。不是仅靠注释声称复用。
4. `rust/src/bin/s_session_v2/content_publication.rs:958–988` 从真实 Begin 时钟行取 ordinal，以同七字段重算 expected_token；`:1000–1014` 实际比较 token，不相等返回 `Begin/clock/attempt 与本次发布不同`。原报告该处引用的生成段准确，本次额外核了真正的拒绝分支。
5. `rust/src/bin/s_structure_session.rs:2964–3044` 的 cancel/persist/verify 共同维护本 attempt 的持久 token、generation、frontier 与配置 epoch。CAS 要求的是当前持有者与持久占用一致，不要求另一独立回放进程复用原运行 token。只保留 frontier/generation 会删去现合同显式区分的 attempt/epoch/session 等字段，不能称等义替换。
6. `rust/src/trading/replay_comparator.rs:115–153` 仅比较数量和 timestamp，业务字段仍 TODO；`rust/src/bin/replay_compare.rs:92–102` 实际使用 MinimalEngine。它不构成正式全状态验收器。已读 SPEC C10.4（SPEC.md:341）与 TEST-SEAMS.md:188，以及 #1411 验收条款，均要求完整状态、真实恢复与预声明非语义身份一一重命名。原报告没有用当前简化实现缩小合同。

本次是源码与合同复核，不是新 Rust 测试 PASS。上述证据足以否定现票归因，不证明所有回放组件已经确定性完备。

## TB-10 的精确承接建议

已有 **#1413（TB-10-I）** 是合适落点。其当前验收第二项明确“45组冻结登记逐条有当前证据，并主动核新增/遗漏入口”，第一项要求默认启动没有第二套生产判断或经济 writer。故旧无协议 `s_structure_session` 分支属于该票现有遗漏入口复核职责，无需另造一个退休票或强行塞进某个 OE 族。

但目前 **不能说该具体分支已经被点名接收**。本次对固定 SPEC 全目录检索 `s_structure_session` 没有命中；已读 #1390/OE35、#1389/OE33 都只有聚合或功能族范围，不能凭同名“结构支持”或“CLI”判定已经完成逐入口承接。#1413 的既有开放职责足以接收新发现，主控应在那里追加精确对象与条件，然后回读确认。

建议追加的接单内容如下。

> 将 `rust/src/bin/s_session_v2/mod.rs::record_begin` 的 `!active` 无协议分支及其 `s_structure_session.rs::make_token` 纳入本票“新增/遗漏入口”清查。逐命令核真实调用、保留用途、默认可达性与写权限，决定按现行 R2 收编或退出。不得把整个 s_structure_session 二进制当旧臂删除，因为正式 v2/S3 同处其中。若发现真实旧持仓/在途写者或责任，依本票既有 #1402 前置交 TB-10-H crosswalk；没有实际用途证据前不直接删除。

完整回放义务仍落 **#1411（TB-07-C）**。主控可追加七字段身份/attempt引用关系的比较提示，但不能预先认定 token 都是可忽略 run ID，更不能去掉 attempt、epoch 或语义时钟。只允许该票原合同认可的预声明一一身份重命名。

## 主控可执行的下一步

- 将本复核与原调查结论追加 #1516，订正 P0 前置归因，不照旧方案编码。
- 在 #1413 追加上述明确对象并回读，在 #1411 登记完整身份与恢复验收提示。它们是现有义务的精确化，不新增 blocker、不替代其真实前置。
- 报告纳入可引用实体并核交付门后，再决定 #1516 的关闭。当前仅凭外盘报告及“归 TB-10”泛指不足以关票。
