# #1621 主线时钟回归与跨域获知顺序修复

日期：2026-10-24  
候选分支：`codex/1621-main-regression-20261024`  
Draft PR：#1694

## 范围

本分支处理 #1691 合入后出现的主线回归，并继续完成 #1621 原 AC 中可达的跨域时钟锁。当前不把本分支视为 #1621 的完整关票证据，原因见文末两个未决项。

## 根因

#1691 将开局签署时效门从原件自报 `known_at` 改为 `record["b_observation"]["first_known_ns"]`，但没有同步迁移三组旧测试：

1. TestOnly 小整数时钟被解释为 1970 年，导致签署尚未生效；
2. 旧负控仍期待自报 `known_at` 驱动时效门，与新合同不一致；
3. 正式 B 写入只核本域 received → first_known → commit，没有核 B 的首次获知不得早于 E 提交。

## 改动

- `0e67abaa52`：将 funding 夹具迁到真实 Unix epoch 纳秒；时效负控改为驱动可审计观察时间；保留自报 `known_at` 不再控制门结果的反证。
- `6bdf8e02de`：新增 E→B 负控。B 比 E 提交早 1ns 获知时，原实现错误返回 Applied。
- `28421d0dc9`：在正式 B 写入路径新增 `B.first_known_ns >= E.commit_ns`；`store::verify_e_copy` 恢复审计复用同一判据。拒绝发生在 B 事务和持久状态改变之前。
- `6478a9c182`：补 S→财务读取负控与正控，以及转移本金批 `known_at` 的观察时间来源断言。
- `328dea2dde`：`cargo fmt`，无行为改动。

生产改动只有 `service.rs` 一行调用与 `store.rs` 一个 18 行判据函数。未改 `tb03c_kill_tests.rs`，避免与 #1622 的 owner 写集重叠。

## 验证

执行环境：windows-5090 WSL，`nc-remote`，独立 target。未在本机运行 Cargo 编译。

| 检查 | 结果 | 证据 |
|---|---|---|
| `6bdf8e02de` 新增 E→B 负控 | 按预期红：原实现返回 Applied | `/tmp/nc-remote/20261024-154648-6bdf8e02de.log` |
| `28421d0dc9` funding 测试 | 14 passed / 0 failed | `/tmp/nc-remote/20261024-155008-28421d0dc9.log` |
| `6478a9c182` funding 测试（含转移断言） | 14 passed / 0 failed | `/tmp/nc-remote/20261024-155343-6478a9c182.log` |
| `6478a9c182` chain 测试（含 S→B 负控） | 3 passed / 0 failed | `/tmp/nc-remote/20261024-155534-6478a9c182.log` |
| `6478a9c182` kill 单点恢复 | 红：续跑第 1 步的时钟夹具违反新锁 | `/tmp/nc-remote/20261024-155648-6478a9c182.log` |
| `328dea2dde` 完整 CI | 运行中 | GitHub Actions run 40076486216 |

E→B 负控还验证：拒绝前后 B 图像不变、没有新回执；把 B 时钟修到 E 提交之后，同一事实可成功应用。

## 尚未关闭的门

1. **恢复夹具兼容性。** `tb03c_kill_s00_capture_open_n1::before_commit` 中，E 恢复后的提交约为 1e6ns，B 仍用约 5e3ns 的观察值。新锁正确拒绝这个夹具。已交 #1622 的 kill 测试 owner 对齐合法恢复时钟或重试轨。兼容修复与完整 CI 通过前，PR 保持 Draft。
2. **X 执行域。** 当前 main 没有 `x_execution_session` / `production_session`，#1523 仍 OPEN。B 的转移测试不是 X。故当前证据覆盖 S→B、E→B 与 B 内转移，不能声明 S/E/B/X 全部验收完成，也不能仅据此关闭 #1621。
3. **非作者独审与 main 合入。** 待兼容修复确定后，对最终完整 diff 另派非作者独审；main 合入与关票仍过原人工闸和 `delivery-discipline.md` 全子句。
