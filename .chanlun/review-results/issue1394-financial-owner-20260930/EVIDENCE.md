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

两份选择性原件ZIP保留31个客户端原件与72个浏览器原件；新Chrome profile、pycache及镜像保留专属scratch，不入仓。整合前ROOT审计中的PENDING是当时状态；最终固定9f923两轴PASS、精确代码整合3b53744与实际运行补审见上述最终报告。全部55份归档inputs的bytes/SHA已由根逐一核验。

## 仍未完成

本片仍需S3 writer具名资源与七字段实际执法、controller、完整同源查询及累计历史/全资源和9011双跑。现有验收客户端已严格支持finance/2并完成上述实际浏览器路径，但经济HTTP没有独立证据闭包节点入口；本次不独立证明FinancialBasis节点闭包/七字段正文，亦不重算没有前根的顶层经济root。现有页面与脚本归后台验收，产品新前端仍归#1460。条件FIFO/稳定点归已裁定的#1416，依#1407与#1361原生前置推进，不反挂成本片开工门或新政策待裁。其它TB与六Destination仍未验收。未跑本票远端CI或本票DevSkim，不把#1392扫描归成本票扫描。没有改变1349未定公用资金/cap政策与真实场所权限。
