> 链接转换阅读版；正文语义与原稿一致。签署原字节：[原稿](../../../../payload/sources/newchanlun-1339-architecture-20260908/author/independent-design-state-contracts.md)，SHA256 `f5ea3426c08bcffa5e3a40a7d29631169d1776b5ffb2a50f4c526a766bc3629d`。当前批准状态见归档根APPROVAL.json；原稿中的待审状态保留为发生时记录。

# 本候选自声明的经营与工程状态合同

> #1339，R1。承接35条FG与RF-01；下列软件状态组织是待采纳设计，不是新增缠论裁定或资金规则。本附录补分类目录CC-058没有覆盖的经营/提交/接纳状态，不用几个状态名替代已签FG的全部语义。

每个轴都限定在一个明确对象、已齐证据与固定规则版本上。数值比较用精确有序域；多个独立轴取受约束乘积，不合成一个“总体正常/异常”。实际事实违反合法状态不变量时保留事实和具体违例，停止相关新增；不能把它强塞到合法叶，也不能从账中删掉超界成交来维持分类外观。缺金额、未裁政策、不可读账、未完成计算属于结果资格，不是金额符号或阶段的新叶。

静态证明包按程序/语义/政策版本证明下列分区无遗漏、互斥、各迁移保持不变量；运行时在具体事务前后检查见证并记录所用包。阶段/资金政策仍未齐的迁移原样登记缺口，不宣称enum本身证明真实经营输入总可归类。没有每bar重跑Lean的要求；没有运行测试的结论。

| 轴/对象及域 | 完整分支或公式族 | 允许迁移及约束 | 后端输出/前端入口 |
|---|---|---|---|
| ES-01 私有决定尝试，已保留attempt_id及本地日志前缀 | 尚无Prepared；Prepared；Committed；AbortedBeforeCommit 四叶，由唯一日志终点决定 | 无→Prepared；Prepared→Committed或Aborted；Committed不可改写为未发生。已Committed撤回由许可/外效/尾责轴表达。重复相同事件为自环；其他迁移拒绝。 | Decision尝试、账版本/锁定/收据和撤回关系；重→决定→完整尝试史 |
| ES-02 每个公共责任的新增发送资格 | Absent；Reserved；Dispatchable；Closed 四叶 | Absent→Reserved；Reserved→Dispatchable或Closed；Dispatchable→Closed。Closed对该授权终结，不靠新epoch复活，合法新决定须新责任身份。Closed不表示责任上界为零，也不抹已MayHaveSent。 | grant状态、许可版本/关闭原因、全部命令；共同责任详情 |
| ES-03 已创建物理命令的不可逆发送位M与已证不可再执行位T | (0,0)、(1,0)、(0,1)、(1,1) 四个原始签名 | M只能0→1，由P1同域MayHaveSent提交；T只由可证未发关闭或场所终态/完整累计证据置1。M0T1是永不发送已关闭；M1T1仍可接迟到已发生成交细节。T后语义更正不重新发送旧命令；推翻证据时标失效并封闸。 | command全状态、发送/终态证据、历史累计前沿；逻辑订单→物理命令 |
| ES-04 一个Capture token对一个可能受影响约束 | Pending；ResolvedCovered；ResolvedUnverifiable 三叶 | Capture创建Pending；核证后只对该token/约束转Covered或Unverifiable；新事实用新token。重新核实可将Unverifiable→Covered并保留旧判断版本；不能用e1删除e2。逻辑Open由ES-05计算。 | 影响范围、token来源/前沿、逐约束证书/缺口；外部事实→责任约束 |
| ES-05 单个共同约束的当前准入条件，所有校验位已可判 | P=存在未核token、U=未解缺口、V=当前证据/时效/源连续性有效，(P,U,V)∈Bool³共8签名；仅(0,0,1)Open，其余7关闭 | Capture/Resolve/新缺口/证据失效各按同域事务改变相应字段；不直接写Open。多个约束全Open才满足共同条件；缺任一可判证据将U置真而非假设V。新证据重核可以复开。 | 全部失败位和受影响准入、证据版本；共同约束矩阵 |
| ES-06 外部事实的归属资格，已规范Recorded且语义已解析 | Unmatched；UniqueAssignment；ConflictingAssignments 三叶（可证明的完整分配映射候选数0、1、>1或相互矛盾；一份唯一映射可同时归多个重，不以收款重数计） | 解析/查询可补齐到Unique；新证据推翻原匹配则新版本冲突并先P4封闸。仅Unique及费用/分配政策齐备才可正式应用；三状态不删除原报。原报尚未解析仍由Pending覆盖，不假称已知Unmatched。 | 原报/命令/责任关联、候选归属与冲突；事实详情 |
| ES-07 每个已存在的(fact或allocation,chong)应用目标 | NotApplied；Applied 两叶；期望目标尚无法确定时不创建虚构键，保留ES-06 | NotApplied→Applied仅本重原子事务；重放相同内容自环，异内容产生冲突事件且旧收据不覆写。更正是新fact、新应用键，旧Applied不倒退。 | ApplicationReceipt、账前后差异/原责任；事实→各重入账 |
| ES-08 已建立Voice的业务许可与责任尾账 | B∈{Active,Exited} × R∈{Outstanding,Retired}四原始组合，受实际证据约束 | B只能Active→Exited；新Voice另ID。R只有证明所有原责任可核、已应用、尾费有覆盖或为零才Retired；后续推翻覆盖的更正可重开Outstanding，归原Voice。Active且暂无尾责不等Voice退出；Exited且Outstanding必须保留可见。 | 业务退出与责任退休分列、旧Voice晚到事实；Voice时间线 |
| ES-09 固定Q>0、已可核的本重同方向筹码量q | 合法域0≤q≤Q分q=0、0<q<Q、q=Q三叶；已齐原始数值域另保留q<0、q>Q违例两叶 | 任何合法本地合成前后都核整体q及全部未决；q在合法三叶间的迁移须真实fill/权益转移及已批动作。外部超界仍如实记账并冻结，不能截断成0/Q。多空方向另由Voice不可变语义指定。 | 固定Q、实际量、未决潜量、超界原事实；重→专属筹码 |
| ES-10 已按批准会计定义可求的每重成本C，以及两个连续已知成本值 | 状态C<0、C=0、C>0；事件(old_sign,new_sign)∈三值²共9签名，另带实际ΔC和事件来源 | 只有真实经济应用或已批会计调整变C，行情估值不触发。到零事件为new=0且old≠0；继续零成本自环不是再次回本。9符号码只是完整描述，不能自动决定TStage；分母/定义未齐不给伪C。 | 成本状态、正负现金流/费用、到零事件和原收据；成本曲线+事件账 |
| ES-11 已具名本金目标P及实际回收R、固定货币/会计版本 | R<P、R=P、R>P三叶（R的负数同样是<P）；P尚未定义的请求不在此语义域 | 回收由真实事件及批准本金/费用政策改变；跨过等号与超回收均保留，不能只看C符号。新政策不得回写当时值；费用更正按原史应用。 | 本金目标/回收/缺口及政策来源；阶段→回本链 |
| ES-12 已开户且完成阶段初始化的一个TStage与方向 | Long:{降成本,退本金,赚股}；Short:{降成本,退本金}，共5方向×阶段合法签名；Short赚股为禁止项 | 已裁通常向降成本→退本金→Long赚股推进；每条转移须真实成本/回本前件、获准政策和单重事件。费用导致跨零后的反向/重新分类处理、初始本金定义未齐部分保留FU缺口，不在本轮自定迁移。待初始化单列请求资格，不能伪设一个阶段。 | 唯一TStage、实际前件、未批准转移及政策；重阶段历史 |
| ES-13 P3单份权利转移，固定donor/recipient/rights及批准政策 | Proposed；SourceLocked；Escrowed；DonorDebited；RecipientPending；Activated；Cancelled 七叶 | Proposed→SourceLocked→Escrowed→DonorDebited→RecipientPending→Activated。前两态可有据取消并本地解锁；Escrowed以后取消须同公共域唯一未激活决议、源/目标回执及回还协议；已Activated不就地Cancelled，反向转移另身份。历史经过的各步保留。 | 源权利锁、托管、双方账收据和唯一激活；换档→权利转移 |
| ES-14 同逻辑订单family一次改价尝试 | Active；CancelRequested；AwaitingFinality；RemainderCertified；ReplacementPrepared；ReplacementCommitted；Installed；Abandoned 八叶 | 按P7顺序进入下一步；Installed把新revision作为family当前Active并保留旧尝试。放弃新价可进Abandoned但不能取消已知外效或直接释放旧潜量；若旧单已确证取消，放弃只留下未用权利，是否新经营再决定。每family最多一个未决剩余分配转移。原子amend是具证能力路径，其请求/回应/查询同此family，不能并列两份额度。 | L、F、各独立成交分配潜量及共享/互斥物理修订、剩余转移及取消/改单证据；逻辑订单修订史 |
| ES-15 同一观察请求的交付资格与历史模式 | 交付:{CompleteCut,PartialCut,Gap}；历史:{AsKnown,RecomputedWithRevision}，6个原始组合 | Complete/Partial须给固定cut与缺项清单；Gap须给缺口及新快照token，不假称空结果。后续请求可得到任一交付结果；每一session/请求的历史模式不可悄悄切换，重算另具名请求。 | 当前切面/缺项、续接、模式/版本；完整观察导航 |
| ES-16 同一经营请求的判据向量与政策资格 | 已批准且适用的n个门分别Pass/Fail；不适用门带N/A且不参与AND，故原始码{Pass,Fail,N/A}^n再受各门适用域约束；政策依赖的缺项集合K为空/非空另轴 | 所有Fail完整保留；N/A不能填Pass；K非空只产生AwaitingPolicy(K)，不伪造Hold或结构否定。全适用门通过且K空才可尝试P1，能否分资金仍须P3已批准政策；改变政策/输入新请求，不以失败改窄宽判断。 | 完整判据向量、适用域、K及typed动作；经营决定→准入依据 |

以上列表对本候选自声明软件状态的定义域给出全分支；有限分区的形式证明、实际数据库事件归约器总性及场所到证据类型的桥尚未完成。尤其ES-12的未裁迁移不因enum存在而变成已裁；本稿没有完整实现动作量、阶段、增长或场所模型。动作“唯一”只按已批typed action规则成立：原始多结构事实、最终动作、权限/资金准入和真实外效是四个不同对象，不能为了最终一动作把原事实裁成互斥。

额外跨轴不变量必须同时验：ES-02 Dispatchable依赖ES-01 Committed收据；ES-03的M0→1还依赖所有相关ES-05 Open、结构BeginRevision已解除、当前授权/权限及P7量预算；ES-02 Closed不强制ES-03 M=0；ES-07 Applied不证明ES-05可释放，P2要求对应责任的全部必要收据与外部证据；ES-08 Exited不证明Retired；ES-10成本到零不自动使ES-11回本；ES-13 Activated唯一消耗该份托管权益；ES-14不能为旧潜量尚未解除时的新物理命令分配额度。每条可达迁移需要具体事件见证，未列迁移禁止或按具名未裁项等待，不能临场选“最像”的状态。

FG完整消费映射：FG-001/003/004/007/008/009/010的结构、身份和读法还依赖分类目录对应CC轴；FG-011/018/019/025/028/034/035的制度、容量、频率、统计与模型条件仍按已签需求和FU清单。它们不是新增有限FSM叶，数值/集合参数需给完整有效域和实际公式；未批准的参数或公式不得默填。其余经济事实按ES-01–16及主文第4–9节输出，具体35行对应关系见同名JSON。该映射不是35条行为已经通过验收的结论。
