# #1323 完整生产程序 SPEC：审阅入口

本轮把已采纳的 R2 架构整理为覆盖整张 #1323 图的实施总单。六项终点、完整结构观察和逐重经营均保留；本轮交付的是待批准的 SPEC，程序、前端、回放及场所验收尚未实施。

## 批准对象

[SPEC 正文](/tmp/newchanlun-1323-spec-20260909/SPEC.md)包含97个故事、12个实现章节、18组验收和11个拟拆切片。正文与下列规范附件共同组成批准范围：

- [完整需求、分类与状态](/tmp/newchanlun-1323-spec-20260909/inputs/SPEC-COVERAGE-INPUT.md)：44项结构需求、35项逐重经营需求、RF-01/RF-02、62项分类或公式目录、10项组合合同、16项工程状态及所有观察/旧入口义务。
- [接口合同](/tmp/newchanlun-1323-spec-20260909/INTERFACE-CONTRACTS.md)：各域调用、消息身份、结果、失败与恢复、本地原子组。
- [故事到验收与切片的映射](/tmp/newchanlun-1323-spec-20260909/IMPLEMENTATION-CROSSWALK.md)：345条目、496条细项验收逐一连接到故事、验收组、切片、终点及成功路径依赖。
- [当前有效合同索引](/tmp/newchanlun-1323-spec-20260909/EFFECTIVE-CONTRACTS.md)：明确真正分类的量词，保留并存角色/公式/描述，并用现行字段替代旧状态与旧入口候选措辞。
- [测试缝与验证边界](/tmp/newchanlun-1323-spec-20260909/inputs/TEST-SEAMS.md)、[具名残留决定](/tmp/newchanlun-1323-spec-20260909/inputs/RESIDUAL-DECISIONS.md)及[排程依赖](/tmp/newchanlun-1323-spec-20260909/PLANNED-DAG.json)。

R2及已披露的精确交付窗口已采纳，记录见[架构采纳](/tmp/newchanlun-1323-spec-20260909/ARCHITECTURE-ADOPTION.md)。本稿新增的具体技术提案主要在C09：同机独立进程和各域独立SQLite WAL持久化、短事务与不可变结构切面、稳定点有限FIFO服务、X独占调用及进程恢复。批准这些技术提案不证明其存储隔离、故障活性或场所能力已经通过实测。

本金/增长、配对及共同资金分配、部分费用归属、具体量档和成本实例、计样边界、权限及退出策略的真实残留仍具名保留。已裁数学按原口径同步与补证；本稿不提供未定资金默认值。正确返回缺政策或不支持只能通过失败路径，不能关闭必需的成功路径。

## 独立审阅与修订

两路独立文档终审均为 **PASS_WITH_GATES，当前0H / 0M / 0L**：[范围评审](/tmp/newchanlun-1323-spec-20260909/review/SCOPE-SPEC-REVIEW.md)与[协议评审](/tmp/newchanlun-1323-spec-20260909/review/PROTOCOL-SPEC-REVIEW.md)。两份结论绑定同一份S2-R1[20件文件清单](/tmp/newchanlun-1323-spec-20260909/evidence/REVIEW-INPUTS-S2-R1.json)，实际阅读范围分别写明，未将哈希核验冒充全部语义阅读。

S1初评发现的三项M和一项L分别为分类量词误用、故事至具体实施/验收缺连接、时间字段排除过宽、现行字段混入历史措辞，均已关闭。S2终审又核出几条具体义务缺少场所/部署映射，S2-R1已补齐并经两路复核关闭。原稿与原评保留。另把目标部署旧发送入口隔离置于场所验收之前，把带仓迁移及默认启用置于其后，避免整票依赖循环。

协议评审保留六组尚未执行的实施证据门：存储与空间隔离、有限服务、完整条件实例、真实发送隔离、责任/Pending/P7、不可变切面与AsKnown。它们已有具名验收和切片，不因文档通过而销项。

文档自检已通过：97个故事与正文一致，496个验收ID唯一且均有具体映射，24件来源SHA一致，11切片依赖无环。结果见[静态文档核验](/tmp/newchanlun-1323-spec-20260909/evidence/DOC-VALIDATION-S2.json)。这些检查没有运行项目、Lean、UI、进程故障、paper或live测试。

## 发布和下一步

本轮没有向GitHub发布、创建或分派实施票，没有修改生产代码、主线、服务或交易状态。旧 #1327–#1333 排程保持暂停；整图及goal尚未完成。

批准本份SPEC后，按[发布与拆票准备](/tmp/newchanlun-1323-spec-20260909/PUBLICATION-PREPARATION.md)归档完整规范与独评实体、发布图内总单、更新真实关联并拆分合资格切片。真实未决项先建立承接及阻塞关系；实施、main合入和真实交易按已有明确授权分别处理。不能只发布会失效的临时链接或以本稿TB编号伪充GitHub票号。

发布前批准门依据实际读取的[to-spec技能](/Users/silencehan/Projects/NewChanlun/.agents/skills/to-spec/SKILL.md:17)第3条：“place the approval immediately before publication”。本次需批准的是这份首次交付的具体SPEC及技术提案；此前R2架构采纳保持有效。

当前生命周期以本入口和[最终交付清单](/tmp/newchanlun-1323-spec-20260909/DELIVERY.json)为准。冻结输入中“等待独评”的草拟状态是当时记录；最终独评已完成，SPEC用户批准仍待取得。冻结规范正文不因评审状态更新而改写哈希。
