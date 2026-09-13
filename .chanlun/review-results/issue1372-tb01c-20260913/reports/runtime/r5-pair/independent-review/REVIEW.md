# #1372 最终有界独立评审：AC3 / R5 双跑 / 当前 GUI

**产品行为：PASS_BOUNDED_TESTONLY_C。完整交付：NOT_VERIFIED_DELIVERY_GATE。** 固定候选 `3ff8c47ea4ab2aeb4672722e204d9f20b35f9b7f`。本轮没有新增已确认产品缺陷；AC1–7 在声明域内通过，AC8 的 GUI/记录子项通过，但适用 DevSkim 零发现门失败，不能据本报告关票、释放依赖或合入 main。main 批准是另项动作门，B 的旧例外不覆盖 C。

本会话真实身份 `/root/r7_codex_product_review`，运行时 `codex_native_subagent`。我曾实施 Rust；本报告对运行原件、Q/消费者及报告边界作独立核验，Rust 源码独立结论只援引他人的 P3/P4/queue 报告。没有新启服务、访问活 API/DB、重跑产品、修改源码或旧证据。只读边界来自任务指令，不宣称 OS 沙盒。

## 三臂证据如何汇合

R5-A/B 两独立新进程/目录的 **495 条完整记录及原字节全部相同**，每份 161080375 字节，SHA `02fdd97ed3f2efbc2493a44506d1bc33cc3619fe57f0d25a781c39a50edd0442`。160 个实际输入结果、160 个原身份 receipt、160 个 Delta 与其真实 index_frontier 所指完整 batch，加其余13表、schema、lifecycle，共495。所有 BLOB 均核 typed/base64/bytes/SHA；保留语义时间、原身份、顺序、first_known，未排除字段、未建立身份改名表。15表、160代全量闭合，非只比最终摘要。

每臂 op95 在 AcceptedPending/Begin 后真实 SIGKILL S，Q 保持；recover epoch2 后同一原消息继续至Commit。Q在观察到109后真实SIGKILL/epoch2重启，S保持且继续输入。A故障事件在 events.jsonl 第555/559/660行，B第543/547/646行；具体原回包、PID/nonce/clock/epoch绑定由 EXAMINATION.json 指向。停止回执中的退出观测警告原样保留，未声称存在额外独立进程探针。

两臂修订 lane 都先完整安装96；后续 Watch取得97..100四批（99三撤回三替代），却在同cut分页时被真实Q109故障截断，保持96；紧随 AsKnown ECONNREFUSED 未安装。两次都保留 NOT_VERIFIED，**不承担正控**。单独最终 AC3 按事前无Qkill计划，从原cursor95/head95断连后续至99，114请求/113页于 **23254.249667ms** 内安装，期间S推进至144；该臂495中的前494子域同故障双跑，lifecycle不同明确保留。[AC3独评](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-final-ac3/independent-review/REVIEW.json) 是独立正控，不回写两臂失败。

## 八条验收条件

票面冻结文本来自 TB01C-ENTRY-READINESS.json 的 `/acceptance_conditions_verbatim/0..7`，只使用合同文字，不继承其早期B阻塞状态。签署SPEC为 `efc1ddc4d6c015f4f8c6d44f904d704a88308b46` 中 C08–C10，第310–341行，SHA `42fc34cd0bb6736469db5b6e652f861bee41b6ccc5808677125fae831b75bd11`。完整逐条原文、子项理由、原路径/SHA/JSON pointer见 REVIEW.json。

| AC | 结果 | 实核依据及边界 |
|---|---|---|
| 1 | PASS | 正式launcher；两臂固定16的5页129条在S16→24期间完整；Q2重启后原token offset31保持。 |
| 2 | PASS | 原身份/因果/完整连续Delta同cut；当前正常链加同字节Client原坏帧、旧帧、迟到页拒收。 |
| 3 | PASS | 独立AC3修订正控；两臂真实Gap37..41/33..33重建41；普通保留8与160永久历史分离。 |
| 4 | PASS | 完整160/500ms无ACK节流；三声明窗均有接纳/Commit推进，资源边界具名，末段积压不隐去。 |
| 5 | PASS | 真实S afterBegin与Q109分别SIGKILL，另一PID保持；原身份恢复；结构CompleteCut与economic:not_started分列。 |
| 6 | PASS | 两新目录/进程495全值及原字节相同；全部15族/BLOB/顺序/语义时间均保留。 |
| 7 | PASS | 当前AsKnown96=849条、Recomputed156=1689条/160=1745条；过滤补验116→1→116，完整候选保持。 |
| 8 | NOT_VERIFIED | GUI目录→修订实例→输入/关系/见证、无损全候选通过；适用DevSkim门失败，整条交付未满足。 |

## 实际负载与资源

96 seed +64 live（48新增、16 revision）；发送间隔500ms、客户端总10s、末offer drain30s为运行前声明的 TestOnly 工程参数。客户页31、Watch4、普通保留8没有升为生产默认或SPEC值。窗口只取完整位于声明闭区间内的首末点；不借窗口外迟到样本。

| 臂 | 三个10s窗 Commit增量 | live最大回复ms | live传输在途峰值 | 已观察accepted pending峰值 | 末offer至phase结束ms |
|---|---|---|---|---|---|
| R5-A | 20 / 20 / 18 | 2244.684959 | 4 | 1 | 2245.4325 |
| R5-B | 20 / 19 / 18 | 2146.837333 | 4 | 1 | 2147.639333 |

每臂实测窗口端点与行号见 runtime EXAMINATION；峰值是已定义观测口径，不推断所有瞬间全局队列长度。Q冷启动/变化仍捕获当前全域，缓存只复用同字节证书与编码，联合预算；没有generation/mtime捷径。Rust源字节缓存上限不等于RSS上限。以上是有限窗口进展与有界回执事实，不证明无限长历史或生产稳定2Hz。

## HTTP 与实际 GUI

A：121请求/119完整响应、110完整页、3284行次、5份安装候选、8份完整Delta、2个Gap。B：113/111、102页、3036行次、5候选、8Delta、2Gap。两处缺完整响应恰为各臂具名故障读取；没有掩成成功。全部公共帧身份/hash/causal_refs、每页顺序/总数/offset/token及所有实际Gap、Delta和候选由不导入Q的独立原件投影器核对。中间capture_digest只核原来源与一致绑定，没有从最终库重造所有中间捕获。

当前Chrome的11 ASCII语义导出双层解析后与解码JSON一致且无U+FFFD。四完整候选为：160/57页1745条、AsKnown96/28页849条、156/55页1689条、健康Watch156→160/57页1745条。实际Watch四批157..160全字段/顺序匹配，159有三撤回三替代。实际点击目录、input_refs、relations与revision witness；修订详情及截图可见evt-0002 revision2、seq98、price1030502。current/rendered候选相等来自页面脚本真实回执，未伪造两份外部rendered原件。

初次过滤即时读数116→116→1保留未完成；另次先等checkbox和rows目标完成，再得116→1→116，后台116与完整候选不变。大可访问性snapshot仍含替换字符，仅定位控件。GUI网络964附件均hash核过，但本轮没有逐页重新解析964附件；语义依据是四完整候选及实际Watch整帧。旧六类传输负控、旧实际响应与迟到历史页只在Client/HTML逐字相同范围复用R4和无损重采独评；不称当前Q实际产生坏批次，异化身负控也不称实际session2迁移。

## 冻结、旧失败与交付边界

R5-A封存1164件354292032字节；B封存2203件443855476字节（含GUI1079件90407398字节），全部SHA/长度相符。每臂458份事前源快照均核，22份root文件逐项等于3ff Git对象，436份为外部冻结构建/源码/运行材料。A/B其中455份源快照同字节，3份不同仅具名运行路径/端口/计划配置；没有以当前工作树替代候选。封存核验不等于每个附件都读过语义，READING.json明确范围。

R1负载失败、R3 consumer/原生fetch失败、R4损坏导出与故障中断、旧ASCII40件中两control附属变化、原Ready超1MiB失败、R5两处中断、初次过滤未完成均保持原名分。当前纯解析器三次早期失败亦保留：过窄脚本文字、误要求browse提示stderr为空、假设两种stop回执同shape；后续精确适配原件，并未更改产品判断或证据。详见 EXECUTION.json。

既有Rust独评中旧P4排队跨期缺陷保持FAIL，新queue门有他人五臂复验；P3采集sidecar目录集合FAIL与产品修复PASS分开。本报告没有把这些旧记录改为过去也通过。

冻结DevSkim run34751431403的scanner与SARIF上传成功，但zero-findings gate failure；checkout14071107与3ff同树已具名绑定。扫描具体分诊由另一工位处理，本报告不裁新豁免。四个CI job成功是根当前接回事实；本次没有另行在线刷新四job，不能用它覆盖已实读的DevSkim失败。root打包草稿八AC正负映射在本限定范围未发现越界，最后应写入本结论并保留交付门。

**此包可以交审，尚不能据此关票/释放依赖/合入。** TestOnly CC006通过不等于116目录全部算法、全市场域、经济链或#1323全图完成，亦不结清#1448旧R6迁移。所有逐项原件路径、SHA和定位集中在 REVIEW.json/INPUTS.json，避免把外部摘要标签当成产品证据。

最终补注：事前 clock-plan 预声明160 accept、161 Begin、161 Commit，共482个接纳/Begin/Commit时槽，另1 recover_ns；实际权威 s_clock_events 是160 accept、161 Begin、160 Commit、1 recover，共482行。op95 attempt0的预声明Commit未发生。这两个482计数口径不同，完整计划及所有实际时间均纳入对拍，见 CLOCK-COUNTS.json。

已接收并实读另一原生工位的最终DevSkim分诊：基线外11235键＝10888继承债务＋2旧baseline行移＋345真正C新增；345中339个具名摘要/Git对象、5处有意本机地址、1处固定函数setTimeout，作者未确认新增产品安全缺陷。我未独立重审345处，也未全仓重扫；实际风险总数未知，#1385 strategy314继续needs_review。现名zero-findings gate实际按位置键baseline差集判定；随后步骤摘要1122k超1024k的显示失败不能解释掉前序差集exit1。分诊没有豁免或合入效力，也不适用于随后新增文档head。
