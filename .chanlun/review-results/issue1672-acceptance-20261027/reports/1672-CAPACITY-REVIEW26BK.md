# #1672 容量 SPEC 增量独审 26BK

## 结论

**受限候选 PASS，可以提用户 spec 门。** 候选限定为旧 `CAPACITY-PROPOSAL.md` 的一次性四卷各 16 MiB 改为 32 MiB，加上逐 op 页数与图像大小观测。32 MiB 是待实跑的有限规格，不是已证明足够的容量。不得附带批准 26BJ 的增长模型、生产格式修改或自动继续扩卷。

**26BJ 作为完整决策依据 BLOCK。** 其中若干估算被写成保证，不能随本次通过一起采纳。下文给出可直接采用的收窄文本，不要求先完成无关的增长研究。

冻结源码为 PR1702 `ccc115ddfb029adba8aa8fa40f65805328e58302`，工位 `/Volumes/AgentStorage/issue1672-fin2-hfs-20261025/worktree`。现场 HEAD 与 PR一致，工作树干净。独审为 GPT 6.1 Sol，26BJ 作者为 Opus。现场时间取 `date -u`，2026-10-26T13:47:25Z。

throughput checkpoint: n/a, read-only investigation。

## 原批准及本次改动

完整核读 `/Volumes/AgentStorage/issue1673-fin2-spec-revision-20261025/spec-candidate.md`。§7.2 明定每卷16 MiB，§7.3不足即停，§4.4保留资源预检，§14保留精确head、实跑、独审与main门。GitHub评论6389918994实际回读，确认用户2026-10-25批准的是该FIN/2 spec实施，不含新扩卷。

本次只请求将#1672的E及三个B独立UDIF/HFS+测试卷各改32 MiB，总名义128 MiB。原8倍页预算、cleanup余量、reader限额、4 MiB业务文档、1 MiB帧、30秒、全部WE7输入及生产64 GiB/双跑148 GiB保持。

冻结 `s_session/tests/drive_tb03c_fin2_hfs.sh:34-50` 实际由一处 `hdiutil create -size 16m` 创建四卷，并核HFS、设备号各异。`tb03c_fin2_tests.rs:26-42` 的staged/store各为33,554,432 B。`content_store.rs:323-342` 先核实际容量不超过这两帽，再按 `capacity/[8×(page_size+24)]` 设置页帽。因此32 MiB名义卷与现reader边界兼容，仍须现场核实际容量，不能静默调reader。

## 数字核验与限制

旧稿的关键算式重算成立。实际C=16,736,256 B，P=4096 B，分母32,960 B，页帽507。假定32 MiB卷仍损失40,960 B文件系统容量，估算页帽1016。该文件系统开销只是比较假设，新卷必须实测。

失败读数来自已保存IMPLEMENTATION及票评论6395752735。m-19在B0失败，E的6 Capture/3 Resolve已经完成；不是E页帽失败。B0失败前326页、max507、freelist0、实占1,368,064 B；待提交图像771,028 B。`ceil(771028/4096)=189`及326+189=515都是整页追加估算。SQLite本地payload可利用旧叶页空隙，overflow又有链指针，所以不能把它称作严格新增页下限。旧稿已明确这一点，应沿用旧稿而非26BJ的强化说法。

32 MiB相对该515页估算约多501页。B1/B2、恢复与cleanup峰值尚未测得，不能把515到530页当全流程上界，不能说32 MiB一定跑完。20/24 MiB未被证明不够，选择32 MiB是有限试验的工程取舍，不是数学最小值。

## 26BJ 须删除或降级的主张

1. 删除“原合同内无解”的穷举判断。已证明当前候选在现页帽下失败、checkpoint不会提高页帽。未证明任何保持协议的实现优化都无解。可写“继续本候选的扩卷路线须spec批准”。
2. 删除“32 MiB能跑完、不是把失败推后”的保证。冻结测试 `tb03c_fin2_tests.rs:408-483` 确实只含每B四次Apply，m-19/20/21是末次业务Apply；但后续recover仍会写epoch，物理峰值及cleanup未测。
3. 删除“cleanup任何尺寸下都满足”。`content_store.rs:151-180` 比较实际可用空间，而非总容量。`needed=2或4×[max_pages×(P+24)+65536]`，有固定开销。总容量比值不是实际空闲空间证明。原检查必须继续实际执行。
4. 降级平方/立方增长为有条件假说。`tb03c_allocation.rs:408-440` 确实按Resolve提交读取两份完整E图像，`tb03c_book.rs:931`确实累计保存record，`store.rs:740-754`确实追加完整commit图像。它们证明重复嵌入和历史累计机制，不能单凭一次290,925 B增量决定增长阶。构造还包含`e_commit_proof.image`，见`tb03c_fin2_tests.rs:267-279`，所以“两份约14万E图像加RAW”的解释不是完整字段分解。
5. 删除“十轮即超4 MiB”和“#1673全95因此更早暴露”的定量预测。未有E各代实际大小或逐轮拟合；95是S输入数，不自动等于95轮Resolve/Apply。此风险可交#1406/#1673消费，不作为本次扩卷门的事实，也不先改`tb03c-book/1`。
6. “唯一缺的读数、零额外运行成本”改为“复跑顺带取得逐op观测”。除图像大小外仍缺表/索引分配、恢复峰值与实际cleanup空闲读数。打印、查询本身也有成本，应在原时钟内执行。

## 可直接采用的批准范围与验收

建议呈用户的文本如下。

> 批准仅将#1672 FIN/2 TestOnly的E及三个B独立UDIF/HFS+卷由每卷16 MiB修订为32 MiB，并实施及完整复跑。保留原8倍页预算、cleanup/checkpoint预留、现有reader全部资源帽、WE7全输入、12 Apply、四域恢复、21次原消息重送、两个macOS ignored测试实跑及完整原件要求。复跑逐op记录E与B实际页数、图像大小、分配空间及cleanup余量，含恢复和重送前后。所有生产资源、资金、隔离与期限不变。32 MiB再不足即停并回报，不授权自动扩卷。本批准不含main合入、关票或真实资金/场所动作。

旧稿§5六项验收全部保留。冻结测试中HAND逐轮对拍在`:424-477`，原件计数在`:480-489`；四域关闭/recover/重开/audit及图像对拍在`:491-514`；原信封重送、epoch合法变化、seq及表不变在`:516-570`。两ignored测试须在精确候选的macOS/HFS+实际通过。FIN/1 WE7回归、容量/共享载体/非挂载根/非HFS负控、实际卷回收及完整归档均不能省。

新增观测优先读取成功后真实持久图像与页数，不要把失败诊断`apply_image_sizes`原样移到成功后再计算“下一次Apply”。该helper会构造下一图像，见`:254-313`。观测不能新增业务调用、提交、时钟窗口或掩盖错误。观测改动随新精确head一起独审。

## 证据与后续责任

原稿路径 `/Volumes/AgentStorage/issue1672-hfs-resend-repair-20261025/CAPACITY-PROPOSAL.md`。完整核读同目录IMPLEMENTATION与REVIEW-NOTES，以及26BJ。源码通过冻结git对象逐段读取，不拿当前main行号替代。知识图对本项目路径无匹配、Rust索引对目标新符号无结果后回落冻结对象读取。

GitHub run40178218217实读head为ccc115ddfb，run与macos-session终态cancelled，不是整套成功。job121123431838原始日志端点本次HTTP404。因此本审复用已保存失败读数与票面登记，不声称重新独立取得原日志或实跑通过。这个缺口不妨碍有限规格提案，仍禁止把旧run当新规格验收。

主控可用本审的受限候选PASS提spec门。用户批准后由现有#1672实施owner续接，完成精确head实跑、非作者整票独审后再提main门。26BJ关于增长机制的代码事实可登记至#1406，定量模型维持假说，不将它塞进本次容量批准或另设无证据阻塞。

本审按Investigation完成，未另派孙代理。已读Poteto全文、standing、how、no-comments与Comment Sicko提示、technical-writing、unslop及Prove It Works。Prove It Works使本审核原spec批准、冻结代码及真实run终态，区分保存读数和本次可得原日志。no-comments实施与删注步骤不适用于只读文书审查，源码删除0、恢复0，没有新增抑制或编码。未改票、实现、refs、store或卷，未运行Rust。
