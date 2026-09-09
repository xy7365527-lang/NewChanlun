> 链接转换阅读版；正文语义与原稿一致。签署原字节：[原稿](../../../../../payload/sources/newchanlun-1339-requirements-20260908/author/parts/EVIDENCE-REUSE.md)，SHA256 `4d56ffca7fd01682aca65c48510230e694a040c448cb4d2bc52ed19f46c0e9d8`。当前批准状态见归档根APPROVAL.json；原稿中的待审状态保留为发生时记录。

# #1339 既有证据复用导航（外置工作草稿）

本稿把已封存的新 #1335 静态全链扫描与 #1323 三代表链报告挂到完整需求和验收准备上。**可复用的是准确范围内的静态事实、义务处置与锁导航；没有运行验收，也不以这些成果预设 B。** 本稿不重新裁语义、不重扫源码、不读取旧 #1321/#1322/#1325/#1326 原件。

## 来源和字节核验

六个扫描文件均从 `SCAN-DELIVERY.json /canonical_current_files` 取最终 frozen 路径，SHA 全部匹配；未使用根目录同名活动草稿。FINDINGS 的 SHA 与交接一致，EVIDENCE 包 SHA 与 ROOT-RECEIPT 一致。ROOT-RECEIPT 和 SCAN-DELIVERY 的当前字节 SHA 已记录于同名 JSON；它们没有另给外部预期 SHA，不能称新增独立签署。

| 来源 | 精确路径 | SHA / 核验 |
|---|---|---|
| scan_report | `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` | `a50acf92da8f9abf878b0d9397e6a6b0dbc8fa04970ecd69aaa526fd08ffdcd1`；匹配给定/收据绑定 |
| obligations | `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` | `7469dd62fa74a188dbc7924aa75c35e1e0717dbf3414f3c49440764e56dd8466`；匹配给定/收据绑定 |
| semantics | `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SEMANTIC-CONSTRAINTS.json` | `1bb4312b02ec39d7932234c1fa8051204ac30e23a672d4398f2dcbd14786bbee`；匹配给定/收据绑定 |
| contract_readable | `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.md` | `cbbf3cc6203f2662475ff4119b2f20c16d16ef67eb8fa889dfbd60118d86ee9f`；匹配给定/收据绑定 |
| contracts | `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` | `cfd2743c373760617adab87e8cbfb7437b80159c2969e7114f2ccb86dfdb7b40`；匹配给定/收据绑定 |
| scan_handoff | `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` | `af2cc7b70c0ff1646318d5b2078de5f563835c7ee4b84e3adea68ad4a386368a`；匹配给定/收据绑定 |
| findings | `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` | `b13df8091089c38d6a4337d3ad3813e305fae8930898ec8cf5651bed9901221c`；匹配给定/收据绑定 |
| root_receipt | `/tmp/issue1323-frontend-state-requirements/ROOT-RECEIPT.json` | `16958f1e88f0e7793436751a3f2066930d53ec9a06f3c5ae8dad7b573fafd82c`；本轮记录；无另给期望值 |
| scan_delivery | `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/SCAN-DELIVERY.json` | `4d24de8692c915f95544eb7c4f041e8dd403bfda503e9fe1a97e4a5e715455ea`；本轮记录；无另给期望值 |
| three_chain_evidence_bytes | `/tmp/issue1323-frontend-state-requirements/EVIDENCE.json` | `a525b8c3c0ddf2de47dcc4a4b8e8aa6ea6ff50c88e9b64fe8426aa8791f20907`；匹配给定/收据绑定 |

**版本与单位：** #1335 扫描原源码为 `644075a6abcbfcda74815daed68e06eff6ddaf63`，候选语义为 `7af65b34ec7f3cbb445a420263b82f39844ce05c`；三链报告原记录 HEAD 为 `6df8d1921c72c84da90987f8f6c02ebc11fe6aac`。本轮核的是制品字节，没有重验其引用源码。171 合同、94 锁射程单元、91 源绑定分别是不同单位；均不是正确实现数或通过测试数。冻结扫描最终为 0H/0M/1L，LOW 是报告否定句残留旧 168/89 计数，原件不改。

## 已查事实、未建立结论与有限补查

下列每个 EV 域的完整 `source_path + line_range/json_pointer`、合同 ID、锁 ID 和原 evidence ID，均在配套 JSON 中。这里的“已查”一律指原成果所核版本；运行状态统一为 **not-run**。

### EV-001 收据、版本与覆盖单位

复用最终封存静态扫描与三代表链报告的证据范围，保留历史版本、未读及运行未知。

已查：#1335为171合同/94锁射程单元的静态扫描交付；其0H/0M/1L是制品评审，LOW是冻结报告否定句旧168/89计数残留。；三链报告原收据记91个来源字节绑定、0失败；本轮核报告/收据及EVIDENCE包字节，未重验这些原文件。

未建立：171合同不等于171个正确实现，94锁不等于94次测试通过。；#1335的源码版本644075a6、候选语义版本7af65b34与三链报告HEAD6df8d192不同；任何源事实都不自动当今日源码复验。；828条R有字面context不等全文精读；1194条R无字面context继续是未读范围，不能称无关或缺件。

有限补查：只对后续真正用作当前断言的合同核版本/变更范围；无漂移及无新问题时复用原报告，不重做全链扫描。

合同导航：以收据/需求处置为准。

准确来源：
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/SCAN-DELIVERY.json` `/counts_and_units`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/SCAN-DELIVERY.json` `/unverified_or_not_claimed`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/SCAN-DELIVERY.json` `/retained_low_findings/0`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L7–L31`
- `/tmp/issue1323-frontend-state-requirements/ROOT-RECEIPT.json` `/source_file_bindings_checked`
- `/tmp/issue1323-frontend-state-requirements/ROOT-RECEIPT.json` `/three_representative_chains_only`
- `/tmp/issue1323-frontend-state-requirements/ROOT-RECEIPT.json` `/all_project_execution`

### EV-002 已有需求约束的可复用导航

16组SEM与10条原文关系是既有要求和来源标签的导航，不能成为本轮全定义需求白名单。

已查：既有权威分层、同判据、持久重、三方向、资金与量的角色、净毛约束和未定量均分项保留。；OBLIGATION-DISPOSITIONS将义务未证、事实疑点、表示问题、缺材料、运行未知分栏。

未建立：这是原成果所用要求的处置，不是本轮对当前正本/110课的全覆盖精读。；不能把候选合同关联当符合性证书，不能由代码标签新定语义、Owner、资金布局或规则。

有限补查：由主线新增完整需求表对齐当前正本；发现遗漏时只补该语义或验收域，保留本表作为已查材料导航。

合同导航：以收据/需求处置为准。

准确来源：
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SEMANTIC-CONSTRAINTS.json` `/items`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SEMANTIC-CONSTRAINTS.json` `/original_relations`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L83–L93`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L16–L22`

### EV-003 行情身份、时钟、会话与层级输入

生产者/消费者可比前提包含symbol/合约、行情来源、原始与合并索引、实际时间、结构级别及会话。

已查：缓存读合并写、REST/WS参数差异、Theta量化合成时钟、按TF各自创建引擎的回放路径均有静态合同。；显示TF、结构递归level、操作级别和Voice depth已有区分。

未建立：没有统一行情身份/时钟/观察版本的运行契约验收。；同TF或同裸索引不证明同结构级别/同一输入；TF并行回放不是结构递归。

有限补查：确定本轮拟支持入口后，按入口各取一个身份/时钟/版本样本核字段映射；先用既有合同，只有新增/变动边界补读。

合同导航：IO-C01, IO-C02, KERNEL-C006, KERNEL-C016。

准确来源：
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L35–L45`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L7–L9`
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L14–L14`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/0`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/1`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/140`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/150`

### EV-004 库能力与有效挂载的区分

结构分类、同级别分解、证书簿和managed授权有实现或生产体，但每个实际入口的挂载与消费必须单列。

已查：OperationSequence库与pipeline挂载存在；所核Owned流式入口传空levels，因此该通道未生效。；E2eO/managed有实际消费调用；候选和链证书保终态历史，独立active-nest/retrace有状态与恢复合同。

未建立：库输出不等挂入所有生产入口；managed返回或授权不等实际策略决策写回。；结构向上构造、同级别分解、多级别联立、主动定位闭环不能互替。；active-nest/retrace模块可见、重导出或手工测试不证明主入口已消费；fresh和incremental不互为完整历史oracle。

有限补查：只为完整需求表中所选入口核有效挂载/返回消费；不重查已明确OperationSequence空挂载，不以这条断边断言全仓无等义实现。

合同导航：KERNEL-C008, KERNEL-C009, KERNEL-C011, KERNEL-C012, KERNEL-C013, KERNEL-C014。

准确来源：
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L43–L49`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L8–L9`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/5`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/7`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/142`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/143`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/145`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/146`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/147`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/148`

### EV-005 走势状态：Rust生产输出到UI的断边

三链R1已有Rust MoveBlock构造与Classification输出；现有图表WS另走Python，快照消费者没有安装状态。

已查：Rust MoveBlock含kind/dir/status/level_lift，Active尾和Classification.levels[].moves实际构造已查。；所核WS生产源为Python RecursiveOrchestrator；Python move快照确实序列化；两个UI hook只log snapshot。

未建立：不能说后端完全没有走势分类；不能说UI已显示同一份Rust生产状态。；Python高层快照、overlay和Rust输出不能拼作一次一致观察链；未做当前服务/UI运行验证。

有限补查：以已知断边设未来验收：同一对象/切面从生产输出到传输再到显示逐字段可追溯；需求阶段补齐需显示的其它分类轴，不重复R1扫描。

合同导航：KERNEL-C002, KERNEL-C004, KERNEL-C017, IO-C06。

准确来源：
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L20–L24`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L39–L47`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/136`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/138`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/151`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/5`

### EV-006 小转大：必要条件、生产门与事件时间

三链R2已查必要条件行的真实生产路径和信息丢失，不能把候选、准入门或无输出解释成发生/未发生。

已查：XiaozhuandaCandidate与RunResult.turn_class_rows有生产链；启用门且需要重建索引是产出条件。；当前投影缺完整c′、证书主键、确认/获知时点及修订历史；所核HTTP/WS/UI未接。

未建立：必要条件不保证大级别转折完成；经济XzdEvidence::gate_pass不是UI现象确认。；bar=turn_source不能冒充获知时点；门关闭/未重建/无命中不能统一视为不发生。；产量、非空样例、按当时可知回放未运行。

有限补查：先在验收表固定必要条件与完成事实的独立语义及时间字段；后续仅补所选生产入口的证据保真和非空用例，运行留待批准的验证阶段。

合同导航：ACT-C037, KERNEL-C013。

准确来源：
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L26–L30`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.md` `L79–L79`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/72`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/147`

### EV-007 中枢扩展：关系/升档输出与完整高级结构

三链R3已核核心分离且外缘重叠判据与level_lift输出；高级别中枢全构造、血缘及发布仍未认证。

已查：关系判据和level_lift=1的盘整块构造已查，下一级输入另取lc.upper_moves。；HTTP overlay另算Python结构，UI画各级中枢框，未接Rust扩展关系及对象前后关联。

未建立：level_lift不是完整操作升档或高级中枢血缘发布证明；矩形变大/多一个框不是扩展事件验收。；本轮不能从R3得出所有高级中枢构造缺失，也不能声称全构造已完成。

有限补查：这是值得后续定点补查的链：只沿已知upper_moves入口到高级结构/血缘生产与出口，说明何时获知及修订关系；不因此重扫所有分类器。

合同导航：KERNEL-C008, KERNEL-C017。

准确来源：
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L32–L36`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L45–L49`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/142`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/151`

### EV-008 传输、UI消费与撤回/upsert

后端结构diff、传输包和UI状态写者是三个不同合同；已有事件或文档不证明完整撤回安装。

已查：Python结构diff与TaggedEvent有生产者；BSP身份diff有局部锁。；UI marker追加、move撤回忽略、move几何diff不全已定向查过；REST/WS多个写者和overlay返回校验边界已扫。

未建立：pure diff不是持久历史，BSP局部锁不覆盖move和全部几何/UI消费。；文档upsert_semantics不能当已接通收据；f64 first-seen追加集不提供旧前缀撤回。；现有消息格式不证明同版本完整payload、去重、顺序和旧代际隔离。

有限补查：将现有断边直接变为验收场景：同ID修改、撤回、重建、重复/乱序/旧会话消息、完整几何更新；需求阶段只查契约缺项，不先写UI。

合同导航：IO-C06, KERNEL-C003, KERNEL-C019, C-LOCK-009。

准确来源：
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L24–L24`
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L36–L38`
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L46–L49`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L39–L41`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/5`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/137`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/153`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/106`

### EV-009 回放、断线与结构恢复

重算、全史重放、快照反序列化、断线续接及交易回执恢复各自限域；现有独立恢复壳不补主图断边。

已查：REST创建会话未绑定该UI WS、seek字段不一致、WS断连不销会话/停播放均有源合同。；按TF回放预先构造全数据高TF bars；独立LiveEngine有gap/重连机制；观察dump和回放文件寿命有扫描记录。

未建立：独立LiveEngine定义不等主生产接线；快照恢复/journal fallback不保证完整live状态相同。；最终修订重算不能代替当时可知回放；目前未验断线续接、断档重取、同会话控制、重启恢复。；诊断dump双跑相同也不等客户端历史发布相同。

有限补查：先区分恢复对象/保留状态/失效范围，再为拟支持入口列切面+游标+时点验收；定点核相关实际consumer，运行验证另阶段。

合同导航：IO-C02, IO-C07, KERNEL-C016, KERNEL-C014, ACT-C034。

准确来源：
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L30–L30`
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L38–L38`
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L48–L48`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L39–L41`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L8–L9`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/1`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/6`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/150`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/148`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/69`

### EV-010 完整N重与独立权利的证据边界

既有扫描保留持久重、一次voice、结构layer/chain/campaign之间的身份、寿命和资金差异。

已查：多个家族的生产体与必要consumer已扫；run-local账、树/链、父结算和共享pool/两书并存的实际边界有合同。；已明确每重身份/独立现金成本筹码与全局上限并存，重内不穿零、重间可反向且不仲裁。

未建立：voice/chain/campaign类名或结构层数不等持久重，不能据此宣称N重全经营已实现。；13个positional模式家族不是13种同义完整多重赋格实现；共同helper/trade字段不能合并它们的权利。

有限补查：本轮完整赋格要求直接挂到相应义务与合同，不重读所有家族；只有候选评估必须回答的当前identity/lifetime消费缺口才补读。

合同导航：ACT-C020, ACT-C038, POS-C004, POS-C005, POS-C011。

准确来源：
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/3`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/6`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/8`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L61–L67`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L91–L91`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L19–L19`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/55`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/73`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/126`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/127`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/133`

### EV-011 毛约束、净额外效、阶段与量

按重毛计提与净省禁复用是既有要求；纯代数API、净目标镜像、模拟fill和venue经济事实必须区分。

已查：SeparateLedger有代数谓词/净投影，DualLedger有分腿fill API而主π填单不消费，模拟cash/fee/拒开/强平与TW实际来源已查。；CampaignBook阶段/核销/增股是独立模型轨，不能回填主pending fills。；行情升档部分保留与资金容量溢出分重已分开；未定规模、频率阈值和其它钱规则保持未定。

未建立：代数守恒和物理NETTING不证明逐重完整财产权/毛funding/borrow或交易所逐仓保护。；共用阶段/cash helper不证明主引擎经营阶段闭环；不能从默认参数裁定新钱规则。

有限补查：将需求分成经营权利账、共同毛约束、净额外效及外部回执映射四项验收；后续只核拟保留入口缺失的逐腿consumer或在途结算，不重做代数扫描。

合同导航：ACT-C009, ACT-C025, ACT-C026, ACT-C036, ACT-C038。

准确来源：
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/9`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/10`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/14`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/15`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L57–L59`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L91–L93`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L20–L22`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/44`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/60`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/61`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/71`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/73`

### EV-012 订单、部分成交、退出及在途尾账恢复

真实出口、模拟fill、结构终态和经济结算不互相代替，退出不能只删结构或清UI。

已查：maker部分成交/取消、SQLite日志与全史重放、NETTING目标差下单、主策略无逐腿回执回填等实际合同已查。；结构摘挂先于经济拒绝且无统一回滚；CampaignBook满平会清除模型/核销，与venue pending不同。；在飞表API存在与被当前主链消费分开；typed闭合不自动关闭持久registry。

未建立：start未recover与journal延迟commit是静态边界，不等运行故障报告。；stop close_all_positions、结构清空或模型核销不能证退出后所有成交/费用/更正都结清。；未运行取消后晚到成交、重复回执、部分成交后退出、崩溃恢复等验收。

有限补查：把完成经营与继续接收尾账分别写入需求；后续围绕选定出口做一条从退出到晚到回执/费用修订的窄链补查，不重扫其它出口。

合同导航：ACT-C003, ACT-C005, ACT-C006, ACT-C019, ACT-C038。

准确来源：
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L51–L59`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L9–L10`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L28–L29`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.md` `L47–L48`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.md` `L80–L80`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/38`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/40`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/41`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/54`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/73`

### EV-013 观察、诊断、准入与经济联合条件

观察/候选/确认/授权/准入/执行属于不同角色，容量、频率、经济及制度条件不可被统计结果或结构证书代替。

已查：typed准入实际消费、诊断信号与生产门、μ/LCB/χ训练目标与venue账区别已有静态核查。；TargetAttributes/容量与统计输出存在，测量至真实联合门不自动闭合。

未建立：诊断更宽不许参与准入；UI显示候选不新授交易权。；统计成功、历史绩效、cap默认值不证明实测容量/频率或最低可操作级别。；不把独立联合条件改写成新规定执行顺序。

有限补查：按每个输出登记用途资格与消费门；后续只核本轮新增的联合门输入及测量出处，保持钱规则留白。

合同导航：KERNEL-C012, KERNEL-C013, ACT-C033, ACT-C031, ACT-C035。

准确来源：
- `/tmp/issue1323-frontend-state-requirements/FINDINGS.md` `L46–L51`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/7`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/12`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/OBLIGATION-DISPOSITIONS.json` `/items/13`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L69–L73`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L17–L21`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/146`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/147`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/68`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/66`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/70`

### EV-014 锁、非空性和运行验收的上限

已有锁与形式化前件应复用其准确射程，不重复开发同名锁，也不把静态assert或共核对拍当完整链通过。

已查：94锁单元记实际assert/Lean前件、非空/注入/退化、feature/CI选择及执行状态。；#1087包含postscan/FNV/字段装配价值；同核fresh重跑、probe投影、Missing共占位和Prelude只比同值的界限已查。；BSP diff、C2 cutoff与非空局部fill/TW锁可作为将来验收组件；stream/batch有共享输入与ExpectedRule级联限制。

未建立：所有项目/测试/Lean/CI/服务/交易/回放/browser/SDK/API执行均not-run。；完全分类形式化是给定前件下的结论，不构造Rust实现；空Type可能性及入口有效域必须保留。；局部非空与共享影子并不补全全定义、逐字段、独立实现或交易全链验收。

有限补查：在完整验收表逐项复用锁ID及实际覆盖域，再列真正缺的非空、因果、修订和多重尾账用例；批准实施验证后运行所需最小集合。

合同导航：C-LOCK-001, C-LOCK-003, C-LOCK-004, C-LOCK-009, C-LOCK-015, C-LOCK-021, C-LOCK-023。

准确来源：
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/SCAN-REPORT.md` `L95–L101`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L12–L12`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/NEXT-STAGE-HANDOFF.md` `L31–L31`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/SCAN-DELIVERY.json` `/counts_and_units`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/SCAN-DELIVERY.json` `/unverified_or_not_claimed`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/98`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/100`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/101`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/106`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/112`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/118`
- `/Users/silencehan/Projects/NewChanlun-independent-1334/artifacts/scan/fanin/candidate-r2-preparation/20260906T130901299141Z/parent/CONTRACT-SEMANTIC-DISPOSITIONS.json` `/contracts/120`

## 16组既有要求的导航（不作为全定义白名单）

各行来源分别为 frozen `SEMANTIC-CONSTRAINTS.json /items/i` 与 `OBLIGATION-DISPOSITIONS.json /items/i`。原始 requirements 来源行及 SHA 只沿用冻结材料中的出处，本轮未重新读取或认证其原文。当前正本与完整赋格语义由主线独立整理；这里仅节省重复取证。

| 约束 | 对应要求 | frozen 处置指针 | 已查合同导航 |
|---|---|---|---|
| SEM-01 | 语义证据权威边界 | `/items/0` | C-LOCK-010, C-LOCK-013 |
| SEM-02 | 同一判断的宽严规则 | `/items/1` | C-LOCK-004, C-LOCK-008, C-LOCK-023, KERNEL-C013, POS-C011 |
| SEM-03 | 同判定跨级语义一致性及例外 | `/items/2` | KERNEL-C008, KERNEL-C016, POS-C009, POS-C010 |
| SEM-04 | 既定领域单位与术语区别 | `/items/3` | ACT-C020, ACT-C038, POS-C004, POS-C005 |
| SEM-05 | 既有范围要求与保证金口径 | `/items/4` | ACT-C006, ACT-C009, ACT-C026 |
| SEM-06 | 既定分解语义、多重并存与三个方向 | `/items/5` | KERNEL-C009, KERNEL-C012, ACT-C027 |
| SEM-07 | 独立资金、不等式及身份键 | `/items/6` | ACT-C010, ACT-C020, ACT-C026, POS-C011 |
| SEM-08 | 区间套主动定位目的与经济条件 | `/items/7` | KERNEL-C013, KERNEL-C030, ACT-C033, POS-C006, POS-C009 |
| SEM-09 | 重内同向与重间反向的对象边界 | `/items/8` | ACT-C008, ACT-C009, ACT-C025, POS-C011 |
| SEM-10 | 记账规范、净额中间态及历史有效域 | `/items/9` | ACT-C025, ACT-C026, ACT-C036, C-LOCK-021 |
| SEM-11 | 量问题的范围及两个子问题 | `/items/10` | ACT-C009, ACT-C019, ACT-C025, ACT-C035 |
| SEM-12 | 已采纳的量规则选择 | `/items/11` | ACT-C008, ACT-C013, POS-C004, POS-C008 |
| SEM-13 | 规模容量判据及落地条件 | `/items/12` | ACT-C031, ACT-C035, KERNEL-C024, AUX-C019 |
| SEM-14 | 开重频率条件 | `/items/13` | ACT-C031, ACT-C035, C-LOCK-022 |
| SEM-15 | 空仓不挪用、回本后增长及容量换档 | `/items/14` | POS-C010, ACT-C009, ACT-C038 |
| SEM-16 | 原文未给出的量与适用范围 | `/items/15` | ACT-C031, ACT-C035, POS-C009 |

## 本轮应保留的空缺分类

- **尚未全量整理的需求：** 全定义对象/分类轴/关系/事件与完整多重赋格联合验收；16组、10关系和三例均不是边界。
- **已定向静态查过：** R1/R2/R3 的库生产、真实输出、HTTP/WS与UI断边；同级别分解有效挂载；身份/寿命、模拟账/venue账、测量/准入、锁的射程。
- **尚未认证的具体链：** 高级中枢完整构造及血缘发布、拟支持入口的完整观察快照与游标修订、完整N重经营/退出/在途尾账与恢复。既有报告保留这些缺口，不可补写为已完成或断言全仓绝无实现。
- **今日源码未复验：** 旧静态事实若成为当前架构评估前提，先核所用合同源版本和变更；仅重读真实变化与新疑问。
- **运行未做：** 所有项目/测试/Lean/CI/服务/交易/回放/浏览器/SDK/API；非空产量、因果回放、断线、崩溃、晚到回执、性能也没有本轮结果。
- **已有但未读与缺源分开：** 1194条R无字面context及其它未读范围保持未读；未供/隔离材料按原扫描边界，不能给它们新覆盖信用。

## 接回主线的顺序

1. 由本轮正本/赋格与观察契约工作汇成完整需求验收表；本导航直接填入已查静态证据，避免重扫。
2. 为尚未覆盖条目区分新需求遗漏、旧资料未读、当前版本未核、源不可得与not-run。
3. 仅在完整高级中枢血缘、选定入口实际挂载、观察一致性与退出在途尾账等具体缺口补读有意义producer/consumer；不重新通读171合同来源。
4. 用户确认完整范围后，用同一组场景重评三候选；之后方向/spec/实施/运行验收按主线批准门推进。

所有封存原件和主仓文件均未改动；本交付只生成 `EVIDENCE-REUSE.md` 与 `EVIDENCE-REUSE.json`。

## ST / FG / OB 汇合规则与领域关键词

**逐项信用只授予具体已查断言。** 同域合同、关键词和既有义务关联只能导航，不能把整条 ST/FG 要求标成“实现已查”。“未查/未认证”也不等于“缺实现”。OB 关联来自本轮根代理的观察契约草稿，只表明同域，不是冻结证据或验收签署。

| EV | 领域关键词 | 同域 OB | 授予证据信用的边界 |
|---|---|---|---|
| EV-001 | 验收范围、版本、读过与未读、静态与运行、计数单位 | OB-020 | 本域只证明收据和覆盖单位，不直接证明任一ST/FG实现。 |
| EV-002 | 权威、SEM、原文关系、义务处置、同判据 | OB-003, OB-020 | 仅既有规范/义务导航；ST/FG逐项语义须对当前正本，逐项实现须另找合同原证。 |
| EV-003 | 数据身份、输入版本、raw/merged/source索引、结构级别、操作级别、Voice depth、会话代际 | OB-001, OB-002, OB-015 | 数据与时钟合同已定向静态查；任何完整ST/FG身份贯通或同版本保证仍仅同域导航。 |
| EV-004 | OperationSequence、结构塔、同级别分解、联立、主动定位、证书终态、有效挂载 | OB-002, OB-004, OB-005, OB-010, OB-011 | 给定库体、Owned空挂载和证书/授权消费是直接静态事实；完整ST/FG三方向闭环仅同域导航。 |
| EV-005 | 走势类型、盘整/上涨/下跌、MoveBlock、Active尾、Classification、snapshot handler | OB-003, OB-005, OB-015, OB-019 | R1已直接定向查Rust构造→独立Python传输→UI只log；其余走势分类轴/有效域不能借R1取得逐项实现信用。 |
| EV-006 | 小转大、必要条件、XiaozhuandaCandidate、turn_class_rows、获知时点、确认、准入 | OB-003, OB-009, OB-010, OB-013, OB-017 | R2已直接定向查必要条件产出门、字段投影和UI未消费；完整小转大完成/产量/因果回放仍未认证。 |
| EV-007 | 中枢扩展、核心分离、外缘重叠、level_lift、upper_moves、高级中枢、血缘 | OB-004, OB-005, OB-009, OB-010, OB-019 | R3已直接定向查关系/升档块及HTTP/UI断边；完整高级中枢构造和FG主体升档只能作同域导航。 |
| EV-008 | 状态变更、结构diff、upsert、撤回、同ID修订、几何字段、乱序/重复、UI安装 | OB-004, OB-008, OB-014, OB-015, OB-019 | move撤回忽略、几何diff不全和BSP局部diff锁是直接静态事实；所有ST/FG对象的完整变更/撤回实现仍仅同域导航。 |
| EV-009 | 回放、当时可知、后来修订重算、断线续接、游标、快照、重启、缓存恢复 | OB-006, OB-007, OB-009, OB-015, OB-016, OB-017 | 指定REST/WS断边与独立恢复壳寿命已定向静态查；完整ST/FG因果回放/断线/崩溃恢复没有逐项运行证明。 |
| EV-010 | N重、持久重、独立资金、专属筹码、独立权利、voice、chain、campaign、重内合成、重间不仲裁 | OB-011, OB-012, OB-014 | 只对所核家族的账/树/链/寿命和共享池给直接静态事实；不把家族/合同整体关联当FG完整N重逐条实现证明。 |
| EV-011 | 毛计提、净额外效、净省禁复用、SeparateLedger、DualLedger、TW、回本、增股、容量溢出、未定量 | OB-011, OB-012, OB-014 | 纯代数/API、模拟fill来源和主π不消费DualLedger是直接静态事实；FG全部毛约束/阶段/独立经济权利只有要求导航，不能据此报已实现或缺实现。 |
| EV-012 | 订单、部分成交、取消、退出、在途尾账、晚到成交、journal、恢复、费用更正 | OB-012, OB-014, OB-016, OB-017 | 指定maker/journal/NETTING/campaign/registry边界已静态查；完整FG退出/尾账闭环尚无逐项证据与运行验收，不等全仓无实现。 |
| EV-013 | 观察、诊断、授权、准入、经济、容量、频率、T+1、测量、最低可操作级别 | OB-007, OB-010, OB-011, OB-013 | 所核typed门、诊断产出、统计生产与消费范围已静态查；FG完整联合门仍只是需求关联，测量产量与门有效性not-run。 |
| EV-014 | 锁射程、非空、Lean前件、feature/CI、共核对拍、Missing、ExpectedRule、逐字段、因果 | OB-020 | 指定assert/feature/前件的源码射程是直接静态事实；不能迁移为ST/FG逐项通过，所有运行仍not-run。 |
