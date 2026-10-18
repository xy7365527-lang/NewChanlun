# #1419 精确运行提案（未授权）

**authorization=false；execution_authorized=false；product_run=NOT_RUN。** 本文与同名 JSON 是 reviewable proposal，不授予执行许可。source_commit/source_tree/source_manifest_sha256/binary_sha256/frozen_run_rule 全为 `PENDING_SOURCE`，不能拿 donor 的既有 binary hash 冒认为新源。

机器合同 `EXACT-RUN-PROPOSAL.json` 保存完整 argv、feature、原始输入/expected 的全字节长度与 SHA256、顺序、时钟、切面、恢复步骤和资源含义。`RAW-INVENTORY.json` 保存受控原件与未替换的市场对照。四份 expected 的语义详见 REPORT 与完整 ledger。

构建候选接口只为 `s_structure_session`，原 features `s_session,s_capture_native_testonly`、`--no-default-features --locked --release` 保持。当前禁止实际 build。将来经准入的 S/advance Native 臂才可显式设 `S_CAPTURE_NATIVE_TESTONLY=1`；不得继承其他实验路由。没有读取宿主凭据或宿主环境变量。

准入后的精确顺序候选：两臂 fresh-a/fresh-b 串行、各从本次空 TestOnly DB init；第 1…111 条使用 `messages.base.jsonl` 的前 111 行；固 cut111，保存六族/内容，停 S 并 reap；`recover-after-111` 到 epoch2；核同 cut 后，用 `advance-0111.epoch2.json` 送第 112 条，再 serve 送 `messages.recovery-epoch2.jsonl` 第 2…28 行。该文件最后三行依次是 `revise-non-zn-tail`、`withdraw-second-center`、`restore-original`。两臂每条输入字节与计划相同，不临场生成替换 raw。

固定前缀输入条数 39/63/87/111/123/135/139；首个非空中心对候选在 123，完整本例在 139。初始完成后的 input_frontier=138；三次修订后分别139/204/269。generation、structure_cut、index_frontier、profile/rule/catalog 必须取同次真实回执：一次65事件修订只提交一代，所以 watch 不能把 affected seq_range 当作 input_frontier。

snapshot 页必须保持六族、固定 cut 和 token；重建七字段得到 projection_ref，再验证 qcut3；content 始终使用该 projection_ref 和同锚续页，不能把 qcut3 字符串当 root。变化、撤回与恢复保留全部 raw replies、节点、AsKnown/重算模式和真实对象身份。两臂比较不删除业务时间、顺序、refs或修订；capture/进程身份差异原件另列。浏览器必须核同源同 cut，新对象、变化、六变量、连接/尾部和证据，不能用 toy 页面代替。

原 S 资源7项完整保存在 `S-RESOURCES.reference.json`：1,048,576 frame bytes、read 2000ms、write 1000ms、queue32、connections48、response10000ms、audit_cache268435456 bytes。原 Q21 项完整保存在 `Q-RESOURCES.reference.json`：capture2000ms、verify5000ms、reply8388608 bytes、page31、watch4等全部字节不变。输入10秒、单操作30秒、总期限900秒、retain4不放宽；不得使用功能测试的600000ms代替。

前轮未准入累计提案原值保留：reply128MiB、两臂 DB/WAL/SHM268435456 bytes、工作堆268435456 bytes、内容store134217728 bytes、日志8388608 bytes、整次900秒；含两臂和恢复、错误与重试不重置预算。这些**不是已授权执行帽，也不是 OS 硬帽**，须根按新输入与最终源另核。单个输入实测最大21647 bytes只证明原件体积，不证明结构输出负载可在原帽内完成。

通用接口准备已完成到可独立验证的程度：`toy_supervisor.py` 真实监督固定 Python toy 的 Popen 子进程，非阻塞读、总截止/失联/取消、TERM→KILL、wait/reap；`driver_v2.py` 假传输完整闭包与领域预期比较。产品启动仍不开放。正式 OS 内存/磁盘帽、同步IO截止、EINTR、完整后代树/Native测量资格未获证明，不得将 toy 结果迁移为这些结论。

实际剩余门：最终源和独审绑定、原帽下准入、实际 S3/Q 和新进程对拍/持久恢复/浏览器验收、市场源认证/原量/全历史义务。本候选只准备本票已允许的 TestOnly 结构子域，不能销项整票或扩大资金政策。
