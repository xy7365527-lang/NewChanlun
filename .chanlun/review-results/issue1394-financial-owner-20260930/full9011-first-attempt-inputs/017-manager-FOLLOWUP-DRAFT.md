#1394 runner 的一次集中修正。run-2 已受管取消并保全；当前 clean candidate HEAD `5cc69fa6d08313f2c20f6ba730832e7a39873d35`。

只改同一 isolated checkout 的 `.scratch/issue1394-full9011-runner-20261001/` 产物；不改 tracked/root 原件，不启动真实 S/Q/Controller/HTTP/audit，不打开已有 SQLite。你并非唯一工作者，禁止覆盖他人改动。

Root 已冻结具名全9011首试输入 `/Users/silencehan/Projects/NewChanlun-1394-content-v3/.scratch/issue1394-full9011-root-inputs-20261001/`：`ROOT-ACCEPTANCE.json` SHA `2f1753a4aed09c4fa28163354107a7891241649294813b96bc6ba2d99b0a8de4`，`CALLER-OBSERVATION.json` SHA `145874d694753b248c18cd199da6b8d5aa3cff145d1344df7f090c22fbf79378`，S7 resource SHA `b3d8b67b723d881a98dd0f4116fdc9651cf05b207111b19dad26fa69c2c54e69`，Controller-A/B SHA 依 ROOT-ACCEPTANCE 各自具名。原 Q21 文件亦有固定 SHA。必须直接以这些已冻结文件为事实与门，不再要求另造 `full9011-independent-bound`、容量证明、额外预算 schema/量纲或重复 root 批准。保留原 S 七值和 Q21，caller 的每请求/启动30s、每 case 观察1h、cleanup30s、6GiB 开始空闲、4GiB 自有 artifact 上限、100ms 采样，均是私有 owner 观察/停机门，不是生产资源量纲、容量保证或 9011 工期预测。可先开展全量首试；失败/超观察上限/中断即停止并保全，不把 partial 当 complete。严格 RSS/heap/IO/内部阶段 peak 当前未测，写 `UNMEASURED` 和缺口，但不能凭此制造预先容量证书硬门，也不能把采样最大当 true peak。

新 Controller 路径严格按 ROOT-ACCEPTANCE：DB 在 `.scratch/issue1394-full9011-formal-runtime-20261001/{A,B}/s.sqlite`，socket 在 `/private/tmp/nc1394-f9011-20261001/{A,B}.sock`。只在 Root 后续显式运行时允许新建这些 fresh 路径；本轮不创建。原件 hash/mtime/inode及 SHA 先后保全，RUNNING/Ready PID argv nonce resourceSHA、每条 request/reply 原 wire 与 native reply/persistent receipt、全9011不缩短不重排不重试、A 完整后才 B、完整 DB schema/tables/columns/rows/BLOB token/native hash 对拍、STOP/status/stop/reap/cleanup 失败不 PASS 均须保留。独立离线反例测试覆盖截断、少1、重排/错hash、同clock不同时序、资源缺失、重复域、partial、清理失败。最终精简 RESULT 显示离线测试真实退出与 `S_Q_NOT_RUN`。未来如 A/B 9011 与全量逻辑门都过，只允许 S 双跑范围结论，`full_resource_capacity=false`、C/整图仍未完成；未测严格峰值照实列为缺口，不是 S 首试准入门。

独立 Python reviewer 已定位的实际缺陷也在这一次修完，并加有针对性的纯离线反例：

1. `runner.py` 当前 `own_destination`、`validate_paths` 限死 worker HERE/output A/B 和 `/private/tmp/n1394-*`，与 Root 已冻结 `Rootruntime`/`nc1394-f9011-*` 矛盾。只准 ROOT-ACCEPTANCE 精确声明的 A/B DB、state、socket/output；启动前验 fresh、owner、inode/目录身份，不能扩大任意路径权限。
2. `runtime.py` 当前 cleanup 前撤销 ITIMER、逐操作 30s 且 snapshot/save 无全局限期。按 Root 的 case 1h + cleanup30s 绝对 deadline 留保全预算；用真实 `st_blocks` 合算自有 artifact 4GiB、启动前空闲6GiB 门，不以采样代替；超限停止、保全失败证据，清理不得无限拖长。
3. `evidence.py` 的 `row_count=True` 可等于 1，完整 schema/columns 未逐元数据且 Python `True==1==1.0` 会误判。拒 bool/int/float 类型混淆，保留 SQLite 原类型、`PRAGMA table_xinfo` 全元信息、BLOB 原 token；加负例。
4. `runtime.py` 的 `client.exchange` 异常目前绕过 exchange.json；清理失败可丢 snapshot。每次异常也保全已有 request/reply wire、可核持久 receipt/state；snapshot 做不了须记录失败/UNMEASURED，绝不补造或延长预算。
5. `runner.py` 目前自行 contract.S7/budgets 未绑 ROOT-ACCEPTANCE、S7、CALLER-OBSERVATION SHA/逐值，可能悄改 30s/100ms/6GiB/4GiB/顺序再入门。核 Root 冻结文件路径、SHA、完整值与 Controller A/B/Q21，不修改 root 原件来绿测。
6. `evidence.py` 的逐条 clock/native receipt 还漏 `s_clock_events.attempt_index/message_key/attempt_ordinal/generation`、`s_input_messages.accepted_seq/attempt_count` 与 `s_protocol_meta.next_attempt_ordinal/logical_phase_frontier`。同错 A/B 相等也要拒。原生单次序列预期 accept index=-1/ordinal NULL/gen=n-1，begin/commit index0/ordinal=n/gen=n；event 与 message_key 关联原 header。逐列具体类型和数值须核当前 native 代码及冻结 clock 原件，不能猜/填默认。新增各字段相异负例，且拒 `row_count=True` 对 1 行的混淆。

修正仅围绕以上已定位问题。运行纯离线材料门、精确 Root 输入 gate 和这些反例；不得新造容量证书、peak 框架或额外批准门。结束即给出最终文件 SHA、测试命令/退出码、S/Q/原库 `NOT_RUN`、真实运行门是否可由 Root 后续审定。不要写长报告，不要提交、push 或合入；管理者会接回并复核差异。
