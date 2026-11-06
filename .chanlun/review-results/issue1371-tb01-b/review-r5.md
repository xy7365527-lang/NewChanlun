# #1371 TB-01-B R5 独立复审（同ID超时续接收尾）

> 编号声明：本提交中的 #1371 仅指 GitHub Issue。
> BASE：`2212eba31e0b7419907e622e79f02f7c2cc37706`；唯一待审源码：`3ddb5c2b7f9f250861640d40d779d1a6a5fd8955`。
> 前轮源码：`667ce698eacd8c8f50ec5846f24c779b492010a8`；前轮报告tip：`41ed7163e8bc15bc7cc5af1fb06af22d98a6589c`。
> 只新增 `review-r5.md/json`；R1–R4八份历史报告逐字保留。不加载作者会话。

## 结论：FAIL（新增1项M）

**失败原因是消费者请求/响应cut与模式绑定缺口 R5-M1，不是30分钟TimeoutError。** R4-H1/profile持久绑定与R4-M2/后端cut0原反例已在本轮修复复核中通过；新M1表明“响应内部自洽”仍未等同“属于这次请求”。

另外两种未完状态单列：当前真实GUI/原生专项尚未完成；超时前独立/tmp原始证据未跨容器保存。它们不能抹成PASS，也不被算作产品FAIL。已完成的验证不重跑一遍，本次只为M1补取完整原始材料。

## 1. 续接状态与范围

本次收到同ID的505行捕获后，先核HEAD仍为3ddb、dirty=0，尚无R5报告。原 `/tmp/issue1371-r5-hsdx565a` 不存在，kernel变量亦未恢复。**没有从R4或空白状态重做方案/整套验证**：采用同会话捕获中已实际完成的源码审查和测试，明确其原始文件可用性；只重建M1补证所需本平台binary、正常小轨迹和真实HTTP/HTML函数对照。

完整范围始终为BASE..3ddb，不是最后文档提交。该完整diff/log及41ed..3ddb三提交已在本轮超时前读完；补取时重新核HEAD/源码hash未变：

- `a0b6f17a` 产品修复；
- `50f22cac` 只补profile测试健康对照；
- `3ddb5c2b` 两份正式作者报告。

完整BASE..candidate共21文件；本轮尾差为3产品文件、2既有测试文件、2新作者报告，launcher未改。两个测试文件调用真实CLI/HTTP/HTML函数，不是第二生产结构解释器。完整diff SHA=`9fb1626335f3882fcfa8e9fc0a4a4a0c78c5337660e736f34b1bfa411c160e63`；增量SHA=`50d806f4f14666045c7579ffe7603800bc55b1b571336d5fa57c72cdb8efd7a1`。

| 本轮实际源码 | SHA256 |
|---|---|
| `rust/src/bin/s_structure_session.rs` | `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901` |
| `s_session/s_readonly_server.py` | `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c` |
| `s_session/browser/index.html` | `9c3d2c3703b9e765f3d65e2cd76759ff5716e83781fd49c6ec5457aad8565498` |
| `s_session/launch_s.sh` | `65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d` |
| `s_session/tests/r9_contract.py` | `89eeaff7430a098c4abaf239b71f0d684da6dd1f53038dba9d2f84667cdadc2f` |
| `s_session/tests/r9_consumer.cjs` | `8d8a467640e6a98b2c42bacc3f1c70f2ac778bf68ea307329ab00299e727557d` |

公共证据绑定（下文每个AC/ID均适用）：本机binary SHA=`ee39fb99e4d5f7d4f9a47aefbb7979ed7a384858dec48149a9794cbefb952842`，与超时前构建值一致；Linux aarch64、rustc/cargo1.97.1、项目uv Python3.11.16/HTTP SQLite3.53.1、Rust SQLite3.53.2 bundled、WAL/FULL。目录SHA=`938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`；profile文件SHA=`2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc`；其规范内容hash为`0e9dfc9befd6878e0c844ee62c0f44576532ee8f13fe38c45f6bf467fe92d64e`；rule=`s2-axis-quantifiers`。

只验证已声明TestOnly逐笔一对一精确退化OHLC、严格无包含、前两点定向、无同价竞争的CC-006。补取轨迹依次单点10000→第二点11000→第三点10500(TOP)→第三事件rev2=11500(RISING并撤旧TOP)→第四点10800；received_at固定`2026-09-09T00:00:00.000Z`，timestamp=源坐标。补取gen3/4与原三杀gen1/2是不同推进调度，**不跨轨迹归一化generation**。

## 2. 规范、作者与根证据等级

- 本轮超时前已读SPEC C02/C08/C09、共同身份/S/StructureRecord/Delta/原子组/恢复读集、effective、crosswalk、coverage与TEST-SEAMS，A最终验收/归档和原报告。补取再次核六payload hash未变；规范清单`REVIEW-INPUTS-S2-R1.json` SHA实际为`38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`。有效17条pointer在JSON items保留；historical/not_effective不作依据，不重开批准门。
- 正式作者`implementation-r10.md/json`与14个具名附件再次逐bytes/SHA核验，共**84343463字节**。只读正式材料，不读取作者session/raw/prompt。作者的21副本、168健康HTTP/168故障HTTP/147CLI与根的重算是**资料核对**，不是本评审恢复后重新执行的计数。
- `ACTUALLY_RUN_BEFORE_TIMEOUT_CAPTURE_SUMMARY`：本评审同ID超时前实际执行，捕获有完成结果，但原完整/tmp文件不在。不得冒称它们已完整随本报告归档。
- `ACTUALLY_RUN_AFTER_RESUME`：本次补取的真实launcher/S/HTTP与实际HTML函数，完整inputs/argv/exit/原body/变换/缓存保存于JSON附件。
- Node的DOM和输送调度受控，**不是实际GUI或TCP丢包**。根当前Mac锁定；新GUI/profile/cut0/三杀原生专项未完成，不用旧GUI或编译通过代替。

## 3. Standards与已完成检查

遵AGENTS.md及`.sandcastle/CODING_STANDARDS.md`，只评审不修码；无push/merge/main/GH写/harvest/全局队列服务变更或经济外效。

超时前（同源码hash）：`cargo build/check/clippy --locked --features s_session --bin s_structure_session`、`cargo fmt -- --check`、`cargo test --locked --features s_session --bin s_structure_session --jobs 1`，**均exit0，35 passed / 0 failed / 0 ignored**。会话文件clippy无新增命中；库内既存warnings保留。完整原日志随/tmp丢失，只有已载入同会话捕获摘要，不伪造日志附件。

本次仅重建补证binary：`cargo build --locked --features s_session --bin s_structure_session` exit0，完整build.log入附件（库既存57 warnings）。没有为了Timeout重复35测试/27副本/三杀全部流程。

补取第一版私有模拟DOM脚本在负例未创建`mode-indicator`时取`textContent`抛TypeError，外层exit1；这是**取证脚本错误，不是产品失败**。只改私有脚本的可选读取，在同一个已生成gen5原库上重启自有只读服务、重跑Node。原失败源码/输出与修正后exit0均保存，未改生产文件。

## 4. 新发现 R5-M1（M，阻塞）

**位置**：`s_session/browser/index.html:347-359`（validateSource）、`:426-435`（load）、`:498-514`（Gap读取）、`:577-590,631-637`（applyStream指定cut校验/提交）。

`validateSource`只要求响应本身满足两种合法组合之一：AsKnown/as_of_generation==generation，或RecomputedWithRevision/null。`load`校验后直接用**请求mode**同步UI，却未核返回cut/mode是否匹配请求；`applyStream`即使明确请求as_of4，也接受另一种合法模式对。pageEpoch只防别的请求完成覆盖，不负责当前收到的载荷与当前请求对应。

### 4.1 新合同健康控制与内部矛盾负例

本次真正从launcher单点开始产生gen3旧TOP、gen4更正RISING/撤旧TOP、gen5追加新TOP；实际S只读服务支持所有所用as_of路由，来源HTTP**全部200**。

| 控制 | 实际结果 |
|---|---|
| load(current)直接读真实current | gen5 / RecomputedWithRevision，指示器当前 |
| load(asof,3)直接读真实asof3 | gen3 / AsKnown，指示器AsKnown3 |
| 健康Delta3→4 + 真asof4 | cursor3→4，正确RISING/撤旧TOP，ok-box |
| 只改Snapshot.profile_hash为64位`f`（其余完整新合同字段保留） | cursor仍3，完整before/after相等，明确profile不一致 |

这不是旧schema被拒，也不是回放服务缺路由404。超时前另外已测单profile_id、单history_mode、单as_of_generation内部关系破坏均拒绝；本次不把该摘要冒称再次运行了全部那些负例。

### 4.2 响应自洽但请求不匹配：三个具体结果

1. **显式历史cut被另一完整历史响应替换**：先真实读取AsKnown3（旧TOP）和AsKnown4（RISING+撤旧TOP）。TestOnly输送将后者完整JSON重送给`load("asof","3")`。响应本身不删改字段；`currentState`接受gen4，status为空，指示器`AsKnown @ generation 4`。用户请求的是3，不是4。
2. **当前查询接纳旧历史并误标当前**：`load("current")`的实际current源响应为gen5；TestOnly重送此前真正取得的完整AsKnown3。接受后`Snapshot.history_mode=AsKnown`、gen3旧TOP，而指示器显示 **`RecomputedWithRevision（当前）`**，status为空。
3. **增量路径接受合法但错误的模式对**：指定asof4的完整新合同响应仅变换`history_mode=RecomputedWithRevision`及`as_of_generation=null`（明确是**两个字段、一个模式对**，不是单字段）。其余session/cut/目录/profile/对象/关系/见证均不变；实际`applyStream`游标 **3→4** 并给出ok-box“全字段一致”，指示器为 **`RecomputedWithRevision @ generation null`**。

**范围限定**：这是实际HTML函数、真实HTTP源材料和受控输送反例；不声称S正常HTTP路由会自行返回错cut，也不声称本轮实测了真实GUI/TCP重送。原始HTTP body、requested路径、original/delivered完整JSON、前后缓存与指示器均内嵌 `consumer-results.json`，脚本提供明确输送变换。

**后果**：有效响应不等于有效的这次请求。历史/当前模式分证和指定cut要求仍可被误报成功；旧历史图形可被标为当前。违反AC3/6及OB015/016的观察身份/请求对应，不能因全部内部字段自洽而豁免。

**最小修复**：把期望mode/cut传入验证上下文并在任何提交前核验：
- `load(asof,N)`：返回AsKnown且generation/as_of_generation都等于N；
- `load(current)`：返回RecomputedWithRevision/null；
- `applyStream`、Gap指定cut读取：必须匹配请求的AsKnown cut。
保持现有pageEpoch、deepClone和完整内部字段比较，不改结构判据或补新默认语义。

## 5. R4两项与R1–R3处置

| 原finding | 本轮结论/证据限度 |
|---|---|
| R4-H1 | **原反例已修（本轮源码+超时前独立运行；关键消费者负例已补取）**。三种持久阶段及27profile/definition副本包含规范64位错误hash全部失败零写；owner暂停期间也能拒绝。新合同健康control与单profile_hash负例已在补取中再确认。M1是请求模式/切面绑定的新触发，不重新打开该profile存储反例。 |
| R4-M2 | **后端完整cut0原反例已修（本轮超时前独立运行）**。15阶段3HTTP+2CLI完整AsKnown0同型深等；profile_at_cut与本冻结B初始化/首次发布状态投影有源码依据；首观察/旧TOP保留。原独立raw文件随超时丢失，只保留捕获摘要，不冒称原始15阶段已内嵌。 |
| R3-H1 | **原wrong-root/tuple反例修复保持**。原10坏根副本重跑统一拒绝；旧响应缺新schema及404不能充特异证据，补取使用完整新合同200真实路由。 |
| R3-M2 | **原缺载荷反例修复保持**。verify_delta_payload和七集合校验正文与R4同hash；本轮缺witness原反例/35测试锁已跑。R4的25副本仅按其未变载荷逻辑有限复用，非本次又跑全部25。 |
| R3-M3 | **已修保持**。正常1→2→3前缀观察首版不漂；本次补取从launcher单点起步到更正/追加成功。 |
| R3-M4 | **已修保持**。原缺scope错误统一拒绝；只继承原200/0接纳事实，不延续缺原始末态证据的“推进后scope仍缺”判断。 |
| L-NEW-1 | **精确整数wire问题修复保持；原生产不可达/BigInt普遍兜底解释撤回**。旧真实writer发布的大整数必须在新reader精确输出；已实际运行，非人工SQL造旧库，不声称旧OS/整旧crate环境。 |
| L-1 | **已修；原非launcher/低优先级解释继续撤回**。真实丢Commit回执原argv重试幂等、原ID Query只读；accept重放不是Query。 |
| L-2 | **已修**。负/重复参数拒绝保持；合法cut0完整稳定另已验证。 |

其他旧根/辅助反馈逐条对照：

| 原ID | 当前处置 |
|---|---|
| ROOT-B-R1-01 | 本轮超时前完整cut0/已发布历史/三杀未发布修订边界已复验，R4后端profile/历史问题修复；新R5-M1属请求绑定而非旧内部过滤反例。 |
| ROOT-B-R1-02 | 当前HTML新合同健康与内部矛盾负例已补取；原深拷贝/pageEpoch保留，但不替代实际请求cut/mode匹配，新M1未闭合。旧GUI不当当前通过。 |
| ROOT-B-R1-03 | 当前HTML新合同健康与内部矛盾负例已补取；原深拷贝/pageEpoch保留，但不替代实际请求cut/mode匹配，新M1未闭合。旧GUI不当当前通过。 |
| ROOT-B-R1-04 | 首载current与有界输入源码仍在；健康current/asof加载函数已补取，实际GUI仍未验。 |
| ROOT-B-R1-05 | 超时前原10坏根/缺载荷/缺scope副本共同拒绝，35cargo锁执行；R4未变载荷检查有限复用，不称全部任意坏值或25副本本次再跑。 |
| ROOT-B-R1-06 | 超时前原10坏根/缺载荷/缺scope副本共同拒绝，35cargo锁执行；R4未变载荷检查有限复用，不称全部任意坏值或25副本本次再跑。 |
| native-delivery-query | 新owner前件变更后三杀/原ID Query/幂等/两活writer与两途中坏profile均本轮超时前实跑；捕获摘要保留，原raw已丢。 |
| native-current-unpublished | 本轮超时前完整cut0/已发布历史/三杀未发布修订边界已复验，R4后端profile/历史问题修复；新R5-M1属请求绑定而非旧内部过滤反例。 |
| native-future-AsKnown | 本轮超时前完整cut0/已发布历史/三杀未发布修订边界已复验，R4后端profile/历史问题修复；新R5-M1属请求绑定而非旧内部过滤反例。 |
| PY-DRAFT-H01 | 超时前原10坏根/缺载荷/缺scope副本共同拒绝，35cargo锁执行；R4未变载荷检查有限复用，不称全部任意坏值或25副本本次再跑。 |
| PY-DRAFT-H02 | 超时前原10坏根/缺载荷/缺scope副本共同拒绝，35cargo锁执行；R4未变载荷检查有限复用，不称全部任意坏值或25副本本次再跑。 |
| PY-DRAFT-H03 | 当前HTML新合同健康与内部矛盾负例已补取；原深拷贝/pageEpoch保留，但不替代实际请求cut/mode匹配，新M1未闭合。旧GUI不当当前通过。 |
| PY-DRAFT-H04 | 超时前原10坏根/缺载荷/缺scope副本共同拒绝，35cargo锁执行；R4未变载荷检查有限复用，不称全部任意坏值或25副本本次再跑。 |
| PY-DRAFT-H05 | 旧真实writer整数发布库经当前reader无损读取已在超时前实跑；继续撤回生产不可达及BigInt普遍兜底解释。 |
| PY-DRAFT-H06 | 当前HTML新合同健康与内部矛盾负例已补取；原深拷贝/pageEpoch保留，但不替代实际请求cut/mode匹配，新M1未闭合。旧GUI不当当前通过。 |
| PY-DRAFT-M01 | 按R1–R4明确处置和本轮固定源码/运行摘要有限复用，继续撤回原全部H/M已闭合等未限定断言。 |
| PY-DRAFT-M02 | 首载current与有界输入源码仍在；健康current/asof加载函数已补取，实际GUI仍未验。 |
| PY-8C-H07 | 本轮超时前完整cut0/已发布历史/三杀未发布修订边界已复验，R4后端profile/历史问题修复；新R5-M1属请求绑定而非旧内部过滤反例。 |
| R2-all-HM-fixed-assertion | 按R1–R4明确处置和本轮固定源码/运行摘要有限复用，继续撤回原全部H/M已闭合等未限定断言。 |

原R1错误AC顺序不继续沿用：AC3=Delta/浏览器原子观察；AC4=三杀；AC5=原ID Query/旧epoch。原“全部H/M已修复”“六集合即全部字段”“生产不会产生整数”等过宽结论继续撤回。原scope只有接纳200/0证据，没有末态原始meta时不延续“推进后仍缺”说法。

## 6. 超时前完成的本轮实跑及限度（不重做）

这些结果来自**同ID已载入的505行捕获**，不是作者自述。对应完整/tmp文件已丢，故以下不能称已将所有原始矩阵/三杀文件重新封装。

- **profile/definition**：27独立副本（3phase×原profile字段7变异 + definition语法/内容2变异），每个有同实例8健康HTTP200；注入后8HTTP503与7CLI exit1/StorageUnavailable，逐CLI和整副本前后schema/十表/逐列hash不变。三phase为静止已发布、正确接纳后的pending、首次pending尚无batch。规范64位错误hash也拒绝，不只格式门。
- **完整cut0**：init及7次accept未advance/7次publish，共15阶段；3HTTP+2CLI AsKnown0同型、同数组顺序、同值；组前后指纹不变。不是仅对象集合等值，也不是每HTTP都单独采指纹。
- **三点真杀**：

| 阶段 | 实际S PID | Signal / wait | kill后gen | 结果 |
|---|---:|---|---|---|
| after_begin | 2072 | SIGKILL / -9 | 1 | 原库recover epoch2后完整Snapshot/AsKnown1/Watch等control |
| after_batch | 2091 | SIGKILL / -9 | 1 | 完整不可达batch不冒充发布；恢复后等control |
| after_commit | 2110 | SIGKILL / -9 | 2 | 原ID Query知已发布；原argvadvance幂等不多cut |

三臂原输入/received/业务ID与control调度固定，未删first_known/receipt/对象/关系/见证；不是只杀Python、函数return或SIGSTOP，不声称恢复token/整个DB物理字节与control相等。

- **新owner路径**：两活旧writer PID3070/3083，正式换代后release自身exit1、新owner继续；另两当前writer PID3078/3091在after_begin/batch期间单独坏profile_hash，再release，被新owner根检查StorageUnavailable拒绝。四臂没有发signal冒充崩溃，前后指纹不变。这覆盖新`verify_begin_owner→verify_reachable_root`调用边，不能只凭函数名复用。
- **旧writer兼容**：实际git6eeed旧writer locked/offline原生构建及当前6CLI/5HTTP读取exit0；旧大整数精确文本、schema/十表前后不变；空发布后首次绑定、旧无batch首次pending健康。不是SQL造旧值，也不是整个旧OS/crate环境复现。
- **大整数**：当前CLI/HTTP source_coord=9007199254741000..1002、revision=9007199254740993已实际通过。A同hash严格核/目录/profile原入口只限语义oracle复用，未拿A测试数充本轮新协议证明。

## 7. 根事实、作者材料与剩余平台边界

根已对作者14个正式附件逐bytes/hash/语义核对；其21副本统计、35bin/4local_shape、PID4607/4626/4645、旧库/健康边界是**作者运行由根资料核对**，不能写成本评审的PID/计数或根独立Linux重跑。

根本轮提供当前macOS arm64 fmt/bin-test/clippy/build exit0、35测试、415源码前后不变，冻结BIN `402d21ca9aec78713df02fdf1d3bd2664eee3bd0870089612bebe897d8030d02`。profile/cut0专项、三杀及GUI尚未完成，Mac锁定；不从编译成功或旧GUI推新GUI通过。

记录（只引用用户给定内容/hash，未冒称打开宿主文件）：
- `inputs/TB01-B-R10-ROOT-INTAKE.json`：`d746da5c2a18f09432c85bfe559fa2a59e14a14dec3b348f96fbff060528e011`；
- `inputs/TB01-B-R10-ROOT-REPORT-SEMANTIC-CHECK.json`：`3424a24f1da24a931b60fc35fdb278e519c672676c211d637f8b2d903e49432b`；
- `evidence/native-b-r10/native-build-binding.json`：`5aad23efc67de5d63e0bdf7f52e77504a7c5fc29b21030123af28e66b5b32c32`。

open_db/record_connection_pragmas/epoch等同hash只可有限复用旧实际writer连接wal/FULL/fullfsync配置读数；新profile根与owner路径已经改变，不能只凭cmd_recover/persist_batch函数正文同hash跳过新前件。当前超时前已用三杀/四owner臂覆盖其Linux影响面。不是fcntl调用或硬件断电保证。

## 8. 8AC逐项（公共绑定见第1节）

下表每项证据键对应JSON checks/作者索引；原始补取argv/exit/输入/输出在附件，超时前缺原raw的地方明确标注，不用一个总计数代替。

| AC | 结论 | 已证/缺口 | 证据 |
|---|---|---|---|
| AC1 | PASS_BOUNDED | 正式launcher到同一严格Rust核，TOP→撤旧RISING→新TOP成立。 新GUI未完成。 | PRE-TIMEOUT, FINISH-M1 |
| AC2 | PASS_BOUNDED_WITH_EVIDENCE_LIMIT | profile固定绑定/版本分立/原ID重放与首获知链已核；补取中单profile矛盾不提交。 超时前27矩阵原raw遗失，保留捕获摘要并用正式作者原件旁证，不冒称全部独立raw已归档。 | PRE-TIMEOUT, AUTHOR-R10-VERIFIED, FINISH-M1 |
| AC3 | FAIL | 内部字段/六集合健康与单字段矛盾控制成立。 自洽响应未绑定请求模式/cut，仍可误提交；真实GUI未验。 | FINISH-M1, R5-M1 |
| AC4 | PASS_BOUNDED_DATA_WITH_EVIDENCE_LIMIT | 三点精确S SIGKILL/wait=-9，原库恢复完整Snapshot/AsKnown1/Watch等control。 同会话捕获可见完成记录，独立原raw未跨超时保存；不等硬件断电/真实GUI。 | PRE-TIMEOUT, AUTHOR-R10-VERIFIED |
| AC5 | PASS_BOUNDED_DATA_WITH_EVIDENCE_LIMIT | 原ID Query、Commit后幂等、活旧writer/新owner拒绝与成功已完成。 新前件已在owner期间坏profile实跑；独立原raw遗失，零写只按捕获强度报告。 | PRE-TIMEOUT |
| AC6 | FAIL_CONSUMER | 后端完整AsKnown0的15阶段稳定、gen1+历史正确。 旧历史可被消费者标当前，显式历史请求可被替换成另一cut，模式分证未闭合。 | PRE-TIMEOUT, FINISH-M1, R5-M1 |
| AC7 | PASS_S_STORAGE_BOUNDED_WITH_EVIDENCE_LIMIT | 三phase27坏profile/definition及owner途中坏profile准确停止，E/B/X未启动。 不等经济分区全域成功；真实UI状态待根。 | PRE-TIMEOUT, AUTHOR-R10-VERIFIED |
| AC8 | INCOMPLETE_EVIDENCE | M1完整原始HTTP/变换/命令已补取并内嵌，八报告不改。 超时前独立/tmp原始矩阵/三杀日志丢失；明确不当产品FAIL原因，但证据保留不足与GUI未验仍在。 | FINISH-M1, PRE-TIMEOUT, AUTHOR-R10-VERIFIED |

## 9. 17来源子义务逐项

signed_pointer与完整required_result在JSON items沿有效SPEC逐项保留；只承接B结构子域，不销完整来源ID。

| ID | 结论 | 证据与未验边界 | 键 |
|---|---|---|---|
| A-CC-006-03 | BOUNDED_DATA_RECORDED | 同核相同输入/时钟三杀对control完整结构相等；原始自跑文件遗失，不伪称可完整解包。 | PRE-TIMEOUT |
| A-CC-006-04 | PARTIAL_GUI_PENDING | 真实更正/撤回/追加、四比较/三根见证可查；实际HTML函数不等GUI。 | FINISH-M1, PRE-TIMEOUT |
| A-ST-044-02 | PARTIAL | 持久原子集合原反例修复保持，健康应用成功；请求模式仍可混，见M1。 | PRE-TIMEOUT, FINISH-M1 |
| A-ST-044-03 | PARTIAL | 后端恢复与历史续接有限成立，消费者可接纳错误历史请求响应；C压力仍外。 | PRE-TIMEOUT, R5-M1 |
| A-ES-15-01 | PASS_S_ONLY | S CompleteCut/economic not_started分立，不是经济ack或跨域完整证明。 | PRE-TIMEOUT, FINISH-M1 |
| A-ES-15-02 | BOUNDED_DATA_RECORDED | 新根进入owner阶段，两次途中profile坏拒绝，三杀/换代迁移有完成捕获；原raw遗失。 | PRE-TIMEOUT |
| A-RA-04-01 | PASS_S_ONLY | 旧TOP身份/first_known/证据/撤回史保留；经济责任不销。 | PRE-TIMEOUT, FINISH-M1 |
| A-RA-10-01 | PARTIAL_CONSUMER_FAILURE | 后端cut0稳定和profile来源已修；消费者可将AsKnown标为当前Recomputed。 | PRE-TIMEOUT, R5-M1 |
| A-I-02-01 | BOUNDED_S_STORAGE_RECORDED | 坏profile/definition三phase和owner途中准确停止；不算全域故障乘积。 | PRE-TIMEOUT, AUTHOR-R10-VERIFIED |
| A-OB-004-01 | PASS_BOUNDED | 版本/修订替代关系、profile固定绑定已核，不扩大任意对象域。 | PRE-TIMEOUT, FINISH-M1 |
| A-OB-008-01 | PARTIAL_CONSUMER_FAILURE | 内部单字段矛盾会拒绝，但完整自洽旧响应/合法错模式对仍被提交，不满足请求对应。 | FINISH-M1, R5-M1 |
| A-OB-009-01 | PASS_BACKEND_BOUNDED | 缺右邻无TOP、后端cut0不回填、首获知保留；消费者选错cut另列M1。 | PRE-TIMEOUT, FINISH-M1 |
| A-OB-014-01 | NOT_RUN_OUTSIDE_B | 无经济事件不补造应用；未发计划/可能已发/成交实例未运行，不报空域PASS。 | PRE-TIMEOUT |
| A-OB-015-01 | FAIL_REQUEST_BINDING | pageEpoch不替代当前响应与请求的cut/模式对应；实际旧AsKnown3可被标当前。 | FINISH-M1, R5-M1 |
| A-OB-016-01 | PARTIAL_GUI_PENDING | 三故障恢复数据成立，但新consumer绑定缺口与当前GUI未验保留。 | PRE-TIMEOUT, R5-M1 |
| A-OB-017-01 | PASS_S_DATA_BOUNDED | 固定源/时钟后端历史相等，不从bars造经济恢复；请求切面误配不据此豁免。 | PRE-TIMEOUT, FINISH-M1 |
| A-OB-020-01 | INCOMPLETE_RAW_RETENTION | 新M1完整原始补取入报告；旧独立/tmp随超时遗失，不把摘要/作者输出冒称独立原始数据。 | FINISH-M1, PRE-TIMEOUT |

## 10. 证据出包与NOT_RUN

本报告JSON仅封装**恢复后实际仍可核验**的M1补取原始脚本/完整HTTP body/原始与变换State/前后缓存/真实argv/stdout/stderr/exit/build日志，xz+base64附bytes/SHA；不假装封装已经遗失的超时前独立raw。另有作者正式14附件的完整校验索引，可在其报告自行恢复，但不能替代个人原始执行身份。

还原：

```python
import json, pathlib, tempfile, base64, lzma, hashlib
r = json.loads(pathlib.Path('.chanlun/review-results/issue1371-tb01-b/review-r5.json').read_text())
out = pathlib.Path(tempfile.mkdtemp(prefix='issue1371-r5-evidence-'))
for n, a in r['evidence_artifacts'].items():
    assert pathlib.Path(n).name == n and a['encoding'] == 'xz+base64'
    b = lzma.decompress(base64.b64decode(a['data']))
    assert len(b) == a['bytes'] and hashlib.sha256(b).hexdigest() == a['sha256']
    (out / n).write_bytes(b)
print(out)
```

M1复跑入口：当前固定源码、本平台项目Python/libpython环境，执行附件 `capture.py <新的空/tmp目录>`（将consumer-evidence.cjs放同目录）；它启动正式launcher与原始输入并调用实际HTML函数。不要对已经有s.sqlite的目录重跑init；若只补取Node可用`restart-node.py`打开已生成的同库。脚本仓路径绑定`/home/agent/workspace`；Linux `/proc`部分不冒称macOS可原样运行。测试输送明确在脚本中，非真实网络故障。

未完事项：
- **R5-M1待修**，修后针对请求cut/mode健康与冲突对照补验，不扩大结构/G语义；
- 新版真实GUI/DOM、真实断连及恢复前后页面，当前根未完成；
- 当前macOS profile/cut0专项与三杀/现场fullfsync影响面未由本评审运行；根build/test事实分列；
- 超时前独立完整raw材料缺失，不重新伪造。根可据同ID捕获和作者独立来源对照决定需补哪些原始面；本次只将M1关键原始补齐；
- C1372全分页/保留/过期/慢消费/长历史、全调度、经济责任/计划/可能已发/成交、RA10完整经济史、I02/TB05全经济分区、ST044高级扩展、其余CC/G/FU/非退化市场仍在后续，不是B缺陷也不被B局部通过销项；
- 无全量cargo/history重放、无main/GH写/生产启用/订单或资金外效。

**本次运行TimeoutError与一次私有取证脚本TypeError不算产品FAIL；最终FAIL只基于已再次取得完整原始对照的R5-M1。** 仅提交R5两报告，八原报告不改。COMPLETE只表示评审返回，不批准#1371/父TB/#1323关闭、main合入或经济动作。
