# #1419 M7D：有界原件持有技术设计包

绑定 #1419 的阶段工作草稿，基线 `sandcastle/issue-1419` / `2c6cb4e335bf6ce8888499b6de870b6d27f0c6a9`。本阶段只交设计，不改 Rust、CI 或合同。按本轮主控裁决，方案 B 是恢复 #1481 已批准的 Δ 合同，不是新合同。推荐下文方案 ii；方案 i 仅作技术比较，不作为合同完成态。

首次资源拒绝的诊断收据尚未返回。M7C 的 g22/g25 是最后可见读数，既不是失败代定位，也不是提交成功收据。本设计对局部 J、父 remaining、后续保留门分别成立，不预判其中任何一门。M4 的 13377 次重读保持独立，不在此设计中增加全局 memo 或审计去重。

## 1. 原件与合同差异

本轮实际执行 `env -u GH_TOKEN -u GITHUB_TOKEN gh issue view 1481 --comments` 及同命令的 `1644`，均退出 0；另以 JSON 读回正文与全部返回评论核对。临时原件在 `/tmp/issue1419-m7d-{1481,1644}-original.json`，不入仓。下表票面行号按本轮 `body.splitlines()`，不是网页排版行号。Rust 路径在本文件统一相对 `rust/src/`；其他路径相对仓根。引号内为原文片段，判断列为本设计的解释。

| 原件 | 逐字摘引 | 对本设计的约束 |
|---|---|---|
| [#1481](https://github.com/xy7365527-lang/NewChanlun/issues/1481) 票面 :15 | “写前不再重读全部旧原件；夹持失效即 stale 停写，不降级、不回落。” | 在线旧前缀靠生涯夹持，不靠常驻全史正文。 |
| 同票 :24 | “新代只核本代新增节点与变化对象，未变子树按「同位置同摘要」继承上一代结论，异位置同摘要按新上下文重核；不常驻全部原件；写者保留结构有具名冻结帽，超帽即启动失败、不降级。” | 保留完整历史；换存储形状不能换判据或帽。 |
| 同票 :25 | “接受时完整 `ImmutableHistoricalAudit` 规范序列化逐位相同” | 不以 fingerprint 或计数代替语义对拍。 |
| 同票 :16/:26 | “拒绝集合与类别须相同，首报位置不要求相同”；“代码中不存在「Δ 失败或资源不足 → 旧审计重判」分支” | 首报位置非跨实现合同，但每次失败的真实来源必须保真；生产不重判。 |
| [#1644 批准评论 6308923666](https://github.com/xy7365527-lang/NewChanlun/issues/1644#issuecomment-6308923666) | “② 每次重启从原件重核全部历史”；“④ 接受静默磁盘损坏延后至重启/离线核对发现，后台复检可选默认关。” | 不能跨生涯信任旧证书，不能扩大已批准的损坏延迟范围。 |
| `.chanlun/review-results/issue1644-p6-audit-ledger-20261021/AMENDMENT.md:9/:39` | “完整审计仅留测试与离线核对、不进准入”；“与内容库及 WAL 合计，不另立帽”；“每请求只读写 Δ 行” | 旁挂台账在盘，计 `max_store_bytes` 同账；不是免计费的内存第二表。 |
| 同文件 :25/:29/:60 | “(节点摘要, 上下文 Flavor{Value, JsonInner}, 核验模式 Mode{Public, Template})”；“序列节点（字节块、元素序列、字段序列）不是认证单位”；“摘要相同但上下文或模式不同的不止步，按新单位重核” | 不能把序列摘要或裸 digest 当通用成功资格，完整解释上下文继续参与核验。 |
| 同文件 :35/:37/:45 | “台账格式换版本即整份丢弃重建”；“内容库 Commit 成功后再提交该代台账事务”；“从 g=1 起按代调用与在线**同一个** Δ 步函数” | 冷启动与 recover 重建同一台账；旧台账只诊断，完成前不接写。 |
| 同文件 :119/:121 | “同 ID 换内容、缺节点、历史关联变化不受限，照旧写前拒绝”；“写前不再重读全部旧原件” | 未读旧子树的介质损坏按已批边界处理，不能把缺节点等一并延迟。 |
| `docs/adr/0042-s-writer-incremental-production-path.md:40/:42` | “监督器 recover → 增量路径持久态重建 → 继续增量”；“运行中出错时执行 `discard_runtime_state` 并 stale 停写” | 不自动重试，不用完整生成恢复。 |
| 同 ADR :44/:58 | “只放弃保留，本请求的严格审计与回复照常成功”；“回退＝换回上一版本二进制和对应版本的资源档” | 前句特指构造候选保留，不是 H1 原件准入失败的放行授权。回滚靠版本，不设运行时双轨。 |

当前实现与合同的差异及费用生命周期如下。源码注释是实现证据，不覆盖上述合同。

| H1 当前实现与原文 | 当前行为 / 合同差异 |
|---|---|
| `s_content.rs:2286-2295/:2340-2347`，`NativeNode` 自持 `CodecVec<u8>`、proof、dirty 链；`NativeCaptureNodes` 是单个 HashMap 和 `index_permit` | capture 跨代累计唯一 digest 正文。与“不常驻全部原件”有差距，未发现逐代复制全前态的证据。 |
| `s_content.rs:3593-3615`，`finish_owner_step` 注释“完整原件与根证书继续保留” | 仅清 public/template memo、dirty proof、重置 reads，不回收节点；清 proof 不等于释放原件。 |
| `bin/s_session_v3/audit_owner/effects.rs:1462-1493`，从本事务 `j.nodes` 真回读并 append，参数为 `j.cap` | 将本代新增原件移入累计 capture。`bin/s_session_v3/audit_owner.rs:3911-3913` 的尾部补入参数为 usize::MAX，仍同父收费；两入口不能混作同一 J 门。 |
| `s_content.rs:3540-3553`，先核实际 frame，再核“同摘要异原件” | 正文相等义务要保留。重复读取不产生重复 digest entry，13377 次重读不是新增 13377 个持有节点。 |
| `s_content.rs:3558-3573`，先 `reserve_native(growth)`，再比较 cap / try_reserve，成功才替换 index_permit | 单体翻倍时旧新索引许可共存。父账可能先于局部 J 拒绝。 |
| `s_content.rs:2314-2336`，`SourceClaim` 为 `body.len()+97`，“不在父账物理reserve；父账物理只由正文块自身的Native许可计一次” | E/O 来源角色账与父账物理许可分开；不得因回读又对同一正文 allocation 物理双计，也不得只删账不析构。 |
| `s_content.rs:3522-3524`，“零增长也核本次活动域与原许可同父”；`s_content/capture_allocation/scratch.rs:205-218/:105-114` | 分配前由活动 WorkScope 向原父账/origin reserve，正文许可随 backing 移交；载荷实际析构后释放同父许可。失败必须保留类型化 origin。 |
| `bin/s_session_v3/audit_owner.rs:3298-3314/:4110-4159` | CompleteCandidate 还持 audited、Q、ExactRows、资格许可；resources 最后析构。finish 后按累计原件重算来源/容量费，末尾另核总保留门。只减 capture 不保证整账通过。 |
| `bin/s_session_v3/audit_owner/native.rs:1565-1609/:1643-1659` | reserve 同核 remaining 与 total live；释放旧 origin 仅减 total live，不返当前域 remaining。只能在合法边界回收后按原公式冻结新域，不能把旧释放额记作当前新额度。 |

当前两锁 R=536870912 B，冻结 A=R−O−U；`bin/s_session_v3/audit_owner.rs:4372-4375/:4409` 令 P=min(A,parent.remaining())、J=floor(P/8)，故 J≤67108864 B。O/U、失败请求 A/P/J 均未实测，理论上界不是实值；两方案沿用这些公式与原父账，不增加一个并行预算池。

## 2. 方案 i：受管分段索引

数据形状为私有 B+ 树：叶段至多 S=4096 个完整 digest→NativeNode 槽，按完整 digest 排序；目录页至多 64 个子指针和分隔键，页本身也持许可，不设会全量翻倍的全局 Vec。段大小是布局常量，不新增政策帽。除根外分裂后占用至少半段，目录同理。lookup 沿目录找叶段，再二分完整 digest；约 O(log64(N/S)+log2 S)，同 digest 继续逐字比较 tag/body。无弱哈希、无截短摘要。叶槽移动时正文 backing 与许可按值移动；不得跨变更借用槽地址。

满段只分裂该段并沿目录路径分裂，整次所需新页先在同一个工作域合并算出需求，按既有局部 cap 和父账 reserve，不能拆成许多小申请绕过 J。完成链接后才销毁旧页、归还旧页许可；失败先销毁私有新页，原树不改且本次错误不清除。不实施全表压实。每页实分配 Layout/容量均入账。正文、SourceClaim、owner codec 成功树继续按当前语义持有，不移动到旁挂库。

冷启动/recover 从 g1 用同一 Δ 步重建段树与证书，原件全史仍逐代读完；旧内存地址、段号或磁盘台账不能签资格。缺节点、篡改、同 digest 异正文继续走现有完整核和类型化错误。段内顺序不改变业务遍历顺序、认证顺序、effects 有序事件；段分裂的物理预算事件会变。纯 codec 与 qualified/certified 的有效域不变。

记 N_g 为累计唯一原件数，B_g 为累计正文实际容量，W 为含控制余量的槽宽，D_g 为目录页许可。保守单段许可模型 Q=(2S+16)W，实际实施须以 Layout 校核这个上界，不能照估值收费。常驻约为 `B_g + [ceil(N_g/S), 2ceil(N_g/S)]×Q + D_g + 证书/来源角色账`，仍是 Θ(全史原件字节)。一次叶分裂新增申请至多 `2Q+目录分裂路径`，此时旧页许可仍在；每代峰值为旧存活量加本代正文、分裂页及既有 scratch/effects 等共存量，不能把若干历史 peak 相加。

结论：消除整块翻倍尖峰，仍常驻全部原件，**不满足“不常驻全部原件”**。只可作为后续方案的内部数据布局；不建议单独实施后宣称 B 已完成。

## 3. 方案 ii：持久原件 + 有界前沿

### 接口、数据与前沿

一个私有原件读取模块集中管快照、读回、核摘要与许可。调用者在同一 Δ 步持 `StepReader`，以完整 ContentRef/上下文调用 `read_checked`，取得不能越过本步的 `ReadLease`；只在原有同位置/同摘要且完整上下文相同的条件下查询成功结论。调用者不能传“已认证=true”，也不能在 miss 后换审计核。cold fold 与在线调用相同接口和单位核，差别仅是已建立的生涯认证前缀。

- 持久正文仍只放现有内容库节点表；临时未提交节点仍由当前事务/有序 journal 管理。Commit 前任何尚未持久化的正文都不淘汰。磁盘全历史不清理、不截断、不减代。
- 内存只有本代新节点及变化对象所需材料 Δ_g、当前读租约、遍历前沿、控制原件和身份封印。旧正文不建立全史内存 digest 表；按内容库已有索引点查。node/cert 台账在原旁挂文件，沿既有格式版本规则扩字段，不新增内容库 schema。
- 前沿不是“最新 K 代”，也不是当前四根的整个可达闭包。它是当前正在核验的路径 continuation、序列折叠的聚合状态、必须共存的字节比较帧，以及下一增量操作的必要路径描述符。已完成子树正文立即随租约析构，continuation 留 digest/游标/聚合值，旧枝按需再读。非活跃历史对象只保身份/根引用，不因被引用而 pin 整个闭包。
- 令 h_g 为当前编码树最大导航深度，k 为由调用图证明的并发 reader/比较路径数，w_g 为生产 bounded-layout/2 的当前工作窗材料。前沿节点数界为 `f_g ≤ k(h_g+1)+w_g`，字节界须另加每帧实际 body capacity 与聚合状态；不得把单节点正文假定为常量。逐调用点给出 k、窗与深度证据是实施片 1 的验收门，未证明前不签“有界”。4096 仅在下节作敏感性估算，不是前沿帽。
- 变化对象本身可能很大，Δ_g 以完整本代检查所需字节计量，不只数 journal 新 digest。`NativeSourceState` 仍有全量对象、raw 与索引元数据，`AuditedState`/Q/ExactRows 亦在；本方案承诺**原件与其读取工作集** O(Δ_g+前沿)，不声称整个 owner 内存与历史无关。发现其他字段持正文须迁移为可回读描述符；纯领域历史元数据单独记账，不能藏进“前沿”。

### lookup、错误、恢复和引用存活

lookup 顺序固定为：核当前生涯/父账/快照有效 → 本步 pending 或活租约中的真实原件 → 同一事务持久节点点查 → 实际 frame(tag,body) 核摘要 → 同一个上下文单位核。已有认证单位按 AMENDMENT:60 止步，不为了填缓存展开旧闭包；异位置/Flavor/Mode 仍重核，序列携 UTF-8/数字进态及 kind/extent/format 等原条件，不能凭序列摘要跳过其所属值核。按需读取是唯一 reader 的存储分派，不是判据失败后的重试。

`SqliteAuditBase` 当前是借 Connection 的固定域及 plain 材料/成功表（`s_content/sqlite_audit.rs:128-169`），不是可直接挪入 Native 的受管 owner。须新增 WorkScope-aware 的事务读回 adapter，接现有 `bin/s_session_v2/content_store.rs:893-919` 的 scoped 读与核帧机制；保留 `bin/s_session_v3/history.rs:3876-3898` 的 ordinary memo 排除门，不能借此开放 M4 memo。读前预留 SQL 行复制、正文、CheckedFrame、路径、页缓存可归属部分的许可，按已有资源档核存储/渲染限制；不是出 SQL 后才补账。

缺失返回原缺节点类别，摘要不符返回原 NODE_DIGEST，已有同 digest 的插入仍须对数据库实际 tag/body 逐字比较，不能用“实算 digest 相同”代替正文相等。未来或无关 stage 不得修补本 stage 缺原件。读、parse、核验失败立即撤销当次可复用成功并传播第一个 typed origin；不得清错误后从另一个来源重读。外部改库仍先 stale，未被夹持观测到的介质损坏只享有原已批延迟边界，不能新增缺节点/异正文免责。

单个请求的既有逻辑检查顺序和已锁定的先错关系保持，例如 manifest 核帧先于解码、夹持/身份检查先于暖复用。跨 Δ 与完整 oracle 只要求接受/类别相同，按已批合同不要求首报位置相同；新 I/O 的资源拒绝位置须如实记录，不伪装为原门。新租约各自析构、归还同父许可，不能覆盖先前成功节点或重置父失败记录。

| 当前真实持有者 / 引用 | ii 的迁移及存活义务 |
|---|---|
| `s_content.rs:1999-2003` Borrowed body、`:2362-2364` CheckedFrame proof、`:3023-3084` NativeCountsSubsetReader 借 capture 并取长度/来源费 | 改为有生命周期的受管 ReadLease；不得把对可淘汰槽的 `&[u8]` 留入证书。长度/角色费由实际本步原件和已认证记录取得，不靠失效地址。退出帧再释放 backing。 |
| `s_content.rs:2523-2529/:2731-2741` 根 HashSet 与 seen/bytes/elems/fields/key_cmp 成功树 | 这些是 digest/聚合描述符，并非 NativeNode 裸指针；但当前可信性依赖同一不可变 Native owner（`:3940-3998`）。改为同写者生涯、同封闭持久域、完整上下文与判据版本的能力引用；活跃部分保内存，其余按既有旁挂格式存可重建结论，不能每代复制全部证书。失败 revoke、模式隔离、当前父健康门不减。 |
| `bin/s_session_v3/audit_store.rs:1216-1241` RefValue/RefOwner；`bin/s_session_v3/source_audit.rs:916-927` SourceFields/payload；`bin/s_session_v3/source_contributions.rs:408-410/:448-556` NativeRc<Object>/seed/各索引 | ContentRef/digest 本来可作描述符；SourceFields/payload 的实际缓存正文须逐类转为带内容库身份与解释上下文的延迟读取。NativeRc 继续保实体身份，不借它 pin 整份旧 capture。禁止只改 Handle 名字而保留全正文。 |
| `bin/s_session_v3/audit_owner/effects.rs:1254-1315` QualifiedSourceCut；`bin/s_session_v3/state.rs:1482-1506` HistoricalCertificate/QualifiedSource | 资格绑定 seal、epoch、root、raw、规则、内容库/二进制身份和完整单位/根；内容淘汰不撤资格、不重签资格、不延长到新生涯。完整 canonical 通过描述符和本步 reader 实化；H1 仍不持 Ledger 引用。资格转换许可照旧提交前 reserve、交接后消费。 |

冷启动与 recover 丢掉旧内存能力及旧台账准入权，从 g1 逐代认证折叠、每步放掉非前沿正文，再重建构造运行态。它全历史读完，却不将全史原件重新攒回 capture；重建前沿与运行中在同代边界的语义内容相同，不要求地址/页号相同。内容库 Commit → 台账 Commit → 可交接资格/前沿的时序保持；任一步失败 stale，等监督器 recover。台账失败不自动读回旧完整状态继续写。

### 费用、F 与 deadline

原件常驻 `M_g = B(Δ_g∪前沿_g)+I(Δ_g∪前沿_g)+活租约/证明/能力描述符`，即 O(本代材料+前沿字节)。每代峰值取实际同时存活最大值：`max_t[其他owner存活_g(t)+M_g(t)+当次SQL/codec/scratch/journal/资格交接(t)]`；不能从 M7C total_peak 减一个估算节点数得出新 peak。磁盘正文与台账继续随历史增长，全部计库/WAL/台账同一存储帽。

每块正文仍只有一个父账物理许可；SQL 临时副本是另一真实 allocation，要另收。SourceClaim 只在角色实际持有该正文期间有效，淘汰须同时证明无租约再析构并终止相应角色账。持久证书描述符仍按自身容量收费，不能把 `.node_count×97` 改小而留下原角色或漏账；若既有角色合同被核实要求全历史永久同额收费，须走 §6 的 P 门，不在本票自减。

跨代回收放在已交接成功且所有旧借用结束的合法请求边界，先销毁 backing，再释放许可，随后按原 O/U/A/P/J 公式冻结新请求。旧 origin 释放不返当前 remaining，故回收不许绕过冻结/父账健康门。首次回收、长闭包、冷折叠和资格交接都必须量出共存峰；J、R、冷启动窗与所有资源帽原值不变。

新增 SQL miss 会增加 SQL 读取及核帧 F；少持 proof 也可能重做 F，命中本步租约则不需新 SQL。AMENDMENT:60 的旧单位止步仍保留，因此不承诺、也不允许每代扫全史换低 RSS。记录逻辑查找数、SQL 实读、核帧次数/字节和拒绝事件，不能把旧 reads 换口径后沿用同名数。每次新增回读在 reserve/SQL 前及核帧后采样现有 deadline/取消，并沿长循环原采样点检查；全程使用原请求同一个绝对 deadline，不重启计时，不得批量读完再采样。物理预算事件/F/采样次数可变，失败 typed 来源与已锁定逻辑门序不变。任何要求删采样、改 F 定义或放宽 deadline 的实现转 P，不在本票处理。

## 4. 代入估算与三种首拒门

以下全部是**估算**，不是实测分配或成功收据。M7C 表见同目录 `M2-NOTES.md:1895-1912/:1969-1976`；W=225 B 仅取历史布局收据 `.chanlun/review-results/1488-acceptance/receipts/20261021-040601-cdd1f8690f.md:15`，非当前 CI 的 size_of。i 取 S=4096、Q=1,846,800 B；紧密页数为 ceil(N/S)，分裂实际页数至多其两倍，目录另计。ii 仅作“若前沿为 4096 槽”的敏感性情景，以 `225×(Δnodes+4096)` 算槽负担，不将该值作为新帽或已证明前沿。Δnodes 是相邻日志计数差，不等于完整 Δ_g 字节。

| 场景 | 已见 N / Δnodes | 已见 native_live / total_peak B | i 紧密页数 / 索引许可估算 B | ii 槽负担情景估算 B |
|---|---:|---:|---:|---:|
| C/D g22 | 109677 / 5862 | 112674716 / 195024666 | 27 / 49863600 | 2240550 |
| E/F g25 | 112896 / 9086 | 117281429 / 200371967 | 28 / 51710400 | 2965950 |
| H1 admission g1 | 2943 / 2943 | 4433281 / 7922105 | 1 / 1846800 | 1583775 |
| H1 admission g2 | 5043 / 2100 | 7463124 / 12637523 | 2 / 3693600 | 1394100 |
| H1 admission g3 | 7834 / 2791 | 12598058 / 16863110 | 2 / 3693600 | 1549575 |
| H1 admission g4 | 10878 / 3044 | 14754843 / 24292875 | 3 / 5540400 | 1606500 |
| H1 admission g5 | 13099 / 2221 | 15193193 / 26653484 | 4 / 7387200 | 1421325 |
| H1 admission g6 | 15809 / 2710 | 22389762 / 35878494 | 4 / 7387200 | 1531350 |

这六代是当前集成构建的 admission 冷/recover 锁，不是纯 H1 分支性能实测，也不是只有三代的 qualified 锁。i 在 C/D/E/F 分裂稀疏时索引许可估算可达 99727200/103420800 B，未必降低常驻。历史容量模型下一档整块申请 103222800 B，i 一次两叶分裂约 3693600 B 加目录路径，但仍有旧页共存，且 J 的实值未知。ii 的正文、证明、目录及其他 owner 费用未列入末列；没有 body 分布与实测前沿，不能给 RSS/总峰或通过代数承诺。若仅比较来源角色项，当前累计 97N 在 C/D/E/F 为 10638669/10950912 B，ii 仍须逐角色按真实存活收费，不能把这两数直接当节省额。

| 首次拒绝诊断结果 | i 的适用性 | ii 的适用性 / 必查 |
|---|---|---|
| 局部 J | 若确为累计索引满档，可消除整表单次申请；须以整个分裂批次核 J，不能拆账逃帽。正文与后续总保留仍增长。 | 缩小原件索引及每步新增工作集，符合合同；本代材料/前沿本身过 J 仍照旧拒绝。须证新路径未转去 usize::MAX 入口。 |
| 父 remaining / 总帽 | 仅降低旧新整表共存尖峰；若首拒来自正文、SQL、证明、资格，不能据此签通过。 | 降低跨代原件 live，并在原冻结公式下减少旧池负担；不把旧 origin release 当本域返款。仍核 remaining、L0+A 及所有其他分项。 |
| append 后保留门 | 仍 Θ(全史正文)，无有界保证；即便短期过门仍不完成合同。 | 回收原件可降低来源/容量项，但历史状态、ExactRows、Q、资格/台账等仍可能拒绝。按 `bin/s_session_v3/audit_owner.rs:4075/:4083/:4100/:4153` 分项举证，不能只报 nodes。 |

主控用返回收据确认首拒阶段、入口、requested、scope live/limit、remaining、parent live/L0+A 及索引 len/capacity/W/G/J，决定首个实施落点；无论哪一门，恢复 ii 的合同理由都成立，不等于当前两锁已被证明可通过。

## 5. 原验收全保留

下表是实施时必须保留的验收面，不是本阶段运行收据。原测试名、场景、代数、冷点与全部语义断言保留，CI 必须实际执行。局部验证先做靶向锁；S86 全前缀/9011 双跑仍在原具名重型窗口，不能减代、推迟验收或拿六代代替。

| 锁 / 位置 | 两方案都须逐位或逐类保持的内容 | 可变的物理观测 |
|---|---|---|
| M3 四条 `w2_h1_cold_*`：`bin/s_session_v3/audit_owner/w2_h1_structure_tests.rs:379/:521` C/D、E/F；`bin/s_session_v3/audit_owner/w2_h1_window_tests.rs:96/:207` W 未定、bounded2 | 原完整生产 W2 串；cursor 条件义务；seed/center/pair/revision、窗口、raw 绑定、所有冷点续写完整 canonical；逐 cut 实际 H1 资格；full audit 调用零增长。 | i 的目录/分裂费与 peak；ii 另有回读/F/SQL/材料驻留数。N 的历史逻辑总数必须保持，不能把驻留数冒充原累计 nodes。 |
| H1 admission：`bin/s_session_v3/audit_owner/admission_tests.rs:157/:271/:322/:347/:461/:601-737/:741/:817` | 冷负例接受/类别、own commit、foreign stale、正式入口 never_full_audit；原六代，冷3/冷5、真实 recover、epoch2 pending g6、最终冷6；台账与完整历史 bitwise；写前失败、无候选、资源释放。 | 新物理索引/读回费用可测；原 canonical、代数、台账折叠与控制门序不变。 |
| certified `bin/s_session_v3/audit_owner/history_clamp_tests.rs:399-544`；qualified `:829-1234` | certified 原六代六时钟，snapshot/delta/runtime_root_tables 与 ordinary 对拍；qualified 三代资格1/2/3、规则/内容库/二进制身份/roots/units，旧资格释放、提交失败 stale；不得凭淘汰降为无资格组件。 | 候选真实保留容量可下降；资格含义和存活时点不得变。 |
| effects `bin/s_session_v3/audit_owner/effects.rs:2438/:2748/:2798/:2923/:3064/:3099/:3122/:3175/:3208` | before-SQL/hooks、重复noop和不可达节点、有序真实事件/stage、缺失不能被未来修补、receipt一用/绑定、同J同父、失败不抹旧成功、取消原传。 | 分配/reserve事件可变；业务有序 seal/计数必须不变，不漏 effects。 |
| codec/证书 `s_content.rs:8628/:8693/:8727/:8764/:8781/:8843/:8869/:8899/:8926/:9044/:9085`；原件锁 `:9237/:9271/:9324/:9447` | 完整成功才晋升、部分失败revoke、同digest异正文、模式/角色/版本/头部、暖命中也核父健康、同父共存、重复释放仅新许可、坏正文不插入。AVL 独立原地址/复杂度锁继续保留。 | i 保留证书树，原证书局部事件不应漂；ii 可变证书载入/租约物理事件，但不能降低任何拒绝义务。 |
| 预算锁 `bin/s_session_v3/history.rs:10085/:10324/:7383`，后者为 `publication_native_cut_scope_real_budget_preserves_origin`；`bin/s_session_v3/audit_owner/history_clamp_tests.rs:596/:622/:654/:683/:1234` | 分配前持票、同父/当前origin、精确峰值门及少1B拒绝、无写/无slot、失败来源/桥保真、真实析构归还、控制rebind先于暖观察。 | 布局变化影响的精确字节峰与原资源事件摘要只能按新实测重冻，少1B边界本身必须保持；不改J/R/任何帽。无关局部预算测试期望不动。 |
| `bin/s_session_v3/source_audit.rs:5529-5624` `full_some_audit_keeps_frozen_read_and_budget_trace` | counts=(91,13,11,14,0,0)，reads=227347、F=248860，read_hash=`13bd2c89eae9fa7590d79dcf172f3ee2419ced7b826191ecc73a888f6c3052c2`，budget_hash=`20e6e506fdbbe095e5838c68df99add568377e9881a26460ae97540da00da37d` 全部逐位不变。 | 两方案限 owner 持有；该 ordinary MemoryNodes 冻结轨迹不在改动面，不能借 Native 回读变化重冻它。M4 13377 保持独立。 |
| `bin/s_session_v3/audit_owner.rs:4511-4520` `cache_flush`；`bin/s_session_v3/audit_owner/history_clamp_tests.rs:44-67` small_cache smoke | history_strict 条件下，flush 继续在 effects finish 前；事务/夹持/checkpoint门序、M6 Box 拆帧不动。 | 不把 CACHE_SPILL 增长当压力spill确认，不声称smoke证明flush回归；新I/O不得悄悄迁移flush。 |
| #1481 :25/:28/:29；AMENDMENT:134-142；ADR0042:89-103 | 三层对拍、丢台账重建、重建+k步≡不中断、独立进程双跑、schema不变、原重型规模/性能/恢复门与回退正控。 | 冷启动耗时/RSS、Native SQL/F、分项peak、物理预算轨迹需重测；完整语义、拒绝类别、内容库表/回执/公开根不变。 |
| #1481 :27 单条读数硬门 | 普通非test二进制在 g∈{86,1000,4000,9011} 或原M2选点，审计耗时/读量对Δ_R成正比，三个大g点系数相对变化≤20%，残差对g斜率95% CI含0，SQLite规模项≤G^0.2；每测量代 `Δ_R(g) ≤ Σ_f(u+e+⌈log₂count_f(g)⌉+1)+C₀_base+C_bi·bi_births(g)+C₁·⌈log₂N(g)⌉`，超界代数0；五常数沿用#1491在S86前缀的实测冻结值并入manifest；30s调用门不放宽。原前置#1488段A/#1491段B保持。 | 读数文件继续记五常数、各选点F(g)、Σcount、Δ_R、上界值和超界代数。ii 新增SQL/F仍受这些门约束，增长超门即未通过，不能以“物理观测可变”免验。 |

重冻只覆盖已枚举的物理变化，需原命令、构建SHA、资源档、输入摘要和原始输出，经非作者对拍后交主控接受。先列差异面，再测；不得用本节估算生成期望。事件须分业务序列与分配序列，后者可变不授权修改前者或改统计定义。新 deadline 采样次数需记录；已有 deadline/取消优先关系锁仍通过，不能为通过减少采样。

## 6. 合同差异门与推荐实施

推荐 ii，因为它恢复“不常驻全部原件”；i 不能独立履约，而且有分段稀疏后常驻更高的代价。ii 不必改变已批的判据、认证存活、历史规模、资源帽或观测定义。以下是明确的合同差异单，**均不采纳**；若实施发现必须如此，停在设计/spec/P 门，由主控交用户裁定，不能借本设计自批。

| 原文 | 会构成差异的新文 / 条件 |
|---|---|
| #1481:24 “不常驻全部原件” | “分段后允许常驻全部原件”会使 i 成为合同变更，不能以消峰代替履约。 |
| #1481:24 同位置同摘要继承；AMENDMENT:45 生涯内结论、冷从原件重建 | “只要digest相同即可复用”或“淘汰正文即撤销/重签原资格”，以及“重启信任旧台账”都改变认证语义。 |
| AMENDMENT:119 缺节点/同ID换内容/历史关联变化照旧写前拒绝 | 将这些一并延迟到重启，或为省内存省去逐字比较，是新增放宽。 |
| AMENDMENT:37 内容库schema不变、提交后台账；:39 同存储账；:35台账格式规则 | 新增权威内容表、将证书变跨进程权威、提前提交台账或不计旁挂字节，均须另裁；仅依原条款扩派生cert字段不属于此差异。 |
| SourceClaim 的当前合同角色 `s_content.rs:2314-2317`；既有预算/观测锁 | 若必须取消仍存活的来源角色费、另签独立父账、改F/reads定义/取消采样或容忍不完整qualification，均为合同差异。真实析构后的角色终止、按原定义产生的新物理读数不是改定义。 |

建议同票分片实施，落点等首拒收据由主控确定。每片均单独评审、CI 实际运行；前片通过不替代后片和原完整验收。

1. **引用与前沿证明片**：逐类列出上述持正文/借用者，证明 k/h/窗的界、逐位材料身份、角色收费和首错链；只引入私有读取接口与生命周期，接现有存储实现作对拍，保留全部验收。界无法成立则继续设计，不挂“有界”名义交付。
2. **受管回读片**：实现同事务同父 ReadLease，覆盖缓存命中/miss、已落库首次可达、同digest异正文、缺失/篡改、上下文/模式、foreign stale、deadline/取消及不足1B；核 F/SQL 与失败来源。旧 ordinary memo 门不变，不接 M4 全局去重。
3. **引用与证书迁移片**：NativeCountsSubsetReader、SourceFields/payload 和能力描述符接回读；按 AMENDMENT 格式规则存派生结论，资格/receipt存活与事件seal不变；typed失败撤销与同父释放锁、certified/qualified锁通过。任何半迁移构建仍由单一判据核验，不用旧/新失败分派。
4. **边界回收与冷恢复片**：合法边界析构非前沿原件，沿原冻结公式换请求域；从空折叠不再重攒全 capture；核父remaining、总live与角色账、cold/recover/台账丢弃。执行原M3四锁及admission六代，不减历史、冷点或代数；用诊断首拒原场景核实际过门情况。
5. **验收收口片**：只对§5中实际受影响的物理值提交重冻证据，原 full_some 不动；保持owned/shard现有运行域，补入必要锁并实际执行，原具名重型窗口照约交付。不得用短场景替代原C/D、E/F或把未通过记为完成。

主要风险是隐含借用未迁完导致悬垂/额外pin、SQL miss 与冷折叠延迟、证书表悄悄重新全量常驻、旧origin回收不能改善当前remaining、非capture全史元数据继续撞后门。分别以生命周期类型、租约/台账分项实测、按代驻留清单、原冻结门及全账分项收据处理，不提高帽或删校验。

回滚以提交/二进制版本为单位，旧新内容库schema与公开内容完全一致。新旁挂格式可丢弃，换版本必冷认证重建；按AMENDMENT:68-71验证旧版续写再新版冷开续写与对照逐位相同。运行中失败仍stale等recover，不“回滚reader”或切旧完整审计。若某片未通过，保留原件与原收据，由主控回滚该片，不回退其他代理改动；回滚到当前累计实现只恢复旧状态，不表示合同已完成。
