# #1394 Q 对象公开顺序窄修

已修复导出器把 Q 公开序误作 snapshot 存储序的问题。作者定向自检：普通及 `-O` 各 **36/36** 方法通过，包含 **14 个新方法＋22 个旧方法**。同一组两种模式，不计作 72 个不同方法。新字节等待管理者固定组合 SHA 并双路独审；本包不代签独审，不宣布产品或整票通过。

本目录为绑定 #1394 的工作草稿。固定干净基线 `3f769f9c6581122451eb24603924bcd533b8419f`；工作区 `/Volumes/AgentStorage/issue1394-sandcastle-integration-20261018`；分支 `codex/1394-acceptance-integration-20261018`。父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者 `/root/implement_1394_scheduler_v1`。执行者是 Sandcastle 外部 Codex 工蜂，真实 session_id 由运行时登记，不声称注册为父任务的原生子代理。

## 修改范围和行为

`captured_cut.py` 仅改全对象 basis 核验段。`wanted` 来自已经通过分页、公开排序、六族全集、独立源闭包及投影根核验的 Q rows。请求必须逐项等于这个公开原序；无重复且集合严格等于 snapshot 全部活动对象。`selected_objects` 保持请求原序，逐项按其真实 `snapshot_index` 读取独立已核 snapshot 记录，核索引规范、边界、唯一性及 id/ref/hash；不以 Q 排名替换存储索引。没有修改、排序或重编码任何收到的 Capture 请求或回复。

新增 `test_capture_order.py` 的 14 方法覆盖：Q 原序 `carrier/source/terminal` 与 snapshot 索引 `2/0/1` 合法通过；公开序与存储序相同的 `0/1/2` 正例；集合相同但请求原序不同拒绝；请求漏、重、额外或替换对象拒绝；selected 错位、漏重额外项、错 id/ref/hash、非规范/越界/重复索引、拿 Q 排名当索引拒绝。还核原字节不改写、Q wire 乱序、独立源与 wire 分离、basis proof 原值，以及现有 MemoryTransport 捕获到导出的有限内存接口。

按任务允许的“最小相关既有锁”，同时只调整 `test_capture_bridge.fixture`：原 fixture 按存储序构造请求和回复，本来就不满足本次已批准的 Q 原序合同，不能继续当作合法正例。现在从构造中的 Q rows 生成合成请求，selected 沿公开序返回且保留原索引；增加 `storage_order` 仅供同序正例构造。其余辅助函数、10 个旧测试方法与实现保持原字节。不是对真实捕获做迎合性改写，也没有改 `product_capture.py` 或 `test_product_capture.py`。

`FIXTURE-COMPARISON.json` 记录固定 clock=100 的前后有限构造对拍：旧构造器从本次固定基线载入内存，未执行旧测试集合。源 nodes、完整输入及每条非 basis 捕获的 request/reply/binding 逐字一致；basis 的每个 selected 字段原值不变，只有合成请求/回复的列表序改变。首闭包仍为原 **5940 字节**。实际捕获从未交给这个构造器，toy 原件也不被称为真实持久库证据。

## 红绿与保护证据

修前只执行 14 个新方法：退出 1，原输出为 `FAILED (failures=11, errors=3)`，共 8 个方法有失败或错误。决定性反例是合法 Q 序被旧实现拒绝、存储序请求和 matching selected 反而被放行；其余若干失败是检查尚未到达 selected/proof 边界时的诊断匹配失败，不冒称为独立的新缺陷。同序正例修前即通过。红灯后测试字节未变。

修后普通模式 36 方法用时 1.262 秒；`-O` 同组 36 方法用时 1.282 秒，均退出 0。22 个旧方法精确选取为桥接 10、review02 的 N1–N5 7、review03 的 N2 3、adapter 2。N2 的 5940 成功、6040 超原请求帽拒绝、6040 精确帽成功，逐页自身帽与 session/reader/Job 帽均保留；N1 空游标、N3 独立 Job 及归属、N4 观察尺寸与 Voice 帽、N5 原截止路径全部通过。未重跑旧 37/38/68/91/82/host123 全集合、ps/socket 三项或 R-F5 集合。

`TEST-SELECTION.json` 在修前登记影响面、完整方法身份、选择理由与未运行项。`RED/NORMAL/OPTIMIZED-RESULTS.json` 分别收实际 argv、cwd、退出码、原输出文件长度/SHA，以及每次运行前后 30 份 Python 指纹；三次各自前后不变。`TEST-RESULTS.json` 引用这些实体，不重复复制指纹。没有把包装脚本的退出 0 当作红灯测试通过。

`STATIC-CHECKS.json` 核实 30 份 Python 的内存 AST、普通/优化 compile 和 `git diff --check`。旧 29 文件中 27 份与固定基线逐字一致；修改 2 份、新增 1 份。静态方法身份总数 **231→245**，全部 231 个旧方法身份保留，其中 driver **41**、scheduler **27**。导出器 basis 块前后源码逐字不变，完整 source/refs/proofs、cut/anchor、全 basis closure 顺序/末页/可达全集、对象/Job 原帽及截止没有另开判据。

151 个旧报告文件逐件与开工指纹及固定 HEAD 复核一致，记录在 `TEST-REUSE.json`。R-F5 的 supervisor、16 方法与 revision5 包原字节保留；读侧 adapter 两文件原字节保留。两者仍只是原作者自检、尚待独审。其余 adapter 方法本轮未重跑：虽然 adapter 源文件未变，但共享 fixture 已有上述窄差异，因此不能声称完整依赖 SHA 都未变；复用须连同本次 fixture 对拍、两项原 adapter 锁及新内存接线锁审阅。

## 源码与协议来源

`BEFORE-SOURCES.json` 固定开工源码和旧报告；本次源码完整长度/SHA 在 STATIC-CHECKS 和 DELIVERY-FILES 中。导出器当前为 26595 字节，SHA256 `982d9a9a82af5dd49c9500e11adc4de64fa856156318e7f7de25081b8ac9c89d`。

已读 AGENTS、CLAUDE、wayfinder、CODEX-CHILD、ask-matt、世代宪法、product-capture REPORT/INTERFACE-CONTRACT/SOURCE-FREEZE/PROTOCOL-SOURCES 和 revision3 CONNECTION-CONTRACT。按 ask-matt 已批准小修路径执行；未另开流程或票。运行时无可调用知识图谱工具，按指定文件和必要 rg 回退。

历史来源完整提交是 `b46ba296993371cb56b2dff7ef878647dc0665ad`、C1 `99e819192bd6c91b1dd90e058a93c2a98d5aaaf3`。原 14 文件 freeze SHA256 `7b984bfcd74e7f22f7d229f12a317cb7746cedb3e4371a17a5282ef2f4c1d8ed` 未改。`PROTOCOL-SOURCES.json` 记录本次通过 `git cat-file blob <完整SHA>:<路径>` 对 14 份原来源及两份补读的长度/SHA/tree 复核，另附两版本 financial_reader 对象索引和 wanted 返回序的逐行摘录。关键依据为该文件 537–574 行的原 snapshot index 与 wanted 顺序，以及 projection.rs 341–395 行的公开排序。

补读仍取同一 b46 tree：q_http_input.rs 46589 字节，SHA256 `fed1f97b7a17a129263939837ef0039ab9634bb8727ef4a7f111c95542a32a24`；q_wire.rs 19309 字节，SHA256 `f4b2afc858001a5b8a62da73fced7d3283640b2834dd4995aaeef39a15ffb6f1`。原协议摘录按旧封包路径引用且已纳入旧报告字节核对。没有把历史来源冒充新统一 producer，也没有改任何 Rust。

## 交付和仍欠义务

DELIVERY-FILES.json 绑定三份本次源码/测试及本目录报告实体的长度、权限、SHA；不含自身摘要或未来 commit。独立审查状态为待审，管理者负责固定组合 SHA 和双路独审。没有认领、评论、commit、push、merge、切分支、改 Git 配置或启工蜂；没有读取凭据。

`execution_authorized=false`，产品全部 `NOT_RUN`。本次只跑 Python 有限 toy 测试与只读 Git/静态核验；未运行 cargo/rustc、真实 S/E/B/Q、Long、真实 DB、网络、浏览器、资金、真实产品进程或 OS 探针。read_guard 仍为候选接口，不新增 authority/spec 硬前置、统一 producer、生产 decoder、功能模块或资源帽。

实际统一 source 及 financial_reader 组合语义审、两角色 ARTIFACTS、同域 terminal 全标签/完整 K/source/独立 γ、各 owner Voice 与独评、OS/原逐对象资源/根精确准入、fresh A/B、故障组、负域、比较、完整历史、资金、隔离、恢复仍欠。前端归 #1460，K4 排除；不勾 P11，不关闭 #1394/#1361/#1340/#1323/#1279。
