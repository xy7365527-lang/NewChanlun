# PR #1449：重复全历史校验性能分诊

**判定：风险成立，当前不作为 TB-01-B 的必修阻断；它是已经具名留给 #1372（TB-01-C）的持续输入、固定切面分页、短读与实际负载义务。** 这不是“评审已PASS所以忽略”。代码在历史增长时会做越来越多重复工作；B的有限成功轨迹没有证明长历史可用，C必须处理并验证此风险。

对象为已发布提交 `750f933bddffc8251a880555b9beae2cdce75690`，评论 [Greptile P2 #3996927376](https://github.com/xy7365527-lang/NewChanlun/pull/1449#discussion_r3996927376)。本轮只读查证，没有改代码/Git/GitHub、跑测试/服务或裁新性能目标。

## 源码与实际影响

- Rust `verify_reachable_root_in_tx`（879–1043）读取并逐代遍历整个 `structure_deltas`。每代在624–625行重建前后累计快照，再对批次、对象、撤回、关系、见证和原始修订做对拍。
- 此检查进入accept的IMMEDIATE事务（1332–1338）、advance Begin事务（1927–1934）、后续所有权门（1860–1868，Commit在2228–2233调用）以及recover事务（3648–3665）；catalog/snapshot/query/watch也触发。历史增长会扩大CPU/分配/SQL工作和写锁持有时间，不能当成只在启动时进行的后台审计。
- Python同义路径为322–360 → 166–262，其中185–186同样重建两个快照。`read_state`（771–773）分别调用catalog和snapshot，各自再次完整验根（496、633），所以一次 `/api/state` 做两遍。空Watch也在952先验历史；HTTP读事务1116–1130包住这些工作。没有跨页持有事务，并不等于单次事务在长历史下足够短。WAL读事务也不自动证明会阻塞写者；WAL保留、资源争用和实际延迟尚需测量。

静态工作量应精确表述：设第g代累计快照大小为Sg，双快照物化部分随 `Σ(|Sg−1|+|Sg|)` 增长；在每代近常量增长的常见轨迹中，这部分单次全根检查呈二次增长。逐对象线性查找（Rust631–633/Python194）、排序、批次大小和修订还会增加成本，不能把整次调用普遍断言为精确平方，也不能把这段静态推导当成实测吞吐/时延。

版本核对：此完整载荷校验在基线2212中不存在，属于B为存储完整性加入的工作；Rust与Python在492254、e5b0362、750f933三版逐字节相同。e5→750只新增21份具名报告。因此它是B实现带来的真实性能遗留，但不是R12两个消费者修复或750报告提交新增的回归。

## 为什么归已存在的C，而非事后缩小B

[#1371](https://github.com/xy7365527-lang/NewChanlun/issues/1371) 的8条AC定位在当前票面body行11–18：固定原始输入更正/追加、原子Delta、真实三杀恢复、epoch/DeliveryUnknown、历史视图和S自身损坏停写。AC7确实要求自身前件损坏时停止写入，不能为降低成本直接取消历史完整性门；但票面没有把长历史并发分页/声明负载持续推进的成功门放在B。完整图仍保持。

[#1372](https://github.com/xy7365527-lang/NewChanlun/issues/1372) 已于票面承接：AC1（body行11）用足够非空输入跨多页，翻页时S继续提交；AC4（行14）明确有限短读、不跨页长事务，在实际负载下accept与Commit序号继续前进，积压有界且不得靠停输入/无界内存通过；AC5（行15）进程独立重启后继续；AC8（行18）记录实际负载并在A/B/C全完成后才关闭TB-01。其来源映射有A-RA-13-01、A-OB-006-01、A-OB-018-01、A-OB-019-01，这些不在B的17项承接清单里。

签署SPEC的C08（311行）和C09工程方案（325行）本就要求固定cut及长期不可变历史引用、短事务取得引用、不跨请求读事务；432行明确性能只报实际负载测值。这里的短事务是仍需在C负载下完成的目标，不是当前函数注释写“短”就已证明。RA-13完整经济公平义务仍在后续TB；C只承接结构持续推进与慢消费者隔离。

原R7并未宣称这一面已过：A-ST-044-03明确“C1372保留/慢消费”未证；A-RA-10-01明确未新增分页边界/长历史/保留压力；A-OB-016-01明确没有真实retention或慢客户端负载。R12继续保留这些限制，只对两门及有限函数/GUI证据作增量复审。**性能风险保持OPEN；B bounded PASS不得外推为长历史性能PASS。**

## C需要的证据及改变判定的条件

- **C原已要求的负载证明**：经同一正式launcher，在明确实际输入率/历史长度/修订分布/并发浏览页数/慢客户端/游标保留域/复现时钟下，记录接纳与Commit序号确实继续推进；报告read/write校验工作量、读写事务时长、CPU/内存/WAL或等价资源观测，固定token多页仍同cut。数值目标由既有目标/实际负载声明或后续明确决定给出，不由本分诊自造。
- **优化后保持B完整性**：同输入同切面完整输出等价；旧/中间代Delta缺失、坏batch字节/坐标/profile、内外载荷错配、epoch/Begin所有权、三阶段恢复等原B不变量仍拒绝或恢复正确。可按改动面复用旧证据并做靶向验证，不要求本轮重跑。
- **检查点/已核缓存的边界**：只推荐可论证同义优化，不预选算法。若将逐次历史验证改为曾经验证即永久信任，须说明如何发现旧持久记录被修改/损坏、缓存失效与重启/换代规则；若改变原存储损坏可发现性或持久信任前提，就是语义/合同变化，需相应决策与新证明，不能隐入性能修复。
- **何时重新成为B阻断**：若针对B已声明轨迹发现不可完成、错误StorageUnavailable/超时导致交付失败，或任何优化削弱B原完整性/原子性/恢复行为，则作为B回归重新打回。当前Greptile静态提示和已有原件尚未给出这类反例。

这次分诊没有指定checkpoint算法或数值目标。仅消除同请求重复全校验等可保持语义的实现优化，可以在C实现方案中论证；若改变“旧持久记录仍须可发现损坏”的信任前提，就必须明确处理合同/语义变化。不能把“留C”解释为可以不处理、用小样本宣布C成功，或关闭TB-01/#1323。

## 精确证据

1. `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/PR1449-INLINE-REVIEWS.json`；SHA256 `40dd01d15dc26765f40b26cbffb5466a5086ef47ce3848d7b9f5e2359e96ea94`；/0：id3996927376、body、commit_id、path、line1043；评论是提示，判断另由源码/规范建立。
2. `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/rust/src/bin/s_structure_session.rs`；SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`；L565–676（尤其624–625），L879–1043，L1332–1338，L1860–1868，L1927–1934，L2228–2233，L3217–3285，L3400–3434，L3573–3579，L3648–3665；与750同字节。
3. `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/s_session/s_readonly_server.py`；SHA256 `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c`；L166–210（185–186），L322–360，L494–496，L632–677，L771–773，L950–952，L1107–1157；与750同字节。
4. `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/SPEC.md`；SHA256 `42fc34cd0bb6736469db5b6e652f861bee41b6ccc5808677125fae831b75bd11`；L311固定cut/恢复完整性；L320–325已签工程方案的长期历史/短事务引用；L430–432负载和性能测值纪律。
5. `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/INTERFACE-CONTRACTS.md`；SHA256 `f1a2993c6fa4c6c29c6b9a4de7c1d0334c697aaa03e282f621171c37fc01d0e7`；L18–24 S输入/Advance/Snapshot/Watch合同；L22固定切面分页位置。
6. `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/review-package-r7/inputs/candidate/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json`；SHA256 `76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`；RA13 /rf02_acceptance_obligations/12；OB006/018/019 /observation_contracts/5,/17,/18；规范完整义务，#1372仅承接其结构子域。
7. `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r7-codex-product-review/REVIEW.json`；SHA256 `6eaf1423f9b51b3ac878efcd7732dac147a6093de1c50d87f12eba1eacf0a602`；/ac/6：S自身存储损坏必须停写；/obligations/3/limitations; /obligations/7/limitations; /obligations/14/limitations。
8. `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/r12-codex-incremental-review/REVIEW.json`；SHA256 `2ea4c4f30949b9323f20ff688dafe21fb9ccf1f4b1fc1bd351f8c58d9597fea0`；/limitations/0,/6,/9；/ac/6的同字节复用；17个有限函数用例与GUI不证明长历史成本。
- #1371本次gh只读原响应SHA256 `fdc7ca0cc1d7882cf8603513ebe6ce87191a498808f814e056ec20c8b94d4b4c`；body UTF-8 SHA256 `71672e11e78d9cd2739f9d45a4a10c8a8b3420e2c1ccd11032e30858e7e4e3b5`；完整原响应与body行号已内嵌相邻JSON的 `live_issue_captures/1371`，抓取于 `2026-09-12T17:09:27.962965+00:00`。
- #1372本次gh只读原响应SHA256 `7228834750c340f3ce495595cdda007ce30e26ddb11374c55484ed5978eedc92`；body UTF-8 SHA256 `c8eddb266a2e90a124e02eac6918ccce04f27b514a83f12cf6ea3e1c1fa007a3`；完整原响应与body行号已内嵌相邻JSON的 `live_issue_captures/1372`，抓取于 `2026-09-12T17:09:27.962965+00:00`。

原R7与R12报告不改写；#1448/R6同ID迁移不销项。本分诊不代替CI、分支保护或最终合入动作。
