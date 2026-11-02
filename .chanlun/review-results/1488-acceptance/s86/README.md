# #1488 验收 · S86 输入与对拍工具（切片 S1）

本目录只放输入、小型清单与脚本；运行库与产物在 WSL（`~/nc-remote/1488-accept/s1/`，保留副本
`/mnt/g/AgentStorage/1488-accept/s1/`）。

## 输入（S86）

| 文件 | sha256 | 说明 |
|---|---|---|
| `messages.jsonl` | `b6429b2e5a0f02ea2419b251d10a0cb7c7ca59ca998084bccb85d5d43053bfc5` | 86 条 s-session/3 ingest 信封，每行规范 JSON + LF |
| `clock-plan.s86.json` | `fba1bdfb1d5753877b41c13ca4313553c6e123109378c2ca2b69262ee91750d8` | 86 事件冻结时钟（`s3-frozen-prefix-cost`，ns=100+3n） |
| `bars.s86.json` | `ed62acd411c825c5115a9fc20a69538ea8742d280c468dcc0fe2d7dff9875a05` | 源前 86 根 bar（仅供核对） |
| `signed-catalog.json` | `938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069` | main 6badaa6049 `s_session/catalog/` 冻结副本 |
| `ohlc_integer_tb02a_v1.json` | `482250254133ba120afdd5cd428d1a32d4e5cb40cd7876c556b7007320e599d6` | main 6badaa6049 `s_session/profiles/` 冻结副本 |
| `s-resources.json` | `a1243e98a4823e311499e80b86e2b2f5259108ac16d9c96d3bb7a596a5209bea` | serve 资源档（两臂同一文件） |

来源、档案路径与交叉核对见 `INPUT-MANIFEST.json`。`build_s86_inputs.py --bars <9011 bars> --check`
可从冻结源（sha256 `76472995…0087`）逐字节重建并核仓内 receipt 的 `ordered_request_hashes[0:86]`。

## `run_s86.py`：一次完整路径运行

`run_s86.py --bin <s_structure_session> --workdir <新目录>`：新库 `init` → 新进程 `serve`（Unix socket，
常驻，跨请求热态保留）→ 86 条各一连接一帧 → SIGTERM 停服 → 冷路径 `financial-basis-ref` 取末代
basis 回执。workdir 必须不存在（保证新库）。清除继承的 `S_*` 环境变量，以 Ordinary 模式运行。

逐代读数 `readings.jsonl`：`elapsed_ns`（客户端往返）、服务进程 `/proc/<pid>/io` 增量
（`rchar`/`syscr` 为 SQLite 读 I/O 的进程级代理，`read_bytes` 为真实块设备读）、`cpu_ticks`、
提交后 `stored_nodes`/`stored_body_bytes`、主机 1 分钟负载。main 二进制没有「SQL 节点读次数」的产品
计数器；`syscr`/`rchar` 是两臂可同口径比较的替代量。

## `compare_s86.py`：三档对拍

`compare_s86.py RUN_A RUN_B --mode same-binary|cross-binary --out report.json`，只读打开两库。

| 档 | 内容 | same-binary | cross-binary（main ↔ #1488 段 A） |
|---|---|---|---|
| INVARIANT | 逐代 `manifest_root`/`snapshot_root`/`delta_root`；逐代 FinancialBasis 根（Python 按 `financial_reader.rs::current_basis` 配方独立重算，并以产品 `financial-basis-ref` 的末代 `basis_ref.root` 自核）；`basis-ref.json`；输入文件 sha256（同输入前提）；`init.json`；86 条回复帧逐位（含 `receipt_id`、`index_frontier`）；`s_content_objects`/`witnesses`/`relations`/`observations` 及其余全部非节点表逐行（meta 除环境键）；公开根集合；从全部公开根出发的可达闭包节点（digest、tag、sha256(body)，并逐节点核 `F(tag,body)=digest`） | 必须相等 | 必须相等 |
| CONDITIONAL | `s_content_nodes` 全集（含不可达中间根）、逐代 `stored_nodes`/`stored_body_bytes`、`sqlite_master` DDL、`binary_sha256`、meta 环境键（`sqlite_version`/`platform`/`journal_mode`/`synchronous`） | 必须相等 | 允许不同，报告列入 `conditional_differences_to_explain` 逐项解释（如 #1488 R1 使不可达节点降为 0） |
| ALWAYS_DIFFERENT | 耗时与 `/proc` 读数、墙钟时刻、路径、pid、主机、数据库物理文件字节（页布局、WAL 检查点时机） | 不比 | 不比 |

可达闭包按 APPROVED-SOURCE §5.2 节点表解析子边：0x03/0x04/0x05/0x06 取 `body[8:40]`；0x07/0x12 取 body；
0x13 取两子；0x14 取键值两摘要；其余无子。缺节点、形状不符或摘要不符时退出码 2。
退出码：0 通过，1 门内有差异，2 结构错误。


运行完整性是比较的前提：任一臂 `failure` 非空、代数或发布行数不是 86、`financial-basis-ref` 返回码非 0，
或产物文件缺失时，退出码为 2，不给出 verdict。

## 独审与已知限制

独审（非作者，code-reviewer 子代理，2026-10-20）的结论是「修后可用」。已修复的项：
- 异常归类：缺文件和 `TypeError` 一律退出码 2；
- 增加 86 代完整性门；
- `init.json`/`basis-ref.json` 改为保留行尾逐位比较；
- manifest 标量只取顶层字段，不做深度搜索。

保留的项及理由：
- `responses.jsonl` 是 runner 解析后按规范 JSON 重新编码的帧，不是原始字节。服务端回复体本身就是
  `canonical_json(output)+"\n"`（`rust/src/session_transport.rs:560`），与 Python 规范编码同规则
  （sort_keys、紧凑分隔符、ensure_ascii=False、不接受浮点），所以重编码结果与原帧逐位相同。
  这一点依赖 `canonical_json` 不变，跨二进制对拍时它属于 #1488 不应改动的协议层。
- runner 把 serve 的 stdout 接到 PIPE 且不读取。serve 在服务期间不写 stdout，r8 两跑均无悬挂。
  socket 文件出现后才发首帧，存在「已 bind 未 listen」的微竞态；如果发生，表现为首帧 TransportUnavailable，
  runner 会失败退出，不会产生静默错误。
- 表存在性列在 INVARIANT，DDL 列在 CONDITIONAL。#1488 段 A 不应新增或删除表；若确有新增，cross-binary
  会判 FAIL（保守方向），需要人工解释。
