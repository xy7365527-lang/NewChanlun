# RF-02 / R2 独立协议初评

> #1339 / #1323；冻结作者稿 SHA-256：`8570fc96b47ec90f3a87ac829b78322f1aad9384d95694aafdb53fd5f712e1c5`。
> 结论：**要求定点修文，0H / 3M / 1L。** 本评审是实际读取文本及对抗时序复核，不是运行、场所或架构采纳证明。

RF-02 所否决的公共 BeginRevision、结构/经营共轮和全局观察阻塞，已被本稿的 S 独立接纳、计算、持久提交、结构流和历史合同解除。E-Release 已有冻结精确命令、唯一执行者、紧贴真实交付、不得等待未来业务条件等实质限制，不能只凭四事件排列断言它是普通 Reserved 改名。它也不能据此声称 S-Execute 时经济条件仍共同最新。

需要补四处：经济条件检查阶段的完整分类、避免每次 busy 拒绝的公平条件执行机会、X 对一次物理调用的持久消费，以及权威未决与观察未知分层。以下修文均在当前协议范围内；没有要求预先选数据库或发明资金/场所默认。

## 四项缺陷

### RF02-M01 · 经济前件的检查阶段尚未穷尽，未声明条件可落入已交付例外
定位：PROTOCOL-R2.md 行 54, 58, 71, 77, 105, 109；§4 E-Release条件、§5 S-Execute条件、§7最后段。

对抗时序：
1. 命令依赖一个可失效的权限/源连续性/证据有效条件；E-Release时它成立。
2. 该条件没有被明确分类为执行时条件；E-Release后条件失效。
3. S引用和证书签名仍有效；§7仅说“若声明为命令的执行时条件”才必须最终核验，故所列规则不能排除这项遗漏。

影响：尚不能从完整约束在Release时成立推出所有必须执行时成立的条件也被检查。此项是条件闭包未封闭，不是已证明所有Release后变化都必须阻止既有在途。

修订要求：为每一项适用前件记录条件ID、实际依赖/失效源、检查阶段、证据覆盖和核验方式；区分已被不可撤交付责任合同覆盖的Release时条件与必须最终可核的条件。适用但未分类、无覆盖或无核验能力一律Unsupported/不交付或拒绝，删除“未声明就省略”的可能。不得自行给有效期/资金/场所政策默认。

后续验收：对每一前件分别在Release前、Release后而Execute前、Execute后失效注入，逐项依据分类断言结果；任何新增未分类前件都不能通过。

### RF02-M02 · 公平执行有限任务仍允许所有命令只在结构busy时被拒绝
定位：PROTOCOL-R2.md 行 30, 33, 44, 80, 127, 129；§2本地修订门、§3一次当前核验、§5 busy最终Denied、§9公平调度。

对抗时序：
1. 存在持续合规的B经营需要；已引用的结构事实从未撤回，所有必要经济证据可核。
2. 每轮安排S.BeginRevision→处理一个已到达的精确命令c_n并因busy最终Denied→S.CommitRevision。
3. E及时得到Denied、释放该命令的无外效份额，并按仍当前的经营需要形成c_(n+1)；随后重复上述安排。
4. 所有线程都获公平调度、每个任务有限终止、结构持续提交；仍然永远没有ExecuteCommitted。

影响：最窄的“B产生本地新Committed”例子可能仍发生，因此不把本反例夸大成该记录绝不出现；它打破RA-13及§9承诺的持续合规请求取得真实条件执行机会，不能用交易永久忙拒绝来满足RF-02。

修订要求：给出S本地稳定边界对已就绪条件请求的有限公平处理约束，使上述循环不可达。不得等待E/RPC、资金政策或未来结构条件，也不得无限清空经济队列而停结构；物理先后仍服从已批执行排序，调度不成为资金优先政策。具体新调度尚未读取，初评不预签。

后续验收：保持结构输入不断、引用持续有效、经济前件可核，强制此前busy相位对齐；必须观察至少一个合规请求实际ExecuteCommitted且S继续提交。另以真实依据失效请求验证不可等待未来复活。

### RF02-M03 · 重复结构收据与一次物理调用之间缺显式持久消费迁移
定位：PROTOCOL-R2.md 行 63, 75, 78, 82, 84, 99；§4重复交付、§5返回原ExecuteCommitted、§5唯一执行入口、§6物理外效状态。

对抗时序：
1. 同一个合法X、同一epoch，重复或并发收到两份相同ReleasedExact(c)。
2. 两条处理路径请求S；一条首次提交ExecuteCommitted，另一条按§5返回相同原收据。
3. 若两条路径都把原收据作为可调用依据，会对同一精确命令发两次场所调用；“只有一个进程/epoch”本身不能排除此执行。

影响：稿件已要求不可复用与唯一X，故此项指出实现该合同必需的状态迁移尚未写出，不能只依靠返回同一收据证明逻辑外效一次。它不要求现在选择数据库或假定场所恰一次。

修订要求：在X权威中明确同command_id/plan_hash/epoch的持久原子调用占有/消费迁移。只有第一次成功占有者可进入MayHaveCalled并尝试网络；重复收据或请求只查询原状态。提交后崩溃/未知不自动重发；允许按场所幂等合同重投时须另有明确证据，实际发送路径仍必须fence旧epoch。

后续验收：同ID并发/重复交付、多次原收据、占有前后崩溃、网络无回应、旧writer复活；断言同一调用权只消费一次且责任不早释放。

### RF02-L01 · NotSubmitted把权威未决状态与观察缺证据混在一起
定位：PROTOCOL-R2.md 行 96, 99；§6 E恢复与结构条件状态。

对抗时序：
1. ReleasedExact已在网络上或X已调用S，但回执延迟/丢失。
2. E查不到终态，或与S分区；观察者将其显示为NotSubmitted。
3. 该名称会错误声称请求未提交；当前保守保责条款虽阻止直接释放，却没有给两种情况独立类型。

影响：是状态/知识轴定义缺陷；不声称现稿已经允许以它释放责任。

修订要求：S权威状态用Undecided/NoTerminalDecision等中性名字；E/观察无法获取S状态另标Unknown/Unavailable，并携带last_known及查询前沿。不存在权威终态或观察无结果都不能证明未发送。

后续验收：延迟、丢回执和双向分区分别展示权威状态与观察知识；全部缺证据路径保持最大责任。

## OP-01 的范围化判断

原 R1 P1 第5步已经承认：持久 MayHaveSent 之后，网络调用可能晚于结构撤回，因此不能承诺撤回消灭在途外效。R2 把财务不可撤交付放在一次本地结构条件核验之前，确实改变了边界，但只要§3/4的限制是真的，它是一份已经决定且已交给受控执行流程的条件命令，仍与可改计划、等待未来行情的普通准备有实质区别。

现有四事件证据的合法排列 `E-Release → E_INVALIDATE → S-Execute → S_INVALIDATE` 只反证“两域在 S-Execute 仍共同最新”。没有网络、实际执行者及责任状态的四事件模型不能单独证明交付边界是假。相反，它也不能证明该边界已经实装或原需求已批准所有此类窗口。M01必须明列哪些前件已在不可撤交付合同中覆盖、哪些仍要最终核验，M03必须把单份资格落实为单次物理调用权。

采用这一协议时，必须保留其可见代价：精确Release后E新风险不保证阻止该已交付命令首次场所调用；尚未Release的普通准备没有此例外。S失效在S-Execute前仍严格拒绝。这是独立文本判断，不代替整图采纳或场所能力验证。

## 19 条初始反例映射

“文本闭合”表示按合同不能再用该时序合法绕过，运行仍未验；不把这些状态折算成项目通过率。

| CE | 结论 | 实际依据与剩余 |
|---|---|---|
| CE-L01 | closed-in-text | §1:8–25；§2:29–33；E不再位于正式结构输入/Begin/Commit前置；仍须实际故障注入验证。 |
| CE-L02 | structure-closure-economic-progress-repair | §2:29–33；§8:121；§9:127–129；结构/经营共轮撤销；新本地调度还需排除busy拒绝饿死条件执行。 关联 RF02-M02 |
| CE-L03 | closed-in-text | §8:113–119；结构闭包、切面、流独立；跨域总览和反向消费索引不能挡结构。 |
| CE-L04 | closed-in-text-with-implementation-obligation | §1:23–25；§8:121；禁止经济ack、共享outbox和保留回收反压；资源隔离和归档尚待实施证明。 |
| CE-S01 | conditional-boundary-explained-needs-condition-closure | §3:39–48；§4:54–67；§5:73–82；没有把两次读取当共同最新；经济按真实精确交付前件、结构按本地最终前件分别排序。必须补足条件阶段闭包，不能推广为两域执行时共同当前。 关联 RF02-M01 |
| CE-S02 | closed-in-text | §2:30–33；§5:77–80；§6:90–95；撤回在S自身先封门；分区E镜像不能取得S执行资格；真实first_known保留。 |
| CE-S03 | closed-in-text-with-implementation-obligation | §4:58–63；§5:75–80；精确身份/引用/epoch绑定，迟到回包不得复活终态；物理调用消费需补强。 关联 RF02-M03 |
| CE-S04 | qualified-design-closure | §3:41–46；§4:56–63；§5:80–82；完整冻结、本地已Committed、唯一X、紧贴实际交付、一次当前核验、无等待未来选择，构成实质条件，非仅改名。核验OP01的结论不覆盖未经证实的任意缓存许可。 关联 RF02-M01,RF02-M03 |
| CE-S05 | implementation-obligation-plus-call-consumption-repair | §4:58；§5:76,84；持久和实际网络路径均须fence旧epoch；同epoch重复调用也须持久消费保护。 关联 RF02-M03 |
| CE-S06 | closed-in-text-at-release-boundary | §4:57,67；CR-07；旧P4:215–229；风险规范Capture/Pending与新Release同序、先封闸再解释；Release后按披露的既有交付窗，最终条件按M01修补。 关联 RF02-M01 |
| CE-S07 | closed-in-text-with-certificate-obligation | §2:29,35；§7:105–109；S/E接纳分开，执行校验实际输入集合和证书覆盖，不以结构未撤回冒充估值新鲜。需要具体可验证数据合同。 关联 RF02-M01 |
| CE-S08 | closed-by-inherited-contract | §4:67；CR-07；旧P4:217–220；精确token和引用证据版本Resolve继承；没有恢复成单Valid标志。 |
| CE-S09 | needs-text-repair | §4:57；§5:71,76；§7:109；Release时和已声明执行时检查均有；必须执行时检查的适用集合不可由遗漏声明逃掉。 关联 RF02-M01 |
| CE-S10 | closed-in-text-with-fault-isolation-obligation | §1:16–25；S只保存小型条件结果，E风险/责任/事实不搬入S；真实部署要按职责注入故障证明。 |
| CE-S11 | closed-in-text | §1:25；§5:73,80；§9:127；S条件事务不得外部RPC或持经济锁，不等待远端决议；迟到同ID无法翻转Denied。 |
| CE-S12 | closed-in-text-call-consumption-must-hold | §6:96–101；CR-06/10；旧P2/P6/P7；Released/未知保持全潜量，只有对应S不可复活拒绝或场所终态可释放；P7冻结首次量不变。 关联 RF02-M03,RF02-L01 |
| CE-S13 | closed-in-text | §4:63–65；§5:80–82；§9:131；恢复只对旧身份查询/对账，普通历史准备不自动补发，仍有新经营需求要当前新决定。 |
| CE-RF01 | minimal-book-commit-preserved-execution-progress-repair | §1:12,20；§9:129–131；CR-11；旧R1:270–290；A最大责任覆盖及不等A Applied保留；B本地新Committed路径未被推翻，持续条件执行机会另受busy循环影响。 关联 RF02-M02 |
| CE-O01 | closed-in-text | §8:113–123；正式结构流、原子撤回、固定分页、Gap及AsKnown分离；不得由经济恢复回填历史。 |

## 14 条验收义务映射

| RA | 结论 | 实际依据与剩余 |
|---|---|---|
| RA-01 | closed-in-text | §1/2/4/8；S正式推进，E失败无新Release；已交付窗口明确保责。 |
| RA-02 | closed-in-text | §2:30–33；本域修订完成，不等经济；逻辑修订去重。 |
| RA-03 | closed-in-text-with-fencing-obligation | §1/2/5/8；结构流可独立；E缓存不能绕S门，实际网络栅栏需实证。 关联 RF02-M03 |
| RA-04 | closed-in-text-with-boundary-qualification | §2:30–33；§3/5/6；first_known本地真实、旧依赖被拒、已越界保责；不把普通Reserved纳入在途。 关联 RF02-M01,RF02-M03 |
| RA-05 | needs-text-repair | §3/4/5/7；两个边界有实质区分，不能声称两域共同当前；前件阶段闭包尚缺。 关联 RF02-M01 |
| RA-06 | closed-in-text | §4:58–63；§5:75–80；§8:119,123；传播晚不挡结构，精确ID/版本和获知史不回写；实际重复call另补。 关联 RF02-M03 |
| RA-07 | needs-text-repair | §4:63–65；§5:75–84；§6:96–101；保守责任与查询已给，调用权迁移和未决/未知类型需补。 关联 RF02-M03,RF02-L01 |
| RA-08 | implementation-obligation-plus-text-repair | §4:58；§5:76,84；网络发送fence是明确部署前件；须补同ID调用CAS，并实测旧writer。 关联 RF02-M03 |
| RA-09 | closed-in-text | §8:113–121；正式结构完整性独立，经济旧cut/缺项/consumed_structure准确标注。 |
| RA-10 | closed-in-text | §8:115,123；固定切面、续接相等、Gap、真实双序列历史均有；运行待验。 |
| RA-11 | closed-in-text-knowledge-state-repair | §6:96–101；§9:131；不补发历史、不重测L；未知结果不能推未发，按L01分层。 关联 RF02-L01 |
| RA-12 | minimal-obligation-preserved-not-run | §9:129–131；旧R1 RF01；B新Committed并不等待A账回放；未声称实际运行成功。条件执行进展另见RA13。 |
| RA-13 | needs-text-repair | §5:80；§9:127–129；无经济反压结构，但公平任务仍可能每次busy拒绝，需强化本地稳定机会。 关联 RF02-M02 |
| RA-14 | closed-in-text-with-certificate-obligation | §2:29；§7:105–109；E不是S唯一入口；集合/来源/更正覆盖必须有可检验证书，条件未分类不可默认放行。 关联 RF02-M01 |

## 5 项 OP 与 20 条合同替代

| OP | 结论 | 射程 |
|---|---|---|
| OP-01 | qualified-design-positive | 四事件允许路径只反证“S-Execute时两域共同当前”，不单独证明伪装预留。§3/4限制已Committed且冻结精确命令向唯一X不可撤即时交付，之后仅一次当前结构核验，无未来业务选择；在这些条件确实成立时，可承接真实在途责任窗。原R1也允许MayHaveSent提交后网络调用晚于撤回；R2扩大/分拆其边界须显式保留，不能声称完全等同单点联合核验。 |
| OP-02 | requires-text-repair | 现有“若声明”为执行时条件未封闭适用集合。补条件阶段、失效源、证据和不支持默认拒绝后，具体场所验证成为有边界的实施验收，不是自动新架构取舍。 |
| OP-03 | design-contract-present-implementation-obligation | §7已明确真实集合/来源游标、非同域整数不可比较、经济解释留E、S核覆盖。下一层需定义proof=(source namespace/epoch, event identity and revision, canonical prefix/set, supersession relation, required dependency set)并测试非单调更正、跨源跳号和旧证书；不能把这些未实装一概当新选择。 |
| OP-04 | requires-call-consumption-repair-plus-implementation-obligations | 旧epoch实际网络fence和不确定保责已明确；同X重复receipt不会自动形成一次物理调用，需显式持久消费。场所幂等/回查/终态能力仅可作为待验证假设，不冒称恰一次。 |
| OP-05 | structure-isolation-closed-in-text-liveness-needs-repair | S无E输入/日志/查询/ack/锁依赖的条款足够明确，不是整体经济日志改名；一般公平尚不能避免busy拒绝循环。修文后仍须实际故障域、持续输入和前端验收。 |

20 条 CR 已逐条读取。CR-01–13与已直接读取的R1协议条款核替代关系；CR-14–20只核本轮继承/替代声明，不冒领重审全部状态合同、分类目录、旧入口或方案比较。逐CR记录在配套JSON；CR-03/05/07/08/10/11/14/15/19需随对应缺陷修订或状态结论更新。

## 读取签名与验证边界

本次核17个DELIVERY清单SHA全部匹配；PROTOCOL JSON的10节正文与MD逐节一致。这是文档完整性核验，不是协议模型或项目测试。DELIVERY.md、BASELINE-BINDINGS.json和R1-COUPLING-LOCATORS.json仅核字节摘要，未以它们语义支持本评审。

- `/tmp/newchanlun-1339-architecture-r2-20260909/DELIVERY.json`
  - SHA-256：`b91f8153d995f4dedda65c132eaee263e868b1837368083c42f2625de2d93eb2`
  - 读取范围：全文清单及状态。
- `/tmp/newchanlun-1339-architecture-r2-20260909/NEXT-REVIEW.md`
  - SHA-256：`3d5188f00bf62bd0a95f9e23bf5f739ed3e41f4bfb211262fad3313a2e8d3f64`
  - 读取范围：全文。
- `/tmp/newchanlun-1339-architecture-r2-20260909/USER-DECISION.md`
  - SHA-256：`bde32ba1369879cfec4343d0364d4cddc160238ea5b6a03c7828fe36ba94d0c9`
  - 读取范围：全文。
- `/tmp/newchanlun-1339-architecture-r2-20260909/USER-DECISION.json`
  - SHA-256：`68105375cfa35628881ddd2db271a7606b37985547921758ddd1e820bfd33a3d`
  - 读取范围：全文。
- `/tmp/newchanlun-1339-architecture-r2-20260909/ACCEPTANCE-RF02.md`
  - SHA-256：`d1fcd8d1b36cd90910a561ee60dab1bf820b64ac49f562b3d16cf43f69ccf309`
  - 读取范围：全文14RA。
- `/tmp/newchanlun-1339-architecture-r2-20260909/ACCEPTANCE-RF02.json`
  - SHA-256：`57722029f83edc7f1349f4df5a72a33d4e3fe9a56777f0f625fceea197d00b40`
  - 读取范围：全文14RA。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/PROTOCOL-R2.md`
  - SHA-256：`8570fc96b47ec90f3a87ac829b78322f1aad9384d95694aafdb53fd5f712e1c5`
  - 读取范围：全文1–137行、10节。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/PROTOCOL-R2.json`
  - SHA-256：`26ebb7b85cc3bb1b0ce6a192f8a3c808f23eef359cf605378cffc7339ffcc0c1`
  - 读取范围：结构字段与10节正文逐节程序比对MD一致；部分重复正文工具输出截断，不依赖截断内容另立结论。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/CONTRACT-REPLACEMENTS.md`
  - SHA-256：`7bee1e19839e542978075498117d5d0e9ceb79bbcfc888f12904340f014a60b0`
  - 读取范围：全文1–32行、20CR。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/CONTRACT-REPLACEMENTS.json`
  - SHA-256：`478ef592b12d868de7a29b0592a113895b060e34131d3de90b7328a05cb06119`
  - 读取范围：全文20CR。
- `/tmp/newchanlun-1339-architecture-r2-20260909/author/OPEN-PROTOCOL-QUESTIONS.md`
  - SHA-256：`cfa7dbf06b327ee593cbff8024089f948096af357b7151cde3ef19f684986493`
  - 读取范围：全文5OP。
- `/tmp/newchanlun-1339-architecture-r2-20260909/review/INITIAL-COUNTEREXAMPLES.md`
  - SHA-256：`ecc62a15f58b527ba78e63f9b3db18f32ee0940262cd1e0227d77c169c7b1bb0`
  - 读取范围：作者本代理完整创建并在本轮逐项再映射，19CE。
- `/tmp/newchanlun-1339-architecture-r2-20260909/review/INITIAL-COUNTEREXAMPLES.json`
  - SHA-256：`366a6182601226e16dce67125c894437a2c1a5e6a3654a75ad69d500a27a9657`
  - 读取范围：完整19CE与7不变量。
- `/tmp/newchanlun-1339-architecture-r2-20260909/evidence/ORDERING-COUNTEREXAMPLE.md`
  - SHA-256：`8110d0c8609bde6b9c8d80072722938559bc2c72ccf84d50e080927840d92241`
  - 读取范围：全文。
- `/tmp/newchanlun-1339-architecture-r2-20260909/evidence/ORDERING-COUNTEREXAMPLE.json`
  - SHA-256：`55e24e43ed15f23524e34c11da8924a0eac9e0b5dfbba7ca2b1f00d50640a79f`
  - 读取范围：全文24排列与守卫，复用其结果，未另执行模型。
- `/tmp/newchanlun-1339-architecture-20260908/author/INDEPENDENT-DESIGN.md`
  - SHA-256：`f6054b52f86ba80fafd6386d1ac6ede341c6c6a17cb9efbed5d2dad42571824d`
  - 读取范围：初评直接读1–102、165–321、378–400行；本次沿用同SHA已读P1/P4/P6/P7/RF01/O1–O3；第3节仅定位，不重审62轴。

未运行项目、Lean、UI、回放、paper/live、模型检查或真实场所；未改作者/根文件、旧包、仓库、GitHub或服务。本文保留冻结R2的全部四项发现；作者修订后按新SHA另作关项复核，不倒改本次签署范围。
