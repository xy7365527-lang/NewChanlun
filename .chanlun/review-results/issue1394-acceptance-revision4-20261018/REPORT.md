# #1394 冻结尾差返修04

R-F5、N2 两处指定残余已修复，作者定向自检通过；**新字节尚待管理者固定快照后独审**。正式独审03对基线的结论仍为 BLOCK，本包不改写该结论，也不代表整票、P11 或产品验收通过。

固定基线：`6cb6bbf33eb079f639abae2449ccc9b98d81a11c`。本包为 #1394 活期工作草稿。父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者 `/root/implement_1394_scheduler_v1`。本执行者是 Sandcastle 外部工蜂，真实 session_id 由运行时登记，不声称是父任务原生子代理。

## 实际修改与边界

仅修改 `s_session/tests/issue1394_acceptance/supervisor.py`、`captured_cut.py`，新增同目录 `test_review03.py`。另新建本报告目录；没有修改旧封包、revision1..3、其他工具模块、生产 Rust 或主仓在制品。没有认领、评论、关票、commit、push、merge、切分支、修改 Git 配置或开启工蜂。

R-F5：把 ACK 的身份登记与采样后确认分开，直接 ACK 与 monitor 共享强制采样入口。monitor 首次接收后只用 `receive(0)` 读取立即排队消息，保持消息原序；本批截止固定为「首消息返回时刻 + 原 sample_interval」与本批接收前的 work/heartbeat 截止之最早值，不随本批心跳滚动延长。队列返回 None、到达采样边界、遇 COMPLETE/CANCEL，任一条件均结束本批，先核后代全集及资源，再按序发 ACK。没有新增资源帽或无限等登记的窗口。

两种正式顺序都被锁住：HEARTBEAT→ACK_REQUEST→COMPLETE；已登记101后，REGISTER102→REGISTER103→COMPLETE。另锁 ACK 后排队后代的内部强制采样路径、无队列、空队列边界、陌生PID、伪身份/父进程/合同/nonce、原截止耗尽、持续心跳流和 COMPLETE 抢在登记前。陌生PID不收养、不信号；失败只清理已核身份并保留 INCOMPLETE。`receive(0)` 必须非阻塞，时钟须为持续推进的 monotonic；这些是控制传输的既有实现义务，fake 测试不认证真实 OS 实现或硬中断能力。

N2：每份 closure 页同时保存解析值和 `len(capture.reply)`，逐页以原回复长度核本页 request.max_reply_bytes，包含末 LF 和合法空白。session、reader、Job 原件账仍各自保留，不能代替请求帽。5940 字节／帽5940通过；前加100空格、重新绑定捕获清单与合同摘要后的6040字节／帽5940拒绝；6040字节／帽6040通过。全部16页逐页测试超帽拒绝，financial-basis 的原件对象帽与Job wire账另有正负锁。snapshot分页块和原帧核验逐字不变。

固定协议取证见 PROTOCOL-CHECKS.json：producer 生成规范JSON+LF，session用实际序列化字节核帽，client原样转发；strict_json解析未比较输入与重新编码。因此本次不另加“输入必须等于某次canonical编码”的准入规则。三份固定协议原件的SHA已对旧SOURCE-CHECKS复核；它们仍是b46协议快照，不能冒认新统一执行源。原5940/6040回复及两个请求帽的toy原字节保存在references，索引为RAW-WIRE-EVIDENCE.json。

## 验证与复用

TEST-SELECTION.json 在执行前列明影响面、方法身份与选择理由：旧方法24个，新增13个，共37个。普通与 `-O` 各37/37通过，两模式不是74个不同方法。未运行68/91/82/host123集合或ps/socket三项。没有运行真实toy子进程、cargo/rustc、S/E/B/Q、Long、数据库、网络、浏览器、资金动作，也未读取任何凭据。

最小红灯先验证4个方法：两种正式排队复现、ACK内部强制采样复现、5940/6040原件复现，基线实现全部失败（3 ERROR、1 FAIL）；修改后上述方法均通过。原输出保存在 red-targeted.txt、normal-targeted.txt、optimized-targeted.txt。RED-RESULTS.json、TEST-RESULTS.json记录实际命令、cwd、退出码、方法、输出SHA及运行前后26个Python文件指纹；源码在两模式执行中及执行后均未变化。

STATIC-CHECKS.json记录26个Python文件的AST及普通/优化内存compile、`git diff --check`退出0。原driver41、scheduler27个方法及文件逐字保留。23个旧Python文件与基线及revision3验证指纹相同；Supervisor.stop等6个未改方法、captured_cut的7个未改函数和snapshot分页块分别以精确源码摘要复核。只有两处指定实现文件和一个新测试文件产生工具差异。

TEST-REUSE.json逐项绑定旧报告、连接合同、生产依赖交接、原测试输出与31项既有来源的长度/SHA；连接合同和大包仅沿用原路径，不复制。F1/F2、R-F3/R-F4/R-F6按不变代码及独审03证据复用；N1/N3/N4/N5在本轮影响路径中选旧方法重验，具体身份见FIX-LEDGER.md。未运行的旧集合不称为最终版本全套通过，历史host证据不认证当前包。

正式独审03及完整6条原探针在复制前核对SHA-256，原字节放入references；未复制runtime raw或session日志。报告SHA为 `3842328bbffc0db6c63c4b715bd4a479f63c0f130b225ff4cc4cbcaf47463816`，探针SHA为 `accd04d4973e3bd80cd2d9ef9915b7c374aebcd0f343c4d5cf7523c831d1666f`。DELIVERY-FILES.json列本轮源码、测试、报告、必要原输出与原件，不含自身摘要或未来commit。

## 正式产品义务保持

`execution_authorized=false`，产品全部 `NOT_RUN`。两角色ARTIFACTS、统一新source及financial_reader组合语义重审、真实同域terminal全标签/完整K/source/独立γ、逐owner真实Voice与独评、逐对象资源、正式OS/传输和根精确准入仍欠。A/B、故障、全历史、资金、恢复、隔离仍未验收。路线仍是1488→1419→组合C1→统一新SHA，未新增全TB02 blocker或资源帽。

Journal继续保守隔离candidate：重开不自动恢复接受资格，不能冒称自动崩溃恢复。前端归#1460，K4排除。不勾P11，不关#1394/#1361/#1340/#1323/#1279。差异留给管理者收包与独审；作者自检不能代替独审或运行授权。
