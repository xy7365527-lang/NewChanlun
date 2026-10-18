# #1394 读侧协议适配纵片01

名分：本目录及两个新 Python 文件均为绑定 #1394 的工作草稿。固定工具基线为 `827ac95f3b9aebc36c0a493fe273aac7699481e0`，分支保持 `codex/1394-acceptance-integration-20261018`。父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`；管理者 `/root/implement_1394_scheduler_v1`。本执行者是 Sandcastle 外部工蜂，真实 session_id 由运行时登记，不声称是父任务原生子代理。

已交付 `product_capture.py`、`test_product_capture.py` 和本报告包。普通及 `-O` 各 **38/38** 新方法通过，运行时间分别为 0.794 秒、0.793 秒；这是相同38个方法的两种模式。实现包含真实会话 CLI stdin gate 和真实 Q HTTP socket 输送，测试全部使用有限内存替身与临时文件。**产品 execution_authorized=false，全部 NOT_RUN；作者自检不代表独审通过或产品验收。**

## 已实现行为

`CaptureMachine` 按冻结日程执行 Q 全页 → 单次全对象 basis → 全 basis closure。Q 使用 POST `/api/v3/snapshot` 和完整十一字段 `s-session/3` 信封，保留 HTTP 请求/回复及 body 原件；每页核固定16字段 cut、17字段 token、因果身份、Q producer、offset、页长、六族数量及原序。全部 Q 页的投影根在发送 financial 请求前核对，拒绝换 cut、混 token、重复或漏页。原协议允许新一次审计的 `validated_capture_digest` 改变；token 的首次 `issued_capture_digest` 不得变。这一差异来自固定历史源码，不采用“所有 digest 一律相同”的额外判据。

financial 命令精确为八参数会话客户端 `financial-basis-ref` / `financial-closure`，必须带 `--session`，没有六参数冷读回退。stdin 原 bytes 在 ACK 后原样送入；gate 用 `os.read(0,1)` 读控制行，避免 Python 缓冲读取吞掉产品 stdin。保存 stdout、stderr、退出码、时间、argv、PID/start/group、前后 binary SHA 与回收回执。原 `OwnedProcess` 的控制 stdin 会在 exec 前关闭，因此本模块作限定适配，复用原 `ExecutionAdmission.check_command` 单次消费和 `OwnedProcess` 身份/回收方法。

basis 请求使用 Q 全部活动对象的公开原序，逐项核对象 ref/hash、返回原序及 snapshot_index。closure 固定同一七字段 anchor、`path=[]`，首 `after=""`，之后严格取实收 `next`。每页按原回复字节（含空白和 LF）核自身请求帽，节点升序、总数、根、末页完整性及可达全集均检查。闭包内部类型核验明确使用“收到的同一份节点流”，不冒充独立原库读取器；最终核全部六族与 snapshot 内容一致。未接线 template/json_text/非整数内容仍拒绝，不生产 terminal/K/γ。

session、reader、Q 和两个具名 Job 各自保留原帽。Q `max_frame_bytes` 计完整请求行、HTTP头和 body；Job wire 计原传输字节及 stderr，不以反序列化结果缩小。历史 Q 每一页重投影完整六族，记录费至少计该页完整投影总数，不能只计返回小页。额外全历史审计、Native 存储与堆等实际消耗必须由独立监督提供，不能从 wire 推断。

请求期限取原 whole、当前 capture stage、请求 stage、所属 Job、请求绝对 deadline 和本次原 timeout 的最早值。返回、解析、计费、监督后都检查原截止；没有重试或续期。生命周期初始累计由根冻结，保留 staged/store 既有用量，同一准入对象上的包装器共用账本；已消费请求不能重发，拒绝后整个生命周期停止。跨进程重开仍需根把已有原累计带入新的冻结合同，不自动清零或把本地观测当批准。

## 准入和当前缺件

唯一产品入口 `capture_product` 必须取得精确类型的 `ReadAdmission`；布尔值、模式字符串、字典、MemoryTransport 均在目录创建、Popen 或连接前拒绝。`ReadAdmission` 基于真实 `driver.ExecutionAdmission`，检查根合同内的 adapter、计划、兼容核认、监督实现 pin、单次 receipt、两个已登记 owner、binary、端点、资源和截止。未修改 `driver.py`，也未把 `ControlBridge` 的 toy 限制误归到 ExecutionAdmission 本体。

现有 Supervisor ACK 不证明 endpoint 归属和全部 Native 资源。因此新增一个**明确尚无服务端实现**的 `read_guard/1` 控制接口，定义见 `INTERFACE-CONTRACT.md`。只有根另行 pin 并独立核认其 OS 实现，且每次收到绑定同原件、原截止和累计账的 READ_ACK，才能发产品 I/O；旧 ACK、未知结果、时间或累计回退均拒绝。当前源码中的 SupervisorCore 不实现此接口，本轮没有制造任何可运行产品准入包。该缺口归本票后继 OS/真实控制传输纵片，不阻止本次读侧代码交付，也不声称已完成 OS 实装。

现有 `captured_cut.export_captured` 仍需根外部 `expected_sha` 和独立持久 source。本捕获器不调用它，不从实收数据自算摘要冒充根批准。另有一处已定位的窄接口差异：Q 公开排序不等于 `snapshot.objects` 存储顺序，而现导出器要求请求严格等于后者；本模块保留原请求，建议后继只调整导出器的对象顺序核验，详见接口文件。本轮未改该模块。

## 证据与复用

首次 source freeze 的原文件 SHA 为 `7b984bfcd74e7f22f7d229f12a317cb7746cedb3e4371a17a5282ef2f4c1d8ed`，已复制入 references。14份来源均通过 `git cat-file blob <完整commit>:<路径>` 核原 bytes/长度/SHA，并核 b46/C1 的完整 tree；结果在 `SOURCE-FREEZE.json`。两历史提交只用于协议取证，不是新统一 producer 或运行授权。二者 financial_reader 的差异只改闭包末节点 `done=true` 短一字节的装页计量，不改变 CLI/字段/游标形状；适配器跟随实收 next，不预测每页节点个数。

HTTP 帧和 Q 回复外壳的必要补读取自同一已冻结 b46 tree 中的 `q_http_input.rs`、`q_wire.rs`，单列完整字节 SHA 与行号摘录于 `PROTOCOL-SOURCES.json`；没有回写根14文件 freeze，也没有把补读升级为新运行源认可。其余固定源码摘录、C1 差异和旧 SOURCE-CHECKS/XERJ 的引用均在封包内可查；未安装或重建索引。

独审04原文已按指定 SHA `7f2c2e00bd339578584fe3d933cb358cf436b375a6f8480f56c0f72bd240381c` 核验并复制。N2 已闭合；R-F5 的逐 emit 阶段截止与 emit 后截止/采样残余仍由管理者另修。本轮不改 Supervisor/captured_cut，不改旧报告，也不把现监督基线当作产品通过。

`TEST-SELECTION.json` 记录执行前影响面和最终方法身份；`TEST-RESULTS.json` 绑定真实普通/-O原输出、命令、退出0与执行字节。`STATIC-CHECKS.json` 记录28份 Python 的普通/优化内存编译与范围/空白检查。26份旧 Python 与固定基线及 revision4 指纹完全一致，包括原41个 driver 和27个 scheduler 测试方法。`TEST-REUSE.json` 逐件绑定旧报告、合同、来源索引及输出，只复用原证据，不声称重跑。

没有运行旧37/68/91/82/host123集合或 ps/socket 三项，没有实际子进程、产品、网络、DB、cargo/rustc、S/E/B/Q、Long、浏览器、资金或凭据读取。没有认领、评论、commit、push、merge、切分支、改 Git 配置或启工蜂。作者差异留给管理者固定快照与独审。

## 正式成功义务保持

两角色 ARTIFACTS、新统一 source 及 financial_reader 组合语义审、同域 terminal 全标签/完整 K/source/独立 γ、各 owner Voice 与真实独评、逐对象资源与 OS/实际输送、根精确准入，均仍欠。fresh A/B、24故障、3额外域、5负域、10 compare、全历史、资金、隔离、恢复尚未验收。后继 OS Probe 和全 product adapter 仍是 #1394 的义务。前端归 #1460，K4 排除；不勾 P11，不关 #1394/#1361/#1340/#1323/#1279，不新增“所有外票最终验收后才可编码”的阻断。
