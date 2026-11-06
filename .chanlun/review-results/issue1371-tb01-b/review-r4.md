# #1371 TB-01-B：R4 独立复审

> 编号声明：本提交中的 #1371 仅指 GitHub Issue。
> 原独评 session 续审；不加载作者会话。仅新增 `review-r4.md/json`，R1/R2/R3 六份报告原字节保留。
> 固定 BASE：`2212eba31e0b7419907e622e79f02f7c2cc37706`；唯一候选 HEAD：`667ce698eacd8c8f50ec5846f24c779b492010a8`。
> 前轮源码：`65fb5e72bbee1de6656ba4b902dd506ae526195f`；前轮报告 tip：`257452c5fd02e0b28b385b7bca4c327b76ea6aa0`。

## 结论：FAIL（新的 1H、1M）

**R3-H1/M2/M3/M4 的四个具体反例均已修复并独立复跑。** 但不能因此宣布全部义务关闭：本轮发现输入 profile 绑定仍可矛盾却被前端称作“全字段一致”，S 还可按伪 profile_hash 发布已接纳待处理输入；健康的 AsKnown0 完整响应也仍被未来状态改写。

当前候选 build/check/clippy/test/fmt 全部 exit0，33 项 bin 测试通过；正常前缀、修订、三点真杀原库恢复、大整数及真实旧 writer 库兼容均已运行。**这些通过不覆盖下面新反例。** 根反馈只提供本候选 macOS 构建/33测试/413源码不变事实，明确没有提供尚在进行的产品/GUI通过结论，本评审不代背书。

## 1. 固定绑定与审查面

进入工位 HEAD 正确、dirty=0、分支 `codex/1323-tb01-b-1371`。已核完整 `BASE..HEAD` diff/log，也核 `257452c5..667ce698` 两提交：

- `381f5e1c670cf97b6a1491a3ec3c03c30df406d0`：三个生产文件与两个新测试文件；
- `667ce698`：仅正式作者 `implementation-r9.md/json`。没有把最后 docs commit 当完整差异。

完整 BASE..HEAD 共17文件，包括既有作者/独评/roster历史、4个生产文件及2个新测试文件。新 `s_session/tests/r9_contract.py` 是调用真实CLI/HTTP和记录指纹的入口；`r9_consumer.cjs` 运行实际HTML函数但模拟DOM，不是第二生产结构核。两文件已全文审阅；没有把它们的存在或作者测试结论升级为独评成功。

完整 diff SHA256：`2fe3b84d27402a6367bd9ef6fcf238dab5dfdd2968a857ef8a98b650358f5a1e`；本轮增量 SHA256：`e0ce8eaccb0cb89bbdd25e770c7873c57dfb7173a8d16105df9cce57cacbd70a`。完整commit清单在JSON `binding.log`。

| 实际文件 | SHA256 |
|---|---|
| `rust/src/bin/s_structure_session.rs` | `e26d69cb72832204168feb2c19b226f8c7c323e65c81012a57aff75d587a2f85` |
| `s_session/s_readonly_server.py` | `b705c0a7bcde38613f9e6acd7dc04cede440b56fa5aa40b892906d8baf6a59c7` |
| `s_session/browser/index.html` | `b4577c16a21ea6ce11f7e0a03d93eecb3042a400cced483662d6ecf73e9919a4` |
| `s_session/launch_s.sh` | `65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d` |
| `s_session/tests/r9_contract.py` | `39f0e9cee3d581a1981f882ed057f08f6c057d213dd4d2b6e393bc048f8faf11` |
| `s_session/tests/r9_consumer.cjs` | `c73c252360ad417e724fbc1ca9c9b42febf26f6418deb61836c63f38d6b93c78` |

**公共绑定（下文每个AC/ID均适用）**：本轮独立构建二进制 SHA=`b41f8e9630b3f564cdb05b52610f65cdfabbf2b063acb5e8e80aee43ab359792`（本机重新计算，不借作者hash）；Linux aarch64 ELF，rustc/cargo 1.97.1，项目Python 3.11.16/HTTP SQLite 3.53.1；Rust bundled SQLite 3.53.2、WAL/FULL。输入仍是具名 TestOnly 一对一精确退化OHLC、严格无包含、无同价、前两点定向的CC-006域，rule=`s2-axis-quantifiers`；目录 SHA=`938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`，profile文件 SHA=`2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc`，规范profile内容SHA=`0e9dfc9befd6878e0c844ee62c0f44576532ee8f13fe38c45f6bf467fe92d64e`。

指定六份 SPEC payload 与前轮归档hash逐份复核不变；清单 `payload/spec/evidence/REVIEW-INPUTS-S2-R1.json` 实际4450字节、SHA=`38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`。C02/C08/C09、共同身份/S/StructureRecord/Delta/原子组与恢复读集、effective/crosswalk/coverage/TEST-SEAMS继续有效。17条使用有效JSON pointer，historical/not_effective不参与判定，不重开批准门。

R1/R2/R3报告已读。原作者 `implementation-r9.md/json` 作为待验证材料读取；其中**14份具名附件逐一解码并核 bytes/SHA**，索引见JSON `author_report_review`。没有读取作者session/raw/prompt日志。执行原始命令仅使用这些正式报告授权附件或本评审自有探针。

## 2. Standards 与实际检查

遵 AGENTS.md 与 `.sandcastle/CODING_STANDARDS.md`。无生产修改、无新G/FU选择、无GitHub写/main/push/merge/harvest/队列/服务全局配置变更或经济外效。源码中核心调用仍是 `cmd_advance → ParseLayerIncr::append → merged_bars → classify_local_shape_sliding`；Python新校验器投影已存批次/历史索引，不重判结构，也不推进资金。

| 实际命令（cwd=rust） | exit | 实际结果 |
|---|---:|---|
| `cargo build --locked --features s_session --bin s_structure_session` | 0 | 当前二进制生成 |
| `cargo check --locked --features s_session --bin s_structure_session` | 0 | 成功 |
| `cargo clippy --locked --features s_session --bin s_structure_session` | 0 | 会话源文件0命中；库内既存warnings未隐藏 |
| `cargo test --locked --features s_session --bin s_structure_session --jobs 1` | 0 | **33 passed / 0 failed / 0 ignored** |
| `cargo fmt -- --check` | 0 | 成功 |

argv、完整stdout/stderr、真实cargo退出码均内嵌；不是shell管道末级成功。build/check/test库仍有既存57 warnings，clippy库warning类型/数量另见原日志，**不称零warning**。A同hash oracle限定复用，未跑全量历史重放。

## 3. R3 四项复核

| 原finding | 本轮结论及真实证据 |
|---|---|
| R3-H1 | **原触发已修，完整字段义务仍有R4-H1缺口**。原错历史根/错cut/内外frontier、seq、evidence在Rust和HTTP均StorageUnavailable/零写。新合同健康控制cursor2；单Snapshot.index字段拒绝；同步State各副本的有效错根仍在Delta/Snapshot.index_frontier比较处拒绝，不依赖缺键或404。 证据：R4-REGRESSION, R4-CONSUMER。 |
| R3-M2 | **本轮具名七集合/类型/成员/引用反例已修**。7集合各缺键、错误数组类型、null成员共21副本；另错witness端点/关系端点，5HTTP+7CLI均拒绝且逐次前后完整schema/表指纹相等。不宣称任意多副本一致损坏不可达。 证据：R4-MATRIX, R4-REGRESSION。 |
| R3-M3 | **已修**。正常单点→两点→第三点→更正→追加七代成功，obs-92a89…的首batch/detail保持raw_events="1"，完整Delta/Snapshot六集合可续接；AsKnown1稳定。 证据：R4-PREFIX, R4-CONSUMER。 |
| R3-M4 | **已修（按可支持的原反例表述）**。缺scope、{}、null均统一503/exit1 StorageUnavailable；每CLI前后schema/表指纹保留。原R3只采信损坏scope之后被200/0接纳，不采信缺末态原始meta的“推进后scope仍缺”断言。 证据：R4-MATRIX, R4-REGRESSION。 |

### 必须保留的证据限缩

1. **旧schema不是index特异反例**：R3原始wrong-root 200响应没有本轮新增publication字段。它被拒，只能证明旧schema被拒。本轮没有使用该失败来判index修复：先用新合同健康控制得到cursor2，再只改Snapshot.index；另把State.cut/Snapshot/Catalog及对应证据中的index/batch这一个语义字段所有副本一致改为**同库实际历史batch**。前者报 `Catalog/Snapshot.index_frontier 不一致`，后者报 `Delta/Snapshot.index_frontier 不一致`，游标均不提交。原对象/见证/键集/类型保留，变换前后全文与差异路径已留。
2. **404不构成字段校验**：当前消费者请求 `/api/state?as_of=N`。本轮Node驱动的都是真实S只读服务，支持该路由；数据服务实际已到gen7，逐次按原Delta末代取得旧cut1..7，所有相关HTTP为200。拒绝发生在字段对拍，不是旧回放服务缺路由返回404。
3. **R3 scope末态**：仅继承R3损坏scope后被200/0接纳，不继承未保存末态原始meta的“推进后scope仍缺”断言。本轮缺scope拒绝有逐次前后指纹。
4. **live布尔不是完整原始前后证据**：作者R9与前轮probe的live零写布尔不等44副本矩阵同级留痕。本轮两活旧writer重新执行并保存独立before/after schema及十表columns/rows/hash。

## 4. Spec：本轮新增阻塞

### R4-H1（H）：profile绑定遗漏，使同cut假绿和错误版本下的新发布仍可发生

**代码位置**：Rust `s_structure_session.rs:370-441,580-588,1827-1829,3248-3256`；Python `s_readonly_server.py:166-176,323-353,685-696`；HTML `index.html:305-306,346-370,389-397`。

真实生成旧TOP→更正RISING的gen2库，SQLite只读backup到独立副本；仅做一次单行故障注入：

```sql
UPDATE meta SET value='wrong-profile-hash' WHERE key='profile_hash';
```

**观察到的原始结果**：

- 五个HTTP端点（state/snapshot/catalog/delta0/state-asof1）全部 **200**。同一Snapshot内：
  - 顶层 `profile_hash = "wrong-profile-hash"`；
  - `catalog_evidence.profile_hash = "0e9dfc9b…2d64e"`，仍是正确已封存profile。
- Rust snapshot/watch/catalog/query均exit0；无新输入时advance幂等exit0、recover exit0。新accept用原正确profile得到IdentityConflict，**这不等统一StorageUnavailable或全部写入口已封闭**。
- 第二个独立副本先正式accept第四点10800（待处理），再只做上述单meta变异。随后正式advance **exit0，发布gen3**；新的顶层profile_hash和目录证据一起带上伪hash；HTTP state仍200。错误持久前件没有阻止实际后续发布。
- 当前HTML函数的新合同健康控制成功后，对**只改顶层Snapshot.profile_hash的TestOnly单字段变换**仍提交cursor2、显示ok-box“全字段一致”；改用真实坏profile库的实际HTTP（含真正as_of2路由）重复，也提交cursor2并假绿。这不是旧schema，也不是404。

**原因**：根校验只比较 evidence.profile 与 batch.profile，没有把session里已固定的meta.profile绑定纳入一致性门。Snapshot顶层profile仍直接取当前meta。前端 `publicationKeys` 新增10字段，却遗漏实际Snapshot已有的profile_id/hash；`validateState/comparePublication`不比较顶层profile与同cut目录证据。所谓全部发布字段比较仍不完整。

**后果/合同**：输入/profile版本是已签共同身份及重算来源，不是可删的展示装饰。它发生持久矛盾后旧历史被错误归因，S还可将待处理输入发布在伪版本下。违反AC2/3/6/7、本片OB008/RA10的版本与同cut完整读出。

**最小修复方向**：验证已固定profile_id/hash与当前和历史封存批次/输入绑定一致；坏绑定按StorageUnavailable停止相应读写。Snapshot按指定cut取对应profile或给出明确未绑定态；前端将实际已列profile字段与同cut证据一起比较，不按较小白名单定义“所有字段”。不修改旧原始记录/hash来凑一致。

证据：`supplement-results.json#/matrix/profile_hash_bad`、`profile-pending-results.json`、`node-results.json#/cases/single_snapshot_profile_hash` 与 `#/cases/real_profile_corruption`；原body、真实argv/exit和变换diff均随报告保存。

### R4-M2（M）：合法初始AsKnown0仍混入未来profile和目录状态

**代码位置**：Rust `:2794-2859,2874-2876,3094-3120,3255-3258`；Python `:436-500,685-696`。

**无SQL注入，正常用户输入即可触发**：init后启动实际只读服务，先保存 `/api/state?as_of=0`、CLI snapshot/catalog `--as-of 0`；随后正常单点→两点→第三点→更正→追加与两次大revision，重读同一as_of0。

| 同一gen0/cut-0返回字段 | 初始值 | 后续重读 |
|---|---|---|
| Snapshot.profile_id | `""` | `"testonly_tick_1_1_ohlc"` |
| Snapshot.profile_hash | `""` | 后获知的`0e9dfc9b…2d64e` |
| CC-006 implementation_status | `not_implemented` | `implemented` |
| Catalog.counts.implemented | `"0"` | `"1"` |
| Catalog.counts.not_implemented | `"116"` | `"115"` |

HTTP前后 **5字段变化**；CLI Snapshot/Profile和Catalog也各自不等，均为exit0/HTTP200。gen0本轮被明确声明合法，不能把它当未支持域。gen1后的首观察稳定、没有提前TOP，不等于gen0完整AsKnown稳定。

**原因/后果**：历史元组部分取持久Delta，但profile与implementation/counts仍取当前meta/catalog。后来的接纳和运行字段被套在旧cut0的AsKnown载荷内；这是健康前缀的自然可达问题，不是任意存储恶意改写。

**最小修复方向**：让gen0也有稳定历史profile/目录投影；真正属于当前程序能力/会话上下文的内容若不作历史，应显式另名另层，不混进旧AsKnown payload。补 init→首次accept/advance 后同as_of0完整响应等值测试，不只比较对象集合。

证据：`supplement-results.json#/gen0_history`（HTTP/CLI前后完整原始返回）及 `#/api_cuts/states/0`。

## 5. 已独立执行的正负边界

### 5.1 正常前缀、修订、指定cut消费者

- `R4-REGRESSION` 从正式launcher prefix2起步，后续TOP→更正RISING撤旧TOP→追加TOP，并推进两次大revision；正常六集合重建一致。
- `R4-PREFIX` 从init/gen0/单点10000→两点10000/11000→第三点10500→更正11500→追加10800→两次大revision；每步真实CLI和HTTP。观察 `obs-92a89ddecde02693` 始终保留首batch `batch-199a90a2…48705f` 与 `detail.raw_events="1"`，AsKnown1不变。R3-M3原2字段差异消失。
- `R4-CONSUMER` 实际HTML函数逐次应用七代，底层真实S库已经位于gen7，依然从支持as_of的实际HTTP取得每个指定旧cut；最终cursor7。健康新合同单独cursor2成功；单index/各State包装一致错index/与Delta不一致的目录证据分别被特异拒绝，profile遗漏仍假绿。
- Gap按返回的cut7请求真实as_of7并重建；读取失败后已提交state/maps/binding不变。读取失败由Node响应读取promise受控抛错产生，**不是实际TCP丢包证据**，不把mockDOM当真实GUI。

### 5.2 七必需集合、端点和scope

本轮独立补充矩阵25个拒绝副本：七集合各缺键/错误数组类型/null成员（21），两个见证/关系端点错误（2），scope为空对象/null（2）。每副本 **5 HTTP + 7 CLI**（snapshot/watch/catalog/query/accept/advance/recover）均503或exit1/StorageUnavailable，且每CLI前后完整schema和十表指纹相等。另原R3的10副本（错根/cut/删当前或历史batch/坏gen/内外frontier/seq/evidence/缺witness/缺scope）也重跑统一拒绝。

`required_array/_validate_delta_shape`、batch/独立历史索引对拍确实沿正式读写路径运行，不是只存在测试类型。我们也记录了额外profile坏meta与三个统计证据副本同时一致改错的诊断臂：它们不是25个已拒绝副本的分母，脚本exit0不表示所有诊断臂产品通过。后者要求同时更改内/外/当前目录3副本，不在本轮单点缺陷数中升级为任意协调篡改防护义务；原读数保留，不据此声称任何损坏都被拒。

### 5.3 三点真实S SIGKILL及原库恢复

全部从已发布旧TOP开始接纳新revision，然后对准确S advance PID发SIGKILL，先读`/proc/PID/cmdline`核可执行文件/argv/DB，再wait确认（非return/mock/SIGSTOP/只杀Python）：

| 点 | PID | signal / wait | kill后generation / batches | Query.latest_published | 恢复 |
|---|---:|---|---|---|---|
| after_begin | 2069 | SIGKILL / -9 | 1 / 1 | false | 原库recover epoch2；完整Snapshot/AsKnown1/Watch等control；HTTP Snapshot一致 |
| after_batch | 2088 | SIGKILL / -9 | 1 / 2 | false | 原库recover epoch2；完整Snapshot/AsKnown1/Watch等control；HTTP Snapshot一致 |
| after_commit | 2107 | SIGKILL / -9 | 2 / 2 | true | 原库recover epoch2；完整Snapshot/AsKnown1/Watch等control；HTTP Snapshot一致 |

after_begin/batch时current Snapshot完整等原旧TOP，不混未发布修订；after_batch另一SQLite连接已读完整独立持久batch且hash/长度正确，但根仍旧cut。Commit后回执stdout为空，原身份Query返回权威已发布，原argvadvance幂等不多cut。

control与三臂固定原输入/received/业务ID/调度边界，**Snapshot/AsKnown1/Watch全字段**比较不删first_known/receipt/对象/关系/见证。Begin token与writer恢复史是本来不同的恢复元数据，另列原始读数，不声称所有DB字节绝对等同。

两条活旧writer（PID2644/2652）不发送任何终止/停止信号；正式recover epoch2后release，旧writer自身exit1，已保存before/after schema+十表指纹完全一致；新owner成功推进gen2。不是用旧“零写=true”布尔代替前后读数。

### 5.4 精确整数与真实旧发布库

- 当前新候选原生CLI/HTTP实际源坐标`9007199254741000..1002`、revision=`9007199254740993`无损文本往返；前缀轨迹连续`9007199254740993→9007199254740994`的supersedes也正确。
- 使用已授权正式作者附件的最小Cargo探针（源码来自git `6eeed76e`，不是作者会话），真实旧writer源码 SHA=`addb492ac2c1888904f2ab2d9789ae9a7ebbfc8e0037e4d822f73b67be333be8`。本轮原生 `cargo build --locked --offline ... --bin s1371_legacy_writer` exit0，旧writer真正接纳/发布3代大revision；当前Rust六读+五HTTP成功读取并转换旧supersedes/evidence整数，前后十表/schema完全不变。
- 这复现的是旧writer与当前项目同hash严格库/锁定依赖，不是旧OS或整套旧crate环境复现；该限制保留。未修生产Cargo、未用SQL手造旧整数库，未新造运行时。

## 6. R1/R2及此前反馈处置

六历史报告原件不改；历史过宽结论继续撤回：

| 原finding | 当前处置 |
|---|---|
| L-1 | 已修；原低估/launcher不经过的说法继续撤回：Commit回执前真SIGKILL后原argv advance幂等，Query原业务identity权威状态，无新cut；accept重放不是只读Query。 |
| L-2 | 已修：负查询/重复参数400，未发布cut拒绝；合法gen0能返回但完整历史稳定另见R4-M2。 |
| L-NEW-1 | 既有精确整数wire问题已修；原生产不可达论撤回：本轮真正由旧6eeed writer源码发布大revision整数库，再用当前Rust/HTTP读取，旧supersedes/evidence整数转精确文本，十表/schema不改。只复现旧writer+当前项目严格库，不声称旧OS/全旧crate环境。 |

原根/辅助问题逐条重核，重复项共用证据但不吞义务：

| 原问题ID | 当前证据/限制 |
|---|---|
| ROOT-B-R1-01 | 已发布健康cut的过滤、首发布字段与未发布修订边界由本轮三杀/前缀复核；历史profile/初始cut完整性仍有R4-H1/M2反例。 |
| ROOT-B-R1-02 | 当前实际HTML函数有新contract健康控制、指定as_of真实HTTP、完整index元组矛盾拒绝及缓存不污染读失败验证；实际GUI/真实网络丢包在本轮未验，不借旧HTML结果升级。profile字段遗漏见R4-H1。 |
| ROOT-B-R1-03 | 当前实际HTML函数有新contract健康控制、指定as_of真实HTTP、完整index元组矛盾拒绝及缓存不污染读失败验证；实际GUI/真实网络丢包在本轮未验，不借旧HTML结果升级。profile字段遗漏见R4-H1。 |
| ROOT-B-R1-04 | 源码仍显式load(current)+有界AsKnown输入；当前真正DOM操作NOT_RUN，不能用Node称GUI。 |
| ROOT-B-R1-05 | 原具名根/七载荷/坏scope反例在本轮CLI+HTTP共同拒绝，有前后schema/表指纹；不外推任何损坏组合。profile根约束仍缺，见R4-H1。 |
| ROOT-B-R1-06 | 原具名根/七载荷/坏scope反例在本轮CLI+HTTP共同拒绝，有前后schema/表指纹；不外推任何损坏组合。profile根约束仍缺，见R4-H1。 |
| native-delivery-query | 本轮从旧TOP更正三点真SIGKILL，原identity Query与同argvadvance幂等实跑；两活旧writer释放后exit1，新owner推进，有完整表指纹。 |
| native-current-unpublished | 已发布健康cut的过滤、首发布字段与未发布修订边界由本轮三杀/前缀复核；历史profile/初始cut完整性仍有R4-H1/M2反例。 |
| native-future-AsKnown | 已发布健康cut的过滤、首发布字段与未发布修订边界由本轮三杀/前缀复核；历史profile/初始cut完整性仍有R4-H1/M2反例。 |
| PY-DRAFT-H01 | 原具名根/七载荷/坏scope反例在本轮CLI+HTTP共同拒绝，有前后schema/表指纹；不外推任何损坏组合。profile根约束仍缺，见R4-H1。 |
| PY-DRAFT-H02 | 原具名根/七载荷/坏scope反例在本轮CLI+HTTP共同拒绝，有前后schema/表指纹；不外推任何损坏组合。profile根约束仍缺，见R4-H1。 |
| PY-DRAFT-H03 | 当前实际HTML函数有新contract健康控制、指定as_of真实HTTP、完整index元组矛盾拒绝及缓存不污染读失败验证；实际GUI/真实网络丢包在本轮未验，不借旧HTML结果升级。profile字段遗漏见R4-H1。 |
| PY-DRAFT-H04 | 原具名根/七载荷/坏scope反例在本轮CLI+HTTP共同拒绝，有前后schema/表指纹；不外推任何损坏组合。profile根约束仍缺，见R4-H1。 |
| PY-DRAFT-H05 | 本轮真实旧writer发布大integer库→新reader精确文本兼容已运行，不使用“生产不可达”解释。 |
| PY-DRAFT-H06 | 当前实际HTML函数有新contract健康控制、指定as_of真实HTTP、完整index元组矛盾拒绝及缓存不污染读失败验证；实际GUI/真实网络丢包在本轮未验，不借旧HTML结果升级。profile字段遗漏见R4-H1。 |
| PY-DRAFT-M01 | 本轮实际负/重复查询400，URL编码合法成功；根反馈没有授权省略格式门。 |
| PY-DRAFT-M02 | 源码仍显式load(current)+有界AsKnown输入；当前真正DOM操作NOT_RUN，不能用Node称GUI。 |
| PY-8C-H07 | 原具名根/七载荷/坏scope反例在本轮CLI+HTTP共同拒绝，有前后schema/表指纹；不外推任何损坏组合。profile根约束仍缺，见R4-H1。 |
| R2-all-HM-fixed-assertion | 继续撤回R1/R2过宽“全部H/M已修复”“六集合即所有字段”等结论；本轮按具体原反例及新发现判定。 |

原R1错误AC顺序不继续沿用：**AC3=Delta/原子浏览器，AC4=三杀，AC5=DeliveryUnknown/epoch**。R2“生产不会产生整数”和“BigInt普遍兜底JSON.parse”已被旧真实发布库反证，不再援引。

## 7. 复用、原生/GUI证据层次与成本

- 规范/A基线 `ROOT-ACCEPTANCE.md`、`ACCEPTANCE-INDEX.json`、`review-incremental.md`与归档定位已核。local_shape/inclusion/已签目录/profile/四分支原始输入与A同hash；只复用A的严格四分支语义oracle，B恢复/新校验器不借A过关。不会重报A旧f635已修缺陷。
- 本轮逐函数比较PRIOR65fb到667ce：open_db/record_connection_pragmas/accept/recover/pause/verify_begin_owner/persist_batch/cancel/compute_diff/build_object等同hash；**verify_reachable_root与cmd_advance改了**。即使accept/recover函数文本没变，调用根前件变了，因此三杀/活writer/新读路径已重跑，不以函数同名代替行为。
- 根旧E13同writer连接wal/FULL2/fullfsync1，仅可在上述open_db/sidecar逐字不变的范围复用配置。根本轮明确提供当前macOS arm64 fmt/test/clippy/build、33测试及413源码前后不变，但没有提供当前产品/GUI完成事实。**新版HTML hash改变，不能把旧R8 GUI当当前R4 GUI通过。**也不把连接配置读数扩成fcntl调用或硬件断电证明。
- 根验证每次遍历可达代链、各代历史视图/完整批次，成本随历史增长。本轮限小轨迹/具名副本，无C1372长历史/慢消费性能承诺。HTTP单请求事务结束，不跨请求持有读事务；不以全历史重放填证据。

## 8. 原票8AC逐项

每行共同绑定第1节精确源码、binary、输入/规则/目录；证据键在JSON `checks`，其artifact包含实际argv/exit及完整原始输出。PASS_BOUNDED只表示明示子域，不是整票通过。

| AC | 结论 | 已证与缺口 | 证据 |
|---|---|---|---|
| AC1 | PASS_BOUNDED | 正式launcher→原始输入→同一Rust核；TOP→撤旧RISING→追加TOP成功，Begin后解释/独立batch前根不发布。 真实页面最高缝整体成功仍由AC3/6/8的GUI待验约束。 | R4-REGRESSION, R4-CRASH, A-REUSE |
| AC2 | PARTIAL | raw revision/接纳序/源坐标/对象身份分立；重放/冲突与旧TOP首获知链有效；大revision/source坐标与旧库整数输出无损。 profile输入绑定作为实际已列字段未封闭，R4-H1阻塞完整版本证明。 | R4-PREFIX, R4-LEGACY, R4-BIGINT, R4-H1 |
| AC3 | FAIL | 原错index、scope/载荷反例已拒；新合同支持as_of=N的健康对拍和单index矛盾特异拒绝。 遗漏实际Snapshot.profile字段仍假绿；新版HTML真实GUI未验，不能称完整同cut成功。 | R4-MATRIX, R4-CONSUMER, R4-H1 |
| AC4 | PASS_BOUNDED_DATA | 3个真实S进程从旧TOP修订，SIGKILL/wait=-9，原库恢复Snapshot/AsKnown1/Watch完整等control；旧owner合法拒绝、新owner继续。 不是硬件断电保证；本轮恢复前后真实GUI未验。 | R4-CRASH, R4-LIVE |
| AC5 | PASS_BOUNDED | Commit前/后中断状态明确；原业务identity Query只读；原argv无新输入advance幂等；活旧writer跨epoch自身exit1且完整前后指纹不变。 NoRecord不铸造新经济资格；未运行经济责任分派。 | R4-CRASH, R4-LIVE |
| AC6 | FAIL | AsKnown1与原观察/旧TOP稳定，缺右邻无TOP，原来单点→两点不等已修。 健康AsKnown0仍5字段漂移；旧cut profile可被当前坏meta改写；GUI未验。 | R4-PREFIX, R4-M2, R4-H1 |
| AC7 | FAIL | E/B/X未启动时S独立成功；25集合/scope副本和原10坏根副本统一拒绝。 单meta.profile_hash坏仍200/0，有待处理输入仍发布新cut；不是任意存储故障已隔离。 | R4-MATRIX, R4-PROFILE-PENDING, R4-H1 |
| AC8 | EVIDENCE_RECORDED_NOT_ACCEPTED | 原始inputs/hash/argv/exit/PID/signal/wait/同DB/API/前后指纹已嵌入报告；作者14附件全解码核hash后按层引用。 新实际GUI/根产品验收仍未提供完成事实；失败发现未销项。 | R4-ARTIFACTS, R4-CRASH, R4-CONSUMER |

## 9. 17 来源子义务逐项

逐项signed_pointer和完整required_result在JSON `items`中保留；只承接本片结构子义务，不销完整ID。

| ID | 结论 | 实测/缺口/未验 | 证据 |
|---|---|---|---|
| A-CC-006-03 | PASS_BOUNDED_DATA | 修订三杀与control完整结构/身份/见证等值；同严格核复用，不扩任意市场投影。 | R4-CRASH, A-REUSE |
| A-CC-006-04 | PARTIAL_GUI_PENDING | 更正撤旧/新TOP四比较与三根输入见证真实可查；当前HTML函数成功不等真实页面渲染。 | R4-PREFIX, R4-CONSUMER |
| A-ST-044-02 | PARTIAL | 七载荷必需性/端点一致原反例已拒、prefix观察修复；profile声明矛盾仍破坏完整发布字段等值。高级扩展留后续。 | R4-MATRIX, R4-CONSUMER, R4-H1 |
| A-ST-044-03 | PARTIAL_GUI_PENDING | 原游标/指定cut重建与AsKnown1数据正常；完整GUI、C压力矩阵未销。 | R4-CRASH, R4-PREFIX, R4-CONSUMER |
| A-ES-15-01 | PASS_S_ONLY | scope缺失/{} /null正确失败，S CompleteCut与economic not_started分立；不是经济ack。 | R4-MATRIX, R4-PREFIX |
| A-ES-15-02 | PARTIAL | Begin/batch/Commit/Query/epoch合法迁移与拒绝真实；profile坏绑定后仍推进的非法继续见H1。 | R4-CRASH, R4-LIVE, R4-H1 |
| A-RA-04-01 | PASS_S_ONLY | 旧TOP原ID/first_known/区间/输入/见证/撤回史保留；经济计划/已发/已成交责任不销项。 | R4-CRASH, R4-PREFIX |
| A-RA-10-01 | FAIL_S_SUBOBLIGATION | 健康已发布cut集合/新publication tuple等值不代表所有字段；profile矛盾和AsKnown0漂移仍失败；完整经济历史留后续。 | R4-H1, R4-M2, R4-CONSUMER |
| A-I-02-01 | FAIL_S_STORAGE_BOUNDARY | 经济未启动下S成功、scope/已列坏根拒绝有效；profile坏绑定未停写。不声称经济故障分区全域通过。 | R4-MATRIX, R4-H1 |
| A-OB-004-01 | PARTIAL | 源/对象revision、supersedes/replaces与旧版不复活既有边界有效；profile版本绑定遗漏仍存。 | R4-LEGACY, R4-BIGINT, R4-H1 |
| A-OB-008-01 | FAIL | 真实HTML函数在新合同健康/单index/完整错index对照有效，profile单字段矛盾却假绿；不以schema拒绝冒称原反例特异关闭。 | R4-CONSUMER, R4-H1 |
| A-OB-009-01 | PARTIAL | 缺右邻不提前TOP，旧first_known/首观察不覆盖；初始AsKnown0的profile/目录字段仍回填。 | R4-PREFIX, R4-M2 |
| A-OB-014-01 | NOT_RUN_OUTSIDE_B | S未补造经济应用；没有经济计划/可能已发/已成部分实例，不作空域PASS。 | R4-CRASH |
| A-OB-015-01 | PARTIAL_GUI_PENDING | 新消费者绑定session/gen/cut/index及pageEpoch；profile漏字段仍能静默进入已提交State；真实GUI未验。 | R4-CONSUMER, R4-H1 |
| A-OB-016-01 | PARTIAL_GUI_PENDING | 原库三故障恢复API收敛、Gap按指定cut真实HTTP返回；当前新版页面恢复/断连尚未实跑。 | R4-CRASH, R4-CONSUMER |
| A-OB-017-01 | PARTIAL | 固定输入/时钟三杀结构史一致，不根据bars制造经济恢复；初始历史完整投影仍有M2。 | R4-CRASH, R4-PREFIX, R4-M2 |
| A-OB-020-01 | EVIDENCE_RECORDED | 每项共用冻结源码/输入/构建/hash并指向实际argv/exit/原输出；原live/scope末态证据限缩明确，GUI不冒称。 | R4-ARTIFACTS |

## 10. 原始证据随两报告保存

本轮实际脚本、inputs/payload/hash、CLI argv/exit/stdout/stderr、HTTP原始body、PID/signal/wait/恢复结果、每CLI前后schema/十表指纹均嵌入 `review-r4.json.evidence_artifacts`（xz+base64，逐bytes/SHA）；不依赖 `/tmp` 永久存在，不把作者sessions或运行日志上下文入仓。作者14份正式附件已在其implementation JSON中，另记录本轮解码验证索引，不复制私有会话。

只解码到新的/tmp目录：

```python
import base64, hashlib, json, lzma, pathlib, tempfile
r = json.loads(pathlib.Path('.chanlun/review-results/issue1371-tb01-b/review-r4.json').read_text())
out = pathlib.Path(tempfile.mkdtemp(prefix='issue1371-r4-evidence-'))
for name, a in r['evidence_artifacts'].items():
    assert pathlib.Path(name).name == name and a['encoding'] == 'xz+base64'
    b = lzma.decompress(base64.b64decode(a['data']))
    assert len(b) == a['bytes'] and hashlib.sha256(b).hexdigest() == a['sha256']
    (out / name).write_bytes(b)
print(out)
```

本轮真实入口为项目Python依次执行附件 `probe.py <新空目录>`、`supplement.py <目录>`、`node-driver.py <目录>`、`profile-pending.py <目录>`及`big-coordinate.py <目录>`；当前固定代码和项目libpython环境下运行，脚本仓路径是`/home/agent/workspace`。`node-driver`实际调用`node-review.cjs`并启动自有真实S只读服务；不足的mock/GUI层明确，不用路径404当通过。legacy最小Cargo的精确命令另在checks与附件保留。

探针exit0表示其观测与控制脚本运行完成，**不是反例中的产品也通过**。本报告已把产品200/503与0/1分别列出。

## 11. NOT_RUN与剩余根工作

- 当前新HTML的真实浏览器DOM/网络失败/恢复前后GUI：**NOT_RUN**，容器无浏览器；根此轮说仍在验，不提供未完成PASS。Node actual functions/真实HTTP不能冒称GUI。
- 当前完整macOS产品执行/现场同writer连接再验：本容器未跑。根当前build/test已给事实，旧同hash连接配置仅有限复用；不笼统否定全部平台前件，也不扩成硬件耐久。
- 远端票面即时只读核验：gh exit4未认证，未改认证。固定用户正文与本地已签规范为依据；规范清单和六payload实际hash已核，不重开批准门。
- C1372全部分页/保留/过期/慢消费、长历史/全调度、RA04/OB014经济责任、RA10完整经济史、I02/TB05全域经济分区、ST044高级扩展、其他CC/G/FU/非退化市场仍在后续；E/B/X未启动成功不等经济故障隔离。它们不算B缺陷，也不因B局部通过而销项。
- 无全量cargo/history replay、无真实订单/资金/生产启用或main合入授权。

**下一步**：根将R4-H1/M2交作者修，沿“单meta profile损坏→原cut查询/已有待处理advance→当前HTML”和“init/as_of0→首次接纳发布后同as_of0”两条具体轨迹回归；在新合同健康控制同时成功的前提下，补真实GUI完整字段/历史证明。原R3四反例、33测试、正常三杀/大整数兼容可按实际源码影响面有限复用，不无差别重开整图或全量重放。

结论保持 **FAIL**。仅提交R4两报告；COMPLETE只表示评审返回，不批准B交付/main/关闭#1371/父TB/#1323。
