# PR #1449 未来代际索引行独立分诊

**结论：确认 P2，属于 #1371 TB-01-B 必修存储完整性缺陷；当前 AC7 为 FAIL。** 固定候选 `750f933bddffc8251a880555b9beae2cdce75690`。仅审 [评论3996936244](https://github.com/xy7365527-lang/NewChanlun/pull/1449#discussion_r3996936244)，不处理另外两条P2。

会话 `/root/r7_codex_product_review`，runtime=`codex_native_subagent`。只写本分诊与隔离副本证据；未改产品/Git/GitHub。W13尚未冻结，未复测它。

## 触发、代码与实际后果

g5健康库额外含未发布的索引行：objects的first_known_generation=published_generation=6（活动，或withdrawn_generation=7），或witnesses/relations/observations的published_generation=6。原g1–g5 Delta、批次、根及输入均保持。

Python根核L322–360、Rust L879–1043只遍历已发布代；载荷验证分别L185–186和L624–625构造上一代/本代历史Snapshot。历史筛选把这些未来行排除，所以既有Delta与批次一致性验证看不见它们。Python当前objects L614–620、witnesses L671–677、relations L692–697、observations L699–717直接读全表；Rust对应L3042/3052、L3268附近、L3308附近、L3341附近。现有全表对象生命周期检查只拒绝withdrawn<=first_known，未来但相对有序的行通过。（E2/E3）

**五组反例均在current公开多1条未发布事实，声明的根仍为cut5。** Python实际Handler返回200，Rust正式snapshot退出0，完整Snapshot跨语言逐字段相等。每组AsKnown5仍与健康库完整响应相等：这不是“历史接口已泄露未来”的证据，而是历史筛选正确却没有识别坏库。

每组再取独立坏库副本，实际执行Rust `recover --new-epoch 2`，均退出0，writer_epoch从1变2且新增一条writer_epoch_history；坏索引和g5仍在。Rust recover L3665先调用相同根核，再于L3687后写换代记录。故不是只读呈现问题，**坏库仍发生正式恢复写入**。accept L1338和advance L1934共享该门，源码显示同一漏检；本轮没有动态执行它们，不把它们列为实测。（E3/E9/E10）

## 隔离复现

原件为冻结native run2正常库（g5，2活动/1撤回/9见证/14关系/1观察），WAL为空；逐字节复制到本代理owned目录才用SQLite打开和注入。复制原native二进制的同SHA字节执行。Python原源码compile到独立命名空间，调用真实Handler._read_only并捕获发送内容，不启动HTTP服务器、网络或GUI。27次原生CLI、18次Handler调用，共9个唯一案例。

| 副本案例 | Python current/AsKnown5 | Rust current/AsKnown5退出 | Rust recover退出及实存epoch |
|---|---|---|---|
| healthy | 200/200 | 0/0 | 0，epoch 1→2 |
| future_active_object | 200/200 | 0/0 | 0，epoch 1→2 |
| future_withdrawn_object | 200/200 | 0/0 | 0，epoch 1→2 |
| future_witness | 200/200 | 0/0 | 0，epoch 1→2 |
| future_relation | 200/200 | 0/0 | 0，epoch 1→2 |
| future_observation | 200/200 | 0/0 | 0，epoch 1→2 |
| control_existing_object_published_future | 503/503 | 1/1 | 1，epoch 1→1 |
| control_existing_withdrawal_future | 503/503 | 1/1 | 1，epoch 1→1 |
| control_future_delta | 503/503 | 1/1 | 1，epoch 1→1 |

五个反例current各多出的集合依次为objects、withdrawn_objects、witnesses、relations、observations；其他原集合保留。全部AsKnown5完整响应等于正常对照。三个负控已有守卫正确拒绝，recover前后逻辑状态完全相同，排除了探针普遍绕过校验的解释。RESULT和ASSERTIONS明确记录原件前后SHA未变。（E9/E10/E11）

首轮原件二进制没有执行位，Python读完正常副本后native尚未开始即PermissionError；保留ATTEMPT1和原脚本。随后只给owned同字节复制文件赋执行权限，原件及权限未改。最终脚本exit0意为五个产品反例及对照断言成立，产品结论仍FAIL。（E8/E12）

## 为什么必须在B修

#1371 AC7（票面展开L17）明确要求“S自身存储损坏准确报StorageUnavailable并停止相应正式写入”。此反例以5代的原有效TestOnly库即可触发，不需要长历史、多页或新的性能目标。AC3（L13）要求指定同cut Snapshot与原子变化相等；当前cut5包含无发布变化支持的额外事实，也破坏该集合一致性。签署SPEC L253的原子公布、L311的固定cut、L325的可达引用，以及接口合同L20–23/L55的Commit发布约束均支持拒绝。（E4/E5/E6）

#1372 C的分页、慢查询、实际负载持续推进要求（L11/L14/L18）不承接B已经明写的坏库停写豁免。与此前全历史重复校验性能项不同，本条是当前域已实证的正确性缺陷。它在R12两HTML修复前已存在，不称由那两门引入。

原R7/R12报告保持原字节冻结。R12 AC7曾基于原根/profile损坏与21副本证据给PASS；该证据不覆盖本组未来索引，不能继续支撑当前验收无保留通过。旧Rust `future_withdrawn_object_is_rejected_as_corrupt`（L5068–5104）实际改withdrawn_generation为0，测试非法生命周期，**并未测试超meta.generation的未来已发布索引**。本新增finding纠正当前AC7判断，不篡改旧记录；此前M1/M2闭合与本条并不冲突。（E3/E7）

## 定点修复与回验建议

在Python与Rust共同verify_reachable_root的全表前置门验证四类索引，必须在任何历史过滤和generation0提前返回之前：objects的first_known_generation、published_generation和非NULL withdrawn_generation；witnesses、relations、observations各自published_generation。全部行都检查，包括已撤回对象。保持现有生命周期及逐代内容校验，错误统一StorageUnavailable。

建议这些**已发布索引**的代字段为真实整数且位于1..=metaG；metaG=0时四表应空。仅允许0..=metaG会留下初始空cut问题：Rust L914–921在无Delta的g0提前Ok。g0是初始空切面，正常Commit从g1生成索引。该g0提示是源码推论，本轮没有新增g0注入，不作为已执行案例计数。

不要只给current加WHERE来隐藏坏行，否则AsKnown、恢复及写入仍放过坏库。也不要误禁已接纳但尚未发布的raw_events，或崩溃后未被根引用的不可变batches；SPEC L325允许后一种残留。

回验应令本五反例的current/AsKnown和Rust snapshot/recover都拒绝，并证明writer_epoch/history/根不变；补accept/advance写前拒绝断言。保留健康库、三个已有负控、合法待发布raw与不可达batch、g0空库正控。不需要为了本门重跑完整native长轨迹或GUI，也不应凭这些定点结果关闭全图。

复用入口：`/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-FUTURE-ROWS-PROBE/probe.py`记录全部变异和命令；每个case目录的`read.sqlite`是未被recover写过的坏库，`recover.sqlite`才是已执行换代的副本。新版本测试应再次复制`read.sqlite`到新的owned目录，然后向新binary传` snapshot --db <copy>`、`snapshot --db <copy> --as-of 5`、`recover --db <另一个copy> --new-epoch 2`。现有probe固定750f源码/SHA且拒绝覆盖已存在副本，不能把原报告目录直接当新版本输出。新候选冻结后再由独评接续。

## 证据定位与SHA256

所有完整绝对路径、大小、JSON pointer、逐案例变异前后行及执行回执保留在同名JSON；下表提供主要原件的精确版本。

| ID | 原件/定位 | SHA256 |
|---|---|---|
| E1 | [PR1449-INLINE-REVIEWS-LATEST.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/PR1449-INLINE-REVIEWS-LATEST.json)；JSON /2; id=3996936244 | `d5df34370f81e4b5b3c0b0f17b9ba0b017151169c2d38d0ee07d805c18d88779` |
| E2 | [s_readonly_server.py](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/s_session/s_readonly_server.py)；L166–262, L322–366, L607–747, L1107–1157 | `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c` |
| E3 | [s_structure_session.rs](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/rust/src/bin/s_structure_session.rs)；L189–238, L565–676, L875–1043, L1332–1338, L1927–1934, L3025–3056, L3217–3359, L3648–3710, L5068–5104 | `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901` |
| E4 | [SPEC.md](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/SPEC.md)；L253, L311, L315, L325 | `42fc34cd0bb6736469db5b6e652f861bee41b6ccc5808677125fae831b75bd11` |
| E5 | [INTERFACE-CONTRACTS.md](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/INTERFACE-CONTRACTS.md)；L13, L20–23, L43, L55 | `f1a2993c6fa4c6c29c6b9a4de7c1d0334c697aaa03e282f621171c37fc01d0e7` |
| E6 | [PR1449-PERFORMANCE-TRIAGE.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-PERFORMANCE-TRIAGE.json)；/live_issue_captures/1371/body_lines: L13,16,17; /live_issue_captures/1372/body_lines: L11,14,15,18 | `693ae821b82a0ee50a034887c9a957d3fd58f785ec94df4a06037d1fcb67dcfa` |
| E7 | [REVIEW.md](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r12-codex-incremental-review/REVIEW.md)；L1–5; AC-7 row; 同字节复用与范围; 限制与未检查项 | `4aaf77dadce536c1e807b9790ec045f42cb32f2a339afc55aaa572605c30ad43` |
| E8 | [probe.py](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-FUTURE-ROWS-PROBE/probe.py)；完整执行脚本；冻结源Python输出捕获和Rust原生CLI，所有DB仅为owned副本 | `4284698f862f07a08c983cf8f82d5332c6425b619e740030370f8ecb329ffc6f` |
| E9 | [RESULT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-FUTURE-ROWS-PROBE/RESULT.json)；/cases/0正常；/cases/1–5五反例；/cases/6–8三负控；/original_sources_unchanged=true | `e9e7e76de62a7c0e5f4608065bc63507c0da2635b9ac4165484ce28387c09e28` |
| E10 | [ASSERTIONS.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-FUTURE-ROWS-PROBE/ASSERTIONS.json)；完整Snapshot Python/Rust深等；五反例AsKnown完整响应等于正常；恢复实际换epoch/history | `cb554ce4c36060bf076a89991c7da2d1ef39992e67e27ff0407589bba717bc1c` |
| E11 | [source-hashes.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-FUTURE-ROWS-PROBE/source-hashes.json)；原库、空WAL和native二进制来源SHA，执行前后核对 | `76b31e37e4003c0757e32a49fa4ce290817257df99aae4650f2448502da8f8b3` |
| E12 | [ATTEMPT1.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-FUTURE-ROWS-PROBE/ATTEMPT1.json)；首轮证据binary无执行位，未执行native；仅owned复制文件增加执行位 | `c33ad62abe1201d5e246906585298ac7f31f3264ef607733d85834b3f97b26c6` |

边界：本轮没有证明正常写者自行制造这些未来行；实际模拟的是已由AC7承接的存储损坏/错误迁移。没有新服务、HTTP、GUI、完整测试或SIGKILL；原库从未打开写入。工具边界来自指令约束，不冒称OS沙盒。#1323完整目标、#1448旧R6迁移未满足及范围外义务均继续开放。
