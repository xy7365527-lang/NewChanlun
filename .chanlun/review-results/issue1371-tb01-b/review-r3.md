# #1371 TB-01-B 独立评审 R3

> 编号声明：本提交中的 #1371 仅指 GitHub Issue。
> 同一原独评 session 续审；不加载作者会话。仅新增本报告及 `review-r3.json`，前四份报告保持原字节。
> BASE：`2212eba31e0b7419907e622e79f02f7c2cc37706`；唯一待审源码 HEAD：`65fb5e72bbee1de6656ba4b902dd506ae526195f`。
> 前轮源码：`0298cf429804c611d6ee5f691a1f36ee2a720fba`；前轮报告 tip：`6e1b355b1454c9a20afd50a372058da11df86a30`。
## 结论：FAIL

**1 H、3 M 阻塞，不能通过。** 正常修订与三点原库真杀恢复已经得到本轮 Linux 实跑及根原生/真实 GUI 的有限域支持；但不能据此宣称“全部 H/M 已解决”。新增反例涉及本片的同 cut、持久 Delta 完整性、正常前缀推进、S 自身存储失败边界，不是 C 压力测试或经济全域义务。

根提供的 macOS 构建、正常与恢复 GUI、同实际 writer 连接 fullfsync 证据**不再泛化为未验阻塞**。本次 FAIL 来自下面真实反例，而非“容器没有浏览器”。根已跑的范围与本轮尚需补验的范围分列。

## 1. 绑定、范围与证据等级

- 进入工位：HEAD 等于固定 `65fb5e72bbee1de6656ba4b902dd506ae526195f`，branch=`codex/1323-tb01-b-1371`，dirty=0。完整范围不是最后一笔 R8，而是 `git diff 2212eba31e0b7419907e622e79f02f7c2cc37706..65fb5e72bbee1de6656ba4b902dd506ae526195f`；同时核 `git log` 及 `git diff 6e1b355b1454c9a20afd50a372058da11df86a30..65fb5e72bbee1de6656ba4b902dd506ae526195f` 六笔修复。
- 完整差异 11 个文件：4 个生产文件（Rust 会话、Python reader、HTML、launcher）、作者报告两份、roster、四份旧独评。无新增独立生产源文件；没有漏审另一个新判定器。完整 diff SHA256=`f1da1eb097fe8a8e2ff7af244714088050c51d8edc7dfea1761f2d803683ea13`；增量 SHA256=`f394ba5fc5083e698e3262fc89a726b653428e0d58f2e13f91336ad533b44d47`。
- 已读固定 R1/R2 报告、已签 payload 的 SPEC C02/C08/C09、接口共同身份/S/StructureRecord/Delta/恢复读集、effective、crosswalk、coverage 与 TEST-SEAMS。17 ID 从有效 JSON pointer 回查，historical/not_effective 不作为合同。规范清单 `evidence/REVIEW-INPUTS-S2-R1.json` 实际 4450 字节，SHA256=`38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`；六指定载荷逐份与归档索引 hash 相符，见 JSON `binding.spec_files_verified`。
- 远端票面即时只读核验 `gh issue view 1371 --repo xy7365527-lang/NewChanlun --json number,state,body` exit **4**（未认证），未改认证/服务。发布正文 SHA `71672e11…e3b5` 只作为根提供绑定，不冒称自己重新拉取通过；不重新打开 SPEC 批准门。
- 源码 hash：

| 文件 | 本轮实际 SHA256 |
|---|---|
| `rust/src/bin/s_structure_session.rs` | `0718729cf3535b128ff995d1f6aad75b5385c9042140f5a053db970a75f3dfb2` |
| `s_session/s_readonly_server.py` | `ee29bf801dfec475f603ab9c2aa98b616fdbceb33f3d3a573d18e9d05146e4b6` |
| `s_session/browser/index.html` | `13402a4011034e624de34b4137696a3dd8d3de2e1b35ec854e3a2331ca1591ac` |
| `s_session/launch_s.sh` | `65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d` |

Linux 二进制：`8998820f09644601d2d9bdfdc9fc4293032acea07a22ce0109cb7d6cf71e19c0`（ELF aarch64）；rustc/cargo 1.97.1，Rust SQLite 3.53.2（bundled 0.38.2），HTTP 使用项目 launcher 选中的 uv Python 3.11.16 / SQLite 3.53.1。Linux 与根 Mach-O arm64 binary=`c2bb969c…580a9` 不混作同构建。

**全表公共绑定**：以下每个 AC/细项均绑定上述 head/build，目录文件 SHA=`938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`、profile 文件 SHA=`2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc`（规范内容 hash=`0e9dfc9b…2d64e`）、rule=`s2-axis-quantifiers`。逐场景输入原字节/hash、真实 argv/exit/HTTP body 在 JSON 附件，不以统计计数代替这些绑定。

证据标签：`ACTUALLY_RUN`=本轮真实 CLI/HTTP；`ACTUALLY_RUN_NODE_NOT_GUI`=当前 HTML 函数、真实接口 payload、模拟 DOM/输送调度；`ROOT_RUN_SUPPLIED`=用户内联的根原生/GUI 实测结果，**未在容器打开不可挂载宿主文件**；`SOURCE_REVIEW/REUSE`=实际 git 字节/调用链复核。后两种不得冒称本轮 GUI 实跑。

## 2. Standards

实际 `cargo build/check/clippy --locked --features s_session --bin s_structure_session`、`cargo test --locked --features s_session --bin s_structure_session --jobs 1`、`cargo fmt -- --check` 均 **exit 0**；31 tests passed，clippy 当前会话文件 0 命中（库内既存 warnings 保留，不用管道末端成功替代 cargo 状态）。完整 argv/输出/exit 分别附入 JSON。

代码形状本身不作违规判据。这里实际存在 Rust/Python 两份不等价根验证器，R3-H1 已以同输入不同输出证明行为违约。标准层编译/格式通过不能掩盖 Spec 失败。没有改 production/tests/CI/fixture/roster/作者报告，也未执行 push/merge/main/harvest/GH 写或经济外效。

## 3. Spec：阻塞发现

### R3-H1（H）——坏根在 Rust 被拒绝，HTTP 与前端仍宣布完整当前成功

**位置**：`s_session/s_readonly_server.py:123-149,165-189,390-417`；`s_session/browser/index.html:305-315,479-526`。

**真实触发**：先以正式 Rust 形成 gen1 TOP，再更正提交 gen2 RISING/撤旧 TOP。仅对 `SQLite mode=ro backup` 出来的测试副本执行：

```sql
UPDATE meta
SET value=(SELECT index_frontier FROM structure_deltas WHERE generation=1)
WHERE key='index_frontier';
```

不是伪造结果，也没有破坏 batch 本体；只是将当前根指向同库另一份**确实存在且 hash 合法**的历史 batch。

- `/api/state`、`/api/snapshot`、`/api/delta?after_generation=0` **均 200**。前两者仍声称 generation=2/cut-2，index_frontier 却为 gen1 的 `batch-dba68ce5…af28d`；D2 的 index_frontier 为 `batch-476498b2…ad66eb`。
- 同一副本的 Rust `snapshot/accept/advance/recover` 全部 **exit 1 StorageUnavailable**，验证前后十表 hash 不变。错误原文：`meta.index_frontier=… 与 delta[2].index_frontier=… 不一致`。
- 把上述真实 HTTP 响应送给**当前 HTML 的实际 applyStream 函数**（Node VM，非 GUI），游标提交为 `2`，正式状态保存 gen1 batch 根，诊断输出：`1 活动 + 1 撤回 + 6 见证 + 10 关系 + 0 观察 + 4 源事件，逐字段一致（已驱动正式渲染）`。
- 对 `meta.structure_cut='cut-99'` 或内层 frontier/seq_range/catalog_evidence 与外层不一致的副本，Rust 拒绝、HTTP Delta 拒绝，但 **HTTP state/snapshot 仍 200**。不是统一读门。

**原因/后果**：Python 只核每个 batch 存在/hash/长度及末 frontier，不核当前 index==该代 Delta 根、cut==cut-gen、完整内外11列/封存坐标。前端只比较六集合和部分身份，不比较 `index_frontier`、scope、目录证据等全部语义字段。结构写者已经停写，UI 仍显示成功；“字段存在”与“同 cut 全字段验证”被混用。

**最小修复**：HTTP 全正式结构读入口在同一短事务中执行与 Rust 等义的完整元组验证；前端提交前对最终 Delta、Snapshot、目录/状态包的必需语义字段进行完整绑定比较。不要用后端另一入口更严格或客户端六集合相等来豁免。

### R3-M2（M）——必需 witnesses 键消失仍成功读出、后续写入继续

**位置**：`s_session/s_readonly_server.py:545-552,639-655`；Rust `verify_reachable_root:597-642`、`project_delta_row:3058-3072`。

真实 gen2 库副本仅执行：

```sql
UPDATE structure_deltas
SET delta_json=json_remove(delta_json,'$.witnesses')
WHERE generation=2;
```

`/api/delta?after_generation=0` **200**，D2 明确缺 `witnesses` 键；state/snapshot 200。实际 `snapshot`、随后 `accept/advance/recover` 均 exit **0**，推进 gen3。Rust Watch 共用根核却只校验头部对应（SOURCE_REVIEW），本轮没有把该源码结论冒作另跑过的 Watch 损坏命令。

**后果**：关系/对象引用仍在，但该持久批次缺少必须的见证数据，不能按原游标重建指定历史 cut；非法载荷没有进入 StorageUnavailable/停写边界。不是 retention/分页压力。Python `if key in delta` 与 `delta.get('witnesses', [])` 把缺字段当空集合，Rust 11列头等值也不等于完整批次内容。

**最小修复**：必需集合存在且成员/引用完整；读写前件校验载荷与完整批次引用闭包，或依法从可验批次恢复；缺字段不能默认空成功。

### R3-M3（M）——合法单点→两点的观察 Delta 与 Snapshot 不等

**位置**：Rust `s_structure_session.rs:1961-1999,2050-2054,2081-2084`；HTML `:460,512-516`。

**无 SQL 注入的真入口**：init → accept(e0=10000)+advance(gen1) → accept(同内容 e0、e1=11000)+advance(gen2)。保持本片 TestOnly 一对一退化 OHLC、无同价；这只是缺右邻的正常知识不足前缀，不裁 G001、不扩市场域。

同一 `obs-92a89ddecde02693`：

| 投影 | batch_id | detail.raw_events |
|---|---|---|
| Snapshot gen2 | `batch-199a90a2…48705f`（gen1） | `"1"` |
| D2.observations | `batch-abbf5706…201356`（gen2） | `"2"` |

Rust 对观察表 `INSERT OR IGNORE` 保留首版，却在每代 Delta 重发新 batch/detail。实际 HTML 函数读取实际 Watch 和补取的 **真实 HTTP State** 后报 **2 字段差异**，cursor 仍为 0。正确识别差异不等于合法路径成功，重试相同日志不能自行修复。

**最小修复**：观察记录的 identity/version/内容采用一份一致的不可变合同；已存在者沿用已存完整内容，或为变化建立明确新版本并一致发布；补 1点→2点→3点真实入口与 Delta/Snapshot 对拍。

### R3-M4（M）——缺失 scope 仍成功，新的必需 meta 验证未封闭旧 fallback

**位置**：Rust `:370-444,2444-2448`；Python `:44-47,165-189`。

真实 gen2 库副本执行 `DELETE FROM meta WHERE key='scope'`。三个 HTTP 均 200，snapshot.scope=`{}`；Rust snapshot、accept、advance、recover 全 exit0，持久状态继续变化。

`verify_required_meta` 虽调用 `read_scope`，后者仍把缺项默认 `{}`，Python 相同。结构/经济 scope 是正式快照必需身份，丢失不能作为成功缺省。此处明确是 A 继承 fallback 在 **B 新必需 meta 门**中的未补缺口，不以此否定 A 已修的垃圾库头/坏 JSON 拒绝证据。

**最小修复**：已初始化正式会话的 scope 必填且结构/经济声明可验证；丢失或矛盾按 StorageUnavailable 关闭相应正式读写，不静默空对象。

## 4. 本轮实际通过的边界（不外推）

1. **LINUX-NORMAL**：正式 launcher 带显式输入/profile/本平台 `--bin` 启动 prefix2（exit0），同库真实接纳第三点、更正、第四/第五点及两次大 revision，形成7cut。每代保存实际 `/api/state`，逐次 `snapshot --as-of 1` 完整稳定。最终从delta0重建六集合全部相等；这六集合通过**没有被再称作完整metadata/catalog验证通过**。
2. **LINUX-KILL**：三个新的真实 S PID，均从已发布旧 TOP 开始接纳更正：

| 故障点 | PID | Signal / wait | kill 后 gen / batch | 原ID Query kill后 | 正式恢复/对照 |
|---|---:|---|---|---|---|
| after_begin | 2060 | SIGKILL / -9 | 1 / 1 | latest_published=false | 原库recover epoch2；完整Snapshot/AsKnown1/Watch等control；HTTP Snapshot一致 |
| after_batch | 2079 | SIGKILL / -9 | 1 / 2 | latest_published=false | 原库recover epoch2；完整Snapshot/AsKnown1/Watch等control；HTTP Snapshot一致 |
| after_commit | 2098 | SIGKILL / -9 | 2 / 2 | latest_published=true | 原库recover epoch2；完整Snapshot/AsKnown1/Watch等control；HTTP Snapshot一致 |

每次先实际读 `/proc/<pid>/cmdline` 核 advance+同 DB，再 `send_signal(SIGKILL)`，`communicate(timeout=10)` 确认 wait_exit=-9、stdout为空。after_begin/batch 的 current Snapshot 完整等旧TOP，没有混入未发布修订。完整 batch 由另一 SQLite 连接读到且 hash/byte_len正确，尚未发布根不被当成功 cut。控制轨迹的输入、received 时间、顺序与业务身份逐字相同；没有抹掉 first_known/receipt/object ID。只对 API Snapshot/Watch 全字段作等值，不声称包含真实恢复次数的整库绝对相等。
3. **LINUX-LIVE-WRITER**：PID2241/2250 分别停在 after_begin/batch；不发 kill/stop 信号。正式 recover 到 epoch2 后 release，旧进程自身 exit1、十表不变；新owner继续 gen2。三真杀臂也验证旧epoch重放exit1，Query和同argv无新输入advance幂等且十表不变。
4. **NODE-CONSUMER**：实际 HTML 函数验证 Snapshot fetch 失败后 cursor 保持2、原对象不受浅拷贝污染；晚到旧gen2响应不覆盖先完成的gen3请求。此为受控调度/mock DOM，不冒称实际 GUI；根E7仅提供其声明的真实丢包/重复/换会话范围。
5. **精确整数**：最终7cut HTTP State/Delta完整读取无 numeric integer 叶（bool/null保留）；`9007199254740993→9007199254740994` 与 predecessor文本完整。旧发布库兼容性有限复用根E2，而非人工改坏数据冒称历史库。

## 5. 对前两轮报告的明确订正与 finding 处置

原四报告不改；其历史结论不因保存而继续有效：

- **R1 L-1 已修，但原低估撤回**：它就是实际丢回执/launcher重试路径，不是“只在非必经CLI退化调用”。本轮真杀Commit后原argv无新输入返回原cut的幂等已实证。`accept replay`是写入口，不是只读Query。
- **R1 L-2 已修**：负as_of/after拒绝、URL编码解码和重复参数拒绝有实际读数。
- **R2 L-NEW-1 的生产不可达判断错误**：旧 `project_raw_bars` 确实遗漏 supersedes_revision，catalog 计数也为整数；根旧真实发布库提供反证。BigInt只保护其被调用处，不能普遍挽救JSON.parse已舍入的number。当前最终读出投影已补，接受本轮大整数及根E2旧库有限证据；不是维持原“不阻塞且不会产生错误结论”的理由。
- **撤回 R2“全部H/M已修复”及原两轮“所有17项通过”**：当时的六集合对拍没有证明完整元组，代码出现tmp/validate不等完成验证；本轮H1/M2/M3/M4必须保留为阻塞。AC编号沿原票：AC3 Delta、AC4三杀、AC5 Query/epoch。

逐一复核旧根/辅助问题（有重复者合并证据、不吞结论）：

| 原finding | 当前处置及具体证据/限制 |
|---|---|
| `ROOT-B-R1-01` | 正常历史头/首发布对象字段已修；根E3/E10+本轮稳定AsKnown支持。坏根HTTP仍漏，见R3-H1。 |
| `ROOT-B-R1-02` | 正常路径renderAllFromState已接线，根真实GUI E3/E10有限接回；完整元数据对拍并未闭合，R3-H1实际假绿。 |
| `ROOT-B-R1-03` | deepClone/pageEpoch及失败不提交有源码与Node实测，根E7有限真实断连成功；不泛化所有调度或Gap指定cut行为。 |
| `ROOT-B-R1-04` | load(current)、input默认0已修；根实际GUI初始/历史切换支持。 |
| `ROOT-B-R1-05` | generation坏/中尾代链缺失/无效frontier/内外11列的具体防线已增强；不是全读入口闭合，R3-H1/M2/M4仍在。 |
| `ROOT-B-R1-06` | 当前/历史batch删除时正常accept/advance/recover均拒绝，十表不变，本轮已实跑；不证明所有S内部损坏都拒绝。 |
| `native-delivery-query` | 真实更正Commit后SIGKILL，原identity-key Query、原argv幂等、旧epoch拒绝均已本轮重跑；不是accept冒称Query。 |
| `native-current-unpublished` | after_begin/after_batch current Snapshot完整等于未更正旧TOP，query显示rev2未published；本轮三杀实证。 |
| `native-future-AsKnown` | 未发布as_of=99 HTTP503带Unavailable细节，明确拒绝而非正常空混合；不要求特定传输码。 |
| `PY-DRAFT-H01` | generation=broken 三HTTP503，当前验证有效；不自动推成scope等所有meta有效，见M4。 |
| `PY-DRAFT-H02` | 完整持久代链破损按StorageUnavailable拒绝；游标ahead有Gap。此非C正常retention压力成功，后者仍在C。 |
| `PY-DRAFT-H03` | 客户端sessionBinding/pageEpoch修复与根E7新session重建有限支持；HTTP的完整根身份仍有H1缺口。 |
| `PY-DRAFT-H04` | 已存在的深层成员逐字段校验增强；缺witnesses键仍成功，R3-M2未闭合。 |
| `PY-DRAFT-H05` | 精确wire问题不能称生产不可达；当前修复由本轮大revision与根E2旧发布库证据支持。 |
| `PY-DRAFT-H06` | R2浅拷贝不安全，R3已deepClone；本轮实际函数快照失败后cursor2/对象无变化，有限修复支持。 |
| `PY-DRAFT-M01` | 标准URL解码/重复参数400已本轮实际HTTP确认。 |
| `PY-DRAFT-M02` | 首载显式current、有界历史输入已修，源码+根GUI证据。 |
| `PY-8C-H07` | 历史frontier严格i64及不存在cut拒绝已有防线；完整历史头损坏仍需H1补齐。 |
| `R2-all-HM-fixed-assertion` | 撤回前轮未限定的“全部H/M已修复”结论；不能凭函数存在、六集合等值或mock脚本初态推出真实协议全义务。 |

## 6. 根证据的有限接回、源码复用及限制

以下是 **ROOT_RUN_SUPPLIED**，不是本容器读取宿主原件或个人亲跑macOS。路径/hash/来源限制逐条在 `review-r3.json.root_evidence.references`；只采用用户已经内联的具体结果。

- **E1/E3/E4/E10/E11**：固定65fb原生构建和正常7cut、三故障恢复前后实际GUI/API；接受相应成功实例。HTTP/CLI Watch外层原有两项目录键差异保留（catalog_evidence/catalog_run_status），仅共同字段及完整inner等值，不删键伪称原封装相同。before12是保存响应复核、after15为新增GET，不能说before又做了GET。
- **E13平台前件已经有限接回**：根四个实际writer PID31637/31640/31641/31642，在同一传入connection回读wal/FULL2/fullfsync1。本轮从git逐函数抽取，open_db、record_connection_pragmas、cmd_accept、cmd_recover hash与根给定值**一致且跨6eeed→65fb逐字相等**；不把另开pragma连接或数据库meta当同writer连接证据。此仅配置读数，不是fcntl系统调用/断电保证。
- **E5/E6/E8/E9复用**：本轮逐函数重核根给出的 verify_required_meta / verify_reachable_root / verify_begin_owner / persist_batch_before_publish / testonly_pause / accept / recover 等hash，全部相等。cmd_advance不同（计数wire变化），未把它整体标同hash。根E5只测CLI且明确R8 HTTP坏根矩阵未跑，**不能覆盖R3-H1**。恢复/活writer核心本轮亦重跑。
- **E7消费者复用**：58dab及6eeed的HTML SHA=`13402a40…591ac` 与65fb一致（本轮实际核git blob）；可复用根真实丢包、重送原字节、session重建的有限轨迹。该结果不能证明本轮错根/未比字段或所有竞态。
- **A oracle / E14**：A ROOT-ACCEPTANCE、ACCEPTANCE-INDEX、incremental及归档索引定位已核。local_shape SHA=`57537f3a…3ae97`、inclusion SHA=`e94a0257…0fbf`、目录及原四分支输入同hash。只复用其严格核5滑窗RISING/TOP/FALLING/BOTTOM/RISING；B修订/持久恢复由本轮新证据承担。调用链仍是 `cmd_advance:1608 ParseLayerIncr::append → :1611 merged_bars → :1614 classify_local_shape_sliding`，reader/UI不重判、不推进经济。
- **历史引用/辅助限制**：根E10的动态SERVICE-RESULT旧hash由事后RECONSTRUCTED-READY精确重建，非当时另存；本评审保留这一名分，不冒称验证过原件。E15只评辅助checker，partial响应留痕不足会FAIL，本次27完整响应不受影响；不登记成生产API缺陷。

本报告不以31单测代替这些运行证据，不将用户内联root结果提升为本容器原件字节核验。

## 7. 原票8AC逐项结论

下表的每个证据键对应 JSON `checks` 或 `root_evidence.references`；全部共用第1节head/build/input/rule/catalog绑定，原始argv/exit和输出见内嵌附件。

| AC | 结论 | 实际证据/缺口 | 证据键 |
|---|---|---|---|
| AC1 | PASS_BOUNDED | 正式launcher初始化两点，再真实accept第三点/更正/追加产生TOP→RISING+撤旧TOP→新TOP。Begin实测在解释前持久。 其余CC、未裁G、非退化投影不扩测。 | LINUX-NORMAL, LINUX-KILL, E3, E14 |
| AC2 | PASS_BOUNDED | 源revision/接纳seq/source_coord/对象身份分列；重放原收据、同rev冲突、首获知和supersedes/replaces保留；大修订文本无损。 仅声明单源TestOnly窗口域，不销经济责任。 | LINUX-NORMAL, LINUX-LIVE-WRITER, E9 |
| AC3 | FAIL | 正常7cut六集合等值及根GUI成立，但H1错索引仍假绿；M2缺见证批次成功；M3合法prefix推进Delta与同cut Snapshot不等。 C完整分页压力仍留C，不能用该边界免除当前单批失败。 | LINUX-CORRUPTION, NODE-CONSUMER, E2, E3, E10, E11 |
| AC4 | PASS_BOUNDED | 本轮三真SIGKILL从旧TOP切到修订；原库重开、未决拒绝/换代、完整batch可独立读；Snapshot/AsKnown1/Watch全字段对control等值。 不证明硬件断电或所有调度；正常恢复通过不掩盖损坏前件H1/M2/M4。 | LINUX-KILL, E4, E6, E10, E11, E13 |
| AC5 | PASS_BOUNDED | 三臂原业务identity只读Query；Commit后同argvadvance幂等；两活旧writer跨epoch后release exit1零写，新owner成功。 只证明S业务，不造经济NoRecord资格；没有按新ID重做。 | LINUX-KILL, LINUX-LIVE-WRITER, E6, E8 |
| AC6 | PARTIAL_WITH_FAILURE | 正常历史cut/对象发布字段稳定，缺右邻无TOP，AsKnown和Recomputed分栏由根真实GUI补齐；坏元组仍被HTTP当完整状态。 同一历史健康cut通过不等所有持久元数据/目录历史校验闭合。 | LINUX-NORMAL, LINUX-KILL, E2, E3, E10, E11, R3-H1 |
| AC7 | FAIL | E/B/X真实未启动时S成功且坏batch删除拒绝；但错root HTTP仍成功，缺scope/缺载荷仍准写，不能整体StorageUnavailable通过。 经济未启动不是停止/分区经济组件，TB05/I02全域仍外。 | LINUX-CORRUPTION, LINUX-NORMAL, LINUX-KILL, E5 |
| AC8 | EVIDENCE_RECORDED_NOT_DELIVERY_APPROVAL | 同head/source/input/build/PID/signal/wait/API/持久读数、真实脚本和原始响应完整嵌入review-r3.json；根GUI证据以已内联结果+路径/hash准确引用。 根宿主原件未在容器重读；Node不等GUI；失败项未销。 | LINUX-KILL, LINUX-CORRUPTION, NODE-CONSUMER, E1, E3, E4, E10, E11, E12, E13, E14, E15 |

## 8. 17来源子义务逐项覆盖

不是完整来源ID销项；signed_pointer 与 required_result 已逐项写入 JSON，以下不得被一个总测试数替代。

| ID | B范围结论 | 实证/缺口/未验 | 证据键 |
|---|---|---|---|
| A-CC-006-03 | PASS_BOUNDED | 同一严格核修订三臂与control完整对象/关系/身份/见证/历史等值；不证明任意市场投影。 | LINUX-NORMAL, LINUX-KILL, E14 |
| A-CC-006-04 | PARTIAL | 真实轨迹四比较与三根raw/merged见证可读，根GUI有限通过；持久Delta缺见证未拒绝仍使完整变化查询不可靠。 | LINUX-NORMAL, E3, E10, E14, R3-M2 |
| A-ST-044-02 | FAIL | 单批原子存入并不足；指定根/见证/观察字段失败，不能宣称全字段原子一致。高级扩展未构造。 | R3-H1, R3-M2, R3-M3 |
| A-ST-044-03 | PARTIAL | 三修订真杀+历史正常续接支持；合法prefix观察使普通续接持续不等。C全保留压力不在本片。 | LINUX-KILL, E7, E10, E11, R3-M3 |
| A-ES-15-01 | PARTIAL | S/economic not_started正常分栏；scope丢失返回成功{}不合规。不销双经济前沿。 | LINUX-NORMAL, E3, R3-M4 |
| A-ES-15-02 | PARTIAL | 真实Begin/batch/Commit/Query/换代正负路径运行；损坏载荷或scope后的非法继续仍在。 | LINUX-KILL, LINUX-LIVE-WRITER, R3-M2, R3-M4 |
| A-RA-04-01 | PASS_S_ONLY | 旧TOP ID/first_known/输入/区间/撤回原因链完整保留；经济计划/可能已发/成交全义务不销。 | LINUX-NORMAL, LINUX-KILL, E10 |
| A-RA-10-01 | FAIL_S_SUBOBLIGATION | 健康历史及原始HTTP/CLI共同字段可等值，但指定根与必需见证/观察有反例。完整经济历史留后续。 | R3-H1, R3-M2, R3-M3, E11 |
| A-I-02-01 | FAIL_S_STORAGE_BOUNDARY | E/B/X未启动成功成立；S自身坏前件仍成功未封。不是经济分区全域失败或成功结论。 | LINUX-NORMAL, LINUX-KILL, R3-H1, R3-M4 |
| A-OB-004-01 | PASS_BOUNDED | 原始/窗口对象/结构revision分立；supersedes/replaces/旧版不复活已验证。 | LINUX-NORMAL, LINUX-LIVE-WRITER, E9 |
| A-OB-008-01 | FAIL | 正常重复/丢包路径有限成立；index_frontier变化未识别、必需键缺失成功、正常观察不等，不能全字段销项。 | R3-H1, R3-M2, R3-M3, NODE-CONSUMER, E7 |
| A-OB-009-01 | PASS_BOUNDED | 缺右邻无TOP，晚到更正不改旧TOP first_known；AsKnown正常cut稳定。 | LINUX-NORMAL, LINUX-KILL, E3, E10 |
| A-OB-014-01 | NOT_RUN_OUTSIDE_B | 本片没有经济计划/可能已发/已成部分成功实例，不能以经济空域标PASS；S无补造经济事实边界已读。 | E3 |
| A-OB-015-01 | PARTIAL | 请求epoch晚响应保护实际函数有效，根换session真实GUI有限支持；同generation仍可混不同index_frontier。 | NODE-CONSUMER, E7, R3-H1 |
| A-OB-016-01 | PARTIAL | 三故障恢复的根真实页面收敛+本轮数据对照成立；合法prefix续接仍失败。过期压力留C。 | LINUX-KILL, E7, E10, E11, R3-M3 |
| A-OB-017-01 | PASS_BOUNDED | 固定输入/received/规则/调度，恢复control同结构史；不从bars猜钱。 | LINUX-NORMAL, LINUX-KILL, E9 |
| A-OB-020-01 | EVIDENCE_RECORDED | 每项绑定read/run/reuse/NOT_RUN，原始自跑输出嵌入两报告之一；不声称读过根不可访问原件。 | LINUX-CORRUPTION, LINUX-KILL, NODE-CONSUMER, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13, E14, E15 |

## 9. 原始证据随报告保留与可复跑入口

临时目录不是交付档案。为订正前轮“/tmp可复核”夸大，本次将 **完整真实argv/exit/stdout/stderr/HTTP body、输入payload/hash、PID/signal/wait/恢复读数和测试脚本**直接作为 `review-r3.json.evidence_artifacts` 的gzip+base64文字附件；原始字节长度与SHA逐项列出。没有把作者sessions/context或工具会话入仓。

还原到新 `/tmp`（不覆盖库、不删除原库），例如：

```python
import json, base64, gzip, hashlib, pathlib, tempfile
r = json.loads(pathlib.Path('.chanlun/review-results/issue1371-tb01-b/review-r3.json').read_text())
out = pathlib.Path(tempfile.mkdtemp(prefix='issue1371-r3-evidence-'))
for name, a in r['evidence_artifacts'].items():
    b = gzip.decompress(base64.b64decode(a['data']))
    assert len(b) == a['bytes'] and hashlib.sha256(b).hexdigest() == a['sha256']
    (out / name).write_bytes(b)
print(out)
```

真实独评入口：在同源码hash、相同项目Python/libpython环境下，用一个**新的空/tmp目录**运行附件 `probe.py <目录>`（脚本仓路径绑定 `/home/agent/workspace`，换挂载点仅调整该路径）；然后 `extra.py <目录>`、`node jsprobe.cjs <目录>`。全路径/环境、实际每条命令本次退出码在附件及 JSON checks。探针exit0表示成功记录/断言其预期观测，**不等实现通过**；失败反例原始200响应与CLI0/1均保留。

## 10. NOT_RUN及剩余根验项

- 本轮**新增反例**的真实浏览器DOM/输送故障重验未在容器执行；Node已精确标证据级别。正常与三故障GUI已由根E3/E10提供，不再笼统要求根重跑全部正常平台面。
- 根宿主原件未挂载、未重新读取：使用内联事实、hash及可核源码复用，不声称阅读了绝对路径。远端票面即时核验因gh未认证未完成（exit4）；已签规范清单与payload已本地核hash。
- C1372全分页/保留/过期/慢消费、完整session管理、全调度、TB05全经济分区、RA04/OB014经济责任、RA10完整经济历史、ST044高级扩展、其余CC/G/FU与非退化市场域：明确在本片之外，**不算B缺陷也不因B局部通过而销项**。
- 没有全量历史重放、没有硬件断电保证、没有经济外效或生产启用。

**下一步**：根将R3-H1/M2/M3/M4交作者修；修后按具体反例复跑HTTP/Rust一致拒绝、必需Delta引用完整、合法prefix1→2→3同cut收敛，并用真实GUI确认错根不假绿/正确路径可推进。已通过的正常/三杀与原生平台证据按源码hash影响面有限复用，不要求为本轮FAIL重启整图或改未裁语义。

本次仅提交两个R3报告，不能合main、关闭#1371/父TB/#1323或授予交易权限。COMPLETE仅代表评审返回，**最终结论保持FAIL**。
