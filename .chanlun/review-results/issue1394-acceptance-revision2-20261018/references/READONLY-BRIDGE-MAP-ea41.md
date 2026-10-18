# #1394 同域 Voice 桥：正式 S 输出能力与下一瓶颈

只读核查，2026-10-18。源码基准 `ea41d4dfd19cd7eb2aef5688882159e3d1631dd9`，tree `08db6c57635406ff7d6e27aa590b661ececf6daf`。`b46ba296` 是该提交祖先。未运行产品、cargo、Lean 或长历史；未修改源码、原件、tracker 或其他工位。本文是供根决定排程的工作草稿，execution_authorized=false。

**结论：当前 ea41 的 S3 能发布完整来源绑定的 A/B 对象和历史内容图，并有现成固定切面读取接口；它尚不发布可供本域认证的完成走势、terminal BSP 和完整 K carrier 森林。** 因此，完成一次 9011 结构重放只能补得真实产物与根身份，不能自动补出 Voice 权威。输入本身有没有合法 γ，本轮没有运行，不能下结论。

在飞 #1419 工位已经接入 C/D/E/F，不应再说“E/F 完全没实现”。但它的范围仍到 L0 中枢/相邻对，明确延后完成走势、上级实现和首回试。把其合流到 ea41 后，terminal/γ/K 的缺口仍然存在。不能以加轴名、改 authority_id 或复用 50070 域端点解决。

## 已核现成接口

以下路径除另有说明，均为 ea41 的 Git 对象内路径，以该 SHA 固定，不是主仓当前 WIP。

| 能力 | 接口与证据 | 可交付的事实及限制 |
|---|---|---|
| 正式 S3 写入 | `rust/src/bin/s_session_v3/writer_cli.rs:192` 分派 `serve/advance/recover`；`mod.rs:165` 接线 | 使用已有完整 9011 原件，正式 owner 接纳与提交。需要新精确运行合同；本轮无执行。 |
| 已发布 S3 对象 | `s_session_v3/generation.rs:1069` 调用 `ContentBi::build_borrowed`，`:1115` 后装配六数组；`s_structure_session.rs:101` 明示仅 A/B | 13 轴为 CC-001…010、054/055/056。不是完整 Classification，亦没有 S3 C/D/G/O 生产者。 |
| S/Q 固定切面分页 | `s_session_v3/q_runtime.rs:253` 支持 `/api/v3/snapshot`、`/api/v3/content`、`/api/v3/watch`；`q_snapshot.rs:29` 是 snapshot-token/2 的17字段；`:48` 是首请求8字段 | 可完整枚举固定切面的对象/撤回对象/见证/关系/观察/raw_history。六族见 `projection.rs:20`。保持 token、切面、模式及分页顺序，不能混并不同切面的页。 |
| 财务点查身份 | `s_session_v3/financial_reader.rs:428` `basis_ref_with` | `s-financial-query/1` exact 五键为 schema_revision/session_id/session_generation/writer_epoch/object_ids。返回 `s-financial-basis/3`、basis_ref、snapshot_ref、逐对象 ref/hash、proof_nodes。空 object_ids 仅用于取得基底身份，不证明任何 terminal 存在。 |
| 财务内容与闭包 | 同文件 `:590` `content_with`、`:730` `closure_with` | `s-financial-content/1` 分页读 path/unit/offset/limit；`s-financial-closure/1` exact 五键为 schema_revision/anchor/path/after/max_reply_bytes。七键 anchor 绑定会话/写者/切面/frontier/commit_ns/basis_ref。生产消费需 `--session` 会话路径；冷6参 closure 只是独立对照臂，见 `:166`。 |
| 完整来源证明 | `financial_reader.rs:275` `current_basis` 引用已审20字段 snapshot，`:288` 不重写 cut；`projection.rs:26` 有 raw_history | 可保留 source refs、原始输入与修订历史。来源完整不等于 K carrier 的语义完整，不得把 raw_history 或任意生效块直接命名为 carrier。 |
| Voice 权威消费 | `rust/src/economic_session/evidence.rs:130` `load`、`:179` `verify_witness` | 已有固定原件/独审字节核验入口；该模块开头明确“不生产 γ，不调用旧 π，也不把 Closed/Strict 作为定义 γ 的前置”。 |
| 真正 BindVoice | `economic_session/voice.rs:57` `validate_request`、`:121` `bind_image` | 消费已认证 γ/carrier，并核本 B 已录原始开局、已 Applied allocation、instrument/chong/操作级别/来源/数量。e_cut 取本 B 原事实 cut，不能预填。 |

## ea41 生产边界有硬检查

`rust/src/bin/s_structure_session.rs:98–108` 区分 S2 A…D 的 `s2-axis-quantifiers+new-bi/1+segment-second/1` 与 S3 A/B 的 `s2-axis-quantifiers+new-bi/1`。S3 不是靠换一个规则字符串就能取得 C…O。

`rust/src/bin/s_session_v2/tb02a_facts.rs:947–976` 的目录表只有13/17两种。`s_session_v3/source_audit.rs:1705–1714` 的 typed kind 准入只接受 CC004/005/007/054 与 tb02b::KINDS，其他种类报“未声明typed事实种类”。`generation.rs:1035–1118` 构造知识态与 B 对象后即装配，没有完成走势、BSP、carrier/gamma 接线。

对 ea41 `rust/src` 的精确词搜索，`controlled_gammas/carrier_objects/voice-binding-authority/first_confirming_observation_source` 只命中 economic_session 的权威消费者及 unit 测试。不能把 unit 权威生成器视作正式来源认证入口。

## 当前在飞实现已经做到哪里

已只读核 `/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1419`。HEAD 为 `38aeca9d4699742d40353cff7fe51d1935651176`，本次观察有17个 tracked 修改，另有未跟踪 source_structure.rs、夹具及补丁。它是可变 WIP，不能用 HEAD 冒认本次读到的全部字节，也不能冒称已与 ea41 兼容。

- `rust/src/bin/s_structure_session.rs:109` 新域规则为 `s2-axis-quantifiers+new-bi/2+segment-second/2+center-seed/1+center-pair/1`。
- `s_session_v3/generation.rs:1006–1073` 已在结构域实际调用 `content_structure::build_cd` 与 `build_ef`，将结果对象、关系、知识请求纳入同次生成。不是只注册轴名。
- `s_session_v3/source_structure.rs:5` 只对新规则接纳 C/D/E/F kind，`:55`、`:225`、`:361` 分担 payload/links/full 审计。
- `s_session_v2/tb02a_facts.rs:951–954` 新目录22轴为 CC-001…014、016/017/018/020/022、054/055/056，仍没有 G/O 等轴。
- `tb02e.rs:18–25` 产 seed attempt、center、roles、assignment、relation/change/version。`:32–37` 明示上级、≥9升级、三类/破坏、上级连接等 deferred。
- `tb02f.rs:27–44` 明示 `trend_realization_1426`、`upper_*_1429`、`first_retest_destruction_1434`、`two_step_settlement_r5_1496`；趋势为 `NOT_ASSERTED_DEFERRED_1426`，上级为 `NOT_CONSTRUCTED_DEFERRED_1429`。

2026-10-18T03:26:14Z 读取后 SHA256，供后续发现并行改动时区分版本：

| 相对 resume-1419 路径 | SHA256 |
|---|---|
| rust/src/bin/s_structure_session.rs | fc75d59e8a81cde6ebef41fca035b0b599bc68b2c00e71f5f18e9aae48439907 |
| rust/src/bin/s_session_v3/generation.rs | 471825f38f5a41796346b286386d64f5ad25e06bfbd01ac51ea77248bdda4c0f |
| rust/src/bin/s_session_v3/content_structure.rs | 3e7803272c1503ef9579ab812a81e7b69f5e26a9c4896bef71fd06520d50ca65 |
| rust/src/bin/s_session_v3/source_structure.rs | 5dd619d794f97a06800d4b92210f253c7afc0ddad84ae80b83c3516685255111 |
| rust/src/bin/s_session_v2/tb02a_facts.rs | f33aa9e513191f689b18539b06faf97b159484bbe9f8d88aefef5f8a92d4c5b4 |

本次同时从 GitHub 读取了 #1419/#1424/#1426 当前正文/近期评论及 #1433/#1437 正文。#1419 当前仅三规则/codec专项取得远端通过；完整业务/真实S-Q与跨基线合流仍欠。#1426 的 U/D 各一份是合法低级完成输入给定后的条件推论，raw/正式 owner U0/D0，production_CompletedAt=null。#1424 的既有计划正是 C/D/E/F → S3/Q 的非空纵片，不能重复开一条平行替代。

关键票面定位：[1419三锁限定验收](https://github.com/xy7365527-lang/NewChanlun/issues/1419#issuecomment-6243461987)、[1424在飞合流](https://github.com/xy7365527-lang/NewChanlun/issues/1424#issuecomment-6199266389)、[1426当前接线缺口](https://github.com/xy7365527-lang/NewChanlun/issues/1426#issuecomment-6199095675)。评论描述早于本次WIP读取；有差异时以上面的精确观察为准，仍不把静态接线当运行通过。

## 哪些是实际依赖

当前选定路线要求“同一正式 S 输出的实际 terminal BSP 与完整 K carrier”，所以存在生产能力前置，不能仅把它当 #1394 的 JSON 适配工作。

| 需要的结果 | 实际承接 | 本轮能确定的关系 |
|---|---|---|
| 正式非空 L0 中枢/关系 | #1419/#1424 与在飞 C…F 纵片 | ea41 未具备；WIP已有接线但未取得本域真实成功。#1424 已有 #1488 性能前置，不应另建重复票。 |
| 同一合法读法的形成/完成与走势对象 | #1426，相关高阶递归由 #1429 | 当前 F 明确不宣称趋势实现。#1426 已有有限语义工作例，但尚无正式 raw/owner 完成 U/D。具体γ若需要这些对象，必须消费真实输出，不能将条件工作例时间改为生产完成时点。 |
| 实际 terminal BSP | 按其真实类型消费 #1433 的一类、#1434 的三类、#1436 的二类；全标签输出由 #1437 | 当前源码没有该正式输出。最终选择哪类必须由本域事实决定；没有依据把全部六谓词/全部类型的完成都加成 #1394 新 blocker，也不能先指定端点去逼出所需类型。 |
| 完整 K 来源载体与同级唯一 host | 操作分解 #1431、必要全级构造 #1429，以及相应来源发布桥 | 既有 raw_history/CC-013.segment/CC-022.unit_assignment 不自动等于完整 K carrier。需要按既定载体定义证明区间、成员全集、版本和唯一归属。是否需这两票的全部义务，要按实际选定载体逐项核，本文不自动新增 tracker 边。 |
| 定义γ与terminal/carrier对应的独立认证 | #1394 同域桥接，复用 #1374 已定权威格式 | 现成经济接口已经可消费；新域实例、动态提取器和独审证明缺失。不需要重新选择资金政策。 |

上述“实际依赖”是产物消费关系，不是本次增改的原生 blocking 边。根应先把现成 #1419/#1424 在飞结果接好，再按当前合流树和实际终端路径确认最小后继。不能宣布 #1394 只等 #1426，也不能把全部 TB02 长链一股脑设为新前置。

还存在一个可讨论但尚未认证的独立定义证据路线：在9011同域 raw 与正式 S 根上建立新的离线数学见证，再核其与实际 terminal/K 的桥。这仍要求真实 terminal 和载体来源；不允许借旧 theta_v0 classify 生成另一份输出后叫它正式 S，不允许把手填 completed/Bool/端点当实际产生。当前没有发现能直接执行并认证这条路线的通用入口。

Closed/Strict/N7 不是所有定义γ的通用前置。`economic_session/evidence.rs:1–2` 与已验 #1374 REPORT.md:47 明确保留这个区分。不能为弥补本次来源缺口，反向增设它们为全体γ的新定义条件。

## 最小适配写集与合同建议

现阶段可独立准备的是两个仓外工具和逐实例证明，不必改经济权威消费者。以下是供根批准/分派的候选写集，本轮未创建这些工具，也未给运行准入。

1. 一个固定切面来源导出器。输入为新 source/tree/binary身份、9011原件hash及完整顺序、profile/rule/catalog、正式会话身份、已提交cut和资源/监督合同。经既有 snapshot/content/financial会话接口分页，取得该cut的对象、来源、见证、关系、知识状态和必要闭包，保存原字节/refs/proof；从头到尾固定一个已核切面。它只导出事实，不产生缺失的走势/BSP/γ。
2. 一个同域桥编组器。输入为上述完整导出、实际γ证明、terminal的全部同端同侧标签、完整K森林及各source对象、instrument声明、opening_raw、TestOnly时钟映射。按实际输出枚举候选，验证同源hash/profile/级别/方向/确认前缀/锚/整个来源区间和host唯一性。候选多于一条时保留全集，由既定选择合同决定；本工具不引入新选择策略。
3. 每个被绑定经营owner的一份具名 authority bundle，以及独立 verifier 的 receipt/语义与Lean边界证明。作者不能自签独评。只有通过者才写获准 binding；不复用旧单根给两个新owner。

生产者缺口不能由这两个工具补成。若根选择沿正式生产路径推进，复用现有 C…F 合流作者，后续新增相应G/terminal/carrier内容生产者，并同步其 `generation.rs` 接线、规则×kind准入/source_audit或source_structure、catalog_evidence/目录轴、对象引用/历史索引和原始事实到内容的审计。当前无法在语义生产者尚缺时诚实给出一个“只改若干适配行即可”的精确补丁数；写集应随已认领票和具体生产者冻结，不能整树覆盖 ea41 或复制旧S2全批JSON生成器。

新权威包继续使用 `voice-binding-authority/1`，exact顶层 `schema/authority_id/source/artifacts/bindings`。artifacts至少有 `input/controlled_gammas/carrier_objects/instrument_declaration/opening_raw`，并固定实际terminal、完整K森林、来源对象、γ→terminal→host桥和证明的原始字节。现有运行时要求每件 `name/path/sha256/bytes`，绝对路径、精确长度、SHA匹配；单件≤64MiB、集合≤128MiB，不能以分页为理由放大。

独评receipt使用 `voice-binding-independent-verification/1` 与 `APPROVE_CONTROLLED_VOICE_BINDING`，必须同 authority_id、bundle_sha256、artifacts，并逐一覆盖唯一 `verified_evidence_ids`。格式能加载只说明固定字节核验通过，语义完整性由独立证明负责，不能拿加载成功代替认证。

每个binding exact为 `evidence_id/instrument/chong_id/operation_level/carrier_ref/gamma_ref/structure_view/sigma/n/parent_voice_id/opening_source`。structure_view保留profile_id、execution_level、confirmation_level、nest_depth的分别身份。source必须绑定9011实际input_sha256与本域profile；first_confirming_observation_source来自本次真实前缀证据，并显式给TestOnlyAffineOrdinalToNs四字段。ordinal不能冒充墙钟。

opening_source为 `fact_key/fact_hash/allocation_id/raw_sha256/json_pointer/quantity/units`。开局原件先冻结；运行时仍通过真实CaptureExternal及ApplyAllocation落本B，再从原事实取得e_cut并BindVoice。经济核 `voice.rs:166–182` 将已应用原开局与独审依据逐字段对拍。保持既定 `#1349/atomic-slot-wait/v1`、TestOnly现金模型及各现有资源帽，不修改资金政策。

## 合法失败与验收界限

| 情况 | 必须交付的结果 |
|---|---|
| 9011完整读取成功，但S规则仍只覆盖A/B或C…F | 报实际覆盖/缺失kind与对应知识状态；本域γ、terminal、carrier认证保持缺件，不生成成功bundle。 |
| 本域实际输出没有合法γ/terminal | 保存候选全集、拒绝原因及原始证据，明确当前输入不供该成功实例；不缩历史、不改原件、不用另一域端点。是否准备另一输入由根另决。 |
| 找不到完整K、host零命中或多命中、同级/锚/区间不一致 | 拒绝认证；不能用OperationSequence、当前Active块或候选锚点代替。 |
| 结构修订/切面变化/身份不同 | 拒绝混读，按原合同以新的整体实例处理；不能拼前后两个cut的terminal/carrier。财务读取的ANCHOR_NOT_CURRENT/IDENTITY_NOT_CURRENT要保留。 |
| 超时、分页/闭包/保留/帧帽超限、取消或失联 | 原截止和累计资源账继续生效，停止产生业务成功结果，监督清理/封口仍可独立进入。不能靠重试重置帽。 |
| γ/开局/时钟hash不匹配或开局未Applied | 保留IdentityConflict/MissingDependency等实际拒绝；e_cut由运行时实际生成，不手编。 |
| 独审未通过 | 包保持未认证，不进入BindVoice成功路径。 |

准备阶段可先冻结完整输入、身份、默认禁止执行的导出/编组器合同，并复用现成S/Q和经济读取/绑定实现。真正成功验收至少还欠：新合流源的完整9011正式输出、实际本域γ/terminal/K及独立认证、两个owner所需各自依据、当前driver与两角色ARTIFACTS、精确新运行准入、fresh A/B和真实持久恢复。现有输入存在、静态接线、权威加载和正确拒绝都不能销掉这些成功义务。

读取的原件入口为 `/Volumes/AgentStorage/issue1394-freeze-input-inventory-20261018/REPORT.md` 与 `ROOT-MINIMUM-FREEZE.json`。原9011的close=1151/timestamp=1700000000；旧50070 Voice域的close=753294/timestamp=0、确认ordinal50069，本轮不再找回或重跑这些原件。旧包只用于理解exact消费格式及既有证明范围。本文没有恢复旧PASS，也没有提出以新资金政策绕过来源缺口。
