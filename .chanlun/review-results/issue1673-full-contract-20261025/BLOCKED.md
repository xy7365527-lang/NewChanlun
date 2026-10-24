# #1673 全量合同返修：当前 BLOCK，不能合入

候选：`57c1722fd57ac24e3e43b885aa378ef1d739525a`。本目录是当前失败证据；`issue1673-fin-runtime-20261022/` 的旧 PASS 仅是三条 S 输入、放宽帽与缺件归档的历史结果，不是本候选验收结论。

## 已实施的修复

- owner/Q 物理帧恢复 1 MiB，Q 逻辑文档与共享缓存恢复 4 MiB；S 输入恢复全部 95 条，分页 HTTP 使用同一剩余截止。
- Q 重启的 query/producer epoch 按启动代递增；恢复对拍只排除 writer_epoch、pid、control_instance_id，保留其余 runtime，并加入稳定字段漂移负控。
- 新增独立 Rust S1：同一 FIN/1 夹具、独立 E/B 数据库、正式 `service::handle` 全链与完整终态导出；Python 正式链按同一 canonical 终态对拍。
- provenance 要求可解析 HEAD、相关源码树干净，并记录当前三二进制的实际 build 命令与摘要。

轻量证据：Python contract 15/15、py_compile、`git diff --check`、Rust `rustfmt --check` 均通过；本机未跑 cargo。

## 远端实测的决定性阻断

nc-remote run `/tmp/nc-remote/20261025-021647-57c1722fd5.log`：

1. 当前 HEAD 构建三二进制成功；独立 Rust S1 成功并导出 `{"E", "B:alpha"}` 全终态。
2. 正式 Python 链在全 95 条 S 输入下完成 Prepare → Reserve → Commit。
3. 随后 `r-4 ReadView(B:alpha)` 为 DeliveryUnknown，不能形成完整响应帧。独立 S1 终态 canonical JSON 实测 **26,784,361 字节**；正式入口无法在同一 FIN/1 表示下满足 1 MiB 物理帧，更无法满足分页仍要求的 4 MiB 逻辑文档/共享缓存帽。

根因在 FIN/1：`finance.rs` 把完整 S basis 同时保存在 decision record、witness 和 history。分页只切物理字节，不能把 26.78 MB 的逻辑对象变成合法的 4 MiB 对象。

## 需要的协议决定

- 坚持 FIN/1：批准完整证据的去重表示及公共读取/恢复验收；或
- 采用已有 FIN/2 `FinancialBasisEvidence(basis_ref)` 方向：另批 Prepare → Reserve → Commit 的真实 S 全链和公共证据读取，不得把未批准的 #1672 候选 spec 当作已覆盖。

不得用裁 S、放大 16/64 MiB、删除 runtime/witness/history 或改称成功来消除此 BLOCK。Q 真实重启、恢复后 runtime 对拍、正式终态与 S1 对拍、完整成功归档均未走到，保持 NOT_VERIFIED。

## 证据

- `RESULT.json`：正式运行 FAIL。
- `PROVENANCE.json`：当前 HEAD、干净源树、实际构建命令与二进制摘要。
- `S1-SUMMARY.json`：独立 S1 成功及 26,784,361 字节实测。
- `FAILURE-MESSAGE.json`：正式 B ReadView 的 r-4 失败原件。
- `commands.jsonl`：实际命令与退出码。
- 完整 44 份失败运行原件（含 S1 fixture/terminal）在本机 `/Volumes/AgentStorage/issue1673-full-contract-repair-20261024/candidate-57c1/run/`；这不是关票归档，完整成功证据仍待协议修复后重跑。
