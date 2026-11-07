# X 历史模拟执行接口

`x_execution_session serve --db FILE --profile FILE --writer-epoch N` 在既有
`x-execution-profile/1` 的 `mode: simulation` 下接受 `history_simulation` 对象。
它只产生模拟成交原件，不连接场所。旧 profile 的行为保持不变。

## 冻结输入

`history_simulation` 对应 `runtime::Profile`：模型、session_id、session_generation、
x_id、source_namespace、source_epoch、X socket、E/S authority socket/身份/epoch，
timeout_ms 与 max_frame_bytes。路径必须绝对，三端点不同。完整对象在 X SQLite
冻结；同库不得切成其他 profile，也不得换模型、来源或权威端点续跑。

模型 ID 是 `history-sim/bar-batch-known-frontier/1`。当前支持 per_leg、整数 tick、
整数 quantity/money quantum、确定性 seed=none、显式具名费率与市场滑点。
limit_role/market_role 必须由具名 role_source 明示 maker/taker，不按订单类型暗填角色。
RD12 固定为 `per-leg-source-attribution/1`；未知政策拒绝。netted 返回 Unsupported。容量分配复用经济域 ADR0032 算法，不自建配额算法。

## 单行 JSON wire

请求：`{"protocol":"x-exact-simulation/1","op":OP,"payload":VALUE}`。
每帧以 LF 结束并关闭写半边（EOF），读取回复后关闭连接；字段形状见 `runtime.rs` 与 `model.rs` 的具名类型。

- AcceptExact：payload 为完整 ExactCommand。查询冻结 E 的 ReleasedExact；
  核全命令、计划/条件摘要、会话与 epoch 后持久 Accepted，不问 S、不产生调用权。
- AdvanceBar：payload 为 Batch。每个 close_change 必须逐字对应已接纳命令；
  真正动作前向 S 请求 ExecuteCommitted，绑定 E release 收据与命令 eligibility_ref；
  X 先持久 MayHaveCalled，再原子提交模型状态、不可变原件、批结果与 Observed。
- QueryCommand：payload 为 command_id 字符串，回状态、x_accept 三相时钟与 E causal_refs。
- QueryOriginal：payload 为 external_fact_id 字符串，回 X 冻结的完整原件。

E/S wire 是 `x-exact-authority/1`。op 分别 QueryReleasedExact 与
RequestConditionalExecution。请求包含完整 ExactCommand、command_hash、会话、
authority_id/epoch 与可选 release_receipt_id；回复另含 state、receipt_id、commit_ns。
S 回复必须同时绑定原 E release_receipt_id；命令资格引用必须等于 S receipt_id。
未知、Denied、不可达、不同内容/身份/epoch 均不取得调用权。

ConditionEvidence 当前只支持 released-exact/1、execute-committed/1、
simulation-local-transaction/1 三种能力。每项必须携带完整权威、依赖、失效源、
检查阶段、证据及覆盖，失败语义为 Denied；其他能力返回 Unsupported。
真实 E/S producer 尚须在 #1407 / TB04-D/E 接此 wire，受控夹具不替代它们。

## 时间、恢复与消费边界

X 接纳取系统 ns，要求 received <= first_known <= commit 且 first_known >=
E release_commit_ns；写入与启动冷审计共用判据。提交前调用占有也不得早于 S commit_ns。
同 ID 同内容重送读原结果；改内容冲突。MayHaveCalled 只允许同一完整批内容恢复，
这是本地 SQLite 模拟 adapter 的幂等合同，不授权网络场所重发。

已准入订单在下一 bar 批执行，收盘撤改不抹当根成交。限价严格穿越、开盘取优，
市价只在具名 exec_index 收盘槽；批内顺序 Unknown。fill、fee、scenario bust 各有
独立原件，fee/bust 指回原 fill。原 fill 不因 bust 被改写或删掉。

经济会话须冻结 history_simulation_source（来源身份、producer 身份、x_db、模型摘要）。
CaptureExternal 对照 X 库原字节后可 Recorded+Pending。完整经济分量/short 归因未接通，
Resolve/Apply 明确拒绝；不得把 Pending 当作持仓或完整回放验收。


## 容量 epoch（#1381 R3）

input/scenario 的 capacity_manifest 与 E projection_manifest 分属不同身份、累计和用途。
X 从完整已知命令状态和完整bar计算可成集合；不能由请求方传入子集。集合、额度和合同
语义不变时保留冻结 requested_q 与累计C，只调用现有ADR0032的 A(Cnew)-A(Cold)。
正常成交耗尽的零余量成员留在冻结向量中，不因它退出资格集合而重置；纯bar/ref/容量
数值变化或不改变资格的改价也不重置。实际资格/成员边界才封存旧证据、以合法剩余量
创建新epoch。空集合/全部完成关闭，不造空投影。family累计从不因epoch归零。

限价区间先消费同方向总budget，收盘市价槽只用余额；买卖方向独立，不按每槽复制额度。
每个输出绑定获准命令、family、revision和projection_manifest，容量身份不授执行权。
QueryBatch 的字符串payload为request_id，返回同事务保存的capacity_transitions和
capacity_evidence；不可变fill、fee、bust保留原容量/投影因果引用。close变化先规范排序，
同close同family多个动作拒绝，不靠到达序选胜者。


## 调用前验证与不确定窗口恢复

AdvanceBar 在取得任何 S 资格前完成同一模型的完整只读计算及全部精确命令核对；
非法 OHLC、close 变化、容量、金额或 bust 不消费 Accepted。随后先持久唯一 PendingBatch，
再将各命令置 ExecutionRequested、调用 S、逐条保存实际 ExecuteCommitted 回执为
ExecutionReady。全部回执保存后才置 MayHaveCalled；模型状态、原件、Observed 与清除
PendingBatch 同事务提交。pending 期间只接受同完整批内容恢复或只读，不能用空 close 异批绕过。

ExecutionRequested 的恢复只调用 x-exact-authority/1 的 QueryExecuteCommitted，要求返回
同精确 command/content/session/epoch/E release 的既有不可变 ExecuteCommitted 原件。
NoRecord、未知、不可达或没有确定结果时保持 pending；不得重新调用 RequestConditionalExecution
或把未查到当作再授权许可。已持久回执直接复用，不重复 S 授权。请求发出前崩溃也属于该
不确定窗口；没有 S 确定原件就不能自动前进，不清占有、不补发。正式 S producer 尚未实现
本查询接缝，#1407 / TB04-D/E 必须真实消费；受控 fixture 的唯一授权计数不替代该实证。

冷启动核批请求/结果、模型重演状态、不可变原件表、bust 账及 command/E/S 引用闭合；
缺 command、缺原件、缺实际 S 回执均拒绝恢复，不从剩余表补造证据。


## 完整原件冷重演与 pending 换代

fill、fee、bust 由提交与冷审共用的纯计算生成。冷审从空历史逐批前进，用该批以前的
原成交与 bust 累计核算，再逐字核完整批结果及独立原件/账表；不能拿最终累计倒算前批，
也不能以派生表彼此一致代替原始 batch/model 的确定性义务。既有库缺 authority_profile
或绑定不同均拒绝，不能重建绑定后换端点。

PendingBatch 与 pending_epoch 同事务保存。尚有 pending 时，任何不兼容 writer epoch
必须在 launcher 写 writer_epoch 之前拒绝；旧 epoch 仍可查既有 S 终态并完成原批。
本片不提供 pending 换代接管政策，不改旧命令/回执，不以重发 S 解锁。


## 首次初始化也是一次发布

模拟schema、初始contract/state、exact_commands和authority_profile在同一SQLite事务
内建立。X launcher还将自身x_meta/初始writer_epoch放入同一事务；提交前中断只留下
未初始化状态，提交后中断留下完整冻结绑定。已有模拟库仍严格拒绝缺profile，不能
按表为空补绑。事务内初始化函数接收Transaction类型，不接受裸Connection。
