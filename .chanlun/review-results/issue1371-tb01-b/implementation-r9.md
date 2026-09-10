# #1371 TB-01-B：R9 实施报告与 R9c 交棒

> 编号声明：本报告中的 #1371、#1340、#1323 均指 GitHub Issue。
> 名分：原作者实施报告，不是独立评审或验收批准。原 R3 结论 **FAIL（1H/3M）** 保留不改。
> 已批准范围仍为 SPEC #1340 的 TB-01-B；父图 #1323 六 Destination 不因本报告完成。

## 1. 当前状态与冻结绑定

代码提交：`381f5e1c670cf97b6a1491a3ec3c03c30df406d0`；父提交：`257452c5fd02e0b28b385b7bca4c327b76ea6aa0`。
唯一分支：`codex/1323-tb01-b-1371`。本次收尾仅提交 `implementation-r9.md/json`，保留代码提交为祖先，不改写历史。

四项缺陷已实施修复并取得下文有限实测结果，**待根真实 GUI、受影响原生平台验证及原独评接回复评**。没有 B、父 TB 或整图验收通过；没有 push、merge、main/GH 写入、harvest 或经济外效。

R9b 在交棒前触及 30 分钟外层时限。根告知：SDK closed、容器消失后，终止残余宿主 wrapper，外层退出 **143**。该事实不等于作者正常完成。R9c 接回时唯一未提交 JSON 为 531469 字节、SHA256 `dd53a945e532d0ec2e7e30976d16870ade8b50f47c989563bef40a6cc85ae05e`；MD 尚不存在。本次从该自包含报告继续，没有从头修复或重跑同 hash 的测试。

### 共同证据绑定

下文全部 AC/细项共用本节代码、输入、规则、目录、构建与平台绑定；每项另列实际证据键与未验域。

- 规范清单 SHA256：`38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`。六份指定 payload 的逐文件 SHA 见 JSON `binding.spec_files_verified`，已核与原签归档相符；未改教义、ADR、已签 SPEC/目录、formal 或 CI。
- 目录 SHA256：`938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`，116 项完整保留。
- TestOnly profile 文件 SHA256：`2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc`；规则：`s2-axis-quantifiers`。受测域不超出逐笔一对一退化 OHLC、严格无包含、前两点定向、无同价竞争的 CC-006/local_shape。
- Linux aarch64；项目 Python 3.11.16 / SQLite 3.53.1。Rust 使用项目 Cargo.lock 的 bundled SQLite 构建；本轮独立 `pragma` 连接实际读数为 `wal`、`synchronous=2`、Linux `fullfsync=null`，**不是实际 writer 同连接/macOS fullfsync 证据**。
- 本轮二进制 SHA256：`b41f8e9630b3f564cdb05b52610f65cdfabbf2b063acb5e8e80aee43ab359792`；不将它与旧 writer 或根 macOS 二进制混算。
- 六份 R1/R2/R3 评审原件、旧作者两报告、已登记 roster 保持；R9c 重新核源码 SHA 与评审文件字节、Git blob/mode。没有载入独评私有会话或读取未挂载的宿主原件。

| 文件 | 最终 SHA256 |
|---|---|
| `rust/src/bin/s_structure_session.rs` | `e26d69cb72832204168feb2c19b226f8c7c323e65c81012a57aff75d587a2f85` |
| `s_session/s_readonly_server.py` | `b705c0a7bcde38613f9e6acd7dc04cede440b56fa5aa40b892906d8baf6a59c7` |
| `s_session/browser/index.html` | `b4577c16a21ea6ce11f7e0a03d93eecb3042a400cced483662d6ecf73e9919a4` |
| `s_session/launch_s.sh` | `65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d` |
| `s_session/tests/r9_contract.py` | `39f0e9cee3d581a1981f882ed057f08f6c057d213dd4d2b6e393bc048f8faf11` |
| `s_session/tests/r9_consumer.cjs` | `c73c252360ad417e724fbc1ca9c9b42febf26f6418deb61836c63f38d6b93c78` |

## 2. 四项修复与证据边界

### R3-H1：同一根的 HTTP/Rust 验证与前端完整元组

- Rust/Python 对全部正式结构入口共用同事务根验证：必需 meta、完整已发布代链、11 列对应、当前根与末代 Delta、批次内容寻址/长度/封存坐标、当前目录证据一致性。合法 gen0 和空前沿 `-1` 保持成功；完整不可达 batch 不冒充已发布历史。
- `Snapshot`、`Catalog` 只读输出共同 publication tuple；没有修改既有数据库 BLOB、业务身份、cut 或 first_known。HTTP State、Snapshot、Catalog、Delta 及历史路径不再各跑不同宽严门。
- 实际 HTML 对 Delta 内外头、State/Catalog/Snapshot 元组和六集合完整内容验证。续接后按 **应用末代的 `?as_of=N`** 取快照，不取后续写者推进的新 cut；Gap 按 `rebuild_cut` 重建。流应用后明确显示该代 AsKnown，当前按钮仍单独查询 RecomputedWithRevision。
- R3 原始错根 HTTP payload 已送入当前 HTML 实际函数：拒绝且游标不推进。另测三个 State 包装彼此一致、但与 Delta 不同根的反例，仍拒绝；正常七 cut 可完整提交。均为 **Node 实际函数/真实 payload，非 GUI**。

| 语义字段 | Delta → 指定 cut Snapshot | Catalog / State 对应 |
|---|---|---|
| session/generation/catalog_revision | 同名字段逐值等 | Catalog 与 Snapshot 同名；State.cut 同名 |
| cut | `next_cut` → `structure_cut`；`base_cut` 续接前 cut | Catalog、State.cut 的 `structure_cut` |
| index_frontier | 同名 batch 根精确相等 | Catalog、State.cut 同名；目录证据 `batch_id` 同根 |
| input_frontier / seq_range | 同名字段；`seq_range.to` 等于前沿，from 续接前沿+1 | Catalog 同名，禁止 absent 默认空 |
| scope | `catalog_evidence.scope` → Snapshot.scope | Catalog.scope、目录证据.scope 同声明 |
| 目录运行状态/证据 | 完整内层 `catalog_run_status/catalog_evidence` → Snapshot 同名 | Catalog publication 字段与 CC-006 条目 run_status/evidence 深等 |
| 对象/撤回/见证/关系/观察/源历史 | 七类变化应用后得到六个累计集合，与 Snapshot 逐字段比较 | 不以计数或 branch 代替完整内容 |

Rust Watch 外层不重复目录状态/证据两项，HTTP 外层重复；消费者明确以完整内层对拍 Snapshot，外层存在时还必须与内层相等。没有删键或将布尔当整数；已知精确整数字段必须是规范十进制文本，不接受 JSON.parse 后可能舍入的 number。

### R3-M2：七类载荷与持久引用闭包

七个必需集合为 `upserts / withdrawals / replaces / witnesses / relations / observations / raw_history_added`；缺键、错误数组类型、坏成员、缺见证成员及悬空关系均不能默认空后继续。

本次完整结果锚定可验不可变 batch，与独立存储的同代历史视图交叉核对：对象内容及首发布字段、raw 原事件、见证、分类关系、替代边、源 supersedes 链、观察首版及累计历史集合均作全内容比较。观察原批次必须可达。此校验是读写前件，不从 bars 重算结构、不补造历史事实、不改旧 BLOB/hash。

已有历史数值表示的兼容性由旧 writer 原生发布库实跑承担，见第 4 节。目录计数允许已发布非负整数或规范十进制文本，拒绝浮点/布尔/越域；最终输出投影保留 bool/null/float 的原类型。

### R3-M3：同 observation identity 沿用首版

`INSERT OR IGNORE` 留存首版时，Delta 也读取并使用该 identity 已持久的完整 `batch_id/detail/窗口/理由`；不再把第二批新 detail 冒充同 ID 的首版，不新造业务语义。

正式 CLI 从单点 10000 → 两点 10000/11000 → 第三点 10500 → 更正 11500 → 追加 10800 → 连续两次大 revision，七代 HTTP/HTML 函数续接成功。AsKnown1 全字段稳定；前两点没有提前 TOP。Rust 回归锁 `r9_prefix_observation_retains_sealed_first_version` 纳入必需 cargo test 入口。

### R3-M4：scope 缺省边界关闭

初始化后 scope 必填，须与本片 `{"structure":"CompleteCut","economic":"not_started"}` 声明、已发布批次/目录一致。缺失、空对象、错误形状、null、矛盾均立即 StorageUnavailable；不会先写默认值或换代后再宣布拒绝。正常经济未启动仍合法。

扩展矩阵曾发现 `[]/null` 虽 exit1/零写，但 Rust 错误没有统一标为 StorageUnavailable；已修统一根验证错误出口，最终矩阵通过。只采信原 R3 “坏 scope 在修复前仍被 200/0 接纳”的证据；不采信缺末态原始读数的“推进后 scope 仍缺”。

### 源码定位

- `rust/src/bin/s_structure_session.rs`：`validate_scope:488`、`verify_delta_payload:565`、`observation_id:769`、`verify_reachable_root:779`、`verify_reachable_root_in_tx:783`、`publication_fields:2766`、`read_scope:2779`。
- `s_session/s_readonly_server.py`：`_scope:50`、`_verify_delta_payload:166`、`_verify_reachable_root:265`、`_publication_fields:423`。
- `s_session/browser/index.html`：`validateState:346`、`validateDelta:372`、`comparePublication:389`、`applyStream:469`。

## 3. 编译、回归与失败记录

以下都是 **R9b 实际执行**，R9c 仅对同 hash 源码与自包含原始输出重核，未再次运行产品。完整 argv/cwd/stdout/stderr/真实退出码见 `r9-commands.json`；未用管道末级返回码代替 cargo 返回码。

| 检查 | 结果 | 精确原始记录 |
|---|---|---|
| `cargo fmt -- --check` | exit 0 | `r9-commands.json#/33` |
| `cargo build --locked --features s_session --bin s_structure_session --jobs 1` | exit 0 | `r9-commands.json#/34` |
| `cargo test --locked --features s_session --bin s_structure_session --jobs 1` | exit 0；33 passed，0 failed | `r9-commands.json#/35` |
| `cargo check --locked --features s_session --bin s_structure_session --jobs 1` | exit 0 | `r9-commands.json#/36` |
| `cargo clippy --locked --features s_session --bin s_structure_session --jobs 1` | exit 0；会话文件0告警（库内既存告警保留） | `r9-commands.json#/37` |
| `cargo test --locked --lib local_shape --jobs 1` | exit 0；4 passed，0 failed | `r9-commands.json#/38` |

Python AST 检查 exit0（`r9-commands.json#/52`）、launcher `bash -n` exit0（`#/53`）、最终实际 HTML Node 函数回归 exit0（`#/59`）。launcher 生产字节未改，实际 launcher 入口另由恢复探针 normal 路径运行。

先前失败没有抹掉：旧源码红灯观察差异；scope 错误分类缺口；初次 clippy 的 `cmp_owned` 告警；私有旧 writer Cargo harness 的版本/serde/锁准备 exit101。对应原始输出保留。后三类准备问题修正后，最终源码 cargo 和旧 writer 的 offline/locked 构建均 exit0；不把探针 exit0 泛化为整个实现通过。

## 4. 实际行为证据索引

附件均嵌入同名 JSON 的 `evidence_artifacts`，不是依赖旧 `/tmp` 路径的交付承诺。

| 键 | 实际入口、结果与附件 |
|---|---|
| R9-NORMAL | `r9_contract.py` → 真实 init/accept/advance → HTTP State/Delta；七 cut、观察首版一致及 AsKnown1 稳定。`r9-matrix-results.json#/results/normal`；逐原始输入/payload/hash 与 argv 在该附件 log。 |
| R9-MATRIX | 正常 control 库只读 backup，单项 SQL 变异 **44 副本**；每副本 7 HTTP 均503、7 Rust读写均exit1/StorageUnavailable，共308+308响应。`r9-matrix-results.json#/results/corrupt`；执行入口 `r9-commands.json#/39`。 |
| R9-NODE | 当前实际 HTML 函数，使用已存真实 HTTP 指定cut payload；七代续接、并发后续写者、错根、十字段冲突、精确整数前驱、重复/空增量/断连/Gap/晚响应。`r9-api-cuts.json`、`r9-node-results.json`；**不是GUI**。 |
| R9-RECOVERY | 正式launcher正常轨迹与三点真实 S SIGKILL，同DB正式recover/Query/advance；`r9-recovery-results.json`，命令 `r9-commands.json#/43`。 |
| R9-LIVE | 两条活旧writer无kill/stop，正式换代再release，旧进程自身exit1，新owner继续；重放/冲突/旧版不复活。`r9-live-results.json`，命令 `r9-commands.json#/44`。 |
| R9-LEGACY | git中 `6eeed76e3e12f1ffc3fdc62e23e60f2a3084c37b` 的实际旧bin源，经私有原生Cargo harness构建并真正接纳/发布；不是SQL手造旧值。`r9-legacy-results.json`，命令 `r9-commands.json#/50`。 |

**矩阵原始指纹**：每副本整体前/后及每 CLI 前/后，均保存 schema 全文/hash、十表列定义/行数、逐表 hash、逐列 hash。十表为 batches、catalog、meta、objects、observations、raw_events、relations、structure_deltas、witnesses、writer_epoch_history。比较逻辑行/列和 schema，不声称整个 SQLite/WAL 文件物理字节相等。HTTP 留完整 body/hash；原始 SQL 单项变异在每个 case 内，不把“相等=true”当唯一前后证据。

HTTP 入口：state、snapshot、catalog、delta0、state/as_of1、snapshot/as_of1、catalog/as_of1。Rust 入口：snapshot、watch、catalog、query、accept、advance、recover。变异覆盖当前合法但错误历史根、cut、内外十字段、scope、目录证据、七集合缺项/坏成员及见证/关系断裂。这里不声称穷举所有存储损坏组合。

| 真杀点 | 实际 S PID | 信号 / wait退出 | 同DB结果 |
|---|---:|---|---|
| after_begin | 7952 | SIGKILL / -9 | 原TOP仍是当前cut；epoch2恢复后等于control |
| after_batch | 7971 | SIGKILL / -9 | 完整不可达batch不发布；epoch2恢复后等于control |
| after_commit | 7990 | SIGKILL / -9 | 提交已存在，回执未知；原业务身份Query可知已发布，advance幂等 |

三臂均核 `/proc/PID/cmdline`、可执行文件和 DB 归属再发信号，并 wait 确认退出；Snapshot、AsKnown1、Watch **完整字段**与固定同输入/received/业务身份的无中断 control 比较。回执未知不是未发生；没有更换业务 ID 重做。只杀只读壳不计三杀。

活旧writer PID **8110 / 8119** 分别停在 Begin/batch 后；正式 recover 至epoch2，release后自身exit1。附件只保存 live 零写断言布尔与实际脚本/输出，**未单独存 live 每表前后值**；不冒称与44副本矩阵同等指纹留存程度。

旧 writer 实际库中，`supersedes_revision=9007199254740993` 以及八类 CatalogEvidence 计数确为 JSON integer。当前 Rust Snapshot/Catalog/Watch/Query（含历史）和五 HTTP 入口成功输出精确文本，十表/schema前后不变。旧源码逐字来自 git，核 SHA 见 JSON `behaviors.R9-LEGACY`；harness 依赖本项目原生库与锁定版本，**不是旧OS/整个旧crate环境复现**。项目 Cargo.toml/lock 未改。

## 5. 原票 8 AC

全部按原票编号；“有限实跑”不代表独立验收 PASS。实际输入/hash、命令/退出、输出、平台/故障与未验范围由第1、3、4节和对应 JSON 键共同绑定。

| AC | 本轮处置及未验域 | 证据键 |
|---|---|---|
| AC1 | **有限实跑，待根验**。正式launcher（R9-RECOVERY的normal路径）以及CLI单点前缀；真实TOP→更正RISING撤旧→追加新TOP。相邻严格无包含TestOnly域，同Rust核。 | R9-NORMAL, R9-RECOVERY, A-REUSE |
| AC2 | **有限实跑，待根验**。源修订分立、原ID重放/冲突/旧版不复活；两次>2^53和旧writer原生整数库精确读取。 | R9-NORMAL, R9-LIVE, R9-LEGACY |
| AC3 | **后端/Node有界验证；GUI待根**。完整持久元组及七载荷校验；六集合原子消费者按指定cut全字段对拍。错根与数字前驱不假绿，缺字段不提交。 | R9-MATRIX, R9-NODE |
| AC4 | **Linux三点真杀实跑；GUI/macOS待根**。PID7952/7971/7990：SIGKILL/wait=-9；同DB恢复至epoch2；Snapshot/AsKnown/Watch完整等于无中断control。 | R9-RECOVERY |
| AC5 | **有限实跑，待根验**。原业务身份Query、Commit丢回执DeliveryUnknown、同argvadvance幂等、旧epoch拒绝；两活writer换代后自身exit1，新owner继续。 | R9-RECOVERY, R9-LIVE |
| AC6 | **API/Node有界验证；GUI待根**。缺右邻前缀无TOP；AsKnown1全字段稳定；默认RecomputedWithRevision与指定cut分别绑定；旧TOP身份/first_known和证据历史保留。 | R9-NORMAL, R9-NODE, R9-LEGACY |
| AC7 | **S自身前件有界验证，经济全域不销**。本轮只启动S/readonly；economic=not_started。44副本7HTTP+7CLI共同拒绝，十表/列/schema逐次不变；不是经济故障分区试验。 | R9-MATRIX, R9-RECOVERY |
| AC8 | **证据随报告保存；不等交付批准**。原始输入、argv/exit/stdout/stderr/HTTP body、PID/信号/wait/恢复及前后指纹内嵌附件；真实GUI/原生/独评仍需根。 | R9-ARTIFACTS |

## 6. 17 来源子义务

以下仅是本片分配的子义务，不销完整来源 ID。逐项 signed_pointer / required_result 保留于 JSON `items`；不以实现类型存在、计数或经济空域代替成功实例。

| ID | 本轮证据与未完边界 | 证据键 |
|---|---|---|
| A-CC-006-03 | **有界实跑，待复评**。同核固定输入，三点真杀与无中断对象/关系/身份/见证/历史全字段一致。 | R9-RECOVERY, A-REUSE |
| A-CC-006-04 | **API/Node已验证；真实GUI待根**。更正撤旧RISING/追加TOP；三根raw/merged见证及四比较从实际S输出读取。 | R9-NORMAL, R9-NODE |
| A-ST-044-02 | **API/Node已验证；真实GUI待根**。七必需集合、替代及索引/目录/输入前沿元组验证；消费者六集合与指定cut全字段相等。 | R9-MATRIX, R9-NODE |
| A-ST-044-03 | **有界实跑；C压力保留**。单点前缀持续续接、三点原库恢复；断连/Gap/旧响应为Node受控调度。 | R9-NODE, R9-RECOVERY |
| A-ES-15-01 | **仅S有限域**。只启动S和只读壳；scope economic=not_started，丢scope立即拒绝；无经济ack推断。 | R9-MATRIX, R9-NORMAL |
| A-ES-15-02 | **有界实跑**。Begin/batch/Commit实际暂停真杀、Query/换代；坏记录停写零变更。 | R9-RECOVERY, R9-LIVE, R9-MATRIX |
| A-RA-04-01 | **仅S子义务**。旧TOP ID/first_known/区间/原事件/见证/撤回原因保留；经济责任不销。 | R9-NORMAL, R9-RECOVERY |
| A-RA-10-01 | **仅S，GUI待根**。Snapshot/Delta/Catalog同cut完整元组/集合及稳定AsKnown；默认Recomputed另具名；完整经济历史未验。 | R9-NODE, R9-NORMAL |
| A-I-02-01 | **仅S存储边界**。44副本统一StorageUnavailable零写；经济未启动而非经济域故障乘积。 | R9-MATRIX |
| A-OB-004-01 | **有界实跑**。版本分立、supersedes/replaces、原ID重放/冲突/旧版不复活；旧writer精确前驱。 | R9-LIVE, R9-LEGACY |
| A-OB-008-01 | **持久/Node有界；GUI待根**。真实持久Delta，重复/空增量/请求失败/Gap；完整元组与六集合对拍，不把Node叫真实GUI。 | R9-MATRIX, R9-NODE |
| A-OB-009-01 | **有界实跑**。一/两点缺右邻无TOP；首观察沿用首批次，不回填旧first_known；AsKnown稳定。 | R9-NORMAL, CARGO-TEST |
| A-OB-014-01 | **经济部分NOT_RUN**。S恢复未补造经济记录；计划/可能已发/成交责任属于后续，不能以经济空集合PASS。 | R9-RECOVERY |
| A-OB-015-01 | **Node有界；GUI待根**。同session/gen完整根绑定，pageEpoch晚响应防护；错根、请求失败不改已提交缓存。 | R9-NODE |
| A-OB-016-01 | **恢复/API/Node有界；GUI待根**。三修订真杀同DB恢复后API完整收敛；最终HTML真实页面需根接回。 | R9-RECOVERY, R9-NODE |
| A-OB-017-01 | **仅S有限域**。固定相同输入/received/业务ID/顺序，比较完整Snapshot/AsKnown/Watch，不抹语义差异、不造经济恢复。 | R9-RECOVERY |
| A-OB-020-01 | **证据已归档，非批准**。每项绑定source/build/input/平台/故障/argv/exit/API；原始文本附件可验SHA还原，GUI及独评未验。 | R9-ARTIFACTS |

## 7. 复用与仍未验证的部分

A 的严格核、已签目录/profile未改。A验收入口为 `issue1370-tb01-a/ROOT-ACCEPTANCE.md`、`ACCEPTANCE-INDEX.json` 及其归档路径；R9b 已逐SHA核R3列出的A持久/只读归档，不重报旧f635缺陷。只复用同核有限域 oracle，不以A的200/32旧计数充本轮新测试。

`open_db / record_connection_pragmas / cmd_accept / cmd_recover / testonly_pause / verify_begin_owner / persist_batch_before_publish / cancel_own_begin / compute_diff / build_object_and_witnesses` 与冻结父提交逐字相同，逐函数 SHA 在 JSON `behaviors.A-REUSE.functions`。**它们依赖的根验证前件改了**，所以已重新执行三杀与两活writer；不能只凭函数同hash跨过改变的调用链。根R8的正常/三故障GUI及macOS同writer连接证据保留原有限域事实，不宣称覆盖本次新HTML或所有平台行为。

明确 NOT_RUN / 未销项：

- 最终修改后 HTML 的真实浏览器 DOM/网络故障/恢复前后操作（容器无 chromium/google-chrome/firefox；Node 是实际HTML函数+模拟DOM，不是GUI）。
- 最终构建的 macOS native/fullfsync；根 R8 同 writer 连接结果仅按未改函数字节有限复用，未重读宿主原件。
- 原独评对代码提交 381f5e1c670cf97b6a1491a3ec3c03c30df406d0 的接回复审与根验；原R3 FAIL历史不改变。
- 远端票面即时读取：gh exit4 未认证；未改认证。冻结授权/SPEC以用户及本地已签文件为准。
- 全量cargo test、C1372完整分页/保留/过期/慢消费与长历史压力；本轮只跑靶向33+4测试和列出的有限实例。
- 经济责任/计划/可能已发/已成交、TB05全域故障分区、完整经济AsKnown、ST044高级扩展、其余CC/G/FU/非退化市场域均未销项。

成本：根验证遍历全部可达已发布代链、各代历史视图和封存批次，成本随历史增长；本片不宣称 C 的长历史/慢消费性能通过。每次 HTTP 读在单请求事务内结束，不跨请求持有读事务。

## 8. 附件恢复与复核入口

R9c 已从 JSON **解码全部14份附件，逐长度/SHA校验**，核8AC/17项证据键闭合、源码和原件hash一致；仅完成报告，不重复产品运行。旧/tmp路径保留为当时实际argv/PID/DB来源，不能假设在根或新容器仍存在。

从仓库根运行下列只解码命令（Python标准库即可），得到一个新的私有目录；不改数据库，不执行附件脚本：

```python
import base64, hashlib, json, lzma, pathlib, tempfile
r = json.loads(pathlib.Path('.chanlun/review-results/issue1371-tb01-b/implementation-r9.json').read_text())
out = pathlib.Path(tempfile.mkdtemp(prefix='issue1371-r9-evidence-'))
for name, a in r['evidence_artifacts'].items():
    assert pathlib.Path(name).name == name and a['encoding'] == 'xz+base64'
    b = lzma.decompress(base64.b64decode(a['data']))
    assert len(b) == a['bytes'] and hashlib.sha256(b).hexdigest() == a['sha256']
    (out / name).write_bytes(b)
print(out)
```

归档的实际命令完整保留在 `r9-commands.json`，各CLI/HTTP细节另外在行为附件。`r9_contract.py`、`r9_consumer.cjs` 已随代码提交；恢复探针/API补取/legacy harness脚本作为具名证据附件。**包含 `/proc` 的运行探针是 Linux harness，不冒称可原样在macOS运行**；macOS用户路径用正式launcher/CLI而非修改生产实现来适配探针。

若仅复核实际HTML函数，可把附件 `r9-api-cuts.json` 复制为私有目录中的 `api-cuts.json`；按原R3报告的解码方式，另恢复其 `probe-results.json` 到独立目录，再运行：

```sh
node s_session/tests/r9_consumer.cjs <含api-cuts.json的私有目录> <含R3-probe-results.json的目录> "$PWD"
```

这只重放已存真实payload，不是新HTTP/GUI实测。实际Linux端到端回归入口为项目Python运行 `s_session/tests/r9_contract.py <新空私有目录>`；其后可用 `r9-api-cuts.py` 补取该目录中的指定cut API。所有真实运行必须按本平台原生环境，不能把上轮路径不存在误报成生产失败。

## 9. 根的少量GUI接回步骤

以下是 **待根执行**，不预写通过：

1. 从矩阵附件 `log` 中恢复 `input` 记录的原 `payload` 为新私有输入文件，并核记录的sha256；用本平台最终代码原生构建。正式launcher使用 `one.json`、显式TestOnly profile与新隔离DB启动只读页面；不触碰旧DB或其他服务。
2. 在同DB用正式 accept/advance 推进 `two.json → three.json → correction.json → four.json`，每次让实际页面续接。核前两点无TOP、首观察batch/detail不变、旧TOP撤回/替代/新TOP；网络面应取指定 `as_of=N`，游标、六集合与完整发布元组一致。
3. 再推进 `large1.json → large2.json`，核真实DOM的 revision/predecessor 精确文本。切换旧 AsKnown 与“当前”RecomputedWithRevision，不能覆盖旧first_known或混新cut。
4. 对上述真实库的**受控副本**仅改当前根指向同库合法历史batch，及分别缺scope/缺witnesses；从该副本正式只读服务访问页面，确认503和当前失败状态、游标不推进。健康库页面仍可成功续接，不能只验拒绝。
5. 对当前代码三故障的真实恢复前后，接回受变更影响的页面续接/Gap指定cut重建、断连、迟到旧响应；可复用已记录输入/调度，仍需实际页面证据。原生macOS/fullfsync按未改函数与新前件影响面分列，不要求以整图压力或经济操作补证。

最终源码GUI、原生与独立评审仍由根接回。`<promise>COMPLETE</promise>` 仅表示原作者报告和提交交回，不是独评PASS、main合入批准、B/父TB/#1323完成。
