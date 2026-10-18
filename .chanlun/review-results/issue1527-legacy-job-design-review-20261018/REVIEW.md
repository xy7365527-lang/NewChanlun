# #1527 旧域同 Job 草案独立设计审查

**结论：BLOCK（设计闭合）；C=0，H=3，M=1。** 草案的旧行验证移位、内容域隔离和单 Job 方向可保留；目前不能标为“已可实施”。阻断项是可点名的接口及调用边缺口，不是“G1–G4 尚未批准”本身，也不是要求 I2–I5 全部验收后才能编码。本文未审判当前返修产品是否有这些缺陷。

本报告是 OPEN #1527 的工作草稿。工位为 Sandcastle 外部 Codex 工蜂；管理者 `/root/manage_1527_sandcastle`，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`。真实 session_id 由运行时登记，本报告不宣称原生子代理注册。

## 固定来源与审查边界

下文简写都唯一指向固定对象，**不是当前工作树同名文件**：

| 简写 | 固定源 |
|---|---|
| P | `c1c156ed5851c36395fed35e0dc763f5d5b248e5:.chanlun/review-results/issue1527-legacy-job-proposal-20261018/PROPOSAL.md` |
| IF / T / AC / AM | 同一 c1c156 下该目录的 `interfaces.rs` / `toy.py` / `CHECKS.json` / `MANIFEST.json` |
| E | `f7d34f47cba8a0ea0ceb55499206520f058da61f:rust/src/bin/s_session_v2/evidence_store.rs` |
| C | 同一 f7d34 下 `rust/src/bin/s_session_v2/cache.rs` |
| A | 同一 f7d34 下 `rust/src/bin/s_session_v2/audit.rs` |
| R | 同一 f7d34 下 `rust/src/bin/s_structure_session.rs` |
| J | 同一 f7d34 下 `rust/src/s_content/job.rs` |
| AA / AP / V / AD | 同一 f7d34 下 `rust/src/bin/s_session_v3/admission_anchor.rs` / `admission_private/mod.rs` / `admission_private/fd_wal.rs` / `admission.rs` |
| AUTH | 同一 f7d34 下 `.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md` |

checkout 固定 `ce3dd877359e4ccec76b54f1d3d0f3585d85d78f`，分支为 `codex/1527-formal-i1-integration-20261018`。产品、原报告全部只读。另读取指定父输入 `../ROOT-I1-IMPLEMENTATION-SCOPE.md`、`../I1-RESPONSIBILITY-AND-REUSE.md`（实际文件名）及 `../PRIOR-R4-V3-DESIGN-TRANSCRIPT.md` §1–6；各 SHA256 记入 MANIFEST。没有消费其他 checkout 的活 WIP。

已读 AGENTS 和世代宪法。当前工具表无图查询工具；遵守禁索引限制，代码发现回退固定对象 `git show` 与文本搜索。没有网络、安装或历史运行。作者的 27 个模型项（其中一项含 160 组旧行输入）只复用并检查存档，不全重跑。新增三项纯模型仅覆盖本次发现的接口反例，均非 Rust/SQLite 实测。

## 已核实可保留的部分

1. **纯验证器搬运可保持原语义。** E:2223–2247 与 IF:16–40 逐字相同；`node_id`（E:856–862 / IF:8–14）及 hex helpers 也逐字相同。旧顺序确为 kind∈{0,1} → SHA256(kind 单字节 || 原 payload) → 声明长度转非负 u64 → 叶长度等于 payload 长度。kind=256 不会先截成 0；SEQ 声明长度不在 hash 内，行 proof 不能替代闭包/根核验。P:35、136–138 对这些限制的描述准确。
2. **SQL 类型层不能随纯 helper 搬走。** 普通路径 E:2134–2141 先读取 `(i64, Vec<u8>, i64)`；admitted/owned 路径 E:2214–2219 按 integer、blob 复制、integer、finalize、行验证执行。两路径的既有错误包装本来不同，应该分别与各自原路径对拍，不要求两路径错误串互相相同。禁止把 TEXT/REAL 转成合法输入，也不能把原 finalize 移到 hash 之后。IF 的切片长度转换新增失败仅在非现有 usize≤u64 平台域才可能出现；本次不声称跨平台类型验证。
3. **proof 的公开构造面合适。** IF:50–76 的字段私有，无公开裸 usage、数字费用构造、Default/Deserialize/Clone/Copy；唯一 `verify` 从真实 slice 核出长度。`declared_length` 只读，不给予来源资格。保留一个生产 verifier；旧 bin 的薄包装/re-export 及 E:583、627、1012 的 node_id 调用须仍指向同一 helper，不能复制第二套算法。IF 的示意 trait 不是开放给外部实现“已验证”的生产接口。
4. **键与累计的方向正确。** P:61–63 物理分离旧集合和 S-content 集合，旧域固定 kind+id，共享原 nodes/node_bytes/limits/stopped/t0；同摘要字节跨域仍分别接纳。GraphV2/3 可共享节点编码域，Format/根声明/写资格另核（E:158–192）。新鲜重读先验原行，不能以 accepted 命中跳 hash；重复接纳仍过停止门。保留 J:849–881 的加法及 bytes-cap/nodes-cap 顺序，原 S-content 时点不动。
5. **节点钩子选在 `node_children` 成功后合理。** 普通 E:2143–2145、owned E:1432 是确定位置；坏 SEQ 语法不接纳此节点，wire 仍计；其后子引用、循环、展开或根摘要失败，已成功接纳行不回滚。E:1507–1533、1579–1740、1982–2108 的上下文核验必须继续。部分读在 V:358–367 已逐成功返回量记账，后续错误不清前量。
6. **records/requests 的基本区分正确。** AUTH:303 明确六族 Snapshot / 七集合 Delta 的成员访问，不是外层 Delta SQL 行数；AUTH:304 不把 SQLite 查询或 Reader 构造算 request。本地私有审计无 HTTP/reader 子进程/对端捕获时 request=0 是有依据的结果。LegacyBlob 没有图节点时 nodes=0 可成立，但 wire 和实际逻辑访问仍须计。

以上都是静态结论，不等于来源绑定、父费、类型/借用/ABI或全路径接线已验证。

## 具名问题与必要修订

### H-01 — LegacyAllocationOwnerNotBound：新增集合的分配许可没有进入接口

**定位：P:55–63、130–132、145、151；IF:79–81。** 草案要求新 `accepted_legacy: BTreeSet` 分配前走同父许可，但消费口只有 `&mut Job` 与 row；J:560–576 没有 allocation owner，AA:735–737 也只持 Job 的 RefMut。新集合首次插入会分配，节点 byte cap 或 staged/store grant 都不是 RAM 许可。这是明确缺失的参数/持有关系，不能靠批准“同父”两字补成接口。

固定源能提供参照而不能自动转赠授权：`rust/src/bin/s_session_v3/audit_store.rs:32–36` 的 NativeTree 是 bin 类型，库 Job 不能反向依赖它；同文件 :241–251 的旧 `native_tree_room` 在 work=None 时直接通过。`rust/src/s_content/capture_allocation/scratch.rs:29–57、331–337` 的 NativeReservation/WorkScope 已有同父身份及释放机制，但没有把该 owner 从当前 provider 绑定到 Job 的路径。禁止照搬 None 路或新建父账。

**单一修订建议：** G2 明确一次性 `StartedAdmission::bind_legacy_owner(AdmissionLegacyOwner)` 语义（建议名，不是已存在 API），其 opaque owner 只能由 B1/#1488 的**同一已准父域**提供；Job 内的旧集合持该许可直到集合析构。新节点操作固定为：原门 → 查重 → checked 费用/数量和 cap 检查 → 同父集合增长许可 → 插入 → 提交计数。许可拒绝时不插 key、不更新 node 两计数；已发生 wire/record 保留。重复不申请增长许可，但仍检查期限。固定上界计算、许可容器自身增长、分配失败分类及析构顺序都须落到精确签名/文件；不要公开 `(digest, bytes)` 或调用方可填写的布局数来绕过封闭构造。

proof 是栈值时不虚构 heap 费；进入 Cert/Owned 缓存后增加的对象布局、集合项、来源标记、失败收据须随真实 owner 持有，不能 scope 已退而 retained Cert 仍存却退费。需要的是这条消费接缝的精确冻结接口与最小写集批准，不是先完成 I2 全库峰值验收。B1 仍独立 OPEN，不在本报告替它选 allocator、额度或 S7 映射。

### H-02 — LegacyVisitCoverageAndAbortChannelOpen：成员访问表与短路传播不闭合

**定位：P:73–75、104–114、147、151；IF:96–112。** P:112 已诚实声明尚未审完全部调用边；独立回查发现以下不能留给编码时猜测的具体缺口。

- `R:2296–2297` 调用 `tb02a::enrich_raw_until` 与 `business_json::validate_raw`；后者在 `rust/src/bin/s_session_v3/business_json.rs:747–771` 逐成员核坐标/身份/修订，前者在 `rust/src/bin/s_session_v2/tb02a.rs:291–294` 逐成员 enrich。这些是原有分开的遍历，不能只在 read_raw 外围加一次总数；两文件不在 P:144–148 的写集内。
- `R:4595–4606` 又进入 `tb02a::read_facts`（同文件 :598–628）：它创建另一 Reader、展开事实原件并按成员处理生命周期，再回根做 typed 对象投影。只改根 `read_objects_view` 和两处 Reader::load 不足以显式传递同一 scope，也没界定两次访问的事件边界。
- `A:293` 的 `resume_from -> bool` 和 `C:1610–1657` 的 `resume -> usize` 只能表达复用/不复用，不能表达预算终止。若把新增计量 Err 转 false/0，会沿 `R:1727、1750–1773` 进入全文后缀，形成拒绝后的继续遍历；禁止 `unwrap_or(false/0)`。应给这条链独立 Result 通道，`Ok(false/0)` 只保留原合法缓存 miss，`Err` 立即退出同一审计，不重启、不回不计量入口。新增模型 X1 展示该签名陷阱，不声称当前未实施产品已有此错误。
- `R:1118`、`A:294、305、344` 使用 slice/tuple/Value 相等；如果规范串及比较两侧也算访问，就必须在**原比较本身**暴露惰性成员事件，保持长度先判、左右顺序和首个差异短路。不能先遍历计数再执行原比较；更不能每次 `==` 收整数组 `.len()`。仅说“实际访问”不能决定规范化一次、后续对拍再用时的确切量。
- 失败的 SQL→Value 投影（例如 A:126–129）已经触及一个具名族成员，却可能根本没产出可借的 `OriginalMember`。IF:97–101 的必需成员引用无法表达该事件。需要由家族迭代器/SQL step 在取得该成员时签发一次性 visit，允许携带原局部 Result，不能把“记录访问”误作“合法 Value 证明”。未知行、控制元数据与未取得成员的 SQL 失败仍不能伪造一次成员访问。

**单一修订建议：** 先冻结下表各边的“族、取到成员的位置、局部检查结束点、错误返回型、计数次数”表，再用原 walker 的封闭观察策略接线；不计量入口共用同一业务算法，生产 metered 入口无可选账。建议私有操作形状为 `finish_visit(visit, original_result) -> Result<T, LegacyAuditFailure>`，原错误和 charge 错都留存；比较 helper、resume 及缓存保留统计都显式传播 Result。字串错误格式只在原边界按原顺序包一次。这个表必须涵盖下列实际边，不能把它们留作“一般 helper”占位。

| 固定边 | 必须作出的精确归属（不新添遍历） |
|---|---|
| R:2275–2298 → tb02a:291 → business_json:747；R:1712–1716 | raw SQL 解码、enrich、完整原始核验、发布前沿投影分别触及的成员；未发布 raw 是否纳入 G3 明示扩展 |
| R:4567–4592、4595–4606 → tb02a:600、602、628 | 旧对象/typed 事实、active/withdrawn 的成员投影与生命周期单元；不能把同一 SQL 外层行误当任意六族记录 |
| A:126–142、267–286 | 三独立族捕获、对象身份/出生/撤回索引访问；Arc 句柄 clone 本身不加访问 |
| A:294–350；C:1621–1656 | raw/三族/对象的真实 prefix 比较、左右短路、缓存 miss 与预算 Err 分离；DeltaRow 外层元数据不强算成员 |
| R:1155–1243 | 七数组形状检查不预收成员；CC056 withdrawals/replaces 两侧、raw revision 筛选的实际触及单元归属需明列 |
| A:415–514 | batch objects 投影、birth/death、Delta upsert/withdrawal、对独立索引端点的访问 |
| R:1085–1150；A:519–548、583–590、627–651 | 先 actual 全规范化/查重再 expected；缓存规范串后续比较如何计量、差异短路如何保留 |
| A:555–595、612–639；A:160–201 | batch 三族/raw、supersedes 原始成员遍历、首版 observation；Family::verify 的 Delta、batch、索引端点三面 |
| C:1660–1701 → A:363–395 | 原缓存保留成本的 canonical 化遍历：若算逻辑访问须落钩子及 Result；若只是缓存内部内存统计而排除，G3 明示，不能静默漏掉或新增同样遍历 |
| R:707–734、742–816、1749–1773；E:1744–1775、1780–1961 | certified header、已核展开/解析句柄与 prefix 命中：省掉的成员不虚收，实际发生的比较照裁定计；任何计量错误不得转 PendingFull/None 重跑 |

表中的 `tb02a`、`business_json` 指 f7d34 下上述完整路径。并非所有辅助字段/内存排序比较都应收费；要求根在 G3 把语义边界写死。推荐规范化/完整对拍作为各自的具名逻辑访问过程，内部字符串排序和字段比较不逐字段加费；规范串再次参与独立对拍须映射原成员，惰性比较不另扫一遍。此推荐须由根采纳才成为合同。

### H-03 — LegacyFailureReceiptNotEndToEnd：首错与未入账量缺少贯穿收尾的载体

**定位：P:75、79、121–128；IF:109–112。** P 正确指出 AP:96 的 `gate.and(result).and(ended)` 和 AP:172 的 `budget.and(result).and(reconciled)` 会让晚到 gate 覆盖先到原错。但“改为原 result 优先”也不够：V:138–146 把计量 Err 变为 bool，V:361–367 再变成 `SQLITE_IOERR_READ`；SQL 的业务 String 可能只是**先前预算错误的包装**。progress 同型在 V:714–720。先观察 wire cap，后观察 SQL 错误时，主错必须仍是前者，不能把它误称原业务首错。新增模型 X2 覆盖这一反方向。

此外，`charge_wire_bytes` 返回 Err 不表示本次字节未入账：J:740–769 在未停止时先加量再报超限/过期，而已停止入口 J:741–742 在加量前拒收。相同错误类别的两次返回可能一笔已入账、一笔未入账。只在 Err 分支把 actual 加到失败收据会重复计量；只拿最终 JobSnapshot 又会丢停止后的实际量。新增模型 X3 展示：cap=3，先返回4字节停账，再有3字节已发生的终端观测，正确总观测为4+3=7，不能报4或11。该模型不授权在停止后发起业务 I/O。

目前 IF 没有 `LegacyAuditFailure`、序号/阶段、未入账量或 `original_result` 参数。AP:113–116/AD:141–143 的边界是 String+快照；P:79 的 audit 返回发生在 rollback/close/registration finish 之前，若当场封口，后续清理观测无法回到同一收据。P:148 的写集也没有明确 AD 与 V 的终端传播责任。

**单一修订建议：** 由本次 StartedAdmission 同源持有一个内部 failure recorder，业务单元、计量门、VFS/progress、cleanup 都按观察序号记录首因与后续原因，最终在关闭 statement/connection/registration、结清或登记未结 grant 后取快照并封口。主错取最早实际原因；保留原业务 String 为独立字段，禁止靠字符串解析倒推分类。VFS 的派生 SQL wrapper 可保留为副因，不抢原预算首因。门面可用 `finish_visit(visit, original_result)` 和内部 `observe_io_return(actual, phase)`，后者用原 Job 调用前后的累计差确定未入账量；停止状态及原 t0 不重置，观测量只作同收据事实，不作为另一可消费余额。

收据至少固定：原 Job 关联、阶段/最后完成单元、首因类别及原文、独立业务错、计量首错、清理次错、最终 JobSnapshot、实际未入账 wire/record/request、是否仍有未结预留。业务 Err 即终止 scope，即使 Job.stopped=None 也不能再遍历；后到 gate 仍执行且留痕。已停止时新业务 I/O 的 precheck 必须拒绝；只记录真正已经返回或授权终端报告产生的量，不能为“保量”放松 gate。终端输出若发生在封口之后，须明确同收据的追加观测所有者，不能再起报告 Job。

这项窄修订必须明确 AP、AA、AD 及必要 V 回调的载体接线，不要求本轮证明同步硬 I/O 可打断或真实 EINTR。那些仍归 I4/I5。

### M-01 — LegacyCaptureProofLifecycleUnspecified：同捕获凭据与 owned 分支尚无可执行持有关系

**定位：P:49、67–71、146；IF:89–101。** 文本要求验证 conn/Format/捕获代/FD/同 Job，但 IF 只给 `&PrivateTransaction` 空类型和彼此无生成关系的 source/row 引用；Rust 生命周期本身不能证明两个对象来自同一实例。E:1322–1365 的普通/owned Reader、C:847–857 的“复用任意已有外层捕获”，目前都没有该 metered 身份。

更具体地，E:1583–1586、1599–1601 的派生缓存命中不经过 load；E:2032–2052 的完成区间复用也跳过 load。只有“同一次 metered 捕获的每个派生项均在本 Job 接纳成功后发布”这一不变量成立，才可不另扫闭包仍证明已经计过。不能把一个先在不计量路径建立的同捕获缓存借入新 Job，然后仅在 load 钩子等它出现。C:72–75 的同原件比较不能单独证明它已在该 Job 计过。

owned 另有持有细节：E:1245–1258 的 `OwnedNode::try_clone` 复制 payload/children，E:1427–1445 要返回该 clone；把不可 Clone 的 proof 直接塞进每份 OwnedNode 后就无法按这份草图复用。不要因此增加裸长度构造或重复 hash。推荐在 owned 原缓存条目保留单份 proof，命中时先借此 proof 完成 observed/同 Job 提交，再执行原 node 克隆；返回值不必再拥有 proof。普通 Cert 可共享其单份 proof；跨捕获 retained Cert 必须先执行本次 fresh 行校验、node_children 及 same_originals，重新签发短命 observed，不能携带上次来源资格。

**单一修订建议：** scope 私有工厂一次绑定固定事务/FD见证/Format、当前捕获身份与同一 AdmissionAccounting；给所有 Reader 构造和缓存 memo 发布点列出该身份的传递及失效规则。新 scope 不接受无该绑定的既存捕获；同 scope 的 Reader 可以共享。新鲜行、普通命中、owned 命中、跨捕获 cert 命中/不匹配、派生/完成区间命中分别给一条不新增遍历的序列。失败后既不发布未计量 memo，也不继续使用失败 scope。此处待补的是所有权与身份设计，不声称发生了已编译的借用错误。

## G1–G4 的单一建议与根待裁边界

**推荐只保留这条路线：原 verifier 单一搬运 + 旧域 opaque proof + 原 Job 独立旧键集合 + 原 walker 原地事件 + 同源失败收据。** 不转换 S-content，不新增可选 byte policy，不重复生产旧验证算法，不以计量错误或业务异常触发不计量/较宽路径。

| 根项 | 本审查给出的唯一建议 | 仍需根明确的范围 |
|---|---|---|
| G1 | 旧图单位固定 `1 + payload.len()`，GraphV2/3 同节点编码域；与原 S-content 帧字节共享同一个 node_bytes cap，数量也共享。LegacyBlob 不凭空制造 node。 | AUTH:301 的公式确实只给原分帧域，**新语义需明确采纳**。优先复用已冻结 exact 八字段、原字节及原数值，明确它们约束混合域；没有证据说明必须加 schema 或改数值。若根选择改 schema/额度，再另批兼容/迁移；本报告不自动引入新批准门或要求重签旧锚。 |
| G2 | 保留单一纯 helper、opaque 行证明和同父 owner 的一次性绑定；按 H-01/M-01 完整定义库/bin、分配前许可及析构。 | 搬运写集及新增 owner 消费接缝需精确冻结；不读取 #1488 活 WIP，不占其专属文件，不把 B1 未决转成默认额度。 |
| G3 | 六/七族按具名原过程的实际成员访问；重复访问累计，无第二次计数遍历，metadata/内部字段/仅 Arc clone 不收费；复用合法省略的过程不虚收。 | 采纳 H-02 表的逐边判据：未发布 raw、typed facts、规范串再比较、prefix 双侧及 cache 成本遍历的归属；批准必要窄文件扩展和 Result 错误通道。 |
| G4 | 非约束预算在同一原件、同一缓存状态下，原成功结果与原业务错误串/顺序保持；受限路径在首个实际失败事件停止，保存进度和所有已发生量。 | 采纳 H-03 的先因/派生 SQL 错/晚到 gate 三者优先级和最终收据载体。不能承诺未知后缀原错永远优先；不能用先全图无预算验证再结账解决。 |

G1 是语义裁定请求，**不计入 CHM 的“实现 bug”数量**。草案既未改产品也明写未批准，本报告不把其中候选比较的模型 policy 误报为生产动态宽严。真正实施只能固定选定单位；不得保留运行时切换或异常后回落。

完成四项具名设计修订和根对上述窄扩展的采纳后，可以进入 I1 的定向编码，不必等待 I2–I5 全验收。现阶段只可保留已核实设计组件，不能删除 `LegacyAdmissionBudgetAdapterMissing` 并宣告 I1 解决。

## 验证证据与保留义务

CHECKS.json 区分本次执行、作者证据复用、静态推断和未运行。MANIFEST.json 记录固定源 Git blob/SHA256、父输入及本目录产物哈希，不自我哈希。新增 `counterexamples.py` 不导入/执行产品；X1 为预算 Err 误作 cache miss，X2 为先预算拒绝后 SQL 包装，X3 为同 Err 的已入账/未入账 I/O 区分。它们只证实接口约束，未证明正式 provider 或 Rust 实现。

没有运行 cargo、rustc、build、SQLite、正式 S/Q、旧 binary、历史重型、网络/安装/索引；没有子工蜂、认领、评论、commit、push、merge、切分支或 Git 配置变更；没有读取 .env、认证、钥匙串或密钥。未声称 Rust 类型、借用、链接、ABI、真实 I/O、EINTR 或恢复测试通过。

**B1、I1、I2–I5、完整 9011、64 GiB/store、AgentStorage 148 GiB 卷与独立双跑前置、金融/隔离/真实恢复/全部六目标均保持 OPEN；#1405/#1420 生产 v4 仍 BLOCK_NOT_MERGEABLE。49 GB 仅为含 WAL 的布局估算。** 本结论不代签其他独审，不批准实施、运行、合入或关票。
