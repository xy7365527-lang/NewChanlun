# #1394 读侧adapter独审01（正式BLOCK）

审查材料HEAD 490b53c20810176c5dbbe5ad8dcade47058b8570；真实subject4b3a84ebba8178e03aa04b2bd2a6a4138d9d438a。run d930b994-0b60-4190-b867-ba8e6e2f8860，external session 01a14db6-5581-77b0-9225-53a9c2f8f6ed，2026-10-18T06:42:24.512Z实际退出0。材料快照非生产源。

仅本审面 BLOCK：新 adapter 确认存在原件所有权、输出路径准入、准入前目录占用及工具读取预算四处问题。Q 对象顺序窄补丁未发现新增缺陷。本结论不代表整票验收；execution_authorized=false，产品 NOT_RUN。

## 正式发现

- P1｜s_session/tests/issue1394_acceptance/product_capture.py:904：_cli 在 cleanup 后按路径读取 stdout/stderr，没有要求本实例取得目录所有权。有限内存文件系统探针执行真实 _StdinProcess 构造失败路径：OwnedDirectory 抛 FileExistsError、directory=None、Popen 调用数为0，但 Exchange.reply/stderr 均收进 FOREIGN_PREEXISTING_BYTES。已有文件会被冒充本次进程原件；目录被替换后也缺少原身份核对。最小修复：仅通过本次持有且已核身份的目录/文件句柄读取并封存；未取得所有权时只记录失败，不读取该路径。
- P2｜s_session/tests/issue1394_acceptance/product_capture.py:797：ProductTransport 接受任意 output，根合同路径比较仅在 capture_product:921。持合法形状的内存 ReadAdmission，直接组合公开的 ProductTransport 与 CaptureMachine，可将根冻结的 /MEMORY_REVIEW/a/capture 换成 /MEMORY_REVIEW/unapproved-output；传输替身记录21次调用，状态机返回 UNAUTHENTICATED_RAW_CAPTURE，financial 输出路径也指向替换目录。这未绕过根 grant，但绕过了其中的输出路径约束和外层目录所有权建立。最小修复：在实际传输构造边界核根路径，并要求传入已取得的独占目录句柄。
- P2｜s_session/tests/issue1394_acceptance/product_capture.py:923：capture_product 在 guard 核实际 binary 字节前创建输出目录；preflight:636 只比较登记的摘要值。内存探针在构造准入后改变 binary 原字节，观察顺序为创建输出目录、guard 拒绝 owner binary 漂移、关闭目录，未发网络或启动进程。失效 binary 因此仍会占用目录，不满足本任务要求的准入失败零目录占用。最小修复：目录创建前完成当前 binary、owner 有效性和最早截止检查，并保留发送前复核。
- P2｜s_session/tests/issue1394_acceptance/product_capture.py:473、496：显式 verification.read_max_bytes/read_max_records 没有对处理的原请求/回复计费，最终只计六族记录的 canonical 字节；投影阶段的 ReadBudget 也没有消耗计数。有限内存探针将两项设为852字节、7条，状态机仍处理21份捕获、117196字节原请求/回复及455个闭包节点，并返回 UNAUTHENTICATED_RAW_CAPTURE。Job wire 另有计费，但不能替代这个独立工具预算。最小修复：复用同一读取预算，在解析前按原件长度累计，明确记录计数单位，不以 canonical 长度替代原件。

## 实际验证及read_guard映射

- 审查材料身份：/Volumes/AgentStorage/issue1394-read-capture-audit-20261018，HEAD=490b53c20810176c5dbbe5ad8dcade47058b8570，前后工作区干净，无 skip-worktree/assume-unchanged。它不是生产 source。
- 真实审查来源：原仓 /Volumes/AgentStorage/issue1394-sandcastle-integration-20261018；subject=4b3a84ebba8178e03aa04b2bd2a6a4138d9d438a，tree=d9331d67f751ebf6090019f93d73b6c01940b4ea。ORIGINAL-SOURCE-MAP.json SHA256=8256f61978f49dabe315c3adf2e36a90edef391a0f2f0953eb2d2b141f2f476f；228文件、2662967字节逐件匹配 subject Git 原字节。所有历史 diff/cat-file 均在原仓执行。
- 审查差异准确限定为 adapter 827ac95f3b9aebc36c0a493fe273aac7699481e0→576a13470566a5ca4625eae3d4e89abcb950e346，以及 Q 序 3f769f9c6581122451eb24603924bcd533b8419f→4b3a84ebba8178e03aa04b2bd2a6a4138d9d438a。已读指定纪律、合同、报告、实际实现和测试；未重复审查另一工位负责的 R-F5。
- 首次 freeze SHA256=7b984bfcd74e7f22f7d229f12a317cb7746cedb3e4371a17a5282ef2f4c1d8ed。14份历史文件及两份补读均独立核过完整长度、SHA和tree：b46ba296993371cb56b2dff7ef878647dc0665ad/f8733c328a7f1ab4b19725e0e97ec6dd9caf2bb1；C1 99e819192bd6c91b1dd90e058a93c2a98d5aaaf3/955cf8fabda358b94619714198e52916ac0738ff。它们仅是历史协议证据。
- 补读 q_http_input.rs：46589字节，SHA256=fed1f97b7a17a129263939837ef0039ab9634bb8727ef4a7f111c95542a32a24；q_wire.rs：19309字节，SHA256=f4b2afc858001a5b8a62da73fced7d3283640b2834dd4995aaeef39a15ffb6f1。C1 差异只涉及末节点 done=true 的装页字节计量。
- adapter 24项、Q包20项 delivery manifest 均匹配各自提交；另核230项源码、原输出及复用指纹，无不符。原输出确为作者普通/-O各38方法，以及Q普通/-O各36方法，后者14新＋22旧。这些是作者自检，不冒称本次独审重跑。
- Q补丁仅改变 basis 对象核验段，其前后源码逐字保留。请求须等于已核Q公开原序且覆盖完整活动集合；selected按请求序，通过真实snapshot_index逐项核独立snapshot的id/ref/hash。carrier/source/terminal对应2/0/1的协议关系成立，没有重写Capture原件或改成集合兜底。
- Q修订保护的27份旧Python及151份旧报告匹配；adapter两文件、Supervisor及revision5测试保持原字节。test_driver.py的41方法、test_scheduler.py的27方法保留。共享fixture确有合成basis顺序变化，因此未宣称adapter全依赖不变；复用连同fixture对拍及Q包已记录的两项adapter锁解释。
- 本次仅执行必要的有限内存探针；文件、控制回复和产品传输均使用内存替身。反例结果用于证明调用路径，不作为真实准入或产品成功证据。未重跑38/40/37/68/91/82/host123集合或ps/socket集合；未运行产品、网络、DB、cargo/rustc、浏览器、资金或真实产品进程。未写checkout、Git或tracker，未读取凭据，未启动工蜂。
- read_guard字段映射：kind/schema_revision仅区分候选协议；contract_sha256/plan_sha256对应既有根合同及计划原字节绑定；nonce/phase/slot/request_sha256对应单次请求、阶段和原件身份；lifecycle_id对应跨次累计身份。这些值由adapter提供，服务端须按既有冻结日程独立核对，不能将请求回显当认证。
- read_guard测量映射：owner/endpoint对应原PID/start/group/domain/epoch及端点归属，由独立Supervisor的OS Probe核实；deadline对应原whole/current-stage/request/Job最早截止，checked_at只能证明一次采样时刻。usage_floor/session_requests_floor是调用方已观察下界；job_usage八维及session_requests须来自相应Q、financial session/reader、具名Job的真实累计计数。Native审计、staged/store必须有所属owner的原生计量，RSS和floor回显不能替代。
- read_guard能力映射与最小后继：复用SupervisorCore的身份登记、后代全集、tick、stop及既有有界控制传输，增加候选READ_GUARD处理和逐owner原生计数接线；OS Probe补全过程端点归属，并由独立监督持续执行原截止及TERM/KILL/wait/reap。before/after ACK不能单独证明全过程归属或硬截止。保留原资源帽、单次receipt和初始累计，不新增authority；implementation/capabilities/verdict仅作为根外部核认的固定输入，不能由作者自签取得准入。
- 当前read_guard服务端未实现已被明确披露，本项本身未列为新bug；源码未据此宣称OS能力已完成。仍欠1488→1419→C1统一source组合语义审、两角色ARTIFACTS、同域terminal/K/独立γ、各owner Voice与独评、全资源/OS/根准入，以及fresh A/B、24故障、3额外域、5负域、10compare、完整历史、资金、恢复和隔离。前端归1460，K4排除；不批准P11、不关图票或#1279，也不新增全TB02前置。
