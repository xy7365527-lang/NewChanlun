> 仅转换链接的阅读版；[原字节](../../payload/tickets/1323-UPDATED.md) SHA256 `53b04ddb9efd72021b9f6bb1d35c5fc60487b12dc2177bdb8c328c3e8aff5ba6`。原稿状态是发生时记录，当前发布范围与限制见归档根 README。

## Destination

本图到达时，必须同时为真：

1. **#787 严格多重赋格架构成为唯一生产程序**：一条纵向主干塔／一份 Classification，挂 N 个操作级别旁路；每重各自做唯一同级别分解与向下定位，各自拥有 TStage、Voice 生命史、专属筹码和毛账；旁路与下钻不回流主干；重内单向、重间不仲裁；物理订单仅在最后按场所能力投影。
2. **生产 owner 唯一**：逐 bar 消费 `Classification → 操作级别旁路/向下定位 → Chong` 的模块已裁定、实现并机械锁定；同一判断不存在第二套生产实现或失败兜底。
3. **旧实现全部有终态**：每条旧引擎／简化臂逐一落为收编、冻结影子臂或删除；冻结臂不得参与生产准入和交易决策；#1096 不再图外悬空。
4. **完整历史回放可执行**：历史数据驱动的确是上述完整程序，输出逐重状态、毛账、订单投影、成交与唯一真账；同输入双跑确定。
5. **paper/live 与回放关系已裁定并验收**：共享到哪一层、替换哪些 adapter、谁持有唯一成交/持仓真账均有明确接口与行为锁；#1319 只能按本图裁定验收或废弃。
6. **#1304 与 #1279 有合法去向**：#1304 作为回放功能工作线接入本图；本图完成并通过 SPEC 实施验收前，#1279 不得关图或宣布试运行放行。

## Notes

- **本图带实装**（决策+执行混合：真裁在 grilling/research/task-乙 票内完成；决策锁定后经 SPEC 拆 tracer-bullet 实施票，逐票独立评审与人工合入）。
- **本图到达即关**。
- **2026-09-09 当前状态**：完整需求已确认，R2及其披露的精确命令交付窗口已获用户采纳；[完整SPEC #1340](https://github.com/xy7365527-lang/NewChanlun/issues/1340)和C09技术提案已批准，已发布并按真实依赖拆票。归档在[Draft PR #1341](https://github.com/xy7365527-lang/NewChanlun/pull/1341)，尚未合入main。合资格实施已获本图流程授权；未决语义/资金规则、main和真实外效仍按具名门处理。
- **独立研究发生史**：#1334/#1335/#1336封存成果保留；#1336原有条件推荐不改签署字节。旧#1327–#1333设计排程继续暂停，现有义务已逐票登记真实SPEC/TB承接；未做旧结果对照，查阅或比较隔离旧成果仍须用户明确授权。
- 本图是 #59/#529/#743/#787 的**续图**，不是重开：四张旧图的历史 destination 均正确完成；当前范围是后裁 #787 严格架构的完整生产接线、旧实现总收敛、历史回放与 paper 同核。
- 架构继承链：#787/#960/#961 → `docs/unified-architecture-canon.md`；#827 → ADR 0011；#839 → ADR 0013；#834 → ADR 0014；#842 → ADR 0010；实施发生史 #847。
- #847 是结构层非持久 SPEC；15/15 关闭不等于 π、organic、paper 已完整贯通。
- #1319/#1320 在本图裁定前均保持 `needs-triage`：禁止恢复、合入或关票完成；分支只作候选史料。

- **#1338 图文发生史**：三套候选Archify及对比总图已交付并独立复核；仍OPEN的归档/main门须按该票实体单独核。本次SPEC包不宣称已收编全部四图；旧图原推荐不替代当前R2。

- **#1339 需求与架构链**：完整多重赋格和后台完整结构/经营事实为范围；需求、分类量词、观察合同及候选重评已完成当前批准链，现由#1340承接实施。Codex App承接工作，不迁移Prime运行态。

### 红线

1. 同一判断不得有宽严两档，失败不得触发换判据重判（#799）。
2. 同一个判定每级必须复用同一判定，级别差异只以输入参数出现（#804）。
3. 不得把 Chong 粗化为资金桶：重是 `⟨标的, 操作级别, 专属筹码⟩` 的持久操作单位；Voice 是一次建仓生命史；TStage 每重一份。
4. 不得用“事后联立”仲裁重间方向；多级别联立是结构判断联立，不是跨重订单仲裁。
5. 净额物理执行不得抹掉逐重毛账；场所能力不足只能形成明确投影与退场条件。
6. 实施票不得替人裁架构；未决项必须留 Fog 或开决策票。

## Decisions-so-far

- **继承 #787**：目标架构是一条纵向主干塔／一份 Classification，N 个操作级别旁路各自产唯一同级别分解序列，并各自向下定位。
- **继承 #839/#834/#842**：重持久、Voice 一次性、TStage 每重一份；重内单向、重间不仲裁；专属筹码和毛账逐重保留，物理层最后投影。
- **#1321 已确认的声明上限**：M1 只认包含严格多重赋格与执行面的完整生产程序；ThetaPiStream 或 π batch 单独读数不得冒充完整程序结论。
- **旧图判重**：#59/#529/#743/#787 均为“历史 destination 已完成但新范围不同”，不重开；证据见 `.chanlun/review-results/issue1321-map-coverage-matrix.md`。
- **撤回候选**：A′“唯一结构源→多个资金桶→事后联立→统一执行”被编排者打回，不是裁定。
- **Archify 工具前置 #1324 已完成**：编排者选择先扩展 Archify；无箭头 association、Fog seam、edge bridge、嵌套 containment、depth ruler、read-only connection 已安装并通过 Standards/Spec/A/B 四路评审及安装后回归；#1321 blocker 已解除。
- **#1321 Owner 已完成**：B 分域单-owner 已落 ADR 0027、CONTEXT 和统一架构正本（origin/main `644075a6`）；结构域拥有唯一 Classification，下游唯一重操作 owner 拥有 Chong/TStage/Voice/专属筹码/逐重毛账与生产交易意图；旧入口收编或退役；前端只读 Projection。
- **#1322 全链 scan 已完成**：固定快照 `644075a6` 的终稿 2,445 行；221 边、24 严格锁、197 无锁；四审 PASS。结论是仓内尚无 ADR 0027 端到端入口，报告 SHA `9ead1c42...`。
- **#1326 旧研究结果保留**：已产出有界静态审计、候选和解盲对照，不能替代用户要求的新一轮独立重做；归档/关票流程暂停，旧产物不得输入新作者。
- **#1325 候选 bundle 已选择并关票**：完整 B 按 C1–C5 全选推进；C6 仅作临时非生产隔离区并强制退役。Evidence PASS、Visual PASS、完整 B 覆盖审计 PASS（0H/0M/0L）；top recommendation 只决定起点，不代表单选。设计 DAG 为 #1327→#1328→{#1329,#1330,#1332}，#1329+#1330→#1331，#1331+#1332→#1333。

- **#1334 独立 Archify 已完成**：9视图分清语义义务、当前代码与未决，准确字节通过原生/独立语义/实际视觉验收并封存；不等于程序或完整研究通过。[交付与验收](https://github.com/xy7365527-lang/NewChanlun/issues/1334#issuecomment-5554436733)。

- **#1335 独立全链扫描已完成**：有界静态171合同/94锁，原独审0H/0M/1L；根核309候选、316载荷、317归档成员及2204输入字节。未读与not-run保留，不等于程序通过。[交付与验收](https://github.com/xy7365527-lang/NewChanlun/issues/1335#issuecomment-5559726009)。

- **#1336 独立完整方案已交付**：三主组织B-r1/A2-r3/C3-r1及C2完整变体，整包108文件独审0H/0M/2L、根机械核120载荷/121归档成员；推荐B不等采纳，运行全not-run、旧对照未做。[方案与完整选择依据](https://github.com/xy7365527-lang/NewChanlun/issues/1336#issuecomment-5566225609)。

- **#1339 当前批准链**：需求冻结、完整345条目/496验收点/97故事、62 CC逐域分类量词、R2/RF-01/RF-02及精确交付窗口已采纳；[采纳与归档登记](https://github.com/xy7365527-lang/NewChanlun/issues/1339#issuecomment-5596164716)。S独立正式生产结构，Bᵢ保各重经营，E保共同责任，X保唯一实际调用；前端只读同源事实。所有运行义务仍须实际实现和验收。
- **SPEC #1340 已批准并发布**：规范清单SHA256 `38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`，正文原件SHA256 `42fc34cd0bb6736469db5b6e652f861bee41b6ccc5808677125fae831b75bd11`。[原件及批准记录](https://github.com/xy7365527-lang/NewChanlun/blob/efc1ddc4d6c015f4f8c6d44f904d704a88308b46/.chanlun/review-results/issue1339-r2-spec-20260909/README.md)绑定commit `efc1ddc4d6c015f4f8c6d44f904d704a88308b46`；阅读链接转换不改签署payload。Draft PR #1341尚未main落地。
- **旧票已具名承接**：#1327–#1333及#1096/#1304/#1319/#1320/#1279已逐票追加真实TB去向和未销义务；旧状态/标签/原生边暂存，暂停声明继续有效。#1282旧实验落档和#942逐问取消/未决/转挂处分保持。

## Not yet specified

R2和本次完整SPEC/C09已批准，不再作为缺少选择。以下仅为仍需具体证据/决定的局部成功门；已裁同步、缺证明和新选择各守其名分，不能把整个G/FU集合挂成全局开工门。

1. 初始化/同价身份实例：[RD-01 #1342](https://github.com/xy7365527-lang/NewChanlun/issues/1342)、[RD-02 #1343](https://github.com/xy7365527-lang/NewChanlun/issues/1343)；第二特征序列完整扫描/checkpoint：[RD-03 #1344](https://github.com/xy7365527-lang/NewChanlun/issues/1344)。
2. 独立选父价格挂法、已裁对象域桥、递归N/A合成、当前成本实例：[RD-04 #1345](https://github.com/xy7365527-lang/NewChanlun/issues/1345)、[RD-05 #1346](https://github.com/xy7365527-lang/NewChanlun/issues/1346)、[RD-06 #1347](https://github.com/xy7365527-lang/NewChanlun/issues/1347)、[RD-07 #1354](https://github.com/xy7365527-lang/NewChanlun/issues/1354)。RD-05先核既裁Comparable/Sel对象域，不重裁全集。
3. 本金/费用/跨零、完整量档与增长/换档、争用/cap、部分成交/费用分配：[RD-08 #1348](https://github.com/xy7365527-lang/NewChanlun/issues/1348)、[RD-09 #1355](https://github.com/xy7365527-lang/NewChanlun/issues/1355)、[RD-10 #1357](https://github.com/xy7365527-lang/NewChanlun/issues/1357)、[RD-11 #1349](https://github.com/xy7365527-lang/NewChanlun/issues/1349)、[RD-12 #1350](https://github.com/xy7365527-lang/NewChanlun/issues/1350)。没有默认金额或未批仲裁。
4. 实际场所/账户/profile及权限：[RD-13 #1351](https://github.com/xy7365527-lang/NewChanlun/issues/1351)；计样与按首次触达条款激活的后闸：[RD-14 #1356](https://github.com/xy7365527-lang/NewChanlun/issues/1356)、[RD-15 #1358](https://github.com/xy7365527-lang/NewChanlun/issues/1358)；具体退出授权/责任退休：[RD-16 #1352](https://github.com/xy7365527-lang/NewChanlun/issues/1352)。已取消实验不复活。
5. 完整历史的多重并发、混合市价/限价及同slot竞争模型：[RD-17 #1353](https://github.com/xy7365527-lang/NewChanlun/issues/1353)。继承已裁窄域模型，未裁完整模型不由TestOnly顺序代答。
6. #1303图/链表示终局仍OPEN缓办；保留其B7c对拍锁及#1283引擎链定位前件，是否进入后续frontier仍须依该票定位核实，不因本次SPEC发布自动采纳、消失或添加为全局blocker。
7. 旧关联票额外义务继续对账：#1320七标的十年1min及冻结7/7重型测量未被有界TB-07测试吞掉；#1304报告器/回归锁已有TB-07-D实际接单；#1319实场所范围及#1279自身就绪条件保留。#1338单独归档门未由本次SPEC包自动完成。

## Frontier

**当前执行总单：[完整SPEC #1340](https://github.com/xy7365527-lang/NewChanlun/issues/1340)**。11个TB是验收汇总，各自下挂一上下文可交付的正式程序纵片；共已发布 77 张内部实施票，票面标明证明域、未销父义务及真实blockers。聚合票和叶票未加入常驻sandcastle队列。

1. **首个在跑纵片：[TB-01-A #1370](https://github.com/xy7365527-lang/NewChanlun/issues/1370)**。从精确归档基线`efc1ddc4`定向隔离实装，固定现役Prime通道；同次Rust结构计算→S独立持久→同源API/浏览器，经济进程缺席仍正式推进。运行尚未验收，完成后另开独立评审；不得以CC-006局部成功关闭完整结构分类。
2. 后续结构观察纵片：[TB-01-B #1371](https://github.com/xy7365527-lang/NewChanlun/issues/1371)、[TB-01-C #1372](https://github.com/xy7365527-lang/NewChanlun/issues/1372)，分别承接更正/真实进程恢复、固定cut分页/Gap/持续推进；完整TB-01完成才按原DAG解除TB-02/TB-03前置。

| 已批准TB | 真实验收汇总票 |
|---|---|
| TB-01 | [TB-01 #1359](https://github.com/xy7365527-lang/NewChanlun/issues/1359) |
| TB-02 | [TB-02 #1360](https://github.com/xy7365527-lang/NewChanlun/issues/1360) |
| TB-03 | [TB-03 #1361](https://github.com/xy7365527-lang/NewChanlun/issues/1361) |
| TB-04 | [TB-04 #1362](https://github.com/xy7365527-lang/NewChanlun/issues/1362) |
| TB-05 | [TB-05 #1363](https://github.com/xy7365527-lang/NewChanlun/issues/1363) |
| TB-06 | [TB-06 #1364](https://github.com/xy7365527-lang/NewChanlun/issues/1364) |
| TB-07 | [TB-07 #1366](https://github.com/xy7365527-lang/NewChanlun/issues/1366) |
| TB-08 | [TB-08 #1367](https://github.com/xy7365527-lang/NewChanlun/issues/1367) |
| TB-09 | [TB-09 #1365](https://github.com/xy7365527-lang/NewChanlun/issues/1365) |
| TB-10 | [TB-10 #1368](https://github.com/xy7365527-lang/NewChanlun/issues/1368) |
| TB-11 | [TB-11 #1369](https://github.com/xy7365527-lang/NewChanlun/issues/1369) |

排程保持原11TB DAG，尤其TB-04→TB-09→TB-08→TB-10→TB-11。跨TB故事的共同销项集合不机械变成排程边；每张内部票只销具体子义务。全部必需成功路径、实际权限、最终独評和main/CI证据未齐前，SPEC和六Destination都保持未完成。

**旧#1327–#1333是暂停的历史设计排程**：原两方案选型与旧B组织不恢复，逐票已有当前SPEC/TB接单链接；现存原生关系暂保留作后续收口。#1096/#1304旧blocker未被提前移除，#1319/#1320/#1279继续被#1323阻塞。主图的完成判据始终是原六Destination。

## Out of scope

- 不重裁缠论概念定义或 #787 已定的严格多重赋格结构。
- 不把历史冻结臂的 alpha 读数重新解释成完整程序读数。
- 不在 map 阶段直接实装；图清后经 SPEC 人工批准再拆实施票。
- 不在本图完成数据采购、磁盘采购或与完整生产程序无关的基础设施迁移。
