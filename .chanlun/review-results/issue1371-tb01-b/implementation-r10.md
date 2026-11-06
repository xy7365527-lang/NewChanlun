# #1371 TB-01-B：R10 实施与靶向验证报告

> 编号声明：本报告中的 #1371、#1340、#1323 均指 GitHub Issue。
> 原作者报告，不是独立评审。R4 原件结论 **FAIL（1H/1M）** 不改。
> 范围仍是 SPEC #1340 的 TB-01-B 结构子义务，不代表 B、父 TB 或 #1323 六 Destination 完成。

## 1. 提交与状态

- 冻结父提交：`41ed7163e8bc15bc7cc5af1fb06af22d98a6589c`，进入时工作树干净；唯一分支 `codex/1323-tb01-b-1371`。
- 产品修复提交：`a0b6f17a3def8f27f2ff936980c97e34c9da1c0e`。
- 最终源码/测试提交：`50f22cac47b25685a8a24088cd3b64b3a82e2d4f`；后一提交只补现有测试的健康对照，不改产品。
- 本次正式报告仅 `implementation-r10.md/json`。旧作者报告、R1–R4 八份独评及原 roster 的字节、Git blob/mode 保持。
- R4-H1/M2 已修并有下列有限实际证据；**最终真实 GUI、macOS 影响面与原独评复评仍待根接回**，作者不宣称 PASS。

没有 push、merge、main/GH 写入、harvest、全局服务/模型/队列变更或经济外效。写入只涉及获准的三个产品文件、两个既有测试文件及两份 R10 报告；launcher 未改。未新建 r10 测试文件，未改规范/教义/ADR/formal/CI、未补未决 G/FU 或资金默认值。

## 2. 共同绑定与证据等级

以下8AC/17子义务均共用本节源码、规则、输入、目录、构建、平台绑定；各行再定位实际命令、输出、故障与未验项。

- 规范清单 SHA256：`38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`；六份指定 payload 已读并逐SHA登记在 JSON `binding.spec_files_sha256`。
- 目录 SHA256：`938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`（116项）；profile 文件 SHA256：`2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc`，规范内容 hash：`0e9dfc9befd6878e0c844ee62c0f44576532ee8f13fe38c45f6bf467fe92d64e`；规则 `s2-axis-quantifiers`。
- 受测域不变：具名 TestOnly 逐笔一对一退化 OHLC、相邻严格无包含、前两点定向、无同价竞争；不把窗口当笔/线段/中枢。
- 实测 Linux aarch64；项目 Python 3.11.16、HTTP SQLite 3.53.1；Rust 继续用项目锁定 bundled SQLite。最终 S 二进制 SHA256：`ee39fb99e4d5f7d4f9a47aefbb7979ed7a384858dec48149a9794cbefb952842`，不是旧writer或macOS二进制。
- 逐份原始输入/payload/hash：`r10-results.json#/log`、`recovery-results.json#/inputs`、`legacy-results.json#/log`。PID/argv/实际exit/HTTP正文与指纹在同一附件内，不混用其他工位读数。
- 已全文读仓内正式 R4 报告，38份具名附件逐bytes/SHA解码验证。根内联的macOS gen0反例仅作为修复依据；没有读取未挂载宿主原件或独评会话/raw/prompt。

证据标签：**实际CLI/HTTP/SQLite**、**实际HTML函数+真实HTTP（模拟DOM/交付调度，非GUI）**、**同hash/正式独评有限复用**、**NOT_RUN** 分列。附件脚本exit0不代替其中产品成功/拒绝的具体结果。

| 文件 | 最终SHA256 |
|---|---|
| `rust/src/bin/s_structure_session.rs` | `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901` |
| `s_session/s_readonly_server.py` | `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c` |
| `s_session/browser/index.html` | `9c3d2c3703b9e765f3d65e2cd76759ff5716e83781fd49c6ec5457aad8565498` |
| `s_session/launch_s.sh` | `65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d` |
| `s_session/tests/r9_contract.py` | `89eeaff7430a098c4abaf239b71f0d684da6dd1f53038dba9d2f84667cdadc2f` |
| `s_session/tests/r9_consumer.cjs` | `8d8a467640e6a98b2c42bacc3f1c70f2ac778bf68ea307329ab00299e727557d` |

## 3. R4-H1：固定profile绑定闭合

### 已改行为

1. 首次新 Accept 在原事务内同时保存 `profile_id/hash` 与 `meta.profile_definition`（规范JSON正文）；`input_profile` 必须与固定身份一致。**没有修改表schema或旧业务身份/原始版本。** 后续接纳不能换定义。
2. 统一根验证覆盖当前/历史读、Query、Accept、Advance及Recover，核已绑定 profile 与独立来源一致；首次接纳尚无batch时也能检验其规范定义。错误不是只由下一次accept报IdentityConflict，所有相应入口在写前失败。
3. 旧格式没有 `profile_definition` 时，使用其已有可达封存批次作独立来源；旧未发布且尚无batch的已声明 TestOnly profile，核其与原声明档案的精确id/hash。**不是为新会话挑默认profile，不在某条核验失败后改走宽松判据，也不给旧库补写。** 无独立来源的域外旧未发布profile明确拒绝，不猜测。
4. 每份历史批次的非空profile与固定输入绑定一致；首次接纳前的空发布允许合法未绑定。Advance在Begin事务内读取已验证绑定，而不是在事务外先缓存；batch/publish的owner前件再次检查根/绑定。
5. Snapshot顶层 profile 只从相应发布cut的不可变batch读取，不借当前meta覆盖旧cut。消费者将实际已有 `profile_id/hash` 加入提交绑定，并分别与同cut Catalog/Delta证据比较；验证后才提交cursor/六集合/currentState/binding。

### 本轮反例与健康对照

21个副本来自实际正常库的只读backup，分为：已发布无pending、正确accept后的pending、首次accept未publish的pending。对 `profile_id/hash/input_profile` 做单项错值/删除；profile_hash另外用**格式完全合法的64位错误hash**，避免只靠格式门宣称绑定修复。

每个副本在注入前，同8个HTTP路由全部真实200，读取前后指纹不变；单项SQL注入后，8HTTP全部503，7Rust入口全部exit1/StorageUnavailable且零写。每CLI/每副本保存schema、十表列定义/行数、逐表和逐列hash前后值。总计168健康HTTP、168拒绝HTTP、147拒绝CLI；不把数字当全域证明。

路由包括current state/snapshot/catalog、delta0、AsKnown0的state/snapshot/catalog，以及已发布/普通pending的state-asof1。**首次pending还没有cut1**，第八路由改用合法 `after_generation=1` 的Gap读取（健康200）；不借“未发布cut本就拒绝”当profile特异反例。该证据问题在归档前发现，只补测试及本项验证，产品同hash检查未机械重跑。

Rust入口：snapshot、watch、catalog、query、accept、advance、recover。原始SQL、完整错误正文、argv/exit及前后指纹见 `r10-results.json#/results/profile_cases`。

### 消费者的特异验证

`r9_consumer.cjs --r10` 连接实际S只读服务。源库已到gen7，TestOnly交付调度只交实际gen2，消费者确实请求 `/api/state?as_of=2`，HTTP200。先成功加载cut1：

- 健康control：cursor **1→2**，出现成功对拍；
- 完整State仅改 `/snapshot/profile_hash` 为64位错误hash：cursor **1→1**，完整currentState、六集合、binding均与之前逐字段相等；
- 完整State仅改 `/snapshot/profile_id`：同样拒绝/保持。

每条保存原始State、变换后的完整State、唯一差异路径、HTTP路由/正文及候选前后状态；不是旧schema、缺键、404或请求失败造成的拒绝。此为实际HTML函数+真实HTTP，**DOM和交付调度受控，不是实际GUI或TCP丢包试验**。

## 4. R4-M2：cut0完整历史稳定

- cut0表示初始化时尚未发布输入解释：Snapshot.profile_id/hash保持空字符串。即使已accept但未advance，也不把未解释来源塞进已发布结构快照；原身份Query/meta仍可查看接纳状态。
- gen1+的profile由对应batch给出，包括首次接纳之前的合法空发布，不用后来meta回填。
- AsKnown目录初始化状态使用已有cmd_init定义的 `not_implemented/not_proved/not_run` 与空运行证据；首次发布后的CC-006使用该发布状态迁移。计数从历史投影的条目统计，不从当前计数回填。已签条目/title/domain/branches/目录版本未改。
- 没有禁止gen0、删除历史字段或重写已发布数据库来“凑一致”。这只落实本实现已有的初始化→首次发布迁移，不新增完整可变目录/经济历史语义。

真实轨迹：init；依次单点10000、第二点11000、第三点10500、更正11500、追加10800及两次大revision，每次在 **accept后未advance** 和 **publish后** 读取完整AsKnown0。共15阶段，每阶段3HTTP+2CLI完整payload均与init深等，并记录读取前后指纹。不是只比较对象集合。

另验：空输入先发布后再首次绑定、旧writer首次accept尚无batch，两条健康路径均可查询/继续推进；pending时当前Snapshot仍等原已发布cut，AsKnown0不漂移。gen1首观察/更正史稳定由本轮正常/三杀控制和既有cargo锁继续验证。

### 载荷对应关系（不以更小白名单改写“完整”）

| 实际载荷 | 校验/来源 |
|---|---|
| Snapshot的10个publication字段 | 原同cut元组：session/generation/cut/catalog_revision/index/scope/input_frontier/seq_range/目录状态/证据；继续核内外与State/Catalog对应关系 |
| Snapshot.profile_id/hash | 新增固定输入来源核验；读取对应batch，cut0明确未绑定；对应Catalog/Delta的catalog_evidence字段，不要求那些包装有不存在的顶层profile |
| Snapshot.history_mode/as_of_generation | 验证AsKnown代际或RecomputedWithRevision/null组合；不删字段 |
| 对象/撤回/见证/关系/观察/源历史六集合 | 保留原完整成员比较；不是branch/数量比较 |
| Catalog items/counts | 动态实现/证明/运行状态及计数按历史cut投影；静态签署描述仍由完整目录提供，本轮不宣称重证任意协调目录篡改防护 |

### 实际源码位置

- `rust/src/bin/s_structure_session.rs`：`verify_profile_binding:780`、`verify_batch_profile:840`、`profile_at_cut:857`、`cmd_accept:1294`、`cmd_advance:1923`、`verify_begin_owner:1860`、`read_catalog_in_tx:2900`、`read_snapshot_in_tx:3217`。
- `s_session/s_readonly_server.py`：`_verify_profile_binding:265`、`_verify_batch_profile:300`、`_profile_at_cut:309`、`read_catalog:494`、`_read_snapshot_in_tx:637`。
- `s_session/browser/index.html`：`bindingOf:346`、`validateSource:347`、`validateState:361`、`comparePublication:405`。

## 5. 原生检查与实际恢复/兼容

完整argv/cwd/stdout/stderr/真实exit在 `commands.json`，下面pointer准确定位最终产品检查；后一个测试提交产品同hash，只重新执行改动的Python脚本/实际HTTP消费者和AST。不用管道末级exit代替cargo。

| 检查 | 结果 | 原始输出 |
|---|---|---|
| `cargo fmt -- --check` | exit0；通过 | `commands.json#/8` |
| `cargo build --locked --features s_session --bin s_structure_session --jobs 1` | exit0；通过 | `commands.json#/9` |
| `cargo test --locked --features s_session --bin s_structure_session --jobs 1` | exit0；35 passed / 0 failed | `commands.json#/10` |
| `cargo check --locked --features s_session --bin s_structure_session --jobs 1` | exit0；通过 | `commands.json#/11` |
| `cargo clippy --locked --features s_session --bin s_structure_session --jobs 1` | exit0；会话源文件0新增告警；库既存告警保留 | `commands.json#/12` |
| `cargo test --locked --lib local_shape --jobs 1` | exit0；4 passed / 0 failed | `commands.json#/13` |

Python AST、launcher `bash -n` 通过；launcher实际入口也用于正常恢复探针。现有R9路径增加两条Rust测试锁，未新建R10测试文件。旧源码先跑新增真实回归exit1，完整AsKnown0漂移红灯材料在 `red-results.json`；不抹去红灯或把旧35/33计数混算。

### 三点真实SIGKILL

新增根/owner前件改变了恢复读写路径，因此本轮重跑具体三杀及控制轨迹，而不是仅凭函数名字复用：

| 阶段 | 实际S PID | 信号 / wait | 同DB恢复结果 |
|---|---:|---|---|
| after_begin | 4607 | SIGKILL / -9 | 原TOP仍当前，epoch2后完整Snapshot/AsKnown1/Watch等control |
| after_batch | 4626 | SIGKILL / -9 | 完整不可达batch不发布，恢复后完整等control |
| after_commit | 4645 | SIGKILL / -9 | 提交已存在/回执未知；原IDQuery权威，advance幂等不多cut |

终止前核实际cmdline、可执行文件和DB归属，再wait确认退出。控制与故障臂固定输入/received/业务身份；不抹first_known/receipt/对象/关系/见证差异，不声称恢复史或整个DB物理字节相同。原IDQuery、旧epoch拒绝与无新输入advance也已运行。没有只杀Python冒充S真杀。

### 旧writer及空发布兼容

使用正式R4附件中的最小Cargo harness和git `6eeed76e3e12f1ffc3fdc62e23e60f2a3084c37b` 的旧writer源（SHA `addb492ac2c1888904f2ab2d9789ae9a7ebbfc8e0037e4d822f73b67be333be8`），本轮locked/offline原生构建exit0。旧writer实际发布3代、大revision前驱仍为旧JSON integer；当前Rust6读/HTTP5读全部成功、精确文本输出、schema/十表前后不变。**不是手改SQL造“旧库”**，也不是旧OS/全旧crate环境复现。

另有旧首次pending健康实例验证无batch时的独立声明来源路径。运行材料在 `legacy-results.json` 与 `extra-results.json`；没有为兼容重写旧profile/batch/hash或制造历史获知。

## 6. 原票8AC逐项

每项共同绑定第2节，证据键由下一节机器附件索引解释；“有限”不等独立验收PASS。

| AC | 本轮结果与未验边界 | 证据 |
|---|---|---|
| AC1 | **有界实跑**。正式launcher→原始输入→同一Rust核正常TOP/更正RISING/追加TOP；新增prefix1..7与三修订真杀控制成功。 | R10-HISTORY, R10-RECOVERY |
| AC2 | **有界实跑，待复评**。源修订/对象身份/结构代分立，profile固定绑定补齐；新旧writer大整数前驱精确，旧TOP身份/first_known保留。 | R10-PROFILE, R10-LEGACY, R10-RECOVERY |
| AC3 | **实际API/HTML函数通过；真实GUI待根**。同cut profile与证据补入比较，健康control cursor1→2；单字段冲突cursor/六集合/绑定完全不变；R3原完整载荷反例由R4已复验，现有cargo锁仍通过。 | R10-NODE, CARGO, R4-REUSE |
| AC4 | **Linux三杀有界通过；GUI待根**。PID4607/4626/4645真实SIGKILL/wait=-9，同DB正式recover；Snapshot/AsKnown1/Watch全字段等无中断control。 | R10-RECOVERY |
| AC5 | **本轮三杀/原ID路径通过；活writer复用限域**。Commit后回执未知，Query原业务身份权威结果、同argvadvance幂等/旧epoch拒绝；不造新ID/经济资格。R4两活writer结果有限复用。 | R10-RECOVERY, R4-REUSE |
| AC6 | **完整API历史通过；真实GUI待根**。init、每次accept未advance、每次publish共15阶段HTTP/CLI AsKnown0完整不变；gen1首观察历史稳定；当前与指定AsKnown分名。 | R10-HISTORY, R10-EXTRA, R10-NODE |
| AC7 | **S存储边界有界通过**。21单项profile副本×8HTTP×7CLI，全部503/exit1 StorageUnavailable且前后schema/十表指纹不变；S/readonly运行，E/B/X未启动不等经济故障分区证明。 | R10-PROFILE, R10-RECOVERY |
| AC8 | **证据归档，未获验收批准**。本轮原始inputs/hash/argv/exit/HTTP正文/信号wait/同DB读数与指纹自包含；源码/构建与17项一并绑定，根GUI/原生/独评待验。 | ARTIFACTS |

## 7. 17来源子义务

JSON逐项保留有效signed_pointer和required_result；完整来源ID的量词及跨域义务不在此销项。

| ID | 本轮证据与限制 | 证据键 |
|---|---|---|
| A-CC-006-03 | **有界实跑**。固定输入/received/规则三杀与control完整结构、身份/见证/历史相等；A同hash严格核只限其原受测域。 | R10-RECOVERY, R4-REUSE |
| A-CC-006-04 | **API/函数有界，GUI待根**。真实更正撤旧RISING与追加TOP，原raw/merged三根见证及四比较保留；本轮未做实际DOM见证展开。 | R10-RECOVERY, R10-HISTORY |
| A-ST-044-02 | **有界验证，待复评**。持久七载荷原R3缺项反例R4已修复复验；本轮profile同cut绑定补齐，单字段消费者反例拒绝，健康成功。 | R10-NODE, CARGO, R4-REUSE |
| A-ST-044-03 | **结构数据有界；GUI/C压力未销**。三杀同DB恢复与AsKnown1续接成功；不以小轨迹代替完整保留/分页/过期矩阵。 | R10-RECOVERY |
| A-ES-15-01 | **仅S子域**。本轮仅启动S/readonly，economic=not_started；profile来源验证不依赖E/B/X或经济ack。 | R10-HISTORY, R10-RECOVERY |
| A-ES-15-02 | **有界实跑**。新根前件在Begin/batch/publish路径执行；已接纳待处理坏profile立即拒绝，不发布新cut或写默认值。 | R10-PROFILE, R10-RECOVERY |
| A-RA-04-01 | **仅S，不销经济责任**。旧TOP原ID、first_known/区间、源输入/见证、撤回史在三杀与control相等；计划/成交责任后续。 | R10-RECOVERY |
| A-RA-10-01 | **完整S历史有限验证；经济史未销**。cut0完整历史稳定，gen1+从相应batch读profile；Delta/Snapshot来源相符，默认Recomputed另名。 | R10-HISTORY, R10-NODE, R10-EXTRA |
| A-I-02-01 | **S存储子域有界**。坏固定profile在静止/pending/首次pending统一StorageUnavailable零写；不是全经济故障乘积。 | R10-PROFILE |
| A-OB-004-01 | **有界实跑**。版本与supersedes/replaces原边界保留，固定profile版本校验补齐；旧writer真实整数发布库读取无损且零写。 | R10-PROFILE, R10-LEGACY |
| A-OB-008-01 | **API/函数有界；真实GUI待根**。实际as_of2响应单改profile字段拒绝，全部非目标字段保留；健康增量仍提交。原乱序/重复/Gap范围按R4复用。 | R10-NODE, R4-REUSE |
| A-OB-009-01 | **有界实跑**。缺右邻无TOP；cut0不再回填profile/目录状态，首观察及旧first_known不改。 | R10-HISTORY, R10-RECOVERY |
| A-OB-014-01 | **经济部分NOT_RUN**。S未补造历史经济应用；无计划/可能已发/已成交实例，不作经济空域PASS。 | R10-RECOVERY |
| A-OB-015-01 | **函数有界；GUI待根**。profile加入已提交绑定；冲突时currentState、cursor、六集合与binding逐字段保持原值，pageEpoch逻辑未改。 | R10-NODE, R4-REUSE |
| A-OB-016-01 | **恢复API有界；GUI待根**。三故障恢复后完整API与control一致；当前页面恢复/断连实际GUI交根。 | R10-RECOVERY, R10-NODE |
| A-OB-017-01 | **仅S有限域**。固定原输入/时钟/业务身份，不删获知差异；cut0未知profile不从未来推回，不用bars造经济史。 | R10-HISTORY, R10-RECOVERY |
| A-OB-020-01 | **原始证据已归档，非批准**。14附件含命令/真实exit/原payload/HTTP/前后指纹/PID信号wait；独评/GUI不同等级不混淆。 | ARTIFACTS |

## 8. 证据索引与复用边界

| 键 | 自包含附件/定位 |
|---|---|
| R10-PROFILE | `r10-results.json#/results/profile_cases`：21副本注入前完整健康HTTP、注入SQL、8HTTP/7CLI实际响应、每次schema/十表/逐列前后指纹 |
| R10-HISTORY | 同附件 `#/results/history`：15个转折完整AsKnown0；`states/current/watch` 保存正常当前/历史/增量；log保存每份输入与命令 |
| R10-NODE | `node-r10-results.json`、`node-execution.json`：真正as_of路由/原始与单字段变换后State/缓存前后；实际命令在commands |
| R10-RECOVERY | `recovery-results.json`、`recovery-probe.py`：正常launcher、三杀输入/marker/PID/signal/wait/同DB重启/API/完整control比较 |
| R10-LEGACY | `legacy-results.json`、`legacy-verify.py`、Cargo清单/lock；旧源可按已给git提交取，不归档二进制或数据库 |
| R10-EXTRA | `extra-results.json`、`extra.py`：空发布后绑定、旧首次pending健康路径 |
| CARGO | JSON checks与 `commands.json` 精确pointer；编译警告/真实exit不隐藏 |
| R4-REUSE | 仓内 `review-r4.md/json` 原件；R3四个原反例由R4独立复跑修复，本轮现有cargo锁仍执行，不无差别重跑全部旧矩阵 |

A `ROOT-ACCEPTANCE.md / ACCEPTANCE-INDEX.json / review-incremental.md` 及同hash严格核、目录/profile入口已复核，只复用其有限受测域oracle，不拿A旧测试数充R10结果。R4两活writer完整前后证据、原25载荷矩阵与根R9大矩阵保留各自有限事实。

`open_db / record_connection_pragmas / verify_writer_epoch / cmd_recover / testonly_pause / persist_batch_before_publish / cancel_own_begin / compute_diff / build_object_and_witnesses` 与冻结父提交同hash，逐函数SHA见JSON。但调用的新根前件及verify_begin_owner改变了，不能仅靠同hash函数跨过改变的路径；已另跑三杀/profile矩阵/旧库。本轮未再跑两活writer全调度或旧大矩阵，不扩成全故障乘积。

旧连接函数的macOS同writer配置证据只有限复用；根R4-M2初始/历史cut0真实GUI反例是依据，不是当前修复通过。新GUI/原生仍需根验。

成本与缺省边界照实：根验证仍遍历可达代链/历史视图，新profile/owner校验增加读成本，未做C长历史性能承诺。旧未发布且无可核独立来源的非TestOnly profile显式StorageUnavailable；不把该域补成产品默认。

## 9. NOT_RUN与根接回步骤

- 最终源码的真实浏览器GUI/DOM、真实网络断连与恢复前后页面操作：NOT_RUN；容器无chromium/google-chrome/firefox。Node是实际HTML函数+真实HTTP、模拟DOM，不是GUI。
- 最终macOS原生产品/fullfsync现场再验：NOT_RUN。根提供的R4 gen0 GUI反例是修复依据而不是新源码通过；旧同writer连接配置仅按open_db/record_connection_pragmas同hash有限复用。
- 原独评对新代码提交的复评及根验：待接回；原R4 FAIL（1H/1M）原件保留，不由作者改判。
- 本轮未重跑R4的两活writer全调度、25载荷副本及根R9大矩阵；相应原有限证据保留，cargo既有锁仍执行，新增前件另以三杀/新profile矩阵验证。不据此称全故障乘积通过。
- 全量cargo/history重放、TB-01-C分页/保留/过期/慢消费压力、其他CC/G/FU/非退化市场域、ST044高级关系及经济责任/计划/可能已发/已成交/全经济历史/全域故障分区均未销项。
- 远端票面即时只读查询gh exit4未认证；未改认证。依据用户授权冻结事实及本地已签规范，不重开批准门。

建议根只接回受影响的少量真实页面轨迹，不重开整图：

1. 用当前本机原生构建、正式launcher和新隔离DB，在init保存真实AsKnown0完整HTTP/GUI；从附件log恢复原输入并核hash。
2. 首次accept未advance、首次publish、正常更正/追加后，原服务同DB重取完整AsKnown0（state/snapshot/catalog）并操作实际Chrome页面；必须无未来profile/目录计数漂移。gen1首观察/旧TOP历史继续稳定。
3. 健康current/指定cut/增量均需页面成功。只在独立副本注入单meta.profile_hash或profile_id错误（含合法64位错误hash），再含正确accept后的pending臂；HTTP/正式写入口应立即StorageUnavailable且零写，页面不假绿。
4. 对实际浏览器同cut响应的单Snapshot.profile字段变换，保留其余全部新合同字段及真正as_of路由；核cursor、六集合、binding与上次已提交值保持。Node结果不代替此GUI。
5. 接回三故障恢复前后页面与macOS新前件影响面，再交原独评会话复评。不得以此次COMPLETE代替验收、main批准或经济动作。

## 10. 还原原始机器材料

14份文本附件全部内嵌 `implementation-r10.json.evidence_artifacts`，每份带长度/SHA及可恢复正文；没有DB、二进制、session导出、prompt或密钥。旧/tmp路径仅描述当时argv/DB来源，不能假定根仍可访问。

```python
import base64, hashlib, json, lzma, pathlib, tempfile
r = json.loads(pathlib.Path('.chanlun/review-results/issue1371-tb01-b/implementation-r10.json').read_text())
out = pathlib.Path(tempfile.mkdtemp(prefix='issue1371-r10-evidence-'))
for name, a in r['evidence_artifacts'].items():
    assert pathlib.Path(name).name == name and a['encoding'] == 'xz+base64'
    b = lzma.decompress(base64.b64decode(a['data']))
    assert len(b) == a['bytes'] and hashlib.sha256(b).hexdigest() == a['sha256']
    (out / name).write_bytes(b)
print(out)
```

实际Linux回归入口是项目Python运行 `s_session/tests/r9_contract.py <新空私有目录> --r10`；其后运行附件 `node-driver.py <r9_contract.py绝对路径> <上述目录>` 驱动实际HTTP消费者。逐次原始argv/环境/cwd/exit在附件中。带 `/proc` 的进程探针是Linux harness，不冒称原样适用于macOS；macOS通过正式launcher/CLI适配取证，不改生产时序。

本次只是原作者交回代码和报告；`<promise>COMPLETE</promise>` 不表示独评PASS、GUI/macOS验收、main批准、B/父TB或#1323完成。
