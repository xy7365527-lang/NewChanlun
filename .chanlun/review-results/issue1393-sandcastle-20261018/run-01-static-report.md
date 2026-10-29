# #1393 静态对账与编码门阻塞报告

状态：**BLOCKED，尚未开始产品编码，不构成静态实施完成或整票验收。**

本次已完成固定源核对、裁定与实现对账、共享接口定位、独立手算工作例和后续验证合同。编码前 reference-coding 门所需的三个指定 XERJ 索引请求均被当前沙箱拒绝；没有把请求失败或管理者缓存中的零命中改记为检索完成。缺的是允许环境中的检索结果，不是新的包含语义裁定。

名分：绑定 open 票 #1393 的工作草稿；不修改正本或已签 SPEC。管理者后续独立评审应覆盖实际交付差异。本报告也尚未经过另一会话独立复核。

## 实际源与授权边界

- 专用 checkout：`/Volumes/AgentStorage/issue1393-sandcastle-20261018-worktree`。
- 分支：`codex/1393-sandcastle-w-20261018`。
- HEAD：`3315ed1d59ba31ffbad3f0b959702eaa92f4ddbd`；起始工作区干净。
- HEAD tree：`8e854b779f62e035a0e00caed9ee060257ff15c5`。这是基线树，不含本报告，不冒称为交付树。
- 权威：本树 `.chanlun/definitions/baohan.md` v1.6，及管理者保存的 #1342 resolution `comment6149622771`。未以缺失的 `ROOT-RULINGS-1393` 转述创设规则。
- 已读：`CLAUDE.md`、`AGENTS.md`、`.agents/skills/ask-matt/SKILL.md`、`docs/agents/{wayfinder-workflow,delivery-discipline,generation-constitution}.md`、`.sandcastle/CODING_STANDARDS.md`；定向读取已签 SPEC 的初始化、观察与持久合同。
- 管理者提供的 `issue1393.json` 为本轮票面来源；`dependencies1393.json` 中 #1373、#1359、#1342 均为 closed。未对 GitHub 执行认领、评论、依赖变更或其他写入。
- 服从外层任务的禁止 commit 指令；内嵌任务中“允许必要 commit”不用于越过该限制。没有 commit、暂存、push、merge、切分支或修改 Git 配置。
- 未启动子代理或其他工蜂。未读取 `.env`、认证文件、钥匙串或凭据。未触碰其他 checkout、冻结包、主仓 WIP 或工位配置。

管理者证据目录：`/Volumes/AgentStorage/issue1393-sandcastle-20261018-management`。该目录只读，没有把外部 session 日志或原始大包复制入仓。

| 原件 | SHA256 |
|---|---|
| `issue1393.json` | `d2002eaf2ceab5b07d1205b12837017d8dcfd921395033ce5f73afb884444e04` |
| `comments1342.json` | `d6531cac6b5230c9ebb31fb7d79097340d53df16ba52c4cc912ac115f784a526` |
| `dependencies1393.json` | `b745ae20a195cad6e65e360b40aa23dda6874d7679e06a7de99295481083ad47` |

## reference-coding 门的实际结果

编码前通过 Python 标准库 `urllib.request` 分别向以下三个端点提交只读搜索（没有访问系统认证索引）：

```text
POST http://127.0.0.1:19200/nc1419gate-docs/_search
POST http://127.0.0.1:19200/nc1320src-docs/_search
POST http://127.0.0.1:19200/nc1320ref-docs/_search
```

三次请求的相同正文：

```json
{"size":3,"query":{"simple_query_string":{"query":"inclusion | replay | snapshot","fields":["content","text"]}}}
```

三次均返回 `URLError <urlopen error [Errno 1] Operation not permitted>`。这发生在建立请求的运行环境，不能推出索引不存在、无匹配、服务故障或认证有问题。没有改网络配置、重装、重索引、提权或绕过沙箱。

随后只读检查了管理者已保存的检索结果：

| 原件 | 实际结果与定位 | 借鉴情况 |
|---|---|---|
| `xerj-reference-search.json` | `references.hits.total=0`；`sources.hits.total=2`，均来自 `nc1419gate-docs`。`content_structure.rs` 文档 ID `088d49d1a65a785e77b1c0a4c09ec24e`，body 第 347 行 `restore_second_cursors`；`source_structure.rs` ID `f85c7d14db8066e3b6b4663b78b7c733`，body 第 1540 行调用 `parse_layer_with_inclusion_facts_store` | 用于确认 #1419 有独立状态/恢复接口，未当成当前 checkout 源码，未移植 |
| `xerj-ref-sample.json` | 命中总数 409，但本地仅有返回的小样本；`node.py` ID `7a03a018a7fa12f974ba888efb5db3e7` 与 `__init__.pxd` ID `8cca42251b0d49f3be836c5941245c25`，均来自 `nc1320ref-docs`。已看 `node.py` 的配置校验片段，不是 W 初始化或恢复实例 | 无可直接借鉴的包含实现；不宣称开源已有参考算法 |
| `xerj-ref-replay.json` | `hits.total=0` | 无 |

上述缓存的 SHA256 依次为：

```text
60d133f3ae79cbf3fd78d776b3d112b71abe1e880a772a8c1c42020f9ec46a37
19a83679e029de5e4cf37ef9ca83a1577771066a2031df3bb4303d0c49e8e0df
eb04fb5662574bc40e5e9a59bbb14bf0179b378fcecdbd38fac1a6b8aa3f4e36
```

缓存不能补足本轮对 `nc1320src-docs` 的检索，也不能把 Nautilus 的泛用配置代码当成初始化参考。原请求的 `content/text` 字段没有经 mapping 确认；已看到样本文档使用 `body`。解除阻塞后应确认目标索引字段，再检索 `body` 中的 inclusion、pending、replay、restore 等，或按任务许可在指定源码索引限定 `match_all` 小样本取回后本地检索。不得把本轮网络失败记为查询零命中。

**解除阻塞的实际缺项**：由可访问该本地服务的执行环境完成三个指定索引的只读检索，保存准确请求、响应、原件定位和借鉴结论；或者向本工位提供这些已完成的检索原件。补齐后沿同一已批语义继续，无须重新规划 SPEC。本轮没有自动把这道用户明示的门降级。

## 实现与剩余义务对账

以下行号均在固定 HEAD 实读。可用图工具列表未暴露 `search_graph`/`trace_path`/`get_code_snippet` 等接口，故按允许的回退路径使用定向 `rg` 和文件读取。

| 落点 | 当前实际行为 | #1393 剩余义务 |
|---|---|---|
| `rust/src/theta_v0/parser/inclusion.rs:41`、`:52`、`:80` | `contains`、`merge`、`strict_dir` 已是明确几何核 | 复用，不改几何 |
| 同文件 `:110`、`:430` | batch 与旧 `IncrInclusion` 已扫描首个 raw 相邻严格对，方向出现时从 raw0 左折叠 | 复用计算；未定时 `merged` 仍返回 raw 副本，须明确区分原始材料与正式组，不能沿该字段制造组身份 |
| 同文件 `:216` | `dense_annotation` 给每根 raw 分配 `usize group`，即使 `only_open_tail` 也只把 confirmed 设 false | 未定记录必须无正式归组；同步消费方，不能用组号 0 充当未定 |
| 同文件 `:334` | `IncrInclusion::from_result` 从末两组合成方向；只有一组时默认 UP | 已知方向折成单组时无法由这个快照恢复方向；须保存真实状态或使用原始证据重建，禁止默认方向，见工作例 U10 |
| 同文件 `:603` | facts 路径首根创建 `seed` 组；首次包含置 `initial_direction_unsettled`，后续只追加 waiting，不再尝试解锁 | 空/单根/全包含全部保持未定原件；首对出现后重折并保留首对证据；更新后按实际 acc/incoming 方向 |
| 同文件 `:806`、`:821` | `IncrFactsState` 未定后立即 return，首根也已经进入 merged/groups | 保存 pending raw；继续检查 raw 相邻对；解锁时仅一次从起点重折；确认长度在未定时为 0 |
| `rust/src/theta_v0/parser/mod.rs:152`、`:197`、`:253`、`:328` | batch 生产走紧凑状态，完整观察走 facts；增量通过 `IncrStrokes` 内部紧凑状态装配 | 两条状态变化须一致；未定期不能由分型或尾部缓存偷生结构；raw 序位与 sparse source coordinate 分开 |
| `rust/src/theta_v0/parser/stroke.rs:180`、`:356`、`:570`、`:608` | 新笔检查未定标志；增量有 checkpoint，并追加未定 waiting reason | 解锁时验证 checkpoint、waiting reasons 和端点重算均恢复一致；保留 RD-02 极值集合与已裁新笔规则 |
| `rust/src/bin/s_session_v2/tb02a_facts.rs:113`、`:334` | 发布 inclusion_step、inclusion_group、知识状态；没有 #1393 的逐 raw Undetermined 身份合同 | 逐 raw 原件与 cut/first_known、初始化政策与首对来源；新组明确指向旧未定记录 |
| `rust/src/bin/s_structure_session.rs:1883` | `compute_diff` 仅在同 slot 替代时产生 replaces；跨 kind/slot 消失记 removal | 未定 raw→组的多对一替代不能只靠现有同 slot 对比；需要同一 R2 owner 的显式对应 |
| `rust/src/bin/s_session_v2/tb02a.rs:282`、`audit.rs:307` | `read_facts` 按 first_known/death 过滤 AsKnown；audit 从持久撤回重建 replaces | 可复用历史过滤/审计框架；必须验证新增替代关系同时贯穿批次、索引、delta、恢复，旧 cut 不回填 |
| `rust/src/bin/s_structure_session.rs:63`、`:1330`、`:2048` | 当前 rule 为 `s2-axis-quantifiers+new-bi/1+segment-second/1`，入口核版本 | 为初始化政策增量编码新 rule；与 #1419 的新规则组合，不能用当前旧串覆盖对方的 `/2` 等片段 |
| `formal/Origin/` | 本轮具名查询 `RawInclusion/rawInitialization/raw_initialization/initial_direction_unsettled/Undetermined` 未找到 raw 初始化定义 | 具名定义及空/单/首对/全包含分支证明待补；该检索不是对全 Lean 语义的穷举证明 |

`inclusion.rs:994` 的现有 `tb02a_initial_inclusion_does_not_borrow_future_direction` 明确断言延迟方向之后仍永久未定，这是 #1342 之前行为的锁。后续应按已批准 W 更新预期并补独立 oracle，不能通过 ignore 隐去。已定输入域的现有 `tb02a_trace_matches_existing_kernel_and_preserves_actual_roots`、`tb02a_direction_is_from_actual_accumulator_not_previous_raw` 可作针对性回归。

没有声称前置票的历史通过证据证明当前 #1393 已完成；本轮复用的是真实源码中的已有核、身份/历史机制与测试入口，未重跑旧全量成果。

## 已裁规则的独立手算工作例

下表是从 #1342 与 baohan v1.6 直接计算的预期，**未执行模型、未执行 Rust**。区间统一写 `[low, high]`，raw 坐标为到达序 0、1、2……。列出组时顺序为 `(锚, 区间, 成员)`；极值集合按到达序。它们不以当前生产输出为 oracle。

| ID | raw 输入/事件 | 预期 |
|---|---|---|
| U01 | 空 | 无 raw、无组、方向未定 |
| U02 | `[0,10]` | 一条 raw Undetermined；无正式组 |
| U03 | `[0,10],[2,8],[3,7]` | raw 相邻对全部包含；逐 raw 未定，无 envelope，无组 |
| U04 | `[0,10],[2,8],[1,11]` | 非链式但 raw 相邻全包含，仍全部未定；不能拿 raw0/raw2 定方向 |
| U05 | `[0,4],[1,5]` | 首对 UP；组 `(0,[0,4],[0])`、`(1,[1,5],[1])` |
| U06 | `[1,5],[0,4]` | 首对 DOWN；组 `(0,[1,5],[0])`、`(1,[0,4],[1])` |
| U07 | `[0,10],[2,8],[3,11]` | 首对 raw1/raw2 为 UP；组 `(0,[2,10],[0,1])`、`(2,[3,11],[2])`；首组 high_sources=[0]、low_sources=[1] |
| U08 | `[0,10],[2,8],[-1,7]` | 首对 raw1/raw2 为 DOWN；组 `(0,[0,8],[0,1])`、`(2,[-1,7],[2])`；首组 high_sources=[1]、low_sources=[0] |
| U09 | U07 后接 `[1,7]` | 实际 acc `[3,11]` 与 incoming 非包含 DOWN；新增 `(3,[1,7],[3])`，不得永远使用初始化 UP |
| U10 | `[0,10],[2,8],[1,7]`，随后接 `[2,6]` | 首对 raw1/raw2 为 DOWN，但首次重折结果只有 `(0,[0,7],[0,1,2])`；继续 DOWN 合并得到 `[0,6]`。现有 `from_result` 单组恢复默认 UP，按源码将得到 `[2,7]`；这是静态反例，待 Rust 运行确认 |
| U11 | `[0,4],[1,6],[2,6]` | 第二组 `(1,[2,6],[1,2])`，high_sources=[1,2]、low_sources=[2]；集合不选单根，不能改邻票准入 |
| U12 | 先提交 U07 的前两根为 cut-A，再提交第三根为 cut-B | A 中两条 raw Undetermined 与 first_known 保留；B 新组显式 replaces 旧未定记录；再查询 AsKnown(A) 仍只有原未定记录 |
| U13 | U12 后将 raw2 修订为 `[1,9]`，提交 cut-C | 当前有效输入再次 raw 相邻全包含，C 未定；A、B 的历史对象和原始修订都保留。之后即使修回旧数值也不能冒充旧事件/旧对象复活 |

后续还须对每个事件边界分别覆盖：同输入新进程重放、pending 中断恢复、激活后单组合并恢复、方向建立证据修订、规则版本改变、旧 cut 查询。手算表不替代这些执行义务，也不宣称 OHLC/volume/untradable 全字段逐位验证。

## 写集与 #1419 重叠面

**本轮实际写集只有本 `REPORT.md`，无产品代码、测试或 Lean 改动。** 没有新增已运行的纯模型验证实体；上节只有具名手算案例。报告保留为未暂存文件交管理者收包，尚不满足入 Git 封包门。

编码门解除后的必要候选写集（须由后续实际 diff 确认）：

- `rust/src/theta_v0/parser/inclusion.rs`：pending/激活/恢复状态与事实；几何原语不改。
- `rust/src/theta_v0/parser/{mod,stroke}.rs`：只处理初始化转换、装配与缓存失效，不扩展新笔判据。
- `rust/src/bin/s_session_v2/tb02a_facts.rs`：未定原件、来源、政策、替代投影。
- 同 owner 必要适配：`rust/src/bin/s_structure_session.rs` 的 rule 与跨 slot 替代；视实际表示适配 `s_session_v2/{tb02a,audit}.rs`。
- 本票独占定向测试、独立 oracle 资产与 raw 初始化 Lean 文件。

用户指定的 #1419 当前在制写集不可因本树没有对应文件而判废。已知重叠候选：`tb02a_facts.rs`、入口 rule 门，以及 parser facts 与 content producer 接口；其 `content_bi/content_producer/tb02b/tb02e`、v3 `content_structure/source_structure/generation` 须由根在组合树独审。本轮这些路径均未改，**实际重叠 diff 为空**。

组合时保留用户指定 `new-bi/2+segment-second/2+center-seed/1+center-pair/1`，只增加 #1393 初始化政策维度；不能拿本基线的旧 rule 串全量替换。不能把本轮缓存读到的 `CoordsRef`/`parse_layer_with_inclusion_facts_store` 名称视为已经存在于本树的接口。

## 票面 AC 状态

| #1393 验收条件 | 本轮状态 |
|---|---|
| 每个未建/建立/修订叶按获准实例真实运行，保原始输入及方向依据 | **NOT_RUN / 未满足**：只有静态定位和手算案例 |
| 已定输入域逐位相同，几何不变，补一般初始化成功 | **未满足**：产品未修改；几何未动是事实，但不等于已实现初始化或通过对拍 |
| 正式会话非真空输入，公共查询/变化与浏览器核对 | **NOT_RUN / 未满足**：本轮明确禁止真实会话、产品二进制和浏览器 |
| 独立 oracle，新进程重放及所改持久边界恢复对拍 | **NOT_RUN / 未满足**：提供 oracle 手算起点，尚无模型/恢复执行证据 |

RD01/W、逐 raw Undetermined、replaces、版本分离、历史 cut、Lean raw 初始化都仍有实施或验证义务。共享 family/A/故事清单不因本报告或前置 closed 而销项。不能关 #1393、父 TB、SPEC 或整图。

## 实际检查与 NOT_RUN

已运行：只读 Git 状态/HEAD/tree 查询；定向源码与合同读取；外部证据 JSON 解析及 SHA256；三次 XERJ 只读请求（全部失败）；报告写入后的差异/范围检查。

**全部 NOT_RUN**：cargo、rustc、产品二进制、Rust 编译与测试、rustfmt（无 Rust 改动）、Python 纯模型、真实会话/API/浏览器、Long/全量历史、真实资金/订单、lake/Lean 检验、fixture drift、CI。本轮 JSON 解析和文件散列不叫纯模型测试；源码推导不叫生产复现。

没有为获得绿灯改 ignore、缩 SPEC、变更准入几何或偷偷批准后续动作。

## 后续精确验证合同（建议，未授权执行）

1. **源绑定**：管理者取得检索原件后继续当前固定基线任务；封包记录实际 HEAD、产品 diff 与 SHA256、oracle 数据 hash。与 #1419 合成后另记录组合树及评审覆盖面；不能把本基线评审外推到组合结果。
2. **独立 oracle**：把 U01–U13 固化为完整 Bar、来源身份、每 cut 预期账；预期不能调用生产函数生成。每前缀比较 batch、紧凑增量、完整 facts；比较原始身份、组锚/成员/极值集合、方向证据、waiting、正式分型/笔/段输出。所有版本分别记，不用 object revision 代替 policy/raw/structure revision。
3. **测试入口**：新增本票独占测试建议统一前缀 `tb02u_`（当前尚未创建）；至少包含 `pending_raws_have_no_groups`、`first_raw_pair_replays_from_zero`、`non_chain_adjacent_containment_stays_pending`、`single_group_down_resume_preserves_direction`、`revision_removes_and_reestablishes_direction`、`old_cut_and_explicit_replaces`。名称是待创建合同，不是已存在或已运行测试。
4. **Rust 后续环境**：库的 CI 现有命令采用 `--features backtest_bin`；S 二进制 required feature 为 `s_session`，源是 `rust/src/bin/s_structure_session.rs`。在获准构建环境中，先以 `--list` 核实际测试名，然后限定 `cargo test --locked --lib --features backtest_bin tb02u_ --jobs 1` 与 `cargo test --locked --features s_session --bin s_structure_session tb02u_ --jobs 1`。已定域仅复用本报告点名的包含与 acc-direction 回归，不扩全量、不加 `--ignored`。这些命令本轮没有执行。
5. **正式持久与观察**：获准之后使用独立临时 S 存储和本票 TestOnly OHLC 输入，走既有 AcceptInput→Advance→公共 Snapshot/Watch/AsKnown→同源浏览器。对 U12/U13 保留 raw revision、first_known、rule、结构版本、replaces 全字段；新进程相同输入重放，并在 pending、首对激活、修订各边界中断/恢复。现有 `tb02a::read_facts` 与 audit 均须覆盖，不能只测内部 builder。
6. **Lean**：新增具名 raw 初始化定义，明确首对搜寻与 W 分支；空/单/全相邻包含保持未定、首对确定后自 raw0 折叠均有无 sorry 的证明。源码静态检查不等于证明编译。只在另行批准的环境执行定向 Lean 检验；fixture drift 会触发构建，本轮不运行。
7. **资源与监督**：建议获准验证时单任务、`--jobs 1`，先使用最多 16 根 toy raw、至多两次修订和固定语义时钟；每个测试超时 60 秒，首次依赖构建须管理者单独分配窗口。运行者记录实际 wall time、峰值内存、退出码、源/参数/feature、差异与中断点；无读数就记 NOT_MEASURED，不预写资源达标。超时或失败保留证据并归管理者，不自行改判据、启动 Long 或真实业务。

结论仅为**静态对账交付、编码门阻塞**。后续仍需完成产品实施、定向执行、独立评审及管理者封包；本报告不声称 #1393 已完成。
