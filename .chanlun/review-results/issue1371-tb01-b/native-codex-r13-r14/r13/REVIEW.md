# #1371 TB-01-B R13 Codex全增量独立复审

**结论：FAIL。** 固定候选 `b9bfe78f2382c169d032915f78bf0cdca27f3f25`，相对 `750f933bddffc8251a880555b9beae2cdce75690` 仅Rust、Python和新r13_integrity.py三文件。8 AC为7 PASS/1 FAIL；17项B结构子义务为16 PASS/1 FAIL；无NOT_VERIFIED。新发现R13-M1：未发布raw坐标损坏时，accept和recover仍实际写入。只有advance拒绝不足以满足AC7。

会话 `/root/r7_codex_product_review`，runtime=`codex_native_subagent`。本代理没有产品/Git/GitHub写入；全部反例只操作owned副本。原R7/R12及750f分诊保持冻结，本报告只判b9；不审尚未冻结的R14。

## 三个P2与新缺陷

| 评论 | b9判断 | 证据 |
|---|---|---|
| 3996936242 源坐标 | 具体输入缺陷已修：非规范/负数/越界拒绝；跨身份碰撞全批回滚；去除静默回退和覆盖。相关存量完整性仍有R13-M1。 | E03/E08/E10 |
| 3996936244 未来索引 | 已闭合：四表六字段全表真实整数1..G门，两端共享root在历史过滤/g0早退前执行。旧5反例、18字段边界和g0索引均拒绝。 | E03/E04/E08/E14 |
| 3996936247 writer epoch | 已闭合：持久、配置、recover目标同一规范非负i64域，错误分别StorageUnavailable/InvalidDomain；旧或等代StaleWriter。0→i64max后可真实接纳推进。 | E03/E08/E12/E15 |

**R13-M1（P2）：坏未发布原始源坐标未进入共享完整性门。** 触发有四种：G0已接纳未发布，或G1已发布前沿外e3，分别将坐标改为`bad`或已被其他身份占据的`2`。Python真实Handler的current/AsKnown均200，Rust snapshot退出0；每个坏库的另一副本accept新事件成功且raw+1；另一副本recover2成功且epoch1→2、writer_epoch_history+1。advance则StorageUnavailable，整个SQL dump不变。（E09/E10 /cases/0–3 /operations）

代码：Rust L1599–1652已实现坐标规范及identity↔coordinate双向归属核，但生产调用仅L2021的advance；共享root L926起、accept L1342起和recover L3714起未覆盖这组未发布raw。Python共享root L340起只核发布代、profile和已发布历史，无同义raw前件核。（E03/E04）

后果是S自有坏库仍可接纳新事实和换writer，违反#1371 AC7“自身存储损坏准确报StorageUnavailable并停止相应正式写入”。A-ES-15-02所承接的非法恢复迁移拒绝锁也未闭合。已阻止新结构发布不等于停止相应正式写入；E/B/X未启动亦不能豁免。这不是C的规模问题，也没有证明这四组AsKnown泄露了未来内容。（E17/E19/E20）

最小修复：Rust/Python共享root核全部raw坐标的规范非负/平台无损整数域和双向身份归属，包含pending raw；advance在原Begin事务复用已读取验证的vector，避免再读/再校验全量。四原例应让读接口报StorageUnavailable、accept/advance/recover失败且全库事实不变；合法pending、同身份revision共享位置、稀疏大整数必须保留。

## 新执行与复用边界

独立主检查49例成立：178个原生进程记录（其中3 SIGKILL和1受控release）、78次真实Python Handler捕获。补充4个产品反例成立：18个原生CLI和8次Handler。合计53个oracle、196个原生进程记录、86次Handler；**这些计数不能写成产品53 PASS**。原件清单共103个文件，逐文件SHA在EXECUTION.json；没有新HTTP、服务、GUI或cargo。

新binary SHA `2299fcc6a1f416ae79fdde176356dcf46d437a3cf4a710530595b7434ed7f391`；SOURCE manifest与b9五个Git对象逐字节相同。没有用旧binary证明新Rust。root另跑45个Rust测试、原消费者10项、未来索引5臂（39 CLI、21 Python read_state）及epoch7臂；本代理审其结果，不重复运行或冒称独立执行。

本代理的新三杀均从三输入一次提交的g1 TOP开始，更正是g2、追加是g3，每臂各有同输入、profile、时间和调度的未中断control。marker核对PID/阶段/DB/exe后SIGKILL，退出均-9。Begin后和batch后保留g1，需recover；Commit后已g2而无advance回执，query原身份、recover和重试不新增代。随后旧epoch拒绝、修订重放、继续追加；g2当前、AsKnown1、g3当前及完整Watch与各自control逐字段相等。对拍对象是完整S快照/流，未声称恢复日志的时间/token与control相同。

Begin读取移动也单独实测：暂停在持久Begin后接纳e3，再释放原advance；旧尝试因frontier变化取消，根留g1、门恢复idle。其废弃batch仅含原4条raw，未混入后来e3；重试后完整Snapshot等于含5条raw的正常control。不可达batch合法保留，发布代门没有误伤它。源位置`9007199254740993,9007199254741010,9223372036854775807`经完整Rust链仍逐字输出。

旧R12的HTML/consumer及协议未改变，原真GUI/launcher记录仅支持相同外壳接线与已验证页面行为；本轮新行为由新native/Handler证明，未把CLI冒充launcher或GUI。更广的CC/G、经济域、C负载及全图仍开放。

## 八项AC

| AC | b9 | 本轮依据 |
|---|---|---|
| AC-1 | PASS | 新b9二进制真实接纳三输入成TOP、同e2新revision成RISING、追加e3成新TOP；大于JS安全整数至i64最大稀疏源坐标不丢精度。Begin内读取与frontier同事务，分类仍在持久Begin之后。launcher与HTML接线沿R12同字节历史证据，未称本轮新跑launcher/GUI。 E03/E08/E16 |
| AC-2 | PASS | 非法/碰撞源坐标整批拒绝且全库dump不变；合法同身份更正和重放继续。三新故障臂与各自对照完整Snapshot/Watch相同，旧对象first_known及撤回历史保留。 E03/E08/E12 |
| AC-3 | PASS | 三臂正常恢复、修订和追加后的Python/Rust完整Snapshot、同轨control和Watch全字段相同；前沿交错取消旧尝试不发布混合cut。R12消费者两个门的同字节HTML/fixture保留且root10项通过。坏库停写缺陷另见AC7。 E03/E04/E08/E13/E16 |
| AC-4 | PASS | 本代理用新binary对Begin后、batch后、Commit后各发真实SIGKILL，核marker PID/DB/exe与-9；在原副本recover2并继续至g3，各自对照g2/g3/current/history/watch逐字段相同。旧三杀不冒充新binary。 E07/E08 |
| AC-5 | PASS | Commit后SIGKILL没有advance回执，原业务身份query可读；recover后重试idempotent，不多代；旧epoch拒绝。非法持久与配置epoch分别StorageUnavailable/InvalidDomain，0→i64max合法、等代和旧代StaleWriter。 E03/E08/E15 |
| AC-6 | PASS | 新执行在更正后AsKnown1与自身control完整相等，原TOP历史保留；pending raw不进入已发布历史，合法当前和历史Python/Rust完整相同。未改历史/HTML协议，沿用R12模式及页面门原证据。坏库仍200不是本轮已证明历史事实泄漏，归AC7。 E04/E08/E10/E16 |
| AC-7 | FAIL | R13-M1四反例中未发布raw坐标已损坏，state/snapshot仍成功，accept实存新增raw，recover实存换epoch/history；只有advance拒绝且零Begin。违反S自身存储损坏准确报错并停止相应正式写入；E/B/X保持未启动不能豁免。 E03/E04/E09/E10/E17 |
| AC-8 | PASS | 保留固定源码/build/commit、完整命令/输入/DB、53个case oracle及四产品反例；真实三杀有marker/信号/恢复及各自对照。8AC/17子义务完整重新定性，未覆盖项与旧证据复用明确，整体FAIL。 E01/E02/E07/E08/E09/E10/E16 |

## 17项B结构子义务

每项原语义理由、限制及原证据指针完整保留于REVIEW.json的inherited_review，当前判断按新证据重定；17场景不替代17来源义务。

| 来源ID | b9 | 本轮依据 |
|---|---|---|
| A-CC-006-03 | PASS | b9新三杀及各自未中断control在同输入/修订/profile/时钟下完整S Snapshot/Watch相同；沿用原签署受测域，内容身份/first_known不归一化。 E03/E04/E08/E13/E16 |
| A-CC-006-04 | PASS | 严格local_shape核未改；新正常三点/更正/追加及稀疏大整数链通过真实Rust路径并核源坐标与完整双端输出。其他G语义仍范围外。 E03/E04/E08/E13/E16 |
| A-ST-044-02 | PASS | 对象/关系/见证的原子Commit代码未改，新的三杀恢复与Watch/快照对拍覆盖受影响门；撤回历史和旧端点仍可追。 E03/E04/E08/E13/E16 |
| A-ST-044-03 | PASS | 新binary实际恢复、继续追加及同cut回读与各自control全字段一致；原浏览器重复不重生效门同字节复用。 E03/E04/E08/E13/E16 |
| A-ES-15-01 | PASS | 新执行的所有健康快照仍明确structure CompleteCut/economic not_started，未启动域无补造记录；结构历史范围不扩大。 E03/E04/E08/E13/E16 |
| A-ES-15-02 | FAIL | 合法三杀恢复与epoch拒绝锁通过，但R13-M1坏未发布raw仍允许recover换代，缺少已承接存储前件下不可发生迁移的拒绝锁。仅advance阻止新发布不足。 E03/E04/E08/E10/E17/E20 |
| A-RA-04-01 | PASS | 新修订与各自control的AsKnown1/current完整深等，旧对象身份/首次获知与撤回次序保留；未扩展经济计划责任语义。 E03/E04/E08/E13/E16 |
| A-RA-10-01 | PASS | 健康S的固定cut、Snapshot/Watch及修订史经新b9三臂完整对拍；HTML Gap绑定与R12原件同字节复用。坏未发布raw没有被本轮证明混入历史内容，存储错误行为归AC7。 E03/E04/E08/E13/E16 |
| A-I-02-01 | PASS | 此票分配的E/B/X真实未启动、S自身健康时推进/撤回/恢复子域由新native成功链支持。全域故障分区仍未验；自有坏库停写单独AC7为FAIL，不从本项健康前提推导坏库合格。 E03/E04/E08/E13/E16 |
| A-OB-004-01 | PASS | 接纳seq与源位置分别持久；新坐标门保一身份一位置，合法revision共享原位置，跨身份占位拒绝且全批回滚；大整数原样输出。 E03/E04/E08/E13/E16 |
| A-OB-008-01 | PASS | b9的完整Delta和同cut Snapshot经新Watch/快照对拍；R12 replaces完备门同字节，root10项含漏项/端点/重复/多字段拒绝与合法重试。 E03/E04/E08/E13/E16 |
| A-OB-009-01 | PASS | 健康未发布输入在g0/已发布快照不可见，历史AsKnown维持当时记录；新三杀旧TOP与修订后完整历史对拍。R13-M1指坏库不报错，不能伪称本项已出现未来内容。 E03/E04/E08/E13/E16 |
| A-OB-014-01 | PASS | 新更正三杀恢复后仍有旧TOP的身份/first_known/源证据/撤回理由，且与control完整相同；经济撤回/已成交分派仍范围外。 E03/E04/E08/E13/E16 |
| A-OB-015-01 | PASS | 前端pageEpoch/会话门未改且与R12同字节；原真GUI错会话拒绝与合法B会话迁移证据有界继承，root10项函数回归通过，不冒称新GUI。 E03/E04/E08/E13/E16 |
| A-OB-016-01 | PASS | 新生产者Watch连续代与本次三杀control全字段相等；Gap/重建门未动，沿R12原同字节协议证据；不新增retention/长连接压力证明。 E03/E04/E08/E13/E16 |
| A-OB-017-01 | PASS | 真实Begin后追加新输入触发frontier变更，原尝试取消不新发cut；废弃batch保留原4条raw，不混入新e3；重试与含5条raw的正常control完整相同。 E03/E04/E08/E13/E16 |
| A-OB-020-01 | PASS | 逐项明示新源码/运行、可复用旧壳证据和实际FAIL；保留原R7/R12与750f反例，不把53个oracle成立写成产品PASS。 E03/E04/E08/E13/E16 |

## 主要证据

绝对路径、SHA、行号/JSON pointer如下；每个case的argv、输入、输出、实存前后事实和信号记录在E08/E10，原数据库及输入文件逐件列在EXECUTION.json。

| ID | 原件与定位 | SHA256 |
|---|---|---|
| E01 | [SOURCE.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/SOURCE.json)；/files；Rust/Python/new test/HTML/consumer固定源SHA | `4c753c2c2095d6dd071f7fbe3b5fd1a47554b8ce1edc68fb701eb30575496955` |
| E02 | [COMMIT-BINDING.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/COMMIT-BINDING.json)；/all_five_git_objects_match_manifest=true；750f→b9仅三文件 | `5d1d94ad6368d13c948287b67767061746fdf33074f575535b656fe76cd50215` |
| E03 | [s_structure_session.rs](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/source/rust/src/bin/s_structure_session.rs)；L107–121,328–352,906–950,1342–1590,1599–1680,1993–2115,2296–2332,2620–2635,3714–3790；增量测试L4429起 | `06cfb85b0d47c11c2d562f4aacd4838aedfb5dbc98cffffe4f3075d660c20ab5` |
| E04 | [s_readonly_server.py](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/source/s_session/s_readonly_server.py)；L322–383和L1126–1176；全表代际门已入共享root，未有raw坐标共享检查 | `26bc65bc7550fe3929dcabe451f6567d4107801b5d007e7eaf76b60598a67d6d` |
| E05 | [r13_integrity.py](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/source/s_session/tests/r13_integrity.py)；完整137行；同源造库、五变异、双端读取和三写拒绝 | `f6b5d87a2caa06474c2b23355fb9fc555e2cc1c40523c248fcd9f745d271a6e5` |
| E06 | [git-diff.patch](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/git-diff.patch)；完整750f→冻结源Rust/Python diff；另E05补新文件 | `7e8b02a8c55bc45183b0c42541799be165c694ef64f2cbd7c5788259a5ddb762` |
| E07 | [independent.py](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/independent.py)；完整独立执行脚本；CLI/Python真实Handler，三次SIGKILL+一次前沿交错 | `410dde55455d42610a771171ae4ee3f26fcc3bf603b6b7600b441c4b5795da71` |
| E08 | [RESULT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/independent-run/RESULT.json)；/cases 49；/calls 178；/handler_calls 78；/failure=null；source_hashes_unchanged=true | `dfcec13a70009298b09a2a9f4d37833648721a7a125974f72c44b34308cd3572` |
| E09 | [raw-integrity-gap.py](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/raw-integrity-gap.py)；完整四个待发布原始行损坏的独立反例脚本 | `72c7643f8e731f0f4e067d8f0ccbeb5f60c18f125c5ae169f88ec6280e5c5a48` |
| E10 | [RESULT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/raw-integrity-gap/RESULT.json)；/cases 4；/calls 18；/handler_calls 8；每case/operations accept/recover发生实存变化、advance零写 | `7810454a507e2bd05666dbaa91335dc3188f2cb78700ee0ad5003d19616d6f8b` |
| E11 | [BUILD-RESULT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r13-codex-incremental-review/BUILD-RESULT.json)；固定binary sha2299fcc6…及source manifest绑定；构建由root执行 | `3bc6d05d18d6ef87c2689c40de7011ff17e24ab56ffd6a390ba147cbcecbacc9` |
| E12 | [RUST-ALL.log](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/r13-build/RUST-ALL.log)；L538–582共45条结果均ok；L584总45/0；不是本代理重跑 | `a4e80284ce418aab84899177e6cb6d8a014d5101a2c5b32e34351abc3d962244` |
| E13 | [CONSUMER.log](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/r13-build/CONSUMER.log)；/cases 10项passed；/failed空；HTML及fixture SHA与R12一致 | `97438ab806a11d26aa70cbde0ea245e6bcb87c7f1b0a7adac2d7c44adf5e3704` |
| E14 | [RESULT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R13-INTEGRITY-GREEN/RESULT.json)；/cases 5；/calls 39；/reads 21（15失败、6健康）；/failure=null；root新运行 | `6cb655f1aca57474adee23f058c84d63e549592955627b5e7ed09d608ec0a8e0` |
| E15 | [RESULT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-EPOCH-GREEN/RESULT.json)；/cases 7；root在新binary下的原epoch反例与正控 | `d20db318fd8a05c6dbacc9d8d08f7ca8b65b550a149598c64389315015d72fd9` |
| E16 | [REVIEW.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r12-codex-incremental-review/REVIEW.json)；/ac 8、/obligations 17，继承原语义理由/逐项证据与限制，不继承未覆盖缺陷的PASS | `2ea4c4f30949b9323f20ff688dafe21fb9ccf1f4b1fc1bd351f8c58d9597fea0` |
| E17 | [PR1449-PERFORMANCE-TRIAGE.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-PERFORMANCE-TRIAGE.json)；/live_issue_captures/1371/body_lines L13/16/17/32/41/51；1372 L11/14/18；性能C限制 | `693ae821b82a0ee50a034887c9a957d3fd58f785ec94df4a06037d1fcb67dcfa` |
| E18 | [SPEC.md](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/SPEC.md)；L253 Begin/Commit、L311固定cut、L315历史、L325持久可达引用 | `42fc34cd0bb6736469db5b6e652f861bee41b6ccc5808677125fae831b75bd11` |
| E19 | [INTERFACE-CONTRACTS.md](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/INTERFACE-CONTRACTS.md)；L13错误、L20–23 S接口、L55本地原子组 | `f1a2993c6fa4c6c29c6b9a4de7c1d0334c697aaa03e282f621171c37fc01d0e7` |
| E20 | [SPEC-COVERAGE-INPUT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json)；/engineering_state_axes_with_r2_replacements/14/acceptance_points/1 A-ES-15-02；/independent_review_implementation_obligations/1/acceptance_points/0 A-I-02-01；来源8AC/17分配由票面持有 | `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c` |
| E21 | [PR1449-INLINE-REVIEWS-LATEST.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/PR1449-INLINE-REVIEWS-LATEST.json)；comment3996936242/3996936244/3996936247和性能3996927376 | `d5df34370f81e4b5b3c0b0f17b9ba0b017151169c2d38d0ee07d805c18d88779` |
| E22 | [PR1449-FUTURE-ROWS-TRIAGE.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-FUTURE-ROWS-TRIAGE.json)；旧750f真实五类未来索引反例；本轮再复制同SHA源坏库 | `356fac42330c0c22467dcbdfe900007be0b470bf48b13832aaf4d5b93538f76a` |

性能3996927376按先前有证据的C分诊继续开放，不新增性能目标。R13-M1是B已承接的当前正确性缺陷，不降格为范围外残余。完整#1323、全部来源量词及#1448旧R6迁移未销项，旧R6仍INCOMPLETE。工具边界由指令遵守，不冒称OS沙盒；本报告无合入或关票授权。
