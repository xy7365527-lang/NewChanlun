# #1371 TB-01-B：R7 Codex独立产品评审

**总结果：FAIL。评审完成，候选不能验收。** 两个消费者缺陷：漏核 `replaces` 内容；Gap重建未绑定本次Gap会话。已审完8 AC与17条来源子义务，不继承旧PASS。

候选 `4922543038fbcfef1a4d711a306708ace3f62e88`；基线 `2212eba31e0b7419907e622e79f02f7c2cc37706`；manifest SHA256 `543cba068be3f3da7b215670b88712ce8e6065ff9222d452a3a7536ba0cafeae`。

会话：`/root/r7_codex_product_review`；runtime：`codex_native_subagent`。这是新的R7，非Prime，非R6同ID。底层精确模型/推理档没有可核回执，JSON明确留空。#1448仍未满足。

## 发现

### R7-M1 · MEDIUM · 增量消费者不核对 replaces 成员，丢失或篡改替代清单仍提交成功

触发：在真实健康会话由 cut-3 的旧TOP消费更正 cut-4 时，只清空 delta.replaces，或只把其中 old_object_id 改错；upserts、withdrawals、relations、catalog_evidence、同cut State均保持健康值。

源码：inputs/candidate/s_session/browser/index.html:388–403, 579–595, 641–652。

后果：页面推进到 generation4，并显示“全字段一致”。替代清单这一已列语义字段损坏未被识别；最终六集合恰能从其他冗余字段恢复，不能证明这份Delta完整。正常Rust/HTTP输出没有被指控会自行制造该错包。

建议：提交前从本批撤回/替代对象与replaces关系提取完整预期替代项，对replaces逐字段、唯一性、成员数和端点/代际进行对应校验；任一遗漏或错误保持原游标与完整缓存，修复后用相同健康对照和单字段反例验证。

证据：E001、E002、E003、E004、E005、E006。独立静态审阅确认；另读取并重算根代理原HTML模拟DOM反例数据。此子代理未执行产品/Node测试。

### R7-M2 · MEDIUM · Gap 重建未将指定cut快照绑定到本次Gap的会话身份

触发：当前页先绑定真实会话A/cut3，取得真实会话B、rebuild_cut=cut-1的Gap；随后指定as_of=1读取收到完整自洽的A/cut1。合法对照是同一个Gap取得B/cut1，允许正式迁移到新会话。

源码：inputs/candidate/s_session/browser/index.html:515–539。

后果：Gap分支在普通流的session检查之前执行，只检查重建模式/cut，随后无条件重绑。错配A/cut1也提交，页面与游标不属于Gap声明的B会话；cut数字相同不构成同一逻辑切面。

建议：在Gap分支要求Gap自身会话身份完整，并将重建State的会话及合同要求的发布身份绑定到本次Gap；通过全部验证后一次提交。仍允许合法新会话B重建，不以旧会话A永远不变代替该检查；错配或缺失须保留旧缓存并允许同游标重试。

证据：E007、E008、E009、E010、E005、E006、E011。独立检查源码及两份未改字段的真实HTTP原件；根代理原HTML模拟DOM以合法跨会话重建作正控，错配仍提交。不是正常服务错投事实，不是真实GUI反例。

## 八项AC

PASS只表示该行的明示本票子域；总评仍受FAIL行约束。

| AC | 结果 | 判断与证据 |
|---|---|---|
| AC-1 | PASS | 旧正式launcher原始回执仍在报告附件；同字节launcher/Rust/readonly启动真实单点后，原身份输入链推进TOP→更正RISING→追加TOP。R7另逐算native run2四比较、三raw见证；Begin短事务先提交后调用唯一ParseLayerIncr及local_shape，无前端补判。 E012、E013、E014、E015、E016、E017 |
| AC-2 | PASS | 接纳序与源坐标分离；同identity/revision同内容返回原receipt，同revision异内容明确冲突，更高revision指向前版。内容/依据变化产生新内容身份，经replaces关联原对象；旧TOP原身份、first_known和见证保留，AsKnown撤去未来撤回元数据。 E018、E015、E016、E017、E019 |
| AC-3 | FAIL | 健康持久Delta和真实GUI路径成立，提交事务覆盖对象/边/端点，服务端完整核七集合；但浏览器replaces内容漏校验且Gap可提交另一会话同数字cut，违反完整变化与指定同cut原子应用。 E020、E021、E022、E017、E023 |
| AC-4 | PASS | 三处实际SIGKILL均有真实PID/命令/DB路径和wait_exit=-9；前两臂原库保留Begin、发布根仍cut1（after_batch另有不可达批次），后commit臂已是cut2且无回执。recover严格提升epoch并保留原接纳身份，后续恢复与各自不中断control全部S视图相等。 E024、E025、E026、E027、E028、E029、E030、E031、E032 |
| AC-5 | PASS | Commit后stdout尚未返回即SIGKILL，缺回执未被当未发生；按原identity query能辨已接纳/已发布，原更正重放复用receipt。recover与发布共用SQLite写序，旧epoch写入拒绝且逻辑表不变，advance同前沿幂等不增cut。 E026、E029、E033、E034、E035、E036、E037、E038 |
| AC-6 | PASS | AsKnown按已发布cut投影完整对象/原始修订/关系/目录，接受尚未发布更正也不污染当前cut；原生GUI先缺右邻无TOP，再cut3旧TOP、cut4撤旧RISING、cut5新TOP，回看cut0与cut3不泄露未来。current明确RecomputedWithRevision并绑定该发布输入/规则/profile。 E039、E040、E041、E042、E043、E044、E045 |
| AC-7 | PASS | 正式S命令及只读外壳不启动E/B/X，scope始终CompleteCut/not_started；S独立完成更正/撤回/追加/恢复。R10同字节后端21个profile损坏副本逐个健康200→故障503，7 CLI拒绝并保持前后schema/十表/逐列指纹；源码与native35测试覆盖根/批次/Delta缺失停写。 E046、E047、E048、E049、E050 |
| AC-8 | PASS | 本次独立审阅已把三杀的正式输入、实际signal/exit、原库恢复、typed持久事实、API、GUI与同轨迹control串联。A原严格分类函数/包含函数/目录/profile的同字节有效域可有限复用，B新增恢复/版本/原子变化由本轮原件与独立源码审阅承担；已发现的消费者反例如实导致AC3及总评失败。 E051、E052、E053、E054、E027、E028、E029、E055、E056 |

## 17项来源子义务

这些是17条来源义务，故障服务的17个GUI场景是另一套集合。以下均不销完整来源ID；完整量词、其他CC及经济残余仍归原SPEC/crosswalk。

| 来源ID | 结果 | 本票判断与证据 |
|---|---|---|
| A-CC-006-03 | PASS | 固定TestOnly输入、source/rule/profile下，三杀每臂与自身control的对象、关系、见证及历史完整深等；内容身份和首知未归一化。 E057、E058、E027、E028、E029、E059 |
| A-CC-006-04 | PASS | 唯一local_shape计算四类之一、两方向与四严格比较，原始/merged三见证来自同批；R7按整数独算归档窗口并核同源GUI。 E060、E061、E062、E016、E063 |
| A-ST-044-02 | PASS | 发布事务一起写对象、withdrawal、witness与relation；当前与历史端点可追，不将撤回历史见证当活动对象复活。 E064、E065、E066、E028、E017 |
| A-ST-044-03 | PASS | 同会话已提交cursor空重试、健康继续与历史回读的S结构关系一致；三杀恢复保留原历史切面。 E067、E068、E023、E043、E029、E069 |
| A-ES-15-01 | PASS | CompleteCut明确只属于S，economic=not_started；Gap不是经济ack或经济完整。未启动经济域没有被补造历史。 E070、E071、E072、E073、E016 |
| A-ES-15-02 | PASS | 三条Begin/batch/Commit实际故障迁移具备前后记录，旧epoch与未授权Begin推进拒绝；recover先核同根再换代。 E074、E075、E024、E025、E026、E076 |
| A-RA-04-01 | PASS | 本片承接的S first_known与withdrawal历史保持原顺序和身份，查询更正前cut不能提前看到撤回。 E077、E078、E079、E043、E017 |
| A-RA-10-01 | FAIL | 健康固定切面逐字段深等成立，但R7-M2让Gap重建提交不属于该会话的相同数字cut，S续接的同切面身份不成立。 E080、E081、E082、E083、E023 |
| A-I-02-01 | PASS | 只审本票分配的E/B/X真实未启动子域；S自有持久库可正式修订、撤回、恢复和历史查询。 E084、E085、E086、E087、E027、E016 |
| A-OB-004-01 | PASS | 源revision、接纳seq、窗口源坐标、object_id及object_revision各自保留；更正新内容身份有旧对象指针，不以新first_known覆旧对象。 E088、E089、E090、E017 |
| A-OB-008-01 | FAIL | R7-M1证明replaces缺成员/错old ID仍获全字段一致；R7-M2证明Gap可混会话。已列语义字段与请求身份未完整识别。 E091、E092、E022 |
| A-OB-009-01 | PASS | 缺右邻只有知识不足；后续更正保留原raw和旧TOP；AsKnown0/更正前3不混入以后profile/目录状态/撤回。 E093、E094、E095、E096、E042、E043 |
| A-OB-014-01 | PASS | 只按票面分配的结构撤回可见性审阅：撤旧TOP保留原身份、first_known、区间、证据及理由；不存在的经济事件保持不存在。 E097、E098、E017、E099 |
| A-OB-015-01 | FAIL | pageEpoch对晚到200/503的隔离有真实GUI证明；但Gap分支绕过与本次Gap会话身份的匹配，合法新会话迁移的错配负例仍提交（R7-M2）。 E100、E101、E102、E103、E104 |
| A-OB-016-01 | FAIL | 失败保留cursor与健康同页重试已有GUI；Gap重建对错会话同cut缺少绑定，故续接最终切面仍可能错误（R7-M2）。 E105、E106、E082、E107、E108、E109 |
| A-OB-017-01 | PASS | 按实际accept/publish前沿停步，确认只在正确cut出现；三杀都与相同原始输入调度control完整S对象/关系/事件深等。 E110、E111、E112、E113、E041、E027、E028、E029 |
| A-OB-020-01 | PASS | 八AC与17来源ID逐项绑定候选、原件及规范指针；旧R6仍INCOMPLETE，seq28未过，旧丢失与可解码原件分别标明。结果不以17GUI场景代替17来源义务。 E114、E115、E116、E117、E118、E119 |

## 产品语义与证据核对

接纳事务以 `(identity_key,input_revision)` 保存原始修订和原收据，接纳序不等于源坐标。选择每个源身份的最高有效revision后按源坐标交给同一个Rust parser；raw/merged对应、四个严格整数比较及三根原始见证随同批次封存。R7没有把目录名或对象数作为结构正确性证明，而对归档Snapshot窗口重新按整数比较核对分支、比较值与见证来源。

Begin、不可变batch持久化、发布Commit是三个真实边界。Begin先在IMMEDIATE事务内核根/epoch/profile、取得token与输入前沿并落盘，然后才解释；batch另事务持久；Commit再次核同token/epoch/前沿，完整对象/边/撤回/历史/Delta及可达根同事务发布。前沿变化只撤销本attempt持有的Begin；recover先验证可达根、严格提高epoch并保留恢复记录，旧writer没有释放新owner的权限。

同源Python只读外壳在一个读事务内取得目录、快照及cut；已发布投影与未发布接纳分开。AsKnown从发布批次和首知/撤回代际重建，cut0保持未绑定初始状态；RecomputedWithRevision为当前已发布输入修订的结果，仍带其规则、profile及input frontier。它不把未来经济事件补写进过去。

本轮纯解析检查了native run2的全部465个产品CLI命令、466份HTTP记录、99份十表typed事实、46份writer连接设置、3次真实SIGKILL和3个完整control比较。文件哈希、payload长度/UTF-8、CLI/HTTP内容、batch BLOB、同前沿/失败零写及原身份query均能复核。548是含ps等在内的全部command记录数，不是548次产品测试；224个Snapshot是重复观察的解析次数，不是224个独立场景。

正常轨迹是g0→g1单点→g2两点→g3旧TOP→g4更正RISING/撤旧→g5追加新TOP；更正接纳但未advance仍保持g3旧视图。崩溃轨迹分别从初始三事件一次发布g1出发：after_begin PID18177和after_batch PID18786杀后仍g1，after_commit PID19130杀后已g2；均SIGKILL/wait=-9。前两臂恢复发布g2，最后一臂恢复不重复生成g3。完整control比较只在各自同调度中进行。

故障run3的97次请求原件逐份核request/response字节、body SHA及其native源；82组GUI记录为正常16、崩溃15、故障51。AX核了模式、cut、身份、撤回/见证、错误和重试；视觉检查6张原截图。after-batch和after-commit的seed-current部分AX较稀疏，后续seed-stream及截图补有cut/状态，未将单个稀疏捕获当整组缺证。

晚到序列必须区分：seq28 req90等待超时返回504，页面所谓new-winner捕获仍g3，不能采作通过；seq30 req94先真实返回current g5并由winner5截图确认，再释放req93旧current g3/200，页面仍g5；seq32 req97先获胜g5，再释放req96/503，页面仍g5。此证据证明pageEpoch在该两条调度下有效，不修复Gap会话错配。

R10同字节后端21个profile损坏副本按三phase逐个核健康HTTP、故障HTTP及各CLI前后指纹，全部拒绝且逻辑零写；15个accept/publish阶段AsKnown0严格JSON一致。native35项测试日志及本次读过的实际测试断言支持根/缺Delta/坏scope/整数等边界；没有把旧测试名称、作者“通过”或模拟函数return当实际进程恢复证明。

旧R6报告仍INCOMPLETE，findings为空不等完成。旧L1无输入推进、L2负cut、R3根/完整载荷/观察首版/scope、R4 profile/cut0与R5 mode/cut请求绑定，在当前源码的对应拒绝/投影及本轮实际证据中重新审阅；R5修复不意味着本轮两个新消费者缺口已解决。

旧启动链原件的精确边界：包内 `review-r5.json#/evidence_artifacts/capture-commands.json` 编码xz+base64，解码SHA `daa022a387401f5b68504c288421a2ec14f260600f900a396224139e36087002`，已保存为工作目录 `R5-capture-commands.json`。索引/1是正式launcher exit0；/4,/5,/7,/8,/10,/11,/13,/14为同库accept/advance，均exit0。/15首次Node返回1是取证脚本DOM缺项，原件保留，不计产品通过。`capture.py`解码SHA `58a147bcb40ee0eee8f66f476aec604fc07a2054a89115952a2589a9b1ae638d`，已全文读并保存。R5超时前/tmp的大矩阵仍遗失，不能以这些补取原件或作者R10材料冒称全部恢复。

根补证为单独的只读输入：原候选HTML SHA与包内一致，仅去尾部autoload并使用已有r9_consumer模拟DOM缝；三个不合法输送都被接收，两个健康正控通过。R7逐项核其输入与包内真实HTTP JSON值深等及结果缓存，不把补证写入原manifest；脚本退出0表示反例断言成立，产品结论是FAIL。

## 限制与未检查项

- R7是新Codex原生子代理；真实可见身份仅canonical task name /root/r7_codex_product_review。未取得独立平台UUID/底层provider解析回执，不伪造旧Prime session/model/thinking。
- 只读与输出路径是本轮指令约束，未宣称操作系统沙盒。未改产品/输入/原证据、未运行产品测试/服务/容器/交易、未写GitHub、未合入。纯Python解析与哈希不计产品测试。根的模拟DOM补证单列来源。
- 所有PASS均为票面明确的CC-006 local_shape TestOnly精确退化OHLC、相邻严格无包含、首方向已知、无tie子域；不关闭整个来源ID、故事、TB-01/#1323、62CC/44ST/10LC。
- F-R7两缺陷均在消费者明确输送负例中成立。当前正常后端载荷未发现这两种坏包；不把模拟DOM描述成真实GUI。
- Run2正常轨迹cut0→5，与三杀初始三事件一次cut1→更正cut2不同。control只同各自调度比较，未改generation/身份来拼接。
- Fault run3是实际浏览器连接TestOnly记录输送服务，使用真实原HTTP字节或明确字段变换；HTTP503不等TCP断连，after99 Gap重送给after3不等retention压力。seq28 req90超时504不采为通过；seq30 req93/94和seq32 req96/97实际晚到才支持pageEpoch结论。
- R5超时前/tmp完整独立profile/三杀/两活writer矩阵仍遗失。本包R10作者原件不是那些独立原件；不过R5补取capture-commands原件仍完整嵌入报告，已按xz+base64解码核SHA；不能笼统说全部R5运行回执遗失。
- A ROOT-ACCEPTANCE原文件不在冻结输入清单；A同hash严格核oracle以已核历史记录限定复用，再由本轮源码阅读与具体native样本独算支持。未将旧4/18/22等计数作为新测试数；不扩到非退化市场、tie/Sel或后续成笔。
- 未逐字审阅429份源文件或所有日志行；六变化代码文件全文已审，完整diff所有hunk核对；其他文件按READING-LOG所列hash、JSON语义解析或抽读等级。82组GUI AX结构解析并抽读关键字段，6张截图视觉检查，其余图仅hash核对。
- 全量cargo/全历史、硬件断电、全writer调度、长期增长成本、TB-01-C分页/保留/过期/慢消费、经济计划/已交付/可能调用/成交责任、全经济故障分区与ST044高级扩展均未审成完成；#1448旧R6同ID迁移仍未满足。
- 根已另开R12修改，但本报告不读取该新候选改变结论。复审须绑定新SHA及增量和对应回验证据。

各AC/来源行的额外限制详见REVIEW.json的limitations。除上述已定位缺陷外，未认定新的本票必需证据阻塞；范围外残余保持开放，不转成B的虚构缺陷。

## 精确证据索引

包内相对路径以冻结P为根。SHA均为本次读取后重算。根补证用绝对路径并明确不属于原manifest。JSON中每一行直接带完整path、sha256和location，不需要依赖此缩写。

**E001** `inputs/candidate/s_session/browser/index.html`
 SHA256 `a04ddb1c67e26207cbc9ec4bd72f7cb36fcac30accf4c3f1a49e5ee10e5de94b`；定位：L388–403只检查七字段为数组；L579–595更新withdrawals并只计数replaces；L641–647只对拍六Map后L652提交。

**E002** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L646–676后端以预期替代项校验replaces，说明其为正式载荷。

**E003** `inputs/candidate/s_session/s_readonly_server.py`
 SHA256 `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c`；定位：L204–214同类服务端校验。

**E004** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/7/acceptance_points/0：漏任何已列语义字段须可识别。

**E005** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R7-CONSUMER-PROBES/probe.cjs`
 SHA256 `a540258346829b659da90c03d74082e748cc21bc35430ac7b688a97e7ebcbe02`；定位：L14–62；实际HTML抽取、原HTTP载荷、五个对照/反例。

**E006** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R7-CONSUMER-PROBES/RESULT.json`
 SHA256 `d929f49ab8ff033c23ea388880bdd76c8348d8e09659f8f359f79865a1d1219e`；定位：/cases/0..4；before/after/requests/message/committed/transform。

**E007** `inputs/candidate/s_session/browser/index.html`
 SHA256 `a04ddb1c67e26207cbc9ec4bd72f7cb36fcac30accf4c3f1a49e5ee10e5de94b`；定位：L515–537 Gap先执行rebuildCommittedFromState；L539普通路径会话门未覆盖Gap。

**E008** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/14/acceptance_points/0，/observation_contracts/15/acceptance_points/0。

**E009** `inputs/evidence/recovered/review-r6/additional-results.json`
 SHA256 `3ae54c6a7d0df578f6aaa0d37776d1ed1be259e46e0487562fb7b7077d69c7e6`；定位：/http/1=B/cut1 State；/http/3=B/cut1 Gap，body SHA另内嵌。

**E010** `inputs/evidence/recovered/implementation-r11/api-cuts.json`
 SHA256 `342e755120668da7fbb1bc238cec81f91a31deab985689a2289af6bd536d3e3b`；定位：/results/states/1=A/cut1 State，body SHA另内嵌。

**E011** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R7-CONSUMER-PROBES/inputs.json`
 SHA256 `a81b331497650e6a9130cddc45c9f9dd274fc0ffa9702423cb2845a198102424`；定位：/s1,/s3,/s4,/watch3,/b1,/gapB；逐项与包内API原件JSON深等。

**E012** `inputs/evidence/reports/review-r5.json`
 SHA256 `58d99aece9208a525f3eb1963db6775d71fb67e119e5fcc1235d13547d893f7b`；定位：/evidence_artifacts/capture-commands.json (xz+base64)，解码SHA daa022a387401f5b68504c288421a2ec14f260600f900a396224139e36087002；/1 launcher exit0；/4,/5,/7,/8,/10,/11,/13,/14 accept/advance exit0。

**E013** `inputs/candidate/s_session/launch_s.sh`
 SHA256 `65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d`；定位：L210–225正式init/accept/advance/只读外壳。

**E014** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L1927–2042：Begin持久化→ParseLayerIncr→local_shape。

**E015** `inputs/evidence/native-acceptance-run2/records/1789207027049758000-17178-advance-step/0062.json`
 SHA256 `e4d7872ddccf1debc8a33b0adc9c830c221055cfb5511b9374b95779a242eda8`；定位：/views/snapshot, /views/catalog, /views/as_known, /views/watch, /views/query；完整JSON值，与同record目录CLI/HTTP收据逐字段核对。

**E016** `inputs/evidence/native-acceptance-run2/records/1789207065924937000-17317-advance-step/0069.json`
 SHA256 `af25dfbbe53ed70ae26677735d62a1be8886d4b169fa346c895c40c70f0c7237`；定位：/views/snapshot, /views/catalog, /views/as_known, /views/watch, /views/query；完整JSON值，与同record目录CLI/HTTP收据逐字段核对。

**E017** `inputs/evidence/native-acceptance-run2/gui-normal/publish-corr-stream-g4.ax.txt`
 SHA256 `4b3f190dba175336be14fc0b3f5563966291faabfb16c1d9c2bbf1bdd687e14d`；定位：AX文本物理行 68,72,74,167,169,356,362；浏览范围、对象identity/revision/first_known、源修订与Watch状态的AX行；具体元数据/内容以本文所列定位。

**E018** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L1294–1529，L1570–1606，L1639–1770，L2268–2442，L3025–3135。

**E019** `inputs/evidence/native-tests/test.log`
 SHA256 `566145f92fcf244bc25a903efe561d10411bcf84cf4bc7140d6680ab8fb91163`；定位：test tests::revision_replay_conflict_and_old_version_no_revive ... ok；test tests::out_of_order_late_revision_rejected ... ok；本轮归档35测试。

**E020** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L2190–2556持久批次和发布事务；L565–766完整delta验证。

**E021** `inputs/candidate/s_session/s_readonly_server.py`
 SHA256 `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c`；定位：L1107–1155单读事务及统一错误边界。

**E022** `inputs/candidate/s_session/browser/index.html`
 SHA256 `a04ddb1c67e26207cbc9ec4bd72f7cb36fcac30accf4c3f1a49e5ee10e5de94b`；定位：L388–403,L515–539,L579–652。

**E023** `inputs/evidence/native-acceptance-run2/gui-normal/g5-empty-stream.ax.txt`
 SHA256 `5ffb21ba432bdcf874297d0a72545b698d635cc5e1290d7a1d43738ce0ede110`；定位：AX文本物理行 68,73,75,165,167,260,262,466,468；浏览范围、对象identity/revision/first_known、源修订与Watch状态的AX行；具体元数据/内容以本文所列定位。

**E024** `inputs/evidence/native-acceptance-run2/records/1789207245791632000-18158-crash-prepare/0041.json`
 SHA256 `a9741274727dbd944d5e26919fe75ff3b6d48ea552dd3a3e77d498218c5e0f6b`；定位：/pid, /signal, /wait_exit, /stdout, /stderr；payload路径按本收据定位。

**E025** `inputs/evidence/native-acceptance-run2/records/1789207389299485000-18769-crash-prepare/0041.json`
 SHA256 `552b67c442b35208e29986297d2ad7fd2bd2f399c5760f7ad141aa5f6c6c3f4c`；定位：/pid, /signal, /wait_exit, /stdout, /stderr；payload路径按本收据定位。

**E026** `inputs/evidence/native-acceptance-run2/records/1789207481717891000-19112-crash-prepare/0041.json`
 SHA256 `9ac4b6735e2371538dc1a71d35054129139bdf83f193dbb759b661801355a48d`；定位：/pid, /signal, /wait_exit, /stdout, /stderr；payload路径按本收据定位。

**E027** `inputs/evidence/native-acceptance-run2/records/1789207266735576000-18304-crash-recover/0127.json`
 SHA256 `37284b01a20768e036bdfb6676da8e6e1b6e8b390956ca486f73e80e91f376c2`；定位：完整 actual/expected 各视图逐字段深等；非仅 full_views_equal 标签。

**E028** `inputs/evidence/native-acceptance-run2/records/1789207406926805000-18840-crash-recover/0111.json`
 SHA256 `1e2eac24ce6a2621a005ae00fc2117178775b734a61e61df2c35fa879ae0a7ba`；定位：完整 actual/expected 各视图逐字段深等；非仅 full_views_equal 标签。

**E029** `inputs/evidence/native-acceptance-run2/records/1789207503956583000-19224-crash-recover/0114.json`
 SHA256 `38580fd8f8cb19591dbc914fd85641b58c676368b021089bd88fa4351820cbba`；定位：完整 actual/expected 各视图逐字段深等；非仅 full_views_equal 标签。

**E030** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L1896–1994,L2210–2240,L2524–2568,L3643–3713。

**E031** `inputs/evidence/native-acceptance-run2/gui-crash/after-batch-killed-stream.ax.txt`
 SHA256 `f9db2b4fb490c65b738eaf847cb45796f77d350af1f2578a018b2093a339a639`；定位：AX文本物理行 68,72,74,246,248；cut-1 / AsKnown@1与无新Delta。

**E032** `inputs/evidence/native-acceptance-run2/gui-crash/after-commit-killed-stream.ax.txt`
 SHA256 `3896ea454f6cc1fb102fa1625be7c13b2ab842587e9ea3f1a4aaa8dd3f016326`；定位：AX文本物理行 68,72,74,167,169,356,362；cut-2 / 撤旧TOP / 原始revision链。

**E033** `inputs/evidence/native-acceptance-run2/records/1789207503956583000-19224-crash-recover/0045.json`
 SHA256 `5dd4d7592142410e11deeca989974a3431195336f3407a71ee8fb942f66859ef`；定位：/argv,/exit=1,/stderr/utf8：旧epoch advance拒绝。

**E034** `inputs/evidence/native-acceptance-run2/records/1789207503956583000-19224-crash-recover/0049.json`
 SHA256 `95f69ffa511fcf6e096880561d613adf88806dd92cc8853bcfbc42874523da5a`；定位：/argv,/exit=1,/stderr/utf8：旧epoch accept拒绝。

**E035** `inputs/evidence/native-acceptance-run2/records/1789207503956583000-19224-crash-recover/0053.json`
 SHA256 `df648e5e2e9071ec136ac382a018f9d85bd887db44fcebd01564e01865ca3459`；定位：/stdout/utf8：idempotent=true、generation2及DeliveryUnknown说明。

**E036** `inputs/evidence/native-acceptance-run2/records/1789207503956583000-19224-crash-recover/0057.json`
 SHA256 `430c583d5c08caab948d694894476208b11fb36590e3ec88a3feb2cf60ca618f`；定位：/stdout/utf8：identity e2/revision2/原receipt replay。

**E037** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L1332–1338,L1927–1964,L3428–3502,L3643–3713。

**E038** `inputs/evidence/native-acceptance-run2/records/1789207503956583000-19224-crash-recover/0102.json`
 SHA256 `5e7d6fd8b0220882d8d32893b4e168f576717e4b234d9de933de2d3d69aea19c`；定位：/views/query：原identity查询结果、latest_published及records；同目录0045/0049旧epoch写命令、0053/0060幂等推进、0057原receipt replay均另见精确收据。

**E039** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L2855–3135,L3217–3426。

**E040** `inputs/candidate/s_session/browser/index.html`
 SHA256 `a04ddb1c67e26207cbc9ec4bd72f7cb36fcac30accf4c3f1a49e5ee10e5de94b`；定位：L405–475。

**E041** `inputs/evidence/native-acceptance-run2/records/1789207005290683000-17103-accept-step/0054.json`
 SHA256 `2ac65569e948b3ab63525e2b824dbc4147335aa7a0991c433a18d8b69c6019ad`；定位：/views/snapshot, /views/catalog, /views/as_known, /views/watch, /views/query；完整JSON值，与同record目录CLI/HTTP收据逐字段核对。

**E042** `inputs/evidence/native-acceptance-run2/gui-normal/g5-asof0.ax.txt`
 SHA256 `a1fd42af1ca85bbd233f3e4ece2790cb878bf1899c12f26da2fb99b6113f7335`；定位：AX文本物理行 65,101,107；浏览范围、对象identity/revision/first_known、源修订与Watch状态的AX行；具体元数据/内容以本文所列定位。

**E043** `inputs/evidence/native-acceptance-run2/gui-normal/g5-asof3-old-top.ax.txt`
 SHA256 `54a3baa1e09e041b85c1ae4f0cd084a7be48ff7fb21550a35d016ebc7b763aeb`；定位：AX文本物理行 68,72,74,246,252；浏览范围、对象identity/revision/first_known、源修订与Watch状态的AX行；具体元数据/内容以本文所列定位。

**E044** `inputs/evidence/native-acceptance-run2/gui-normal/g5-current.ax.txt`
 SHA256 `34c97de23148cb2e7c5daf76edc5e5744b3fcc5ccab274d0deec608f0694f074`；定位：AX文本物理行 68,73,75,165,167,260,262,466,472；浏览范围、对象identity/revision/first_known、源修订与Watch状态的AX行；具体元数据/内容以本文所列定位。

**E045** `inputs/evidence/recovered/implementation-r10/r10-results.json`
 SHA256 `81d5001b8fca24982b94b764f1b32016374b4ace86846583fc035d02f5b5282f`；定位：/results/history全部15阶段：3HTTP+2CLI AsKnown0完整JSON稳定。

**E046** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L274–307,L309–1100,L1332–1338,L1896–1950。

**E047** `inputs/candidate/s_session/s_readonly_server.py`
 SHA256 `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c`；定位：L27–44,L74–424,L1107–1155。

**E048** `inputs/evidence/recovered/implementation-r10/r10-results.json`
 SHA256 `81d5001b8fca24982b94b764f1b32016374b4ace86846583fc035d02f5b5282f`；定位：/results/profile_cases：21个副本的healthy_http/http/cli/before/after。

**E049** `inputs/evidence/native-tests/test.log`
 SHA256 `566145f92fcf244bc25a903efe561d10411bcf84cf4bc7140d6680ab8fb91163`；定位：r9_all_required_delta_collections_and_scope_gate_writes、corrupt_root_variants_block_all_writes、interior_delta_missing_blocks_writes_and_reads及35测试结果。

**E050** `inputs/evidence/native-acceptance-run2/SOURCE.json`
 SHA256 `1b13ddba833fda6b01019ff8a11f38182c7c2ee2f8e2c6c6be4d740a08befa79`；定位：/scope/economic及冻结入口。

**E051** `inputs/full-diff.patch`
 SHA256 `55a132043777f529063685471fc20633555cb50dba2b21eed9bd2852b535257b`；定位：全部25文件hunks；六源码的正向候选与逆向base Git blob校验。

**E052** `inputs/evidence/native-acceptance-run2/SOURCE.json`
 SHA256 `1b13ddba833fda6b01019ff8a11f38182c7c2ee2f8e2c6c6be4d740a08befa79`；定位：429源文件与候选逐字节相同，8输入绑定。

**E053** `inputs/candidate/rust/src/theta_v0/classifier/local_shape.rs`
 SHA256 `57537f3a90c5efc30c8f8449bba6727d0fb69e810fd2bc4ad3ad6f9834f3ae97`；定位：L126–220单一分类；L240–268固定四分支oracle有效域。

**E054** `inputs/evidence/reports/review-r3.md`
 SHA256 `006cb682f7f885c9d966a4cd7ce7d05870e127f1e5f26efdffb28bff3174f326`；定位：L167：A oracle同hash有效域及绑定记录。

**E055** `inputs/evidence/gui-fault-run3/requests/000093/completed.json`
 SHA256 `3d40eb51c8bd4a0c58e8b50a537d90243160598c5d21acf5540957d14eaf16fd`；定位：seq30真实晚到旧200，晚于req94及winner5截图。

**E056** `inputs/evidence/gui-fault-run3/requests/000096/completed.json`
 SHA256 `ac3e03cb5fa1fb651f297f21b2359044b9aa257b7486c76be615ae6918eaf750`；定位：seq32真实晚到503，晚于req97及winner5截图。

**E057** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/classification_axes/5/acceptance_points/2。

**E058** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/158。

**E059** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L1639–1770,L1997–2190。

**E060** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/classification_axes/5/acceptance_points/3。

**E061** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/159。

**E062** `inputs/candidate/rust/src/theta_v0/classifier/local_shape.rs`
 SHA256 `57537f3a90c5efc30c8f8449bba6727d0fb69e810fd2bc4ad3ad6f9834f3ae97`；定位：L133–220。

**E063** `inputs/evidence/native-acceptance-run2/gui-normal/g3-witnesses-expanded.ax.txt`
 SHA256 `93ab1153d9ddb77a2da1a8041e3fe6f2a8b87dade8899cb7abe900c6255e9b4e`；定位：AX文本物理行 68,72,74,284,290；浏览范围、对象identity/revision/first_known、源修订与Watch状态的AX行；具体元数据/内容以本文所列定位。

**E064** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/requirements/39/acceptance_points/1。

**E065** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/80。

**E066** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L2268–2482。

**E067** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/requirements/39/acceptance_points/2。

**E068** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/81。

**E069** `inputs/candidate/s_session/browser/index.html`
 SHA256 `a04ddb1c67e26207cbc9ec4bd72f7cb36fcac30accf4c3f1a49e5ee10e5de94b`；定位：L547–652。

**E070** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/engineering_state_axes_with_r2_replacements/14/acceptance_points/0。

**E071** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/432。

**E072** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L309–329,L3552–3640。

**E073** `inputs/candidate/s_session/s_readonly_server.py`
 SHA256 `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c`；定位：L46–64。

**E074** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/engineering_state_axes_with_r2_replacements/14/acceptance_points/1。

**E075** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/433。

**E076** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L1819–1916,L3643–3713。

**E077** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/rf02_acceptance_obligations/3/acceptance_points/0。

**E078** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/442。

**E079** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L2268–2330,L3083–3135。

**E080** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/rf02_acceptance_obligations/9/acceptance_points/0。

**E081** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/454。

**E082** `inputs/candidate/s_session/browser/index.html`
 SHA256 `a04ddb1c67e26207cbc9ec4bd72f7cb36fcac30accf4c3f1a49e5ee10e5de94b`；定位：L515–539。

**E083** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/INTERFACE-CONTRACTS.md`
 SHA256 `f1a2993c6fa4c6c29c6b9a4de7c1d0334c697aaa03e282f621171c37fc01d0e7`；定位：S.Snapshot/Watch/History共同身份合同。

**E084** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/independent_review_implementation_obligations/1/acceptance_points/0。

**E085** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/465。

**E086** `inputs/candidate/s_session/launch_s.sh`
 SHA256 `65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d`；定位：L210–225。

**E087** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L3750–3821命令分派。

**E088** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/3/acceptance_points/0。

**E089** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/479。

**E090** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L1294–1529,L1639–1770,L2268–2442。

**E091** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/7/acceptance_points/0。

**E092** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/483。

**E093** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/8/acceptance_points/0。

**E094** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/484。

**E095** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L2855–2957,L3025–3135,L3217–3426。

**E096** `inputs/evidence/native-acceptance-run2/gui-normal/publish-e1-stream-g2.ax.txt`
 SHA256 `137877429574b1aa6420ab21115219d9c17e6d2d7fb69370be1bf95ad8dae072`；定位：AX文本物理行 68,138,144；浏览范围、对象identity/revision/first_known、源修订与Watch状态的AX行；具体元数据/内容以本文所列定位。

**E097** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/13/acceptance_points/0。

**E098** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/489。

**E099** `inputs/candidate/rust/src/bin/s_structure_session.rs`
 SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；定位：L2268–2442。

**E100** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/14/acceptance_points/0。

**E101** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/490。

**E102** `inputs/candidate/s_session/browser/index.html`
 SHA256 `a04ddb1c67e26207cbc9ec4bd72f7cb36fcac30accf4c3f1a49e5ee10e5de94b`；定位：L445–475,L507–539。

**E103** `inputs/evidence/gui-fault-run3/gui/late-success-rerun-after-old200.ax.txt`
 SHA256 `1697eae82e7b73af0fcc43f91e9d94c9d132fc33f8ac997498bd7e0a00f9c0d3`；定位：AX文本物理行 68,73,75,165,167,260,262；浏览范围/错误状态/游标的AX行；与同名.json时间戳对应。

**E104** `inputs/evidence/gui-fault-run3/gui/late503-after-old-failure.ax.txt`
 SHA256 `1697eae82e7b73af0fcc43f91e9d94c9d132fc33f8ac997498bd7e0a00f9c0d3`；定位：AX文本物理行 68,73,75,165,167,260,262；浏览范围/错误状态/游标的AX行；与同名.json时间戳对应。

**E105** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/15/acceptance_points/0。

**E106** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/491。

**E107** `inputs/evidence/gui-fault-run3/gui/gap-cut-rejected.ax.txt`
 SHA256 `f122794c02eb68eea3cacf1669559ca3f6d9673fd0780e53a89194c6fcdc0af3`；定位：AX文本物理行 68,72,74,246,247；浏览范围/错误状态/游标的AX行；与同名.json时间戳对应。

**E108** `inputs/evidence/gui-fault-run3/gui/gap-cut-retry.ax.txt`
 SHA256 `053daf4b13ed5abb4d81d725196912df6585109de6726aa299893e1a5f9e1a69`；定位：AX文本物理行 68,73,75,165,167,260,262；浏览范围/错误状态/游标的AX行；与同名.json时间戳对应。

**E109** `inputs/evidence/gui-fault-run3/gui/stream-http503-retry.ax.txt`
 SHA256 `ff6652535b4040cb834bc1a3e687fe27e71f376814807cc0ea206963c87bb096`；定位：AX文本物理行 68,72,74,167,169,356,362；浏览范围/错误状态/游标的AX行；与同名.json时间戳对应。

**E110** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/16/acceptance_points/0。

**E111** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/492。

**E112** `inputs/evidence/native-acceptance-run2/records/1789206940204500000-16549-accept-step/0047.json`
 SHA256 `5ab98a56e38dadfe8320a61b58e3909c218bcee956966a270ae60f974b66db96`；定位：/views/snapshot, /views/catalog, /views/as_known, /views/watch, /views/query；完整JSON值，与同record目录CLI/HTTP收据逐字段核对。

**E113** `inputs/evidence/native-acceptance-run2/records/1789206963023336000-16894-advance-step/0055.json`
 SHA256 `f3ec861d9251486751b0bcd03d3b9b7b8ab7d805c5678be46283c3fa447dabe1`；定位：/views/snapshot, /views/catalog, /views/as_known, /views/watch, /views/query；完整JSON值，与同record目录CLI/HTTP收据逐字段核对。

**E114** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`
 SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；定位：/observation_contracts/19/acceptance_points/0。

**E115** `inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/IMPLEMENTATION-CROSSWALK.json`
 SHA256 `0d31d68e75a6d672c948e8708bfb261549029c7310f7ad4dbb326c087157ce5d`；定位：/detailed_acceptance_links/495。

**E116** `inputs/issues/1371.json`
 SHA256 `9167fa345fcc4284bce16680410f73f8475a1b1734a35356b706460e68bc600e`；定位：/body原8AC与子域。

**E117** `inputs/evidence/reports/review-r6.json`
 SHA256 `e3e26cebd36c60ad68c6bc011e3c19fc2fad37a21abc3e00f313ed0e4d7218ff`；定位：/verdict=INCOMPLETE；/status。

**E118** `inputs/evidence/gui-fault-run3/requests/000090/completed.json`
 SHA256 `ebd5921e28f938ba23f31ddf855a64a8492e294850273897f5740a66a010fd1d`；定位：/hold_result=timeout_not_acceptance；response_status504。

**E119** `inputs/full-diff.patch`
 SHA256 `55a132043777f529063685471fc20633555cb50dba2b21eed9bd2852b535257b`；定位：完整base→候选。

