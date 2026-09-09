# #1371 TB-01-B 实施报告（修订撤回、持久 Delta 与真实进程恢复）

> 票：GitHub Issue #1371（TB-01-B 内部执行单元，SPEC #1340 / 图 #1323 / SPEC #1339 R2）。
> 实施分支：`codex/1323-tb01-b-1371`；精确基线：`2212eba31e0b7419907e622e79f02f7c2cc37706`。
> 编号声明：本提交中的 #1371 仅指 GitHub Issue。
> 本报告是工作草稿（活期绑定票），不是独立评审，也不以自审 PASS 冒充独评。

## 0. 结论（一句话）

在 A 已合的正式结构会话之上，为同一严格 Rust 核补齐 B 片：源事件合法修订链（同 revision 幂等/异内容冲突/
新 revision supersedes、旧版不复活）、有效源位置语义（晚到新版不算第五 tick）、持久 StructureDelta（原子应用
对象/边/端点/见证）、对象撤回（原身份/first_known/发生区间/撤回理由保留）、真实 SIGKILL 三点同 DB 恢复、
正式 writer 单调换代与旧 writer 拒绝、AsKnown（`--as-of`/`?as_of=`）/ RecomputedWithRevision 分栏，以及
同源 Watch（`watch`/`/api/delta`）续接；浏览器经原子 delta 看到旧 TOP 撤回、替代 RISING 与后续新 TOP，并可
追查源修订史。E/B/X 全程未启动，S 修订/撤回/历史/追加/恢复独立成功；S 自身损坏报 StorageUnavailable。

## 0a. 本片最小改动面（只改本票模块）

| 文件 | 改动 |
|---|---|
| `rust/src/bin/s_structure_session.rs` | 修订链接受（raw_events 主键 `(identity_key, revision)` + `supersedes_revision`）；有效源位置计算；对象生命周期（`first_known_generation/cut`、`withdrawn_generation/reason/superseded_by`）；`structure_deltas`（session/generation/catalog_revision/base_cut/next_cut/seq_range/input_frontier/index_frontier/完整 upsert/撤回/替代/见证/关系/观察/raw_history_added）；`recover`（严格单调换代 + 清未决 Begin + 核可达根）；`watch`（同源 delta + Gap）；`snapshot --as-of`；具名 TestOnly 暂停点（`S_SESSION_PAUSE`）；18 项 bin 测试 |
| `s_session/s_readonly_server.py` | `read_snapshot` 支持 `as_of` 全字段过滤（对象/见证/关系/观察/raw_history）；`/api/delta`（同源 Watch）；query string 解析（`:310` 起曾丢 query string 已补）；StorageUnavailable 不变 |
| `s_session/browser/index.html` | 历史模式切换（RecomputedWithRevision / AsKnown）；撤回对象/源修订链渲染；同源 `/api/delta` 续接 + 与同 cut Snapshot 全字段逐字段对拍（对象/撤回/见证/关系/观察/源历史） |
| `s_session/launch_s.sh` | `--writer-epoch` 透传（默认 1），其余流程不变 |

未撤销其他编辑；未切 main、未 push、未 merge、未 gh 写/关票、未改全局服务/生产账户凭据、未运行真实交易。
`formal/` 未改（无 fixture 漂移）；`inclusion.rs`/`local_shape.rs` 未改（A 已合，未扩大 formal 范围）。

## 1. 逐验收条件证据（AC1..AC8）

### AC1 —— 正式 launcher + 真实输入链（TOP → 更正 RISING → 新 TOP）

- 正式 launcher（`./s_session/launch_s.sh`，已加 `--writer-epoch` 透传）端到端回归实跑退出 0：
  `init`（116 目录 / wal / synchronous=2 / sqlite 3.53.2）→ `accept`（7 条）→ `advance`
  （5 对象 = RISING/TOP/FALLING/BOTTOM/RISING）→ 只读外壳 → `curl /api/state` 与 `/` 200。
- 三段修订轨迹（同 launcher 的 init/accept/advance 命令，按 B 票需要多次调用；退出码全 0）：
  - `accept in1.json`（e0/e1/e2=10000/11000/10500）→ `advance` → 快照活动对象恰 `TOP@[0,1,2]`（generation=1）。
  - `accept in2.json`（e2 revision=2 → 11500）→ `accept` 输出 `supersedes_revision=1` → `advance` →
    活动 `RISING@[0,1,2]`、撤回 `TOP@[0,1,2]`（`withdrawal_reason=superseded_by_revision`、
    `superseded_by=RISING.object_id`、`first_known_generation` 保留 1）。
  - `accept in3.json`（e3=10800）→ `advance` → 活动 `RISING@[0,1,2]` + `TOP@[1,2,3]`。
- 晚到臂（e0/e1/e2/e3 先到，后 e2 更正）：更正前 `TOP@[0,1,2]`+`BOTTOM@[1,2,3]`；更正后活动
  `RISING@[0,1,2]`+`TOP@[1,2,3]`、撤回 `TOP@[0,1,2]`+`BOTTOM@[1,2,3]`；`raw_history` 仍只 4 个有效
  源坐标（0/1/2/3），晚到 e2 新版不当作第五 tick。
- 更正路径：`advance` 先短写事务核 writer_epoch、持久 Begin（`advance_state=begun:*`），再解释，Commit
  同事务发布撤回/替代；撤回不是前端/独立解释器补造（见 AC4 三点真杀与 `local_shape` 单测 oracle）。

### AC2 —— 源事件 revision / 窗口身份 / 结构 revision 分立 + 幂等/冲突/不复活

- `raw_events` 主键 `(identity_key, revision)`，`identity_key` 不含 revision；`revision` 为规范 i64
  （wire 输出规范十进制字符串）。`input_revision`（原始文本）与 `object_revision`（对象内容寻址=1）分列。
- 同内容重投：`accept in1.json` 重跑 → 全部 `status=replay` 返回原 `receipt_id`（幂等）。
- 同 revision 异内容：`accept`（e2 rev=1 改价 9999）→ `IdentityConflict`（原 receipt 与 payload_hash 分列）。
- 新 revision：`accept in2.json` → `status=accepted` + `supersedes_revision=1`；关系表含
  `supersedes`（`identity@2 → identity@1`）与 `replaces`（RISING → 旧 TOP）。
- 旧版重放不复活：e2 rev=1 再投 → `replay`（不复活旧 TOP、不改有效值 11500）。
- 更低 revision 晚到（已见更高）：`InvalidDomain`（out_of_order_revision_after_later_accepted），零写入。

### AC3 —— 同一严格核 + oracle 复用

- 复用 A 已合 `theta_v0::classifier::local_shape`（同一 `ParseLayerIncr` + `classify_local_shape`，
  未写第二份判定器）。`cargo test --lib local_shape` → 4 passed（四分支 oracle 原样）。本片未改
  `inclusion.rs`/`local_shape.rs`。
- 会话实测：10000/11000/10500 → TOP；10000/11000/11500 → RISING；11000/11500/10800 → TOP
  （每个对象携 dir_ab/dir_bc + 四个严格比较 + 三根 raw/merged 见证，`held` 全 true）。

### AC4 —— 持久 StructureDelta + 原子应用 + 同 cut 逐字段相等

- `structure_deltas` 每代一行，含 session/generation/catalog_revision/base_cut/next_cut/seq_range/
  input_frontier/index_frontier + 完整 delta_json（upserts 全对象 wire 形态 / withdrawals / replaces /
  witnesses / relations / observations / raw_history_added）。
- 对象/边/端点同一提交事务原子应用；见证/关系/观察 `published_generation` 首获知固定（INSERT OR IGNORE）。
- 同源对拍：Node 复算（与 `browser/index.html` 的 `applyStream`+`canonicalFieldCompare` 同款逻辑）从
  `after_generation=0` 逐代应用 delta，最后与 `/api/snapshot` 全字段（对象/撤回/见证/关系/观察/源历史）
  逐字段相等 → **RESULT: 全字段逐字段一致 PASS**（缺右邻 cut → TOP → 撤回 RISING → 新 TOP，共 4 代）。
- 重复投递（gen ≤ cursor）不重复生效；乱序（期望 gen≠cursor+1）显式 Gap；`watch` 缺口返回 `rebuild_cut`。

### AC5 —— 真实 SIGKILL 三点同 DB 恢复（正式入口，非函数 return/mock）

三点均对精确 S PID 发真实 `SIGKILL`（`wait_exit=-9`，`/proc/<pid>/cmdline` 核 s_structure_session+advance+db），
再以正式入口打开同一 DB：

| 点 | 实际落点（kill 后持久状态） | 恢复 |
|---|---|---|
| ① Begin 后 | `advance_state=begun:*`、generation=0、batches=0、cut-0 | `recover --new-epoch 2` 清门+换代；`advance --writer-epoch 2` → TOP（gen1） |
| ② 完整 batch 后、Commit 前 | `begun:*`、generation=0、batches=1（不可达）、cut-0、index_frontier="" | recover 后 `advance --writer-epoch 2` 重算发布 gen1；不可达 batch 未冒充已发布 cut |
| ③ Commit 后、回执前 | `idle`、generation=1、batches=1、cut-1、objects_active=1 | 同输入重放 `accept` → 全 `replay` 原收据（DeliveryUnknown 按原业务身份查询权威结果） |

- 恢复读集：重开同 DB 读原接纳序/身份（raw_events）、未决 Begin、可达根（structure_cut/index_frontier）、
  writer 代际；未完修订门保持关闭（旧 epoch `advance` 被 `StaleWriter` 拒绝），`recover` 合法恢复后新 writer 继续。
- 不可达 batch：②点 batch 已写但 index_frontier 仍空/旧，恢复不把它当已发布 cut；不产生幽灵引用、不漏已提交 cut。
- 真实换代边界：`recover` 换代后旧 epoch 进程 `accept`/`advance` 均被拒绝（exit 1，零写入）。

### AC6 —— AsKnown / RecomputedWithRevision 分栏 + 缺右邻不提前 TOP

- `snapshot`（默认）= RecomputedWithRevision：绑定最新 input/rule 版本（`rule_revision=s2-axis-quantifiers`，
  对象 input_refs 绑最新 revision/收据）。
- `snapshot --as-of N`（/api/snapshot?as_of=N）= AsKnown：按当时可知全字段过滤。
  - 缺右邻 cut（gen1，两点）：活动对象空、`insufficient_knowledge`（fewer_than_three_bars）观察、无 TOP。
  - AsKnown gen2（TOP 刚形成）：活动 TOP（lifecycle=active，无撤回元数据）、raw_history 3 条、无 replaces/supersedes。
  - AsKnown gen3（更正后）：活动 RISING、撤回 TOP、raw_history 4 条、关系含 replaces+supersedes。
  - 晚到更正不回填过去 first_known：旧 TOP `first_known_generation=1` 保留，不因 gen3 回看被覆盖。
- 当前集合移除不删历史：`withdrawn_objects` + `raw_history` 完整保留旧 TOP 原身份/发生区间/原始输入与比较见证/撤回理由。

### AC7 —— E/B/X 未启动仍独立成功 + StorageUnavailable

- 全程只启动 S（一次性 Rust 命令）与只读外壳；未建 E/B/X 库、未加载经济政策；`scope={"structure":"CompleteCut","economic":"not_started"}`。
- S 自身损坏（覆盖库头为垃圾字节）：`accept`/`advance`/`meta` 均报 `file is not a database`（exit 1，不静默新建空库）；
  Python `/api/state` 返回 HTTP 503 `StorageUnavailable`，不伪造成功。
- 平台前件：Linux 容器实测 sqlite 3.53.2（≥3.51.3 WAL-reset）、journal_mode=wal、synchronous=2（FULL）、
  platform=linux；macOS fullfsync/GUI 未验（见 §NOT_RUN）。分页/保留/过期压力矩阵留 TB-01-C。

### AC8 —— 证据保留 + 检查运行 + 未验项

- 保留：命令/退出码、输入/版本/构建/目录 hash、PID/信号/退出/重启/持久状态、API 前后、Node 全字段对拍、
  杀点原始输入（`/tmp/s_b_inputs/*`、`/tmp/s_b_kill_*`）。Rust 检查：fmt 0、check 0、clippy 本片 0 命中、
  bin 测试 18 passed（A 原 9 + B 新 9）、`--lib local_shape` 4 passed。

## 2. 17 细项绑定

| 细项 | 本片实际证据（入口/命令/退出码/输出） |
|---|---|
| A-CC-006-03 | 三段修订轨迹 + 三点真杀同 DB 恢复（`/tmp/s_b_kill_test.py`，wait_exit=-9 三点）；同一 `local_shape` 核对象/关系/身份/见证一致（18 项 bin 测试 + oracle 4 passed） |
| A-CC-006-04 | 正式更正产生撤回/RISING/后续 TOP（AC1）；浏览器可查四比较/三根见证/历史（`/api/state`、`/api/snapshot?as_of=N`；HTML 渲染函数含 input_refs/比较/见证/源修订链） |
| A-ST-044-02 | 持久 Delta 原子应用对象/边/端点/见证（AC4）；Node 全字段对拍 = 同 cut Snapshot（RESULT PASS） |
| A-ST-044-03 | 跨修订断连 + 进程恢复按原游标接回结构及 AsKnown（AC5 三点 + AC6 as-of）；C 完整分页/保留/过期压力矩阵保留 TB-01-C |
| A-ES-15-01 | E/B/X 未启动 S 修订/历史/恢复独立成功；scope 明确结构 CompleteCut 经济 not_started，结构 frontier 不冒充经济 ack（AC7） |
| A-ES-15-02 | Begin/batch/Commit/DeliveryUnknown/恢复真实迁移与拒绝有前后持久证据（AC5 kill 前后 meta/batches/objects 读数 + 旧 epoch 拒绝 exit 1） |
| A-RA-04-01 | 旧 TOP 首获知（first_known_generation=1）、原身份/证据/撤回史保留；本片不销经济计划/成交责任分派（scope economic not_started） |
| A-RA-10-01 | 同 cut Snapshot/Delta/AsKnown 字段一致（Node 全字段对拍 PASS）；RecomputedWithRevision 另具名绑定最新 input/rule 版本；不销完整经济历史 |
| A-I-02-01 | 经济未启动 S 成功（AC7）；S 自身损坏报 StorageUnavailable 且停写（exit 1 / HTTP 503），不声称全域故障乘积通过 |
| A-OB-004-01 | 版本分立（raw revision / object_id / object_revision）、更正撤旧/替代链（supersedes/replaces）、旧版不复活（AC2） |
| A-OB-008-01 | 持久 Delta 重复/乱序/断连处理 + 原子应用 + 同 cut 全字段比较（AC4 Node PASS；watch Gap） |
| A-OB-009-01 | 缺右邻不提前 TOP（gen1 无对象 + insufficient_knowledge）；晚到更正不回写旧 first_known（AC6） |
| A-OB-014-01 | 无经济事件不补造历史经济应用；本片不销未发/可能已发/已成交责任（经济未启动，无经济写入） |
| A-OB-015-01 | 同一 session/generation、旧代际消息隔离（delta gen 严格 +1，乱序显式 Gap；recover 换代旧 epoch 拒绝） |
| A-OB-016-01 | 撤回断连、真实恢复后浏览器收敛到同 cut Snapshot（Node 全字段对拍；GUI 未验见 §NOT_RUN） |
| A-OB-017-01 | 对照本域对象/边/事件/身份/获知史（raw_history 含 supersedes；AsKnown 按前沿过滤）；无未来输入偷看、无 bars 造经济恢复 |
| A-OB-020-01 | 每项保存构建/输入/平台/故障/PID/信号/退出/重启/DB/API/GUI 证据（`/tmp/s_b_*`；GUI 未验已标 NOT_RUN） |

## 3. 已执行命令与退出码（摘要）

| 命令 | 退出码 |
|---|---|
| `cargo fmt -- --check` | 0 |
| `cargo check --features s_session --bin s_structure_session` | 0 |
| `cargo clippy --features s_session --bin s_structure_session` | 0（本片文件 0 命中） |
| `cargo test --features s_session --bin s_structure_session --jobs 1` | 0（18 passed） |
| `cargo test --lib local_shape` | 0（4 passed，A oracle） |
| `./s_session/launch_s.sh --testonly --db … --port 8793 --writer-epoch 1` | 0 |
| `s_structure_session accept/advance/snapshot/watch/recover/meta`（本片各轨迹） | 0（拒绝路径 1） |
| `/tmp/s_b_kill_test.py`（三点 SIGKILL+恢复） | 0（每点 wait_exit=-9） |
| `/tmp/s_b_node_verify.mjs`（全字段 delta→snapshot 对拍） | 0（RESULT PASS） |

平台：Linux aarch64 容器；SQLite 3.53.2（bundled，libsqlite3-sys 0.38.2）；rustc 1.97.1。

## 4. 哈希

- SPEC 清单 SHA256（票面给定）：`38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`。
- 已签目录 `signed-catalog.json`：`938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`（与 A 一致，未改）。
- TestOnly profile：`2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc`（与 A 一致，未改）。
- 本片四文件改动后 SHA256：
  - `rust/src/bin/s_structure_session.rs`：`0c110dbea358cbbfaafcc3759cd2b5d9931ae6b2e045b88d070ef18812b638b7`
  - `s_session/s_readonly_server.py`：`db8998936c5719539e672a453ac2c1b942e84a46baa87bf7d889edf675b147f5`
  - `s_session/browser/index.html`：`5c311db8a9f3577379a7719289b68d34cddc72aa365014fe5573afa3a9bf42f0`
  - `s_session/launch_s.sh`：`65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d`
- 本机二进制 `rust/target/debug/s_structure_session`：`3daf4e5f30bb3c75b7c1fdf1bf9ce327dba074c50b5912d5306dc03b04571a3e`。

## 5. NOT_RUN / 未验（照实，交给根接回）

- **真实 GUI 浏览器渲染**：容器无 chromium/firefox。已交付可运行 `browser/index.html`（vanilla JS，
  同源 `/api/state` + `/api/snapshot?as_of=` + `/api/delta`），并用 Node 复算同款 `applyStream`+`canonicalFieldCompare`
  逻辑对拍真实 API 响应（全字段一致）。**实际浏览器 DOM/交互渲染未验**，标 NOT_RUN。
  恢复命令：`./s_session/launch_s.sh --db <db> --port 8787` 后打开 `http://127.0.0.1:8787/`，
  按钮「RecomputedWithRevision / AsKnown 回看 generation / 读取新 Delta」。
- **macOS native 构建 / fullfsync / GUI**：本容器只实测 Linux aarch64；macOS 由根接回。
- **>2^53 精确 wire 经真实浏览器渲染**：已在 Rust CLI 与 HTTP API 实测（`/tmp/s_b_inputs/big*.json`，
  源坐标 9007199254741000-1003、revision 9007199254740993/994 全程规范十进制字符串）；浏览器 DOM 渲染该值未验（无浏览器）。
- **全量 cargo test / 全历史重放**：按有界验证优先，只跑本片 bin 18 测试 + `--lib local_shape`；未跑全量。
- **分页 / 保留 / 过期压力矩阵 / 跨域经济 AsKnown / 经济故障隔离**：留 TB-01-C 与后续 TB（本片不销项）。
- **A-OB-014-01 / RA-04 的经济计划、已交付/可能调用、已成交责任分派**：本片不销项（经济未启动）。
- **I-02 / RA-10 全域故障分区及完整经济 AsKnown**：留后续 TB。
- **A-ST-044-01 高级扩展关系**：本片未构造。

## 6. 待根验项（不代独评）

1. 真实 GUI 浏览器渲染与 AsKnown/Recomputed/Delta 交互（恢复命令见 §5）。
2. macOS native 构建与 fullfsync 前件（本容器未验）。
3. 独立评审会话（本报告非独评）。
4. main 合入、真实外效、未决 G/FU（本片未授权）。
