# #1371 TB-01-B 独立评审报告

> 编号声明：本提交中的 #1371 仅指 GitHub Issue。
> 本报告是独立评审（非作者自审）。评审只写本片；整图仍是 #1323 六 Destination，SPEC #1340/#1339 R2 已批准。
> 差异基线 BASE=`2212eba31e0b7419907e622e79f02f7c2cc37706`；待审提交 REVIEW_HEAD=`8c868f968c5f937870a4305dca05a0e0162b5131`；分支 `codex/1323-tb01-b-1371`。

## 0. 结论

**verdict：INCOMPLETE**（Linux/aarch64 容器内可验证面全部通过、无 H/M 缺陷；真实浏览器 GUI 与 macOS native/fullfsync 两项必需验证在本容器不可达、尚未接回，按票面「必需验证尚缺不得整体 PASS」不给 PASS）。

- 容器内源码与 Linux 证据面：`rust/src/bin/s_structure_session.rs` + `s_session/s_readonly_server.py` + `s_session/browser/index.html` + `s_session/launch_s.sh` 的修订链/撤回/持久 Delta/三点真杀恢复/换代/AsKnown/Watch/StorageUnavailable 全部独立实跑通过；同 cut Delta→Snapshot 全字段对拍 PASS。
- 未验证但必需、交根接回：①真实浏览器 DOM 渲染与 AsKnown/Recomputed/Delta 交互（本容器无 chromium/firefox）；②macOS native 构建与 fullfsync 前件。
- 本片不销项：E/B/X 经济、A-OB-014-01/RA-04 经济计划/成交责任分派、I-02/RA-10 全域故障分区与完整经济 AsKnown、分页/保留/过期压力矩阵（TB-01-C）、A-ST-044-01 高级扩展关系。

## 1. 差异与范围核验（ACTUALLY_RUN）

- `git rev-parse HEAD` = `8c868f968c5f937870a4305dca05a0e0162b5131`（等于 REVIEW_HEAD）；工作树干净；分支 `codex/1323-tb01-b-1371`。
- `git diff --name-only BASE..REVIEW_HEAD` 仅 7 文件：
  `.chanlun/agent-roster-2026-09-10.md`（新）、`.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.{json,md}`、`rust/src/bin/s_structure_session.rs`、`s_session/browser/index.html`、`s_session/launch_s.sh`、`s_session/s_readonly_server.py`。
- 未触碰：`formal/`、`rust/src/theta_v0/parser/inclusion.rs`、`rust/src/theta_v0/classifier/local_shape.rs`、`s_session/catalog/signed-catalog.json`、`s_session/profiles/*.json`（与 A 同 hash，diff 为空）。
- 本片 4 个生产文件 SHA256（与作者报告逐字一致）：`s_structure_session.rs`=`0c110dbe…b638b7`、`s_readonly_server.py`=`db899893…147f5`、`index.html`=`5c311db8…42f0`、`launch_s.sh`=`65cb1552…7d1d`。目录=`938b0ef5…08069`、profile=`2cd50e43…bdcfc`（与 A 一致）。
- 本机构建二进制 `rust/target/debug/s_structure_session` SHA256=`3daf4e5f30bb3c75b7c1fdf1bf9ce327dba074c50b5912d5306dc03b04571a3e`（与作者报告一致）。

## 2. 检查运行（本容器实际执行，exit 码）

| 命令 | exit | 结果 |
|---|---|---|
| `cargo build --features s_session --bin s_structure_session` | 0 | 链接成功（PYO3_PYTHON=uv cpython-3.11 aarch64） |
| `cargo fmt -- --check` | 0 | 0 差异 |
| `cargo check --features s_session --bin s_structure_session` | 0 | 0 error |
| `cargo clippy --features s_session --bin s_structure_session` | 0 | 本片文件 `s_structure_session.rs` 0 命中 |
| `cargo test --features s_session --bin s_structure_session --jobs 1` | 0 | 18 passed / 0 failed |
| `cargo test --lib local_shape` | 0 | 4 passed（A 已合 CC-006 四分支 oracle） |

二进制运行时自报：sqlite_version=`3.53.2`（bundled libsqlite3-sys 0.38.2，≥3.51.3 WAL-reset 修复）、journal_mode=`wal`、synchronous=`2`(FULL)、platform=`linux`、writer_epoch=`1`。

## 3. 逐验收条件（AC1..AC8）

### AC1 正式 launcher + 真实输入链（TOP→更正 RISING→新 TOP）——Linux 全过
- `./s_session/launch_s.sh --testonly --db … --port 8794 --writer-epoch 1` exit 0：init(116 目录/wal/synchronous=2/3.53.2)→accept(7)→advance(5 对象=RISING/TOP/FALLING/BOTTOM/RISING)→只读外壳→`/api/state` 200（gen1、5 对象）→`stop`。
- 本片三段轨迹（自建 `in1/in2/in3.json`，exit 全 0）：
  - `accept in1`（e0/e1/e2=10000/11000/10500）→`advance`→活动 `TOP@[0,1,2]`（generation=1，first_known_generation=1）。
  - `accept in2`（e2 rev=2→11500，输出 `supersedes_revision=1`）→`advance`→活动 `RISING@[0,1,2]`+撤回 `TOP@[0,1,2]`（withdrawal_reason=`superseded_by_revision`、superseded_by=RISING.object_id、first_known_generation 保留 1）；关系含 `replaces`(RISING→TOP) 与 `supersedes`(e2@2→e2@1)。
  - `accept in3`（e3=10800）→`advance`→活动 `RISING@[0,1,2]`+`TOP@[1,2,3]`。
- 晚到臂（e0..e3 先到后更正）：更正前 `TOP@[0,1,2]`+`BOTTOM@[1,2,3]`；更正后活动 `RISING@[0,1,2]`+`TOP@[1,2,3]`、撤回 `TOP@[0,1,2]`+`BOTTOM@[1,2,3]`；`raw_history` 5 行仍只 4 个有效源坐标（0/1/2/3），晚到 e2 rev2 不当作第五 tick。
- 更正路径无前端/独立解释器补造：`advance` 先短事务核 writer_epoch + 持久 Begin（after_begin 杀点实测 `advance_state=begun:…`），再解释，Commit 同事务发布撤回/替代。

### AC2 revision/窗口身份/结构 revision 分立 + 幂等/冲突/不复活——Linux 全过
- `raw_events` 主键 `(identity_key, revision)`；`identity_key=(namespace|epoch|instrument|event_id)` 不含 revision；wire 输出 revision/seq 为规范十进制字符串。
- 同内容重投→`replay` 返回原 receipt；同 revision 异内容（e2 rev1 改价 9999）→`IdentityConflict`（原 receipt 与 payload_hash 分列）；新 revision→`accepted`+`supersedes_revision=1`；旧版重放→`replay`（不复活旧 TOP、有效值仍 11500）；更低 revision 晚到→`InvalidDomain(out_of_order_revision_after_later_accepted)`，零写入。
- 新 revision 换源坐标（e2 rev3 seq=3≠prev coord 2）→exit 1 `IdentityConflict：…同一业务身份不得换源位置`，零写入。

### AC3 同一严格核 + oracle 复用——Linux 全过
- 本片未改 `inclusion.rs`/`local_shape.rs`；分类唯一来自 `theta_v0::classifier::local_shape::classify_local_shape`。`cargo test --lib local_shape` 4 passed（四分支 oracle 原样）。
- `--testonly`（cc006_four_branch）实跑 5 对象分支序列 RISING/TOP/FALLING/BOTTOM/RISING，与 A oracle 一致。

### AC4 持久 Delta + 原子应用 + 同 cut 逐字段相等——Linux 全过
- `structure_deltas` 每代一行（session/generation/catalog_revision/base_cut/next_cut/seq_range/input_frontier/index_frontier + 完整 delta_json：upserts/withdrawals/replaces/witnesses/relations/observations/raw_history_added）。
- 独立对拍（本评审用 Python 复刻 `applyStream`+全字段比较，两轨迹）：从 `after_generation=0` 逐代应用 delta 后，与 `snapshot`（RecomputedWithRevision）对 objects/withdrawn/witnesses/relations/observations/raw_history 全字段逐字段比较——主 3 代轨迹与晚到 2 代轨迹均 **RESULT: 全字段逐字段一致 PASS**（对象 2/撤回 1/见证 9/关系 14/源事件 5；对象 2/撤回 2/见证 12/关系 19/源事件 5）。
- 重复投递（gen≤cursor 不重生效）、乱序/断档（手动删除 gen1 delta 后 `watch --after-generation 0` 返回 `gap.reason=cursor_stale_or_retained_delta_missing`、`rebuild_cut=cut-3`）、`after_generation` 超出当前代（无 gap 无 delta）。

### AC5 真实 SIGKILL 三点同 DB 恢复——Linux 全过（真实信号/退出/重启证据）
三点均对精确 S PID 发 `SIGKILL`，`os.waitpid` 返回 status=`9`，`/proc/<pid>/cmdline` 实测含 `s_structure_session advance --db <db> --writer-epoch 1`；再以正式入口开同一 DB：

| 点 | kill 后持久状态（实测） | 恢复 |
|---|---|---|
| ① Begin 后（pause=after_begin） | `advance_state=begun:2883:…:1:2`、generation=0、batches=0、objects=0、cut-0 | 旧 epoch `advance` 被拒（`StaleWriter：已有进行中的 advance…请先经 recover`）；`recover --new-epoch 2` 清门+换代；`advance --writer-epoch 2`→gen1 TOP |
| ② 完整 batch 后、Commit 前（pause=after_batch） | `begun:…`、generation=0、batches=1（不可达）、objects=0、structure_deltas=0、index_frontier=""、cut-0 | recover 后 `advance --writer-epoch 2` 重算发布 gen1；不可达 batch 未冒充已发布 cut（index_frontier 空） |
| ③ Commit 后、回执前（pause=after_commit） | `idle`、generation=1、batches=1、objects=1、structure_deltas=1、cut-1 | 同输入重放 `accept`→全 `replay` 原收据（DeliveryUnknown 按原业务身份查询权威状态），generation 仍 1、raw_events 仍 3 |

- 恢复读集：重开同 DB 读原接纳序/身份（raw_events）、未决 Begin（advance_state/begin_token）、可达根（structure_cut/index_frontier，核 batch 存在）、writer 代际（writer_epoch）。
- 引用完整性：手动删除已发布 batch 后 `recover --new-epoch 2`→exit 1 `StorageUnavailable：可达根 index_frontier=… 无对应不可变 batch`，且无部分换代（writer_epoch 仍 1、advance_state 仍 idle）。
- 换代：`recover` 严格单调（new_epoch≤cur 拒绝）；换代后旧 epoch `accept`/`advance` 均被拒（exit 1，raw_events 计数不变=零写入），新 epoch 成功；`writer_epoch_history` 记录 `(from,to,generation_at_transition)`。

### AC6 AsKnown / RecomputedWithRevision + 缺右邻不提前——Linux 全过
- `snapshot`（默认）=RecomputedWithRevision（绑定最新 input/rule 版本）；`snapshot --as-of N`（`/api/snapshot?as_of=N`）=AsKnown。
- 缺右邻（两点）：活动对象空、`insufficient_knowledge(fewer_than_three_bars)`、无 TOP、raw_history 2。
- AsKnown gen1：仅旧 TOP（lifecycle=active，无撤回元数据）、raw_history 4、无 replaces/supersedes。
- AsKnown gen2（更正后）：活动 RISING+TOP@[1,2,3]、撤回 TOP@[0,1,2]+BOTTOM@[1,2,3]、raw_history 5、关系含 replaces+supersedes。
- 晚到更正不回填旧 first_known：旧 TOP `first_known_generation=1` 保留；新 RISING/新 TOP `first_known_generation=2`（当时未知不提前出现）。
- 当前集合移除不删历史：`withdrawn_objects`+`raw_history` 完整保留旧 TOP 原身份/发生区间/输入与比较见证/撤回理由。

### AC7 E/B/X 未启动仍独立成功 + StorageUnavailable——Linux 全过
- 全程只启动 S（一次性 Rust 命令）与只读外壳；未建 E/B/X 库、未加载经济政策；`scope={"structure":"CompleteCut","economic":"not_started"}`。
- S 自身损坏（覆盖库头为垃圾字节）：`accept`/`advance`/`meta` 均 exit 1 `file is not a database`（不静默新建空库，文件未被截断/重建）；Python `/api/state|catalog|snapshot|meta` 均 HTTP 503 `StorageUnavailable`。

### AC8 证据保留 + 检查 + 未验项——本评审独立收集
- 保留（/tmp/s_review_b_kill/）：杀点原始输入、marker（pid/stage/db/exe）、`/proc/<pid>/cmdline` 读数、SIGKILL wait 状态、kill 前后 meta/batches/objects/structure_deltas 读数、API 前后响应、两轨迹全字段对拍脚本、损坏库 503 读数、bigint 往返读数。
- 未验（本容器不可达，交根）：真实 GUI DOM 渲染、macOS native/fullfsync；全量 cargo test/全历史重放未跑（有界验证优先）。

## 4. 17 细项（每项：证据 / 缺口 / 结论）

| 细项 | 证据（类型） | 缺口 | 结论 |
|---|---|---|---|
| A-CC-006-03 | 三段修订轨迹 + 三点真杀同 DB 恢复（ACTUALLY_RUN，pid/信号/wait/重启/meta/batches/objects 读数）；同一 `local_shape` 核 4 oracle passed，未写第二判定器 | 无（Linux 面） | 通过 |
| A-CC-006-04 | 正式更正产生撤回/RISING/后续 TOP（ACTUALLY_RUN）；对象含四比较/三根 raw+merged 见证/input_refs/源修订链（SOURCE_REVIEW + CLI/HTTP 读数） | 真实浏览器 DOM 四比较/三见证/历史渲染 NOT_RUN | 通过（CLI/HTTP 面）；GUI 交根 |
| A-ST-044-02 | 持久 Delta 原子应用对象/边/端点/见证（ACTUALLY_RUN）；两轨迹同 cut 全字段对拍 PASS | 无 | 通过 |
| A-ST-044-03 | 三点进程恢复按原游标接回结构 + AsKnown 逐代过滤（ACTUALLY_RUN）；C 完整分页/保留/过期压力矩阵留 TB-01-C（票面豁免） | C 压力矩阵未在本片 | 通过（本片子义务） |
| A-ES-15-01 | E/B/X 未启动 S 修订/历史/恢复独立成功（ACTUALLY_RUN）；scope economic=not_started；结构 frontier 未冒充经济 ack（无经济写入路径） | 无 | 通过 |
| A-ES-15-02 | Begin/batch/Commit/DeliveryUnknown/恢复迁移与拒绝有前后持久证据（ACTUALLY_RUN kill 前后读数 + 旧 epoch 拒绝 exit 1） | 无 | 通过 |
| A-RA-04-01 | 旧 TOP first_known_generation=1、原身份/证据/撤回史保留（ACTUALLY_RUN）；本片不销经济计划/成交责任分派（economic not_started，票面豁免） | 经济责任分派不在本片 | 通过（本片子义务） |
| A-RA-10-01 | 同 cut Snapshot/Delta/AsKnown 字段一致（ACTUALLY_RUN 全字段 PASS）；RecomputedWithRevision 另具名（history_mode 字段 + 绑定最新 input/rule 版本）；不销完整经济历史（票面豁免） | 完整经济 AsKnown 留后续 TB | 通过（本片子义务） |
| A-I-02-01 | 经济未启动 S 成功（ACTUALLY_RUN）；S 自身损坏报 StorageUnavailable 且停写（exit 1 / HTTP 503，不静默空库） | 全域故障分区留后续 TB | 通过（本片子义务） |
| A-OB-004-01 | raw revision / object_id / object_revision 分列；更正撤旧/替代链（supersedes/replaces）；旧版不复活（ACTUALLY_RUN） | 无 | 通过 |
| A-OB-008-01 | Delta 重复/乱序/断连（重复不重生效、删除 gen1→显式 Gap+rebuild_cut）、原子应用、同 cut 全字段比较（ACTUALLY_RUN） | 无 | 通过 |
| A-OB-009-01 | 缺右邻不提前 TOP（gen1 无对象 + insufficient_knowledge）；晚到更正不回写旧 first_known（ACTUALLY_RUN） | 无 | 通过 |
| A-OB-014-01 | 无经济事件不补造历史经济应用（SOURCE_REVIEW：无经济写入路径；economic not_started）；未发/可能已发/已成交责任不销项（票面豁免） | 经济责任不在本片 | 通过（本片子义务） |
| A-OB-015-01 | 同一 session/generation；delta gen 严格 +1、乱序显式 Gap、recover 换代后旧 epoch 拒绝（ACTUALLY_RUN） | 无 | 通过 |
| A-OB-016-01 | 撤回断连、真实恢复后收敛到同 cut Snapshot（ACTUALLY_RUN：两轨迹 delta→snapshot 全字段 PASS） | 真实浏览器收敛渲染 NOT_RUN | 通过（数据面）；GUI 交根 |
| A-OB-017-01 | 对照本域对象/边/事件/身份/获知史（raw_history 含 supersedes；AsKnown 按前沿过滤）；无未来偷看、无 bars 造经济恢复（ACTUALLY_RUN + SOURCE_REVIEW） | 无 | 通过 |
| A-OB-020-01 | 每项保存构建/输入/平台/故障/PID/信号/退出/重启/DB/API 证据（本评审 /tmp/s_review_b_kill/* + 作者 /tmp/s_b_*） | GUI 证据 NOT_RUN | 通过（可复核） |

## 5. 缺陷（H/M/L）

无 H、无 M。两个 L（均不阻塞，不在受测必经路径）：

- **L-1（低）**：无新输入时重复 `advance` 推进 generation 并重发布未变对象。
  - 位置：`rust/src/bin/s_structure_session.rs`：`cmd_advance` 提交段 `:1407`（只查 `cur_frontier != frontier`，不查「本次是否有新输入」）、`:1456`（`ON CONFLICT(object_id) DO UPDATE SET published_generation=…`）、`:1658`（无条件 `meta_set(generation)`）。
  - 触发：对同一 DB 连续两次 `advance` 且其间无新 `accept`（正式 launcher 与 AC 轨迹不会产生；CLI 可直接产生）。
  - 后果：generation 计数 +1、产生空 delta（upsert 1 未变对象、`seq_range` 倒置 `from=3 to=2`、raw_history_added 0）、未变对象 published_generation 前移；不破坏已提交 cut、不影响 watch 收敛与同 cut 对拍。
  - 最小修复：commit 前比较 `frontier == last_advance_frontier` 且无新输入时返回 no-op（不推进 generation），或显式声明并测试「无新输入 advance 幂等 no-op」。
- **L-2（低）**：`as_of`/`after_generation` 接受负 generation，返回空历史视图而非显式拒绝。
  - 位置：`rust/src/bin/s_structure_session.rs:2145-2146`（`frontier_at_generation` 对 `gen<=0` 返回 -1）、`:2595-2596`（`parse_canonical_i64` 接受负整数）；`s_session/s_readonly_server.py:470-480`（`_parse_as_of` 接受负整数）。
  - 触发：`snapshot --as-of -5` 或 `GET /api/snapshot?as_of=-5`。
  - 后果：返回合法 JSON 的空视图（无对象/无 raw_history）而非拒绝；不损坏数据、不影响正 generation 查询。
  - 最小修复：`as_of`/`after_generation` 显式要求 `>=0`（负值返回 `InvalidDomain`）。

## 6. NOT_RUN / 未验（照实，交根）

1. **真实浏览器 GUI 渲染与交互**（AC4「浏览器真实接流」/AC7「浏览器能分别查看 AsKnown 与 RecomputedWithRevision」必需成功）：本容器无 chromium/firefox。已交付 `browser/index.html`（node --check 语法通过；`applyStream`+`canonicalFieldCompare` 逻辑由本评审用 Python 复刻对真实 API 响应全字段对拍 PASS；`/` 返回 HTML、含 btn-current/btn-asof/btn-stream/asof-select/raw-history/withdrawn/stream-result 元素）。**实际浏览器 DOM/交互渲染未验**。
   恢复命令：`./s_session/launch_s.sh --db <db> --port 8787` 后开 `http://127.0.0.1:8787/`，点「RecomputedWithRevision / AsKnown 回看 generation / 读取新 Delta」。
2. **macOS native 构建 / fullfsync / launcher .dylib**（C09.2 平台前件）：本容器只实测 Linux aarch64。macOS 由根接回。
3. 全量 `cargo test` / 全历史重放：按有界验证优先，未跑全量（票面允许）。
4. 分页/保留/过期压力矩阵、跨域经济 AsKnown、经济故障隔离：留 TB-01-C 与后续 TB（本片不销项）。
5. A-OB-014-01/RA-04 经济计划、已交付/可能调用、已成交责任分派；I-02/RA-10 全域故障分区：本片不销项。
6. A-ST-044-01 高级扩展关系：本片未构造。
7. >2^53 精确整数在真实浏览器 DOM 的渲染：Rust CLI 与 HTTP API 已实测规范十进制字符串往返（源坐标 9007199254741000-1003、revision 9007199254740993/994）；浏览器 DOM 渲染该值未验（无浏览器）。

## 7. 结论与剩余根验项

- 容器内可验证的 AC1..AC8 与 17 细项所需成功路径全部独立实跑通过，无 H/M 缺陷；同 cut Delta→Snapshot 全字段对拍 PASS。
- 剩余根验项：①真实浏览器 GUI 渲染与 AsKnown/Recomputed/Delta 交互；②macOS native 构建与 fullfsync 前件；③main 合入、真实外效、未决 G/FU（本片未授权）。
- 本报告绑定源码 head=`8c868f968c5f937870a4305dca05a0e0162b5131`；随后 docs commit 不改绑定。
