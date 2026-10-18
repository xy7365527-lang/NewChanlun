# #1394 正式独审01返修与接线准备 v2

六项正式返修均已落实并通过本轮定向回归；固定cut导出和同域桥候选编组已提供可审代码、toy正负锁与接线合同。**正式产品验收仍BLOCKED，execution_authorized=false，S/E/B/Q、Long、完整历史、P11、AC1..5全部NOT_RUN。** 不把正确拒绝、未认证候选或离线成功当产品成功。

当前唯一写集在 `codex/1394-acceptance-integration-20261018`，固定起点 b8c5fd404959ff5fac3e48d4e9449a975d6c0b7e，未commit/push/merge/切分支/改Git配置，未启动其他工蜂，未认领/评论/关闭票，未读取凭据。此工蜂由外部Sandcastle启动，不声称是父任务的原生子代理或自行注册真实session_id。真实运行身份由运行时登记。父任务01a12e01-45a1-7173-a3e8-d338d108fb93，管理者/root/implement_1394_scheduler_v1。

旧 `.chanlun/review-results/issue1394-acceptance-integration-20261018/` 全部保持历史，正式独审01报告与七条探针完整命令/输出原字节收在本目录references；入包SHA分别3c4ff1b9cf589eb0253a647a1a2baef41861d383437691c811895ad6c7be0fb0与c51419e3a664968c0d384174e5337540033e2a8f8925ea6010b88fa3c4f72930。SOURCE-ROUTE-NOTE也收齐，ea41静态地图另以历史身份保存。并未改写任何旧失败输出或测量。

## 返修结果

F1从硬pin对应准备合同原字节重导出Requirements，完整原量词不能靠同时删requirements/steps绕过。F2按各owner冻结epoch与有序原请求全集，恢复只更新具名owner，重送逐项核原字节hash/身份/出现序号。F3成功本地提交必须绑定独立E原件及完整根链、会话/代际、Reserve提交与先后。F4所有核验/序列化/持久化返回前复查最早固定截止，失败写入不留成功stage收据，journal仍保持已消费pending。F5先核合法登记消息再采样，继续拒绝未知PID；F6每次wait和汇总后核原cleanup截止，逾期仍保留reap事实但整体INCOMPLETE并继续其余owner。

逐项原反例→修复→正/负结果、夹具订正与必要journal/oracle连线理由见FIX-LEDGER.md。driver.py、control.py、closure.py和原P0回归源均未动。test_evidence补SQLite显式close。旧41 driver及27 scheduler方法名称全集保留，无删减；beta未重启却跟代表owner升epoch的旧toy输入已订正，成功完整矩阵仍为510阶段。

## 新模块实际能力

`captured_cut.py`接根冻结身份、完整输入bytes和已捕获S/Q/financial请求回复，核token/anchor、profile/rule/catalog、同cut分页顺序、全basis闭包及六族有序全集，对象源ref/hash/index不缺漏。输出UNAUTHENTICATED_FACTS，永不启动产品。独立节点source、外部expected pin与原件保管是调用方信任边界。

`voice_bridge.py`仅接实际terminal全标签、全部K对象及source、根冻结字段映射、独立γ原件、instrument/opening_raw/TestOnly时钟；核本域hash/profile/级别/方向/确认前缀/锚/整个来源区间/唯一host，保留所有候选。无terminal/K/γ时报告缺件，多host、缺来源、原件删减、跨域等拒绝。只产UNAUTHENTICATED_CANDIDATES，作者不能自签APPROVE，不能自动BindVoice。单件64MiB/集合128MiB未放宽。

两模块没有伪装成全协议生产adapter：Q catalog只核冻结引用与投影根，Q content/watch与Delta业务解码仍欠；闭包模板0x08、json_text语义解码和非整数数字重编码未接线时拒绝。真实terminal/K格式尚无统一审后来源，必须由根提供精确字段映射，测试UNIT_TEST_ONLY_*不是正式G类型。γ数学证明、terminal/K语义完整性与作者独立性仍由真实独评负责；一个author_id字符串不构成认证。

CONNECTION-CONTRACT.md给出目前已核命令、payload exact字段、每阶段必需输入/产物/原件、逐对象资源和接线顺序。新生consumer准备工具名分绑定本票，尚无产品调用者；报告是活期绑定#1394的review-results工作草稿，不新增裁定或资金政策。

## 本轮实际验证

第一轮封包检查普通和 `-O` 各 **91/91** 定向测试通过。随后给捕获合同补机器资源门，仅对受影响模块普通/`-O` 各跑 **10/10** 并通过（原9方法+新增1方法）。其余82方法源码与依赖未变，按指纹复用先前通过证据；不是最终重新跑了92项，也不能把两个模式合计为独立业务断言。原输出为normal-targeted.txt、optimized-targeted.txt；精确解释器、工作目录、命令、退出码和耗时在TEST-RESULTS.json。选取scheduler/integration、evidence、financial_oracle、supervisor、journal及直接调用journal的control，加本轮review01和capture_bridge。没有重跑无关全套，也没有以新假测试替代原三个OS依赖测试。

新固定锁14方法覆盖正式六类反例及其全部fake-clock值；桥最终10方法包括大量子例。新增资源门验证session/reader/Job、Q原18+3、S及每E/B资源、旧16MiB/10秒、256MiB retained、stage/whole/cleanup/seal无null且不放宽，原reply和累计wire/请求/节点/记录超帽拒绝。普通模式与-O均通过。先前的开发检查是63方法回归通过、独审锁24方法通过（当时直接导入FinancialTests导致重复发现10方法，已改模块导入，现14方法）、桥8方法通过、合并22方法通过；增加来源区间/K环锁后是上述91；资源门最后只复跑10项，原输出与命令见TEST-RESULTS-RESOURCE-FOLLOWUP.json及两个resource-followup.txt。创建测试文件时两次从仓根直接 `python3 -B -m unittest test_review01/test_capture_bridge -v` 因工作目录错误报ModuleNotFoundError，未运行测试；随后从工具目录正确执行通过。这些早期次数不是最终覆盖计数。

24个Python文件AST/compile成功；工具逻辑无assert准入。git diff --check退出0。STATIC-CHECKS包含最终源码SHA、原41/27方法保留、P0相关文件不变证据；STATIC-CHECKS-91保存91项时的源码指纹，TEST-REUSE记录复用82方法相关字节未变。HISTORICAL-FINGERPRINTS独立核旧DELIVERY 47项与MANAGER-DELIVERY 53项在b8c5 Git原字节全部匹配；这核的是历史，不是将旧清单用于当前变化。SOURCE-CHECKS重核原7个指定pin全部匹配，复用已入包XERJ查询与SOURCE-CHECKS，不重装/索引大树。本会话无可用代码图MCP工具，按允许回退读指定源码。

所有新反例以fake-clock/fake Probe/内存捕获为主。相关旧supervisor/control/journal定向测试内的短命toy子进程都wait/reap；SQLite均显式close。没有cargo/rustc、产品S/E/B/Q、Long、真实DB、真实资金、浏览器或网络动作。inner sandbox的ps/Unix socket能力未变化，旧host普通/-O 123/123只是b8c5修改前历史成绩；本轮没有声称新字节全套123/123。

## 正式成功剩余的最小接线

根尚须固定Native1488→1419审后C…F→组合C1的统一新SHA（Native/C1共改financial_reader，组合需语义重审）、producer与consumer两份ARTIFACTS/toolchain。b46仅旧协议快照，3315仅工具宿主；两者都不能被本工蜂指定为未来执行源。

需真实完整输入产出本域terminal全标签、完整K/source及独立γ证明；给出审后字段映射、各经营owner的authority与真实独评。需完整逐对象资源、实际请求日程和根精确单次准入、真实OS监督与服务传输。然后才能完整writer→回收/custody→financial READY/SessionClosure→E/B CaptureExternal/ApplyAllocation→本B原fact e_cut→真实BindVoice→财务→query/history/watch/originals→S离线恢复→统一回收封口。模块尚不支持的内容形状遇到即阻断，不能以更宽规则或离线替身补洞。

六项工具返修与本轮可独立准备已经交付；P11/完整历史/资金链、fresh A/B与24真实故障组/3额外域/5负域/10compare的产品证明没有取得。#1394/#1361/#1340/#1323/#1279不关闭。最终差异由管理者固定新快照后独审，本轮自检不替代独审。
