# RF-02 / R2 独立协议关项复核

> #1339 / #1323；作者协议 SHA-256：`11f5704589cc1b646b2aa5b9f29fff808fa3af9393e1a31f98e8febc48497a3c`。
> **四项初评缺陷全部关项；读取范围内残留 0H / 0M / 0L。** 仅为架构合同层结论，可继续整图候选采纳评估，不代表协议已实施或运行/场所验收通过。

S自身接纳、计算、持久发布和观察不再依赖E/X可用、经济ack或JoinCut；没有把整个经济安全日志改名为结构存储。新稿保留完整责任与风险Capture/Release同序，并补齐S当前核验、X一次调用消费。

## 四项关项

| 发现 | 新稿证据 | 关项依据 |
|---|---|---|
| RF02-M01 | §4:58；§5:76,84–86；§7:109–123 | 完整适用集合由获准权威与实际依赖确定，不能靠少写字段缩小；逐项列失效源、阶段、证据和能力。E-only有具名权威及不可撤责任覆盖；S/X应检条件不得省略，未分类/未覆盖/不可核均Unsupported。 |
| RF02-M02 | §3:44；§5:80；§9:141–150 | busy只入本地Undecided队列；每次Commit后、下一Begin前有限非零服务，每个已就绪请求有限稳定点内实核。没有结构输入也服务，不无限drain，不等E/RPC/未来业务。实际核验无效则终态，原busy拒绝循环不再合法。 |
| RF02-M03 | §5:75,84–88 | X同ID唯一持久Ready→MayHaveCalled原子迁移；只有fresh赢家可尝试一次网络调用。重复S收据只查原状态，换plan/证书/epoch不能新建同ID。提交后崩溃或未知不重试，实际发送路径必须fence旧writer。 |
| RF02-L01 | §5:80；§6:100–105 | S权威Undecided/终态、X权威状态与NoRecord、观察Unknown/Unavailable分轴，带last_known/domain/query_frontier。无记录、无终态和未知均不能推出未发送或释放责任。 |

M02排队是在未检查前等有限本地服务，不能查到条件无效后等待未来行情。每个就绪请求有限稳定点内须实核，无效即终态；结构也继续推进，原busy拒绝循环不再合法。M03两条重复路径争同一持久CAS，仅fresh赢家能调用，原S收据不再重复授物理调用权。

## 精确交付边界

E-Release只用于已完成业务决定、量价/投影固定、身份不可复用、交给唯一X并保留全部最大责任的命令；随后只有有限本地服务、一次当前核验及一次调用权消费。其与普通可改计划/等未来机会的准备有实质区别。本轮未发现把该边界一律判为预留更名的具体反例；它也与R1的共同单点核验不同，代价须随候选采纳明示。

`E-Release → E新风险 → S-Execute`仍可能是合法已交付窗口，但必须逐项按真实检查阶段判断：E-only要有具名权威与责任覆盖，必须在S或实际调用边界成立的条件仍须可核，证明不了就Unsupported。**未得出S-Execute时两域共同最新，也未承诺经济新风险绝对阻止此前精确命令首次调用。**

四事件24排列只有两个抽象守卫，足以反证共同当前；它没有网络、完整阶段、队列或X原子消费，不能证明或推翻整个交付合同。本复核未重跑或扩充模型，不把合同论证写成运行证明。

## 19 条反例映射

“闭合”指当前合同排除原问题或给出明确能力前件，全部运行验证未执行。

| CE | 定位 | 处置 |
|---|---|---|
| CE-L01 | §1/2 | E不再是正式结构接纳、计算、提交与消费前置。 |
| CE-L02 | §3:44；§5:80；§9:141–150 | 共轮撤销；稳定点服务同时保护结构和已就绪条件请求。 |
| CE-L03 | §8:127–133 | 结构切面/流/关系闭包独立，JoinCut与经济反向索引不挡结构。 |
| CE-L04 | §1:23–25；§8:135 | 禁止经济ack、共享outbox、保留回收反压；资源与归档隔离仍须实施。 |
| CE-S01 | §3–5；§7:115–123 | 精确不可撤交付与S当前核验分别排序，完整阶段条件补齐；不声称两次读取共同最新。 |
| CE-S02 | §2:30–33；§5:77–80；§6:94–99 | S在first_known前本地封门；分区E镜像不能取得S许可。 |
| CE-S03 | §4:58–63；§5:75–88 | 迟到回包绑定ID/依赖/epoch；终态不可复活，物理调用另作一次消费。 |
| CE-S04 | §3:41–46；§4:56–63；§5:80–88 | 完整冻结、真实交付、唯一X、一次当前核验及单次调用权是实质在途合同，普通准备没有例外。 |
| CE-S05 | §4:58；§5:76,84–88 | 旧epoch受实际发送栅栏约束，同epoch重复也受同ID持久CAS约束。 |
| CE-S06 | §4:57,67；CR-07；旧P4 | Capture/Pending与新Release同序，先封闸后解释；已交付仍须检其适用最终条件。 |
| CE-S07 | §2:29,35；§7:109–123 | S/E独立接纳；证书按来源、事件修订、替代和实际依赖集合核覆盖。 |
| CE-S08 | §4:67；CR-07；旧P4:217–220 | 精确token/前缀/证据版本Resolve继承，旧证书不清后续待核。 |
| CE-S09 | §4:57–58；§5:76,84–86；§7:115–123 | 完整阶段强制列入；过期、源连续性和时钟不可核在应检点拒绝，未分类Unsupported。 |
| CE-S10 | §1:16–25 | S小型条件结果不接管经济风险日志；职责故障隔离仍须实际注入证明。 |
| CE-S11 | §1:25；§5:73,80；§9:143–148 | S条件判定无远端RPC/经济锁，排队只等有限本地服务，不等E决议。 |
| CE-S12 | §5:84–88；§6:100–105；CR-06/10；旧P2/P6/P7 | 未知保全潜量、调用权不复用，按对应终态和账收据释放，P7首次量不重测。 |
| CE-S13 | §4:63–65；§5:80–86；§9:152 | 恢复查原身份，不补发历史准备；新需要按当前条件新决定，旧责任不断账。 |
| CE-RF01 | §1:12,20；§9:141–152；CR-11；旧RF01 | A界覆盖和不等A Applied保留；B新Committed及持续合规请求稳定服务机会均有合同。 |
| CE-O01 | §8:127–137 | 正式结构原子撤回、固定切面、Gap、代际和AsKnown向量保留，经济恢复不回填获知史。 |

## 14 条验收义务

14项已有对应合同，实际验收均为`not-run`。

| RA | 定位 | 必须保留的结果 |
|---|---|---|
| RA-01 | §1/2/4/8 | E失败无新Release；S正式结构及消费继续。 |
| RA-02 | §2:29–33 | S中途修订独立完成、去重，不等经济确认。 |
| RA-03 | §1/2/5/8 | E/S分区不阻结构；E缓存不能绕S当前许可。 |
| RA-04 | §2:30–33；§3/5/6 | first_known前先封S门，失效引用最终拒绝，已交付/执行保责。 |
| RA-05 | §3/4/5/7 | 两个不可撤边界分别排序，强制阶段义务补齐，不称联合当前。 |
| RA-06 | §4:58–63；§5:75–88；§8:133,137 | 传播延迟不阻结构，身份/获知史不改，重复收据不重复调用。 |
| RA-07 | §4:63–65；§5:75–88；§6:100–105 | 不可撤边界前后和调用CAS明确，崩溃保责，缺证据不推未发。 |
| RA-08 | §4:58；§5:76,84–88 | 同ID跨epoch不重置，实际网络旧writer栅栏为部署前件。 |
| RA-09 | §8:127–135 | 结构完整独立，经济缺项、旧cut、consumed_structure明示。 |
| RA-10 | §8:129,137 | 固定分页、流快照相等、Gap、旧代际及当时可知双域前沿。 |
| RA-11 | §5:80–86；§6:100–105；§9:152 | 先重建责任/Pending/连续性，再新经营；旧候选不补发，L不重测。 |
| RA-12 | §1:12,20；§9:150–152；旧RF01 | 健康B新Committed不等A账恢复/Applied，A最大责任保留。 |
| RA-13 | §3:44；§5:80；§9:141–150 | 有限非零稳定服务和每请求有限点实核排除busy循环；不无限drain压停S。 |
| RA-14 | §2:29；§7:109–123 | E不再是S唯一市场入口；估值证书缺覆盖交易拒绝而S仍发布。 |

## 20 条替代与 5 项 OP

20CR逐条已核且MD/JSON一致，逐条记录见配套JSON。CR-01–13对照本代理已读R1协议段；CR-14–20只核本轮替代声明，未重审整份旧状态、分类、观察、方案或旧入口材料。

| OP | 设计层处置 | 后续义务 |
|---|---|---|
| OP-01 | 精确冻结委托、一次本地核验和一次物理消费构成实质交付合同；未发现只能判作普通预留更名的具体反例。它不是两域联合当前证明，也不代替用户采纳该边界。 | I-01,I-04,I-05 |
| OP-02 | 适用集合不能缩小、阶段来自权威、E-only有依据、遗漏/不可核Unsupported；实际条件实例未验证。 | I-04,I-05 |
| OP-03 | namespace/epoch、事件及修订、规范集合、替代关系和必要依赖均明列，S核自身记录覆盖而不解释经济。 | I-04 |
| OP-04 | 同ID持久fresh CAS关闭双调用路径，权威与观察知识分轴；真实网络栅栏/场所能力未验。 | I-05 |
| OP-05 | S职责依赖独立，稳定点有限服务排除busy循环，不等E或未来条件；实际隔离与活性仍须验收。 | I-02,I-06 |

## 后续实施验收

这些义务未因转交SPEC/adapter而变成已通过，也不自动构成新的资金政策选择。

- **I-01 真实精确交付边界**：证明真实生产入口在Release前已冻结全部业务/量价/投影，唯一X收的是该不可复用命令，全部最大责任持久保留；不能把未来候选当在途。采纳时保留既有精确交付不保证被后来的E风险撤回的代价。
- **I-02 结构故障隔离与正式观察**：分别停止/分区E日志、解析器、X及经济订阅者，S依赖健康时正式结构、修订、撤回、分页、续接、AsKnown继续。验证存储、索引、队列、ack、归档无经济反压，所有撤回生产者先走本地屏障。
- **I-03 E风险闭包与责任恢复**：实证Capture/Pending/Release同序、全部失效生产者、精确Resolve和源连续性恢复。P7逐步核F加全部独立潜量及终态累计完整性；A坏账但界可核时B实际新Committed。
- **I-04 完整条件实例及证书**：按获准政策/场所逐项实例化清单，E-only有具名语义/责任依据，必须S/X时成立者有当前核验能力。测试遗漏、错误阶段、跨源跳号、迟到/非单调更正、过期、时钟不可核。证明不了的远端读或调用时条件保持Unsupported。
- **I-05 一次调用与真实发送栅栏**：测试同ID并发/重复收据、CAS前后崩溃、旧epoch复活、凭据绕路与网络不明。实际X阶段条件必须和调用占有由受控路径绑定，不能以S已核代替。场所幂等/回查/终态能力逐合同验收，未知不自动重发。
- **I-06 持续输入下双向推进**：给出有限非零额度和公平选择；持续S提交且持续合规请求有限稳定点内实核。单重坏账、慢消费和原busy相位攻击下都观察进展；无效请求只一次终态，不靠停结构/停交易/全员轮或默认资金优先。

## 读取签名与边界

20个manifest SHA全部匹配，协议10节和20CR的MD/JSON逐项一致。INITIAL及FIRST-PASS未变；所签manifest为独评完成前冻结清单，不能倒称本代理读过后来改变的字节。

行政收尾：PROTOCOL-R2.json的`execution_status.independent_final_review`仍残留`not-completed-usage-limit`。根应在整合本评审后更新为完成状态及评审引用，留旧新SHA与精确元数据diff，正文10节/协议MD不变。该残留不是协议缺陷，当前并无额度阻塞。

- `/tmp/newchanlun-1339-architecture-r2-20260909/DELIVERY.json`
  - SHA-256：`b2030cd4f038376541504de22ced99d542a30586b124b5eb7f59bbaf1c74fbb2`
  - 读取范围：全文及本次复核前20件manifest；与根提供SHA核对。
- `/tmp/newchanlun-1339-architecture-r2-20260909/NEXT-REVIEW.md`
  - SHA-256：`68e77ead5397a711c1fc358f7a39ede7c67c65c4ff06c092521115c25c61ee65`
  - 读取范围：全文当前关项范围。
- `/tmp/newchanlun-1339-architecture-r2-20260909/USER-DECISION.md`
  - SHA-256：`bde32ba1369879cfec4343d0364d4cddc160238ea5b6a03c7828fe36ba94d0c9`
  - 读取范围：全文，前轮同SHA。
- `/tmp/newchanlun-1339-architecture-r2-20260909/USER-DECISION.json`
  - SHA-256：`68105375cfa35628881ddd2db271a7606b37985547921758ddd1e820bfd33a3d`
  - 读取范围：全文，前轮同SHA。
- `/tmp/newchanlun-1339-architecture-r2-20260909/ACCEPTANCE-RF02.md`
  - SHA-256：`d1fcd8d1b36cd90910a561ee60dab1bf820b64ac49f562b3d16cf43f69ccf309`
  - 读取范围：全文14RA，前轮同SHA，逐项新稿映射。
- `/tmp/newchanlun-1339-architecture-r2-20260909/ACCEPTANCE-RF02.json`
  - SHA-256：`57722029f83edc7f1349f4df5a72a33d4e3fe9a56777f0f625fceea197d00b40`
  - 读取范围：全文14RA及当前SHA/数量。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/PROTOCOL-R2.md`
  - SHA-256：`11f5704589cc1b646b2aa5b9f29fff808fa3af9393e1a31f98e8febc48497a3c`
  - 读取范围：全文1–158行，10节。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/PROTOCOL-R2.json`
  - SHA-256：`97173d5cedfca3fc454841d0aefce51eb0fe98d8244db3b9ed56322b53d4966a`
  - 读取范围：全部元数据；10节正文逐节程序核MD一致，语义据已全文读MD。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/CONTRACT-REPLACEMENTS.md`
  - SHA-256：`a981bf7af3aa9500f33a7f0a8b95992deb805562bf9f9ad558e1b3067f3db0f3`
  - 读取范围：全文1–32行、20CR。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/CONTRACT-REPLACEMENTS.json`
  - SHA-256：`53447b7197ccc2c55348bbbdc82c4e00ada00b135c9cf746e8a6a03d2b70c95a`
  - 读取范围：20CR逐条程序核MD一致。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/OPEN-PROTOCOL-QUESTIONS.md`
  - SHA-256：`6213c37f1344c51f5d0bf7dbda78b75838a2028ea3c474dfca511541e1264adc`
  - 读取范围：全文5OP。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/AUTHOR-REPAIRS.md`
  - SHA-256：`fb6d5a5e512c6894358d69fc0a33f294ce5599684a232dbb3b448664d7dd3eb0`
  - 读取范围：全文4项修订。
- `/tmp/newchanlun-1339-architecture-r2-20260909/review/INITIAL-COUNTEREXAMPLES.md`
  - SHA-256：`ecc62a15f58b527ba78e63f9b3db18f32ee0940262cd1e0227d77c169c7b1bb0`
  - 读取范围：本代理原完整初始19CE，当前逐项再映射且未改。
- `/tmp/newchanlun-1339-architecture-r2-20260909/review/INITIAL-COUNTEREXAMPLES.json`
  - SHA-256：`366a6182601226e16dce67125c894437a2c1a5e6a3654a75ad69d500a27a9657`
  - 读取范围：完整19CE/7初始不变量，未改。
- `/tmp/newchanlun-1339-architecture-r2-20260909/review/REVIEW-RF02-R2-FIRST-PASS.md`
  - SHA-256：`b0b73a11b7c67a3f8bd6ca348e9b5cd1e3817f57f2111505b4ddb63dd552fc73`
  - 读取范围：本代理完整冻结初评，四项缺陷/映射；未改。
- `/tmp/newchanlun-1339-architecture-r2-20260909/review/REVIEW-RF02-R2-FIRST-PASS.json`
  - SHA-256：`8968781b4e8be34558552870fe6fd886c69ee353644d4e45aba391f915b7b537`
  - 读取范围：全文结构化初评作为关项基线；未改。
- `/tmp/newchanlun-1339-architecture-r2-20260909/evidence/ORDERING-COUNTEREXAMPLE.md`
  - SHA-256：`deaf81c7c577112fe9964157ef1eec8cd03694df08e1676ced042904f66d7494`
  - 读取范围：全文原四事件反例及修订射程限定。
- `/tmp/newchanlun-1339-architecture-r2-20260909/evidence/ORDERING-COUNTEREXAMPLE.json`
  - SHA-256：`3e47298e8572302d7b6f8e28702ce125f14eaf6769ba3f99e65d1049b97d551f`
  - 读取范围：原24排列此前全文读取；当前守卫/摘要/射程及SHA重读，未重跑模型。
- `/tmp/newchanlun-1339-architecture-20260908/author/INDEPENDENT-DESIGN.md`
  - SHA-256：`f6054b52f86ba80fafd6386d1ac6ede341c6c6a17cb9efbed5d2dad42571824d`
  - 读取范围：初始直接读1–102、165–321、378–400；本次据同SHA已读P1/P4/P6/P7/RF01/O1–O3核继承，未重审分类轴。

DELIVERY.md、BASELINE-BINDINGS.json、R1-COUPLING-LOCATORS.json仅核摘要，未用其语义支持评审。未运行项目、Lean、UI、回放、paper/live、完整模型或场所；未修改作者、根交付、旧包、仓库、GitHub或服务。架构采纳、SPEC、实施与main仍按已有明确门推进。

## 元数据修正补签

仅核交付元数据，原正文和协议结论不变。独立比较旧快照与当前JSON，确认只修改`/status`、`/authorship`、`/execution_status/independent_final_review`三叶；10节正文、document绑定及协议MD逐字不变。错误额度状态已清除，新状态明确“架构合同层独评完成、未采纳、无实施或运行验证”。

- 旧JSON SHA-256：`97173d5cedfca3fc454841d0aefce51eb0fe98d8244db3b9ed56322b53d4966a`
- 新JSON SHA-256：`46c7c0fbd53926119d45a8682ca29e8001c445835fbd00895add778d8d1e70e6`
- 原协议MD SHA-256：`11f5704589cc1b646b2aa5b9f29fff808fa3af9393e1a31f98e8febc48497a3c`
- 精确差分证据：`/tmp/newchanlun-1339-architecture-r2-20260909/evidence/REVIEW-METADATA-REPAIR.json`，SHA-256：`ce79078aba3109ce9ff0062d367edff4dd21e599c1a8fa7960fef3e59a139b79`

原读取签名和20件manifest射程保留历史值；本补签单独绑定新JSON，不倒改原签署，不重做或冒称新增协议/场所验证。行政收尾项已关闭。
