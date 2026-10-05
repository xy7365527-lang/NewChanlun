# #1394 财务内容与 E/B owner 接线证据

本封包为 open #1394 的工作草稿，承接已批准 #1340 和 #1394 C 接口增补。后端总图 #1323、goal 保持未完成；前端 #1460 独立。没有合入 main、push、关票或修改 GOAL_SET/SUPERSEDE。

## 固定代码与规范

基线 `7ec958012401acd0515f8be558ae13e925473685`；根产品 `6dcee4da72873ab323132cf2fd64271702a1d8c3`。其全部 Rust 文件与受审 source `d8b4b713e4645d4b7833aca0705b4e35f22ab198` 相同。客户端 source `268f864ed48ef0540637a4073d31e75d04af8705` 的完整独评与后续调用审查保留。

规范为 [已采纳 C 原件](../issue1394-content-v3-20260916/APPROVED-SOURCE.md) §5.1–5.9、[增补登记](../issue1394-content-v3-20260916/SPEC-ADDENDUM.md)，及原 R2 C09。原 SPEC 和封存成果字节未修改。

客户端从当次 S3 basis/content 请求取得完整内容，按一份 Job 累计真实传输、节点和逻辑记录。合法长标量没有额外 4096/8192 字节门；selected 与片段证明在实际 Index 访问前计费，失败与重试不退款。根回归1项通过；新小源110请求、1307节点、386记录，传输与节点规模没有因修复改变。

新 economic-session/2 与 finance/2 有独立版本、13表和本 owner 不可变节点存储。当前和全部规定历史路径的 FinancialBasisEvidence 均完整核验；peer 捕获保持原 image/root。节点发布、经济 image、消息和 receipt 在同一经济事务。首次本地审计、多 peer、S、归约和发布共享 Job；私有入队时间贯穿新 Unix 入口到提交前，公开信封和身份不加时钟字段。失败及恢复只清未发布 staging，不删历史 published 节点。旧 /1 格式及接口保持。

成功存储域明确为每 owner 独占的固定容量 HFS+。新实际载体25124864B，原 frozen staged/store 限额32153600/64307200B；原单域16MiB试验另保留。硬容量和 DB/WAL/SHM st_blocks 实测承担预付/对账，不用 SQLite 逻辑页或节点正文冒物理量。checkpoint下降时的保守账面高估限制未推广为所有配置可用性证明。

## 根工位实际验收

| 验收 | 实际结果与边界 |
|---|---|
| 新 /2 SIGKILL 矩阵 | 6操作 × before/after × 两鲜进程，24/24 exit0；12组原 comparison字段逐项相同。每例有实际PID、SIG9、fresh recover、epoch2、原回执/责任断言。缓存复用，不是24次冷导入 |
| 真半导入 | 新实例删除仅未引用的 setup cache；实际 Prepare 读取真实S，持久 staging=1且commit不增时SIG9。S副本离线，fresh recover后stage0、pub0、经济原像不变、责任20、无Prepared/grant，旧请求StaleWriter。1/1，2.14s |
| 正式CLI/Unix | 固定e_economic_session/b_chong_session的3个serve进程；Prepareα/β→E Reserve→B Commitα/β、同ID重发、QueryDecision/QueryGrant/ReadView/Watch全经单LF+EOF Unix请求。grant30/40、责任90、B Committed/locked30/40、无外效资格；1/1，36.16s |
| 正式离线recover | 三个serve均已停止/reap；仅本实例S binary/DB改名离线，三个正式recover命令均exit0、epoch2、stage0，manifest字节不变 |
| 最终COMMIT真实磁盘满 | 独立新3卷、真实S缓存Prepare；唯一commit hook已观察commits/receipt/message写入且只读连接仍见旧image，再写本卷pressure到真实errno28；hook返回false后SQLite实际COMMIT报disk full。1/1、2.23s，旧seq2/root/image、公责20、receipt/message不变，pub1307保留、stage0，audit和recover epoch2通过 |
| 来源变化与同Job重读 | 当前产品副本、独立两组S/HFS，删400/1307可达缓存节点；staging=300时正式S advance到新basis/commit22。对象仍有效，Prepare合法提交一次并绑定新basis；另一组同Job两次读取成功，requests197→198、wire6577398→6615805、records900→918、elapsed52301→52833ms，同一绝对期限未过。1/1、108.34s；成功Prepare剩stage21、只读probe剩1161，均在recover后归0，不冒称提交即stage0 |
| 正式Q3 HTTP与信号 | 当前产品release CLI query-serve，两鲜进程分别SIGTERM/SIGINT。每次7页/7非空行、独立qcut重算、37返回节点Hash和Watch1，均exit0且监听端口可重绑定；同一Q21档、S单bar副本不变。不是controller、浏览器、完整正文语义或在途停机验收 |
| Q3现有Controller核心实跑 | 两fresh Rust Q3进程真实经Controller.start_process→wait_ready→status→stop，S15/S2各exit0并reap，HTTP Ready与控制回执同值，nonce/PID/epoch/session绑定及端口释放通过。原DB/WAL/SHM bytes/SHA/mtime未变；独立98原件/36检查PASS。旧整体binary仅对应909ccff，当前三条精确source同字节；正式start_q仍Python/2，无S3七门或全资源宣称 |
| 冷审计行为锁 | 独占分支1836d3c38e9，仅state/driver cfg(test)两文件，独评PASS后整合dfab63cd95且两文件逐字相同。两条精确锁实际通过：同连接结果销毁后重审、重开连接和存活Driver请求均重读，损坏已发布节点后拒绝、恢复后通过；旧审计memo在正容量启用时retained/capture0且旧计数不增。提供旧计量映射依据，不等于七字段serve限额执法 |
| 经济只读/2代码适配 | 固定d34dd1e0d2e的3后台Python文件及1测试文件整合为4325ab248b/219c4f2ab1，四文件与独审源码同字节。36协议/单元测试通过，初评新增函数类型注解缺口经仅签名/import返修解除，独立模块导入/注解求值及Spec/Standards通过；不代替正式Q/B实跑 |
| 新只读Q授权真实经济HTTP | 新manifest初始化前冻结独立七读Q源；正式E/B服务产生非空Prepared→Reserve→Committed，实际HTTP的current/token/fixed/Watch/decision Original/grant Original共6次全部200。三域CompleteVector、/2公头Hash/因果、固定cut、责任90/locked30-40/无外效资格与Unix原件一致，原件full objects逐字段相同，owner before==after。最慢Watch15.569s、最大回复1078412B，均受原冻结10s单RPC/30s操作/2MiB回复约束；非browser或结构Q3路径 |
| 严格经济 /2 测试客户端 | 固定9f923877e7d2的四文件已整合3b53744be1，逐字节相同，独立Spec/Standards PASS。修复旧/1作用域、Evidence会话绑定、决定/权利/原历史双向覆盖、完整grant和捕获E关联、原输入解析/政策期限量纲/完整baseline公式与Close类型。a67完整256项及最后Close定向25项通过；两组有重叠，不称最终SHA执行281项。全部失败反例及返修原件保留 |
| 真实经济 /2 浏览器 | 同一case2归档的三独占副本、四正式后台与Chrome CDP；current双Committed→as-known85双Prepared→Watch逐域回同current→全部两decision及两grant Original。12实际HTTP回包全部200，其中6 snapshot、1 Watch、4 Original及1 discovery；最大实际POST回复922213B，完整四原件与正式Unix值相同。owner逻辑状态和三DB字节未变、全部进程reap、三副本普通detach、原归档未变。实际运行及独立事实补审PASS；不是controller/Q3/9011或新产品前端 |

Book、Voice和已有20责任的 setup 仍为明确 library/TestOnly 前置，包含受控权威/操作结果。实际金融步骤没有伪造 grant或手工置 Committed；本证据不成为形式化γ、完整经营模型或生产资金政策。CLI运行原件、原始结果及SHA清单见 [RESULTS.json](RESULTS.json)。

## 失败原件与回收

首初始化在合法空hex body处失败，只有E genesis、pub0/stage0，尚未SIGKILL；修夹具后新卷重跑成功。首CLI在socket路径超过SUN_LEN时启动失败，尚未接财务请求；新实例以短别名冻结manifest后成功。全部失败库/映像/日志保留，不覆盖，也不把编译红当行为红。

矩阵工蜂69卷及根成功/失败实例均普通detach、保留dmg；正式3服务明确stop/reap。原封包挂载核对见 inputs/TASK-MOUNTS-FINAL.json；新增磁盘满探针3卷也已普通detach并由根重新核对，原件保留。此处只覆盖已归档案例，不把仍在运行的新探针算作已回收。临时验收工位独立于产品，仅追加验收及输入回归测试；经济运行入口保持 ignored，不入产品。

磁盘满根复核比对14份原件的binding、manifest、测试源码、执行文件和输出SHA及关键字段；没有重跑已通过案例。生产清理后的原始SQLite primary/extended code均为0，错误原件仍为disk full；不能把SQLITE_FULL常量13说成清理后实测错误码。这是外部压力抢占本卷空闲的故障注入，超出正常独占预付成功域；只覆盖该次Prepare最终事务的原子失败，不推广为全资源、全部操作或可用性保证。

源绑定另核磁盘满的502份Rust源码/Cargo文件：只有finance_tests末尾测试include例外。来源探针核523份tracked rust、79份s_session文件，仅测试include例外；根再核manifest内24份原件SHA。来源变化首次manifest初始化错误、随后两个旧源码副本的无效/期限不确定结果均保留，不用于当前产品结论；最终当前版本新实例通过，四次共24卷已回收。它是具名缓存缺口读取，不是全冷下载或正式E/B Unix路径。

Q探针首次把wire的hex tag按十进制读取，并把TCP TIME_WAIT误作监听未释放；第二个SIGINT探针曾解析半条日志。仅修测试读者，失败原件保留，已通过的SIGTERM复用，新的SIGINT案例成功。72份成功请求/响应/信号原件SHA与源码、失败记录集中保存在inputs/Q3-HTTP-RAW.zip，摘要见inputs/Q3-HTTP-EVIDENCE.json。

冷审计源读114094是HashReader回落到SQLite的实际调用数，不是磁盘I/O或唯一节点数；F调用266797覆盖本线程全部Hash，也不是独占审计Hash账。库/WAL/SHM只在操作后采样，未声称峰值。生成器LANE、SQLite和OS缓存均不在旧审计memo零保留声明内；原输入、clock、阶段数据和两份恢复后的库副本在inputs/COLD-AUDIT-RAW.zip。新增测试及来源/Q原件限定独评Spec/Standards PASS，见inputs/INDEPENDENT-SUPPLEMENT-REVIEW.md；无新改动或疑点，未重跑已通过案例。

只读适配按冻结manifest的显式protocol_revision分派，absent/null继续/1，不从finance或响应猜版本；/2必须配finance/2，未知/混搭拒绝。E/B/Q回执沿同一严格namespace、原请求Hash与因果绑定核心；HTTP既有discovery字段集合与版号保持，只让原finance_revision值准确反映/2。只读Q源的原白名单门没有放宽，不允许借具备写操作的source冒用。原模板仅有写source，实际新Q运行需要初始化前冻结独立只读源和新manifest，不能热改原库。红/绿与返修原件集中在inputs/READONLY-V2-RAW.zip；缺少ruff/mypy/pylint/black/bandit的事实保留，未称这些工具通过或已运行正式服务。

后续已在新隔离域完成真实HTTP。case1七类只读Unix测量成功，Q写请求实际被授权门拒绝且经济状态不变；首次HTTP只得到PartialVector，三个域因短socket别名被Q.resolve展开到129–133B而无法连接，错误原件保留，未改资源或产品。case2在初始化前使用真实短目录，canonical socket为32/36/35B，复用相同七读授权与原Q19/八项经济限额，必要新金融前置35.71s通过后6HTTP请求成功。case2没有重复case1的写负控，不将继承同权限说成第二次实测。

独立manifest/resource/hash、实际CLI与HTTP请求/响应、失败诊断和回收记录见inputs/ECONOMIC-HTTP-MANIFEST.json及ECONOMIC-HTTP-RAW.zip。根核47个原件引用与6条真实HTTP公头/Hash/因果、三个latest cut/finance2及readonly before==after；源码与运行器没有为新case放宽。Q/E/B四进程全部停止/reap，两个case六卷普通detach，短目录回收；根live hdiutil核本任务挂载0。独立事实补审PASS，另离线重算token、6公头、12变化image Hash与native原件等价，见inputs/ECONOMIC-HTTP-INDEPENDENT-REVIEW.md。新HTTP不是任意路径可用性、全资源峰值、browser资产、新结构/3或独立完整证据闭包验证。

## 本轮严格客户端与浏览器补验

客户端代码及输入/历史关联审查见 [独评](inputs/ECONOMIC-CLIENT-INDEPENDENT-REVIEW.md)、[可见输入核对表](inputs/ECONOMIC-CLIENT-INPUT-CHECKLIST.json)。旧四组HIGH、完整原policy/baseline/Close余缺均经具体反例返修；完整256与最后定向25组合复核，不重复无关既有案例。原资金政策、grant身份派生的旧REV以及不可见来源认证边界保持，未新增内容节点API。

真实浏览器原件见 [manifest](inputs/ECONOMIC-BROWSER-MANIFEST.json)、[结果](inputs/ECONOMIC-BROWSER-RESULTS.md)和[独立事实复核](inputs/ECONOMIC-BROWSER-INDEPENDENT-REVIEW.md)。独立标准库重算11POST公头、两端payloadHash、replyID及因果、9份imageHash，核当前/历史/Watch与全部四Original，418个运行及来源文件摘要一致，fresh进程/监听/挂载核对通过。三DB保存字节未变；可写HFS副本DMG整体物理字节改变，原归档未变，不扩大为所有WAL/SHM不变。

两份选择性原件ZIP保留31个客户端原件与72个浏览器原件；新Chrome profile、pycache及镜像保留专属scratch，不入仓。整合前ROOT审计中的PENDING是当时状态；最终固定9f923两轴PASS、精确代码整合3b53744与实际运行补审见上述最终报告。此前55份及新增5份Controller归档inputs的bytes/SHA均已由根逐一核验。

## 仍未完成

本片仍需S3 writer具名资源与七字段实际执法、正式start_q的Q3接线、完整Q3结构客户端及同源查询、累计历史/全资源和9011双跑。现有Controller过程核心已实跑通过，不替代正式入口接线。现有验收客户端已严格支持finance/2并完成上述实际浏览器路径，但经济HTTP没有独立证据闭包节点入口；本次不独立证明FinancialBasis节点闭包/七字段正文，亦不重算没有前根的顶层经济root。现有页面与脚本归后台验收，产品新前端仍归#1460。条件FIFO/稳定点归已裁定的#1416，依#1407与#1361原生前置推进，不反挂成本片开工门或新政策待裁。其它TB与六Destination仍未验收。未跑本票远端CI或本票DevSkim，不把#1392扫描归成本票扫描。没有改变1349未定公用资金/cap政策与真实场所权限。


## 2026-10-01 Q3完整消费者与正式入口增量

当前产品集成为 `5ffac0506b054e9dc8f08287f3f4133247efa5e5`，未main/push。07直接inner头、纯Job及完整真实Client调用、正式start_q与原页面控件均已接入；完整代码两轴PASS。保留原候选三轮Spec/工程红与修复轨迹，未删除或降格原判据。根验证同字节18个session文件、bundle生成输入及原消费者1个TAP下42案例、当前页面4项。

正式start_q只在此前具名single-bar Q21域以两新进程实跑，真实Ready/身份/nonce及v3Snapshot成立，SIGTERM/SIGINT退出0并回收/重绑；67原件与36独立事实检查通过。此运行绑定的是性能小修前a174 Rust树，当前全部Rust新树未作为相同binary实测。

S16两次shadow仅作成本诊断，24应用表及48403行逐字段同旧原件，三audit约占末handle88%、实际SQL COMMIT37–41ms；RSS采样下界与未测全render/Valueheap边界保留。无Budget同次SQL已核帧传递小修两轴通过并集成，27靶向和non-testcheck通过，仍真重读且无跨read缓存；仅F次数锁，尚未测其S16耗时。

消费者实际协议/语义测试使用封存HTTP与S16cut3/4/5源oracle，DOM/HTTP与分配适配器模拟。真实Browser及物理分配、S16正式Q的新资源域、9011双跑与完整crosswalk/资源、S3和其余六Destination仍未验。#1460产品前端独立。新增inputs均绑定原件路径、bytes和SHA，原旧60份inputs完全未变。


## 2026-10-04 当前产物财务小域与隔离副本恢复

当前受测产品固定于 `eb1c60bfb0cfc1e088d13e71e4503fe4b1917ffe`，Rust／s_session 源聚合 SHA 为 `84b6a82e166d9f1a9211ea327eef68d51306fa94f0a02fb68ccdf63f7e007acb`。S、E、B 三个 release 产物均为非测试 binary，实际 features 为 `default,s_session`，没有 Native 或其它 TestOnly 编译特性。继续复用原 `TestOnly-cash-cap-100` 输入、数量、费用、时钟和全部资源值；不为本轮补新的资金政策。

本轮三个新 HFS+ owner 正式 init，Capture／Apply／BindVoice／已有责任前置经实际 Unix 服务执行；原 unit trust authority 与预装的 1307 个已验证内容节点仍是受控 TestOnly 前置。24 条 Unix 请求及 9 条 HTTP 核对通过：两个实际 Prepare、E Reserve、两个 B Commit 与幂等回执均成立，私权 40／50、锁与 grant 30／40、共同责任 90、固定 Q 12／20、实际持量 4／15，均无场所调用资格。15 个历史根及逐域 image／cut 与独立 SQL 账簿逐项相等。

真实本地 Chrome 使用现有只读测试页面，完成 current→85 时点→Watch→四份 Original。11 条实际 POST 包含六次 Snapshot、一次 Watch、四份 Original；公头、双侧 payloadHash、replyID、因果、snapshot token 和全部历史 image／cut 均独立重算。85 时点双 Prepared、无本地 Commit 收据、责任 20；Watch 三域各从 cut 3 到 4，最终固定切面与当时 current 相等；四份完整原件与实际 Unix Query 相同。页面 DOM 和截图对应这些真实回包，不是模拟页面或 #1460 产品前端交付。

原 `run-3` 的 240 秒浏览器协调标记超时，**其 FINAL 仍保持 FAIL，金融结果不是 Unknown**；没有倒签 BrowserDone 或重发财务请求。该运行的 21 个 Popen 已回收、三卷普通 detach、28 份来源 SHA 保持。随后仅逐字复制这三份已停止的 image，在新自有副本上补验：恢复前完整逻辑等于 PRE-Q，三库 primary SHA 等于浏览器前记录；同一 manifest 与产物在逐进程 sandbox 中禁止读取 S／peer／源 image 数据及执行 S，真实同目标执行正控和直接／子进程继承拒绝负控成立。三个 fresh recover 均到 epoch 2、stage 0，其余全部账簿、根、image、回执、消息、1307 个 published 节点及历史切面不变。恢复域 35 个登记进程和 7 个探针子进程全部回收，三个新副本普通 detach，19 项来源封存 SHA 保持。

独立事实复核允许上述金融、实际浏览器与精确已提交副本恢复组成**同冻结产品的具名小域验收**，不修改原运行失败记录。完整结果和限定见 [当前结果](inputs/FINANCIAL-CURRENT-20261004.json)，427 份选择性原件、实际回包、脚本与绑定见 [原件包](inputs/FINANCIAL-CURRENT-20261004-RAW.zip)；DB／DMG／可执行文件原件仍保留在各专属载体，不入包。本结论不扩成 WAL／SHM 全部物理字节不变，也不证明完整 9011、Native、整体 heap／RSS／SQLite／I/O、所有 crosswalk 或六个 Destination。#1394 与 goal 保持未完成，未 main 合入、push、关票或改 GOAL 事件。


## 2026-10-05 Native 来源缓存与正常服务窗口

代码从 `a60b265` 的独立 Native 来源 Counts 原型推进到 `ee8ea240` 的 Driver 接线。当前受测源聚合 SHA 为 `30c47e4b219dabba322c1d214c2fd766f7e9bf640fcbf0165173a609e659a5c9`，非 test Native binary SHA 为 `203e19edab94c29f725e83a084b59110ffdf8224802c8d35709bca5b7a6824b9`，实际 features 为 `default,s_capture_native_testonly,s_session`。每次请求仍读取当前原件并核验 control／raw／history／独立索引／catalog；同一 handler 内的历史结果仍须通过现有原件相等门才能复用。只有精确当前原件绑定后的来源 Counts 可跨请求复用；完整历史结果和 Public／Template／Frame 校验缓存在请求结束释放，保留池按规范源成本计费。该预算不代表整体 heap／RSS 上限。

用户已确认新隔离正常窗口的 512 MiB 源预算，即 536870912 B；30 秒响应期限不变。用户停止故意损坏数据与故障注入后，最后两项 Driver 正常测试通过，覆盖正常 append／repost／restart、输出一致及合法预算变体。此前 36 passed、2 ignored 回归记录按实际时序保留，不叠成 38 项不同的当前覆盖，也未在停止指令后新增故障验证。Driver readiness／proposal 是运行前快照，执行结果以随后两个 ROOT-RESULT 为准。

同一 Native binary 在独立 CopyQualified149 副本上实际收到 150／151 两请求。正常 S7=1 baseline 的整请求耗时为 27.448542667／25.890727542 秒；512 MiB 具名域为 25.398280166／23.490618250 秒，分别减少约 7.5%／9.3%。回执与持久 oracle 均同 baseline，原件封存不变；两域实际 Popen 全部回收、controller 关闭、socket 移除，终态均为 generation151、idle、151 Committed／published、453 clocks、next ordinal152。

原型同一静态副本的有限 audit 收益为约 23–26%，不作为整请求收益。上述两请求窗口不能与原 Unknown 或其它独立窗口拼成完整 9011，也不证明任意历史规模、整体资源资格或六个 Destination；#1394 与 goal 仍未完成，#1460 保持独立，未 main 合入或 push。摘要与原件索引见 [SOURCE-CACHE-20261005.json](inputs/SOURCE-CACHE-20261005.json)，1422 份选择性原件见 [SOURCE-CACHE-20261005-RAW.zip](inputs/SOURCE-CACHE-20261005-RAW.zip)，942709 B，SHA `b2993816962d2c65f236c1f8ad4836b50e1a19f7f04a52a09c2d7a22719ff9fc`。DB／DMG／可执行文件继续保留在专属载体。
## 2026-10-05 历史投影事件扫描与正常耗时定位

隔离分支提交 `a4699cf603b193dab406fcd4d8904790f4e00a61` 将 Native 历史双投影的重复全表筛选、排序改为出生／撤回事件游标。完整六数组、两类主序及逐对象核验保留。独评 revision 2 通过；新增正常完整根／可达材料对拍 1 项、现有正常 Driver 检查 2 项通过，Native 非 test release 构建及 feature-off 检查通过。本轮没有故障注入或损坏数据测试。

同 CopyQualified149 来源、同正常输入和已批准 512 MiB 来源预算，实际收到 150／151 两条完整响应。新耗时为 24.649966042／23.370007625 秒，前一来源缓存版本为 25.398280166／23.490618250 秒，单次减少 2.95%／0.51%。完整响应、持久 oracle 和终态逐项相同，封存文件身份未变，服务 Popen 已回收、controller 关闭、socket 移除。改善幅度小，不能据此断言稳定提速或完整 9011 可通过。

另一个新副本仅在正常请求 151 期间执行被动 CPU 采样；150／151 仍完整 Received，输出与无采样窗口一致。采样中 `read_material` 占主线程包含样本约 38.5%，Verifier 约 35.9%，二者可重叠；采样不是精确阶段耗时，也不是严格资源峰值。该域计时受到采样影响，不用于速度比较。

此前来源缓存完整重放 A 域只完成 165 条 Received；第 166 条在 30.041357375 秒成为 DeliveryUnknown，持久状态虽为 Committed，仍不计作已收响应。B 域未创建，进程均回收；最终快照保存了材料，但完整 9011 终态验证因缺条未通过。上述独立窗口不拼接成完整重放。

聚合结果及逐文件 SHA 清单见 [PROJECTION-SWEEP-20261005.json](inputs/PROJECTION-SWEEP-20261005.json) 和 [原始材料 ZIP](inputs/PROJECTION-SWEEP-20261005-RAW.zip)。归档只保留完整重放失败域的终态摘要、166 记录和快照 manifest，不含封存 DB 或快照行块。全资源资格、正式双跑及整图 goal 均未完成；后续原型另行实测。
## 2026-10-05 Native codec 成功根证书复用

隔离提交 `857e76459bb9609127ee13dc9e099879b1257812` 只保存完整 codec 核验成功的外层根证书，按 Public／Template 分池。新的 Native capture 在封闭接口内逐项确认全部旧 `(digest, tag, body)` 原件仍完整相同后，才把成功根以 Value 上下文放入新的 seen 集合；当前根声明仍真实读取并核验。其他 memo、Frame、旧用量收据及完整历史结果不跨请求转移；当前业务、独立索引和 catalog 核验保留。同一 handler 的合法追加也使用这道原件绑定门。

独评发现并修正了一个正常测试夹具误用公开值 writer 的问题，修正发生在任何测试执行之前；r2 静态通过。随后新增 library 正常测试 3 项、handler 合法增长对拍 1 项及现有 Driver 正常测试 2 项全部通过，Native 非 test release 构建和 feature-off 检查通过。相同原件的小型夹具读取从 74 次降至 2 次、严格 Frame 核验从 53 次降至 2 次；合法追加的严格 Frame 核验从 67 次降至 16 次。这些是组件工作量，不是整请求耗时。

同来源和 512 MiB 预算的新 CopyQualified149→150／151 窗口完整通过，响应字节、持久 oracle、终态与前版一致，封存身份及进程／socket 清理再核通过。整请求为 23.759416459／22.815395542 秒，前版为 24.649966042／23.370007625 秒，单次减少约 3.61%／2.37%。仍只是小幅改善，不能宣称完整 9011 超时问题已解决。

S7 新增私有证书源成本为 `43 + 32 × (Public 根数 + Template 根数)`，与原件及 Counts 成本相加，仍按同一 536870912 字节上限整池准入。容器实际 capacity 单独披露；这不是 heap／RSS 上限或全资源资格证明。规范没有现成的根证书字节配方，此公式是当前私有实现的明确计费口径。

源码聚合 SHA `508de546473df055f9805055331f6ddbfe8a971a4b9baaad7b0816d152948648`，非 test 服务 SHA `9b09fc209ef614aa5d856a6ec9b334f3ef5077bbbd391e80c0863381867026ee`。结果及原始材料见 [CODEC-ROOT-CACHE-20261005.json](inputs/CODEC-ROOT-CACHE-20261005.json) 与 [原始材料 ZIP](inputs/CODEC-ROOT-CACHE-20261005-RAW.zip)。本轮没有故障注入；正式完整双跑、全资源资格和整图 goal 仍未完成。
## 2026-10-05 manifest 组装复用与共享来源预算

隔离提交 `feb63fc972097cbfe89ef0db272208662c9c3a34` 在第一次完整 manifest 组装成功后记录其实际读写原件；发布前重新逐行读取并逐字节绑定 metadata、全部根声明和原件，只在相同后复用组装结果。每次原有 put 按原次序真实重放，实际 SQL 出生许可及最终两根 Public fresh 核验保留。普通、Owned、v2 保持原路径。

初版沿用 65,536 条／4 MiB 的旧缓存限制，小型正常对拍通过，但真实 150／151 两次都在 65,536 条、3,407,328 B 正文时不再发行 workspace。该域虽完整 Received，不能算优化生效；原件和日志已保留。随后改为请求入口 checked 冻结 `S7 − legacy − old_pool`，旧池提前释放不返还本请求额度；workspace 按正文、原件证明、全部上下文及有序 put 序列计来源费用，keeper 同时遵守扣除 legacy 后的 S7 余量。没有另增一份 512 MiB。workspace 在新 keeper 保留前释放；实际容器容量与整体 heap/RSS 另记。

修正后独评通过，4 项精确正常测试、Native 非 test release 构建和 feature-off 检查通过。测试包含实际非空旧池的预算分享，并逐项对拍完整回复、全部类型化持久行、全部节点及可达原件。新正常 CopyQualified149→150／151 域实际发行并重新绑定了两份 workspace：150 的旧池为 382,405,282 B，workspace 为 42,874,531 B，合计 425,279,813 B；151 分别为 382,215,714 B 和 43,267,617 B，合计 425,483,331 B。两代 legacy 均为 0，合计都小于已批准的 536,870,912 B。分别重读 280,452／283,028 条原件并重放 306 次实际 put；这些是实际非 test 进程的诊断记录。

完整响应、持久 oracle 和终态与前版一致，封存文件 stat/SHA 及进程、controller、socket 清理均再核通过。整请求 23.112771583／22.044895291 秒，前版 23.759416459／22.815395542 秒，单次减少约 2.72%／3.38%。复用真实生效，但整请求改善仍小，不据此启动或宣称完整 9011 已可通过。

源码聚合 SHA `047825708159e6c671390807f9c7a88ffda2f5037577560ea66c2ed10cd21f08`，服务 SHA `4881f0f8845405f2e205e5a0a043c73561e2791ee1f4365cee645659eb585c69`。结果、资格日志及未发行初版见 [MANIFEST-WORKSPACE-20261005.json](inputs/MANIFEST-WORKSPACE-20261005.json) 和 [原始材料 ZIP](inputs/MANIFEST-WORKSPACE-20261005-RAW.zip)。未执行故障注入，未改期限，未合入 main。全资源资格、完整双跑及 goal 仍未完成。
## 2026-10-05 发布期间保留已重绑原件 owner

隔离提交 `5d2f19e393369d4375c1adc08580740fc65cff25` 将已实际重绑的原件 map 按值移入私有发布作用域，持续到固定 resolver 和最终两根 Public fresh 核验结束。入口自行核真实 Begin、scope 和 SQL 前沿，借用可变事务且不暴露 SQL、hook 或 callback。原件读取命中仅省 SQL 取材，F 与 codec 核验仍执行；每次真实 put 的原摘要、SELECT、原件比较及 INSERT 保留，目标后续读取保持 cold。受控效果只保留未变原件，未知效果使 base 失效；owner 在后续 Commit SQL 和新 keeper 保留前释放。

独评通过，5 项精确正常测试、Native 非 test release 构建和 feature-off 检查通过。四提交正常对拍实际复用原件读取 73,003 次、4 个 owner 全部释放，完整回复、类型化持久行、所有节点与可达原件对照一致。没有故障注入，也没有调整 wire/candidate 构造。

真实 CopyQualified149→150／151 窗口完整 Received。150 实际复用 1,273,104 次原件读取，fallback SQL 为 308,129 次，其中重绑读取为 280,452 次，put SQL 检查 1,382 次；151 对应为 1,283,045、310,686、283,028 和 1,381 次。命中次数不是唯一节点数。两次未知写入失效计数均为 0，各观测到一次 owner drop；实际保留正文 capacity 为 15,650,293／15,793,507 B，没有第二份完整材料池。

来源预算和上一版相同，旧池与 workspace 合计分别为 425,279,813／425,483,331 B，均在 536,870,912 B 内。完整响应字节、持久 oracle 和终态与前版一致；源库与新库封存 stat/SHA 再核通过，服务 Popen 回收、controller 关闭、socket 移除。整请求耗时 21.117401416／20.086794334 秒，上一版 23.112771583／22.044895291 秒，单次减少约 8.63%／8.88%。这是一组独立窗口的实测，不能外推完整 9011 或任意历史规模。

源码聚合 SHA `b2608e9d2b3b83fc50fa18bf40fe32a1f55247454350e5b1ec329a70f83ef9df`，服务 SHA `e6e63bca8cc2eee21fbb60839e86e7e783cd27834586286528268c48576b5f0e`。结果和逐文件原件见 [PUBLICATION-MATERIAL-OWNER-20261005.json](inputs/PUBLICATION-MATERIAL-OWNER-20261005.json) 与 [原始材料 ZIP](inputs/PUBLICATION-MATERIAL-OWNER-20261005-RAW.zip)。严格 heap/RSS/I/O 峰值、正式 9011 双跑及 goal 仍未完成；未合入 main，前端 #1460 独立。
## 2026-10-05 资源门溯源澄清

已逐项复核签署 C §5.6.1、§5.9.6、原 S7 资源说明和 Q 保留容量登记，见 [当前资源验收口径](RESOURCE-GATE-MAP-20261005.md)。S7 是保留来源字节；job 的 node、staged、store 各有明确量纲。RSS 的实际记录义务保留，但“整个 S 服务所有 heap 分配必须逐项计量”不能由既有诊断的未测声明推导为新增正常验收前置。此前各段 `whole_resources_qualified=false` 和严格峰值未测的历史事实不修改；它们不自动增加合同门。后续按原声明的 S／Q／job／reader 门逐项验收，生产启用和完整9011／整图完成边界保持。

## 2026-10-05 Native 原件扫描分配复用

隔离提交 `a7bdcc4c7fa14595df9a313e3a70556ded4315ce` 将三个 Native 扫描入口的有效 BLOB digest 直接复制到栈上32字节，并在请求内复用完整原件扫描的去重集合容量。每次扫描先清空集合，仍逐行读取到末尾，保留类型、重复、遗漏、正文逐字比较和错误传播。两个 fresh 捕获仍各自持有正文；集合在请求结束释放，不进入跨请求 Keeper。

固定补丁静态独评通过，格式化后与 staged 格式化结果逐字一致。三项新增正常测试及一项 Driver 追加／重复请求／重启测试通过。1845行正常材料的三个入口共5535次 digest 借用、0次 Vec 解码；首扫集合容量增长11次，后续三次完整扫描增长0。它们是执行路径和容量计数，不是malloc或全heap证明。Native 非test服务构建及关闭 Native 的检查通过；首次关闭特性检查因命令漏必需的 `s_session` 特性而退出101，错误记录保留，修正命令后退出0，源码未变。

真实 CopyQualified149→150／151 两请求均完整 Received，耗时20.694808083／19.839043791秒，前版21.117401416／20.086794334秒，单次下降2.00%／1.23%。完整 request/reply wire、持久 oracle、终态逐项相同；原件和新库封存身份、源码及产物绑定再核通过，服务Popen回收、controller关闭、socket移除。启动至成功Ready实测24.682828459秒。独立事实复核通过。非test进程未记录本次scan scratch计数，不以单测计数冒充实窗计数，也不以这一组结果声称稳定提速。

两请求旧池加workspace来源费用仍为425279813／425483331 B，低于同一536870912 B S7。源码聚合SHA `c8ead0d4167da9f6af7629cf916371b950e6f5d41f2a8611f012079a11628aa6`；服务SHA `98e4e61d707f98c0f25a80934f1bf654cc9b5dd4db754ef3edaf768fd5b16726`。结果、绑定、补丁和运行原件见 [NATIVE-SCAN-20261005.json](inputs/NATIVE-SCAN-20261005.json) 与 [原件包](inputs/NATIVE-SCAN-20261005-RAW.zip)。准备文件的pending是运行前快照，未覆盖它们。代码提交与Rust树登记在scan产物ROOT-RESULT，窗口ROOT-RESULT仅登记运行事实。

未新增或执行故障注入、坏数据测试，未恢复取消的birth-only／candidate／wire重构。此增量没有消除跨请求重复核旧历史的增长成本，未重新启动完整9011；既有165 Received、166 DeliveryUnknown失败域保持。完整双跑、其余验收及整图goal仍未完成，前端#1460独立，未合入main或push。

## 2026-10-05 SQLite pager 缺页定位与正常对照

隔离提交 `ecd8371e8fe71f33d2af040d32c2aa3b1bacdfbe` 增加仅 Native TestOnly、显式诊断开启时生效的 pager 计数，以及连接会话内的可选缓存实验输入。默认不改缓存量。每个原 SQL、F、业务核验和事务边界保持；WAL、synchronous=FULL、macOS fullfsync、auto checkpoint均保持。独立静态审查通过，非test Native构建和feature-off检查通过；没有新增或执行故障测试。

同一冻结服务在两个新 CopyQualified149 副本分别运行默认与256 MiB页缓存建议值，各完整收到150／151。默认连接实际 `cache_size=-2000`，耗时20.694023／19.834563417秒；实验显式 `S_NATIVE_SQLITE_CACHE_KIB=262144`，回读 `cache_size=-262144`，耗时17.691335209／17.030979916秒，单次下降14.51%／14.13%。两域的完整请求、回复、持久oracle和终态一致，源封条与绑定再核通过，服务、controller及socket全部回收；合并独立事实复核通过。独立窗口不拼接为正式重放。

两版每请求的pager命中加缺页总数完全相同，分别为18910523／15809238。缺页从5736413／4917505降为500929／499846，减少91.27%／89.84%；事务中途溢写从30756／53012页降为0／0页。实际脏页写出从31208／53462降为24307／35909。统计是本连接的SQLite累计端点差，不是SSD实际读写量；计数没有复位，i32端点均非负单调，不外推为任意长运行无溢出。

SQLite记录的pager当前近似用量在采样点最多为默认2116736 B、实验276057728 B。256 MiB是建议页缓存量，元数据及整个进程内存另算。进程截至最后采样的内核RSS高水位分别为2270969856／2588622848 B，后者多317652992 B；这不是Rust heap或该请求独立峰值。SQLite这些计数的highwater字段恒0，未被当成pager峰值。`cache_spill`查询的有效阈值从483变为63429，仍启用溢写，补丁没有写这个PRAGMA。S7仍为536870912 B的保留审计来源额度，两请求旧池加workspace仍为425279813／425483331 B；启动与请求仍为30秒。

源码聚合SHA为 `86d44f8c22423ea3e7276d6a706fd84762c179ee08188135b995e1f73abb1fe1`，服务SHA为 `0e9cef4460f284d8fa8a60fae155bef11843714b459c239a5c9aabed2ff07bdf`，两域使用完全相同的非test产物。实验值、默认实测依据、脚本、绑定、计数、RSS和清理证据见 [SQLITE-PAGER-20261005.json](inputs/SQLITE-PAGER-20261005.json) 及 [原件包](inputs/SQLITE-PAGER-20261005-RAW.zip)。本结论仅是正常两请求的物理读取成本改善；逻辑历史增长未消除，完整9011、生产默认配置及整图完成均未获证明。未合入main或push。

## 2026-10-05 Counts 来源闭包体积测量

隔离提交 `801f3904c6c7f73d06f2b5b2a09288d139f81e43` 只增加一项 Native、Unix、ignored 正常诊断，原文件前缀逐字不变，未修改缓存资格或生产路径。首次编译因报告JSON宏递归深度失败；拆为三个Map后全部50字段与值表达式保持，R4编译和增量独评通过，原失败记录保留。

从已封存的256 MiB pager窗口151终态，仅按字节复制主库、WAL、SHM到新专属owner；WAL保留，未使用immutable模式。固定测试产物以READ_ONLY、query_only及只读事务完整审核151个cut，随后使用顶层字段导航和三份全新Verifier，统计objects、witnesses、observations闭包及去重union。未播种旧memo，未把共享展开的相邻差集冒充独立proof。精确一项正常测试通过，进程观察13.245708秒，其中完整审核12.050376秒、额外体积测量0.706780秒；这是离线测量，不能当作服务请求耗时。

| 材料 | 唯一节点 | 正文字节与97B节点证书费用之和 |
|---|---:|---:|
| objects全闭包 | 547221 | 85888790 B |
| witnesses全闭包 | 27340 | 4024729 B |
| observations全闭包 | 4412 | 656833 B |
| 顶层导航 | 5787 | 884091 B |
| 上述去重union | 573916 | 89950830 B |

完整capture为2375001节点，正文157406070 B、节点证书230375097 B、SQL绑定965410 B、Key/Counts44998 B、codec根证书2159915 B、frontier8 B，合计390951498 B。候选Counts材料以去重union加原SQL绑定、Key/Counts与frontier，合计90961246 B；相对含codec完整费用减少76.7334%，两边均去掉codec根证书后减少76.6041%。新持久依赖描述、完整proof接纳数仍为null；没有把原料低于512 MiB直接写成已可准入或所有9011可装下。

独立事实复核通过。封存来源前后完整stat/SHA不变，新owner的主库/WAL字节不变，本次SHM也未改变；测试进程由同一Popen回收且PID消失。没有启动服务、发送请求或故障注入。源码聚合SHA为 `b699ab1f9f8acc78fee5a51ed46cede98c9fdf47b22cee63ad9c67f259b9ed98`，冻结test产物SHA为 `dc7ccc01d322cba907b00aa4fa1cdde00c045c74c7a656b03d459556e0821c07`。输入、证书、精确test列表、报告、费用重算和清理证据见 [COUNTS-FOOTPRINT-20261005.json](inputs/COUNTS-FOOTPRINT-20261005.json) 与 [原件包](inputs/COUNTS-FOOTPRINT-20261005-RAW.zip)。稀疏池尚未实装，正式完整双跑和整图仍未完成。

## 2026-10-05 Counts 完整证明组稀疏保留

隔离提交 `92eabc10f2571017bcf31ecd95c7fad4d0054954` 将 Native Keeper 改为按完整Counts证明组保留。单个全新Public Verifier从新到旧取得顶层导航及objects、witnesses、observations完整闭包；整组核验和全部费用通过才接纳。超额组新增材料回滚后立即停止并丢弃Verifier，此前完整组保留；首组即超额则不留池。正文Vec按值移入稀疏索引，排除正文释放。私有证书保存完整state及三根声明和版本，新增费实记；跨请求codec根清空，不能拿裁剪原件为旧全集根背书。全部当前SQL扫描、行错误、旧保留tuple逐字比对、原SQL Binding/Key和独立历史核不变。

静态独评r3通过。预算组正例在r2通过；r3仅改变新增owner测试的合法价格序列，生产字节与r2相同，另6项精确正常测试通过，覆盖完整输出、正文原字节/指针、合法无关原件追加、请求/重复请求/重启与共享S7。r2 owner测试先在Fixture.publish的既有来源组覆盖门失败，尚未进入新保留逻辑；同价失败原件保留，不声称同价一般域通过。旧无关原件变更负例仍采用全池策略的失效计数断言，未修改或执行，不能称完整旧故障回归通过。非test Native构建与feature-off检查通过。

同256 MiB pager建议值和512 MiB S7，新CopyQualified149→150／151窗口完整Received，request/reply wire、持久oracle及终态与基线一致。两次入口old_pool均为88768569 B，基线为382405282／382215714 B；workspace来源费用仍为42874531／43267617 B，共享合计降至131643100／132036186 B，入口余量为448102343 B，未增加额度。完整原件重绑和实际发布put读数保持，旧source与新pool指针别名、每组新增证书费和codec根清空没有在本实窗日志单列，按正常测试与静态证明的范围分别报告。

整请求耗时20.037953583／18.900813833秒，基线17.691335209／17.030979916秒，本次慢13.26%／10.98%，均仍在30秒内。它解决保留材料与整池超额的策略问题，但不是全面提速结论；新增闭包遍历和清除跨请求codec复用的代价没有隐去。pager实际配置及命中/缺页/写页读数与基线相同。进程截至末次采样的内核RSS高水位为2421145600 B，基线2588622848 B；非Rust heap或单请求独立峰值。

源码聚合SHA `63ae89d62f67c3f27e065495617bca753c55bd80f08632a9eb20b45d316e4d11`；服务SHA `4154968619d9abb5f3039c0e6ff531b7dadc588f1145caa4397a334191fc8275`。旧dual构建缓存库在切feature时被服务构建覆盖，旧build时SHA不改写；冻结test/service均经otool确认没有newchan动态库依赖，固定binary/源码/执行日志仍各自绑定。源文件封条、同Popen回收、controller关闭、socket移除均通过。摘要及原始证据见 [COUNTS-SUBSET-20261005.json](inputs/COUNTS-SUBSET-20261005.json) 与 [原件包](inputs/COUNTS-SUBSET-20261005-RAW.zip)。归档含 1380 份材料，只保存一份 643 项冻结源码完整快照；ZIP 为 8892093 B，SHA `4fbda57f26f6f30ee31c610c210c272a149d6e0150ce413ba12976f52a7386bd`。逐文件 bytes/SHA 清单与 ZIP 全项回读校验通过，r2 首次失败、修正补丁及旧构建绑定均保留；DB、编译可执行文件和 pycache 不入包。未运行故障注入，未合入main、push或改GOAL；完整9011、其它验收及整图goal仍未完成。

## 2026-10-05 Counts 子集有限 codec 成功根

隔离提交 `a81aec3b11b0acc91f036c87978181b9bf997ec3` 仅恢复已接纳 Counts 完整组的 Public/Value 成功根，按唯一根去重计入整组预算，清理其余缓存后私有保存。下一请求仍逐字绑定真实原件，再播种成功根；Template、generated_state 和旧全集根不获资格。独评 r1、r2 及 r3 类型修正通过，正常测试共 10 项，其中 r1 的 lib 3 项／bin 3 项沿未变代码复用，r3 Driver 4 项实跑；服务构建和 feature-off 检查通过。r1 混合日志绑定工具错误、旧零根断言失败及 r2 测试类型编译失败均保留，未新运行故障测试。

新 CopyQualified149→150／151 窗口完整 request／reply／持久 oracle 与终态同前版，耗时 17.839879584／16.758944375 秒，单次减少 10.97%／11.33%。old_pool 均为 88782841 B，来源合计 131657372／132050458 B，同享 536870912 B S7；pager 256 MiB 仍为私有建议值。RSS 是截至采样的进程生命周期高水位 2491465728 B。源封条、原件和副本 stat/SHA、同 Popen／controller／socket 回收经根事实复核通过。实窗没有单列 codec 根身份、数量或逐组 metadata 费；精确根资格属于单测与代码证据。跨请求历史读取成本仍存在，单次窗口不证明稳定收益、完整 9011 或全资源资格，整图未完成，产品前端仍归独立工作线。

源码聚合 SHA `f5e596ab408531dac56de7142b323b4ee5ac1d24bcf8a9348d0ea218e682c194`，服务 SHA `0da858872deb645a6a3c305803de429e4c5f73e94e1ba32e8c234ccfa771eac1`。摘要及逐文件清单见 [COUNTS-SUBSET-CODEC-20261005.json](inputs/COUNTS-SUBSET-CODEC-20261005.json)，[原件包](inputs/COUNTS-SUBSET-CODEC-20261005-RAW.zip) 含 1338 项、9099437 B，SHA `a2cd01f53324b237dac8d9b4d922829796f1d5714317a0e87ae17895058d1e00`。只收一份 643 项 service r3 完整冻结源码，旧 manifest、补丁与旧 Driver 足以重构；失败日志、绑定及外部 s.log 字节保留。逐文件 bytes/SHA 与 ZIP 全项回读通过，DB、编译可执行文件和 pycache 不入包；未打开 SQLite 或重跑 Cargo／测试，未 push／merge。
