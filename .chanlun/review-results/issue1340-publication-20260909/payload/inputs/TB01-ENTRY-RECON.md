# TB01 当前 main 入口勘察与可执行切片

结论：TB01 可以从现役 Rust `OwnedIncrementalClassifier` / `ParseLayer` 开始，首个贯通对象建议选择已定的 CC-006 分型窗口有效域。它需要新立正式 S 会话与独立持久域，并把同次真实计算的对象和见证交到只读前端。现有图表/WS 壳、候选簿和 JSONL 冻结存储只能按各自射程复用。TB01 整体横跨计算出口、持久化、进程协议、只读 API、浏览器及故障恢复，不适合作为“一上下文必做完”的单票承诺；建议在 TB01 内分三片，每片都有可见行为，TB01 全部完成后才解除原 DAG 的 TB02/TB03 阻塞。

本报告是 #1323 已批准 SPEC 的只读实施准备，不重新选择架构，不改 11 个 TB 的依赖语义，不声称代码已实现或运行验收通过。正文里的候选输入是可提审的具名工程测试实例，成功范围明确限定，不给未定 G/FU 填默认。

## 1. 证据边界

- 最新远端 main：`644075a6abcbfcda74815daed68e06eff6ddaf63`，由 `git ls-remote origin refs/heads/main` 当场确认；本地 HEAD/main：`6df8d1921c72c84da90987f8f6c02ebc11fe6aac`。`git rev-list --left-right --count origin/main...main = 11 0`，merge-base 等于本地 HEAD，故本地落后 11 提交。未修改 refs、checkout 或宿主在制品。
- 所有下表路径/行号都从远端 main 的 `git show` 内容取得；JSON 附件逐文件列 blob、SHA-256 与是否等于宿主当前字节。尤其 ADR 0027 在远端已有而当前落后 checkout 没有；不能据本地缺文件称未采纳。
- 先用 MCP 图发现/追调用边。有效图为 `Users-silencehan-Projects-NewChanlun-rust`；图把旧 `classify_with_tower` 定位到 mod.rs:609，snippet返回source unavailable。当前实际代码已由 mod.rs:192–194 导出 pipeline，统一入口在 pipeline.rs:412/478。此处图陈旧；其余未索引的 Python/TS/运行时通过远端 blob 查证，未用其他 worktree 图。
- 只读已批准的当前 SPEC/合同/机器coverage/测试缝与必要现役文档。没有读取旧 #1321/#1322/#1325/#1326 原方案或扫描档案；ADR 0027是现役正本文档。没有研究 K4。
- 本轮未运行 Rust/Lean/浏览器/正式 launcher；仅对宿主和远端 `.sandcastle/main.mts` 做内存 esbuild TS 转译检查，两份均 PASS。这不证明通道认证、镜像或运行可用。

## 2. 实际调用入口与复用边界

| 入口（远端 main 行号） | 实际行为与 TB01 处理 |
|---|---|
| `rust/src/theta_v0/classifier/streaming.rs:96` `OwnedIncrementalClassifier::append_bar` | 先私有 `append_parse`（:101）再 `classify_incremental`。现役 stream 与 nautilus 策略实际持有/调用，可收编为 S 内唯一计算者。首个缺口是同次 `ParseLayer` 没有随返回值暴露；不能 Python 重算一份结构补 UI。 |
| `rust/src/theta_v0/classifier/pipeline.rs:384` `ClassifyOutput`，:412 `classify`，:478 `classify_incremental` | 返回 Classification、tower、candidate_streams、operations；全量入口委托同一增量循环。S取空操作挂载，后续O/B再按批准合同接；不运行 `ThetaPiStream` 的交易与账本推进。全量重建若用于输入更正必须复用此同核，并明确成本，不可切宽松判据。 |
| `rust/src/theta_v0/parser/mod.rs:88` `ParseLayer`，:143 `parse_layer`，:236 `append_incr_layer` | 已有 raw→merged→fractal→stroke→segment/tail；需要保存同源原始身份、窗口/成员映射、first_known及证据。`ParseLayer`的prefix诊断值不自动等于正式S cut。 |
| `rust/src/theta_v0/parser/fractal.rs:35` `detect_fractals` 与 :117 起 `IncrFractals` | 双条件TOP/BOTTOM真实计算已在Rust，现成输出只收成立分型。CC-006还需每个有效三元窗口的RISING/TOP/BOTTOM/FALLING、比较值与三根见证；在同一计算处补事实输出，不在前端用价格再判。 |
| `rust/src/theta_v0/stream.rs:117` `ThetaPiStream::push_bar` | :120真实分类后，:133以后依p_t/nav推进交易与账本，:134还有NAV钳位。该整体入口不能用“nav=1/无资金配置”伪装独立S；复用它下面的结构核。 |
| `rust/src/theta_v0/classifier/cand_event/book.rs:17` | `CandidateEventBook`已有身份/首知钟/修订与撤回测试（:362/382/408/420）。可借生命周期测试形态，不能把候选对象的Invalidated语义或终态规则无映射泛化到所有对象。 |
| `rust/src/theta_v0/classifier/level_view_store.rs:196` | `JsonlCompletedEventStore::append`:210–220调用sync_data；`CompletedFreezeAdapter::reopen`:367提供局部恢复先例。只保护完成态冻结，不具备S全对象批次、固定cut、SQLite writer_epoch或全局历史协议。 |
| `src/newchan/gateway.py:855` `_ensure_live_engine` / :946 `ws_live` | 当前网关自身创建RecursiveOrchestrator并预热1m缓存。HTTP/WS与背压技术可借，正式查询进程必须只读S发布；不能把旧核接到正式接口。 |
| `src/newchan/server.py:420` `api_newchan_overlay` | :444重新resample，:454–463调用build_overlay_newchan并有旧参数默认。仅展示壳可借；正式S读取不可继续使用该重新计算路径。 |
| `frontend/src/hooks/useLiveWebSocket.ts:143` | snapshot仅console.log；overlay由另一路REST轮询。`useOverlay.ts:53/73`调用getOverlay并每60秒刷新；两路不能证明同cut。需真正消费同源snapshot/delta、对象/证据链接与撤回。 |
| `frontend/src/types/events.ts:206` | WsSnapshotMessage仅bar_idx/strokes/event_count。正式S合同必须新承载session/generation/catalog/cuts/seq/关系/见证/索引；精确整数wire用字符串，不沿用JS Number。 |
| `trading_system/persistence/database.py:94` | 一个trader全部状态共库，:102 synchronous=NORMAL。不得复用为C09独立S库。新S选SQLite/WAL/FULL、macOS fullfsync、单写epoch、不可变批次和短读事务，依批准C09落实。 |
| `trading_system/live/runner.py:31` / `src/newchan/cli.py:67` | 前者TradingNode只在docstring，实际:61报尚未实装且:69返回1；后者没有S会话命令。需要真实外部S会话launcher。不能拿测试内部构造、旧live骨架或图表服务器启动当完整入口。 |

此处没有在已核远端主线范围找到名为 `compute_native` 或 `serve.py` 的正式S入口；若此前材料以这些名字指另一位置，必须提供精确路径/commit后另核。本报告不把名字猜测当现有可复用能力。

## 3. 首个完整可验证实例（不需资金政策）

建议首片受测能力是 `CC-006/local_shape`（US-ST-003；机器coverage四个 A-CC-006-01..04）。当前crosswalk该故事`direct_success_decision_or_instance_gates=[]`，但仍有真实输入投影、身份和测试义务。TOP/BOTTOM依据`.chanlun/definitions/fenxing.md:94–126`；良构域与四分支以当前coverage `/classification_axes/5` 为准。

输入采用可再生产的原始逐笔事件档案。每个事件带唯一source namespace/epoch、instrument、event identity/revision、接纳序、源坐标、received_at、原始文字值和精确整数值；每一tick作为一个未经聚合/重采样的退化OHLC（O=H=L=C）交给现役parser。以具名TestOnly profile固定本例单位和clock，不用真实账户或价格数据授权，不让profile成为产品默认。若实际tick→Bar适配未证明保真，必须先把这一对一映射验收，而不是直接宣称原始输入投影完成。

主轨迹（整数tick，以下是本报告建议输入，尚未执行）：

1. `10000 → 11000 → 10500`：相邻点区间均严格非包含，第三事件接纳后真实产生中点TOP，证据可返回三根raw/merged成员和全部严格比较。前两事件没有右邻，显示知识不足，不造第五个分型类。
2. 对第三源事件发**新的revision**，把10500更正为11500：前三点成为RISING；旧TOP按其原身份进入撤回/历史链，保持原first_known，不删除审计对象。新版原始事件的revision参加输入身份，不把它误当同身份异内容覆盖。
3. 追加10800：新窗口`11000,11500,10800`产生新的TOP，证明没有E/B/X进程或数据库时S在修订后仍正式向前推进。
4. 分别用`10000,10500,11000`、`11000,10500,10000`、`11000,10000,10500`补RISING/FALLING/BOTTOM；断言所有声明支持的窗口分支实际非真空出现。中枢、线段、BSP为空只能记“当前无实例”，不能充当这些类别已验收。

该域初始化前两点即有严格非包含方向，组锚与内部极值根是一对一且没有同价竞争，因而不代裁G-001初始化包含或G-002同价端点选择。超出此域的相关条目必须列明未支持/待实例，而不是默默用现有默认UP/等价留早。不得把短tick图的局部分型声称为新笔、线段、LevelView或完整缠论构造。

对象身份按已批准namespace/epoch及结构身份依据绑定；本例不根据页面坐标或数组行号生成身份。窗口对象、分型对象和源事件revision各自有名分；更正改变成立事实时保留旧版、明确替代/撤回，不能用相同payload的重放制造新对象。

## 4. TB01 内的三个建议执行单元

这只是把已批TB01内实施义务分配到可交付工作单元，不新增架构、G/FU决策或顶层TB节点。最终模块/路由/文件名仍由实施者按C09和接口合同确定；本报告不冻结新名词。

| 单元 | 每片必须能演示的外部行为 | 主要改动责任 | 结束边界 |
|---|---|---|---|
| TB01-A：正式计算到前端 | 正式launcher启动独立Rust S和只读查询进程；原始事件真实算出CC-006对象/比较/证据；SQLite Begin/Commit发布cut；浏览器从完整目录进入实例并读证据；E/B/X不存在仍接受追加输入。 | 同次parser观测出口、输入身份、S独立库/不可变批次、ReadCatalog/Snapshot、最小只读页面及真实launcher。 | 目录全部列出当前实现/证明状态。只验本域真实结果及持久可查询，不称TB01已全部完成。 |
| TB01-B：修订与恢复到前端 | 同一入口接更正；浏览器收到原子delta并显示撤回/替代与旧证据；真的杀S再启动，未提交Begin保持可见恢复态，已提交批次不丢不重复，继续接纳新事件。 | Append与correction的同核重建/恢复、稳定身份版本、delta outbox与历史、first_known、writer_epoch、读模型原子应用。 | 同cut snapshot与按delta恢复对象/关系/见证逐字段相等，旧记录不被重写。仍不声称完整历史TB07。 |
| TB01-C：分页、Gap与独立持续 | 浏览器固定cut分页时后端仍推进；慢/断线客户端得到Gap及指定cut重建，重复/乱序/旧generation不污染视图；S局部重启、readonly进程故障与不存在E均不产生经济依赖。 | 固定cut/index_frontier、游标保留/回收、短事务读、背压、跨会话隔离、会话级双进程确定性和浏览器验收。 | 关闭TB01全部行为；仅在此之后解除原TB01→TB02/TB03边。TB02、TB05、TB07、TB09及整图仍未闭合。 |

TB01-A已经包括真实提交与正式入口，不能降成只定义schema、写目录或内部函数单测。TB01-B/C也不能只提交测试脚手架，每片都需运行上表外部行为。若某单元仍超过上下文，继续按用户可见行为细化且保留同一TB01销项门，不删验收项。

## 5. 最高测试缝与必要断言

最高缝是新增的**正式S会话launcher → 原始输入 → S真实Rust计算和本地持久提交 → 只读查询进程 → 实际浏览器**。用两个独立新进程、独立数据目录运行同一完整输入档案，读取和UI同源的正式输出；不靠直接调用内部parser或预先手填StructureRecord取得PASS。没有E是真实不启动、不建其库且无经济配置，而不是mock永远返回OK。

必须完成的有界验证：

- 非真空：四种声明支持的窗口类型分别出现；TOP形成、更正撤回和后续新TOP实际发生；对象证据指向真实原始事件和merged窗口，前端可逐项导航。目录中未实现、当前无实例、域不适用、输入不足分别显示。
- 合同：同身份同内容返回原持久收据；同身份异内容IdentityConflict；新revision按替代关系接纳。精确整数和规范payload_hash跨Rust/JSON/JS不丢字节，包含大于2^53的wire往返负对照。
- Cut：固定snapshot token分页不得混入后来cut；delta拥有base/next、持久seq_range、原子upsert/关系变化/撤回；客户端应用后与相同cut完整snapshot逐字段相等，证据与索引也比较。
- 恢复：在Begin后、不可变批次写完而Commit前、Commit后但回执前真实终止S进程；重启恢复原输入身份/持久cut与未完成修订，不产生幽灵引用或误把DeliveryUnknown当未发生。S库损坏应报StorageUnavailable，不能冒充继续正式推进。
- 隔离：另一session/旧generation游标明确拒收；重连重复不重生效；超保留期Gap及可用cut重建。慢查询不持有跨页长读事务、无限等待消费者ack或阻止后续输入。
- Oracle：CC-006的四分支要有独立定义/有界枚举oracle、真实raw投影和程序对拍；仅生产reducer双跑不足。现有`detect_fractals`测试能复用语义工作例，但不能证明正式发布与UI。机器coverage当前只记录代数分区主张，不能当已运行Lean证明。
- 确定性：除预先声明的非语义run身份一一重命名和墙钟性能外，语义发生/获知/确认/接纳/应用时刻、顺序和cut全部硬判。保留输入/schema/规则/目录/构建/profile hash、故障调度和所有输出。

复用TEST-SEAMS的T05/T06/T07形态以及T01/T02首分歧报告；不得继承旧inconclusive也PASS、信号面通过就忽略状态差异、同进程第二实例冒充杀进程恢复等口径。Rust改动按项目fmt/check/clippy和相关行为test；TS按实际build与浏览器行为验收；若动Lean必须跑fixture drift并区分工具链失败。这里没有承诺全历史重放，仍遵本仓靶向/有界验证纪律。

## 6. 新生代码名分与迁移限制

新S launcher及被其真正调用的模块应在本TB01实施票绑定，随受测主线接入后按世代宪法判现役；在ticket分支测试通过不等于已经在唯一main现役。只立类型而生产零调用，必须同时满足“有票号、非测试段引用入口、有测试调用者、票面登记后续驱动链”才是新生入口待驱动；否则不得借该格包装空壳。测试夹具注明TestOnly输入，不把人工结果快照登记为正式结构源。

旧 gateway/server/trading bridge 有真实调用者，不能因不满足R2便自行把整个文件判孤儿或删除。此票将正式S观察路由绑定新owner即可；旧生产发送入口终态和默认整体迁移仍由TB09/TB10负责。TB01结束不意味着旧writer已隔离或生产默认启动已迁移。

## 7. Sandcastle只读可执行性核查

- 能力形态适合有界实施单元：`.sandcastle/main.mts:160`建隔离沙盒，baseBranch=main；每票先实装、后独立review invocation，最多3续跑轮、每次1票；prompt明确不push、不merge、不close，适用main批准门仍在。
- 运行配置存在实际差异：宿主main.mts:308/325用`claudeCode("claude-opus-5")`；远端main:309/326均用`primeAgent(REVIEWER_MODEL,{provider:PROVIDER})`，顶部值deepseek-v4-pro/deepseek。README写的默认不等于实际调用。两份TS内存解析都PASS，重复import不能被报告为语法阻塞。
- 当场 `launchctl list`：sandcastle-claimer和wayfinder-engine有PID；watcher无当前PID。workers.jsonl末12条最新到2026-08-31（#1235 reviewer completed/sandbox closed），没有当前TB01工蜂运行证据。GH_TOKEN_ROTATION_REQUIRED文件不存在，.env存在；未读密钥值，也未测试认证。
- 未启动或重启Prime、Sandcastle、Docker、交易或任何launcher；未改标签/assign/评论。未验证镜像中的浏览器、锁定SQLite版本、macOS fullfsync或当前provider。容器Linux验收不替代目标macOS持久化前件，必须另验。
- 若根决定投递，票面必须带可读的已发布SPEC/合同/完整coverage、精确基线commit、这个受测域与本片验收。`/tmp`路径不保证沙盒可读，不可只把这些临时路径写进票让worker自行猜。投递前应按实际固定执行通道核fresh运行；本报告不擅改服务或选择新模型。

## 8. 未确定项和不虚报边界

本报告确认的是入口、行为缺口与可实施边界，没有运行验收证据。尚未落实：正式S新模块/launcher命名、SQLite具体crate/运行版本及持久化前件、规范字节格式的具体实现、raw tick一对一输入适配与CC-006独立oracle、对象身份/输入更正的代码承载、完整目录的发布打包、浏览器可用性及当前执行通道fresh smoke。它们是实施/验证工作，不能因尚未写代码就重新请求整份SPEC批准；若实现过程中需要越过具名G/FU的语义范围，才按该门暂停依赖部分。

G-001/G-002/G-007等未定域以及其余CC的真实构造/证明仍留TB02，经济政策仍留相应TB；只显示Unsupported只能验失败路径，不能销掉完整图义务。TB01完成只证明受测结构域正式算出并经可靠观察通道交付，不等于44 ST/62 CC/10 LC、经营成功路径、paper/live或#1323已完成。

## 9. 文件指纹

完整行区间、符号和宿主一致性见同目录 `TB01-ENTRY-RECON.json`。以下hash均绑定上述远端main。

| 文件 | Git blob | SHA-256 |
|---|---|---|
| `rust/src/theta_v0/classifier/streaming.rs` | `b44de8cd27e6e624816f25b32ffe96aef78d3dd1` | `b6d6c70a3a1aa86a4b648a3c8088ba8f0a914289709a066b26f16c5b05fdb590` |
| `rust/src/theta_v0/classifier/pipeline.rs` | `a5cc6004d3a5de293034bdd46a026bda4fadf213` | `40a1b5f490da8af491be146c438761ad21f464bada55b0f9b20f41d047947edb` |
| `rust/src/theta_v0/parser/mod.rs` | `21562834fc7ace77a9644a87b4ff62ba904b600c` | `f5b612ae3dde7fb8975cbe9a175ee6cd332b3d5783b8fb8b421c0e7fb029fcd2` |
| `rust/src/theta_v0/parser/fractal.rs` | `54b0f98e34717d7ee8defa5ffcfd42230ca21969` | `f5714f18ce227397772650146c01b6737af130d837fb15a627fffbd0ff713778` |
| `rust/src/theta_v0/parser/inclusion.rs` | `cf3fa5130eba3cf43ac8ebde1527a230e297bc85` | `fa968583b89db63d0dcd1bce65de675e4a274cf22ea4b258b7977026fc23513b` |
| `rust/src/theta_v0/stream.rs` | `3e5cbbf1616a0279580677cfa22cd9445bf0730f` | `7363d915f0908e7c25771c79ae9a5366438e5aca052f338a570103bf8d2014e1` |
| `rust/src/theta_v0/classifier/cand_event/book.rs` | `7b3eb2069b38a80f10ed6e1e18a07dd34eaefb99` | `156b01c5ca72f99e6edc72cdd4665edd9d3d4f5584cdfbbbc89d5a01c0b0f2f3` |
| `rust/src/theta_v0/classifier/level_view_store.rs` | `90e117fe4c164b831e6f3797c62d7634caef7689` | `89a33979ebd73735057d40ad4881e511ab49bf6a258b682625c2530e9e7dfb14` |
| `src/newchan/gateway.py` | `3af301bfc5976abc7bf178ef69f226905c0f41ea` | `2090ffc643277fdc7d80228d29ae1bccb6e6817df4ee55a1ccc9bfe01094561c` |
| `src/newchan/server.py` | `7ee7c1fd48a3ebc57cfc377be0c0045ecac98bc6` | `ac1bc66fa62eb2d7588ab7efef6d203c45cd7fa29c29e9909aeb23ab6e4fb15b` |
| `frontend/src/hooks/useLiveWebSocket.ts` | `0f4e58d44179d58308ad41564de63a8f39c52df6` | `7f08a29a6fbe20636c9f99e40b12ff6a5012996bb7d754d582561d7bfc5ae144` |
| `frontend/src/hooks/useOverlay.ts` | `ddc58e6cca83168ba517efa67769bde269f59bc2` | `632739d037f500055fa6c2fbca3ffaf295ccdf9dc273d4bb224364d164aeab2b` |
| `frontend/src/types/events.ts` | `7d16f3f5ff3caae400455578ce443b79f79f91fd` | `ecfc0830f928447ce7d45232d80b1a40f2519bf0727c8f5c88bb9ebcb95edac6` |
| `trading_system/persistence/database.py` | `5f9e6d375abd298d2b29d98761237a1578d21b7a` | `f49cdcd8f6f2072940e1c28f816c21be15a8983d22824d37d2f996f8e6ee755f` |
| `trading_system/live/runner.py` | `418b8893ec18aff9509d2c0085a804552c0bcd4a` | `b83fba8cdd7f426a1d5fb38e5b871f34f265d78505fcee55afe5da34cff3dc62` |
| `src/newchan/cli.py` | `95ce4f03506a2ea08770e77e56f3a71802ce8669` | `c3274e64f6e649de6633b9e7c08490c8c7f84f7fc20cbb15ed1aa8a3c34d7bac` |
| `rust/Cargo.toml` | `6893a49469f4f4b68e9e2b078e6e1fea26997f2d` | `3947802819b863c19b6f7add05cc51615079ccf03d1a87a6c405f2b2fb503000` |
| `.sandcastle/main.mts` | `fbe82e02902d285c5d0c16357225741f6a24505a` | `e83d1d8d6b01022edfa04dddb7d5255627c0a450f69c6101390b7bfd1b75c533` |
| `docs/agents/generation-constitution.md` | `ebce8dcf5616e840b80cd931b2e07abe71db36a4` | `6871ddb943cbc523c2c5a228bb1046adf9e27072b7c1ba929c7d8c683e23021e` |
| `docs/adr/0027-classification-chong-operating-program-ownership.md` | `52be5777fea615d21beddf5e8c77bb747705945b` | `bf6eb4721ab0d56ad5f1ce3000d74eaaa79f9c497b9b5301697df79f08fa3c1b` |
| `.chanlun/definitions/fenxing.md` | `0e6c977da22ee92b17590a7f88f2265704968052` | `0285fd9d9352b63a6ec1686bd5d7543dcd2a7c2d93c9a2e47d007507fc58f88f` |
