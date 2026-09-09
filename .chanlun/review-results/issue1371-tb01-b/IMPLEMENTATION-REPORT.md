# #1371 TB-01-B 实施报告（修订撤回、持久 Delta 与真实进程恢复，含根评审修复）

> 票：GitHub Issue #1371（TB-01-B 内部执行单元，SPEC #1340 / 图 #1323 / SPEC #1339 R2）。
> 实施分支：`codex/1323-tb01-b-1371`；精确基线：`2212eba31e0b7419907e622e79f02f7c2cc37706`。
> 编号声明：本提交中的 #1371 仅指 GitHub Issue。
> 本报告是工作草稿（活期绑定票），不是独立评审，也不以自审 PASS 冒充独评。

## 0. 结论（一句话）

在 A 已合正式结构会话之上补齐 B 片：源事件合法修订链、有效源位置语义、持久 StructureDelta（原子应用）、
对象撤回、真实 SIGKILL 三点同 DB 恢复、writer 单调换代与旧 writer 拒绝、DeliveryUnknown 原身份只读 Query、
AsKnown/RecomputedWithRevision 分栏、同源 Watch 续接，并已按根评审（ROOT-B-R1 + native crash + Python 专审）
修复：历史 cut 元数据/目录证据不可变、对象 batch/published 首发布稳定、current snapshot 以已发布前沿为界、
未发布 cut 显式 Unavailable、幂等 advance 不产新 cut、可达根损坏阻断正式写入、reader 严格校验坏持久值。

## 0a. 本轮修复（相对 8c868f968c，针对根 REVIEW 结果 INCOMPLETE）

| 根发现 | 修复 |
|---|---|
| ROOT-B-R1-01 / native#8（AsKnown 带未来元数据/目录证据；未发布修订混入 current raw_history） | `read_snapshot`/`read_catalog` 的 AsKnown 头（generation/cut/index_frontier/catalog_revision）与 CC-006 run_status/evidence 按该代持久 delta 复现；current 的 raw_history 以 `last_advance_frontier`（已发布前沿）为界 |
| ROOT-B-R1-01（对象 batch_id/published_generation 前移） | 对象 upsert 改 `ON CONFLICT DO NOTHING`；delta upsert 沿用既有 first_known/batch/published（首发布稳定） |
| ROOT-B-R1-02（Delta 未驱动正式渲染） | 浏览器 `applyStream` 验证通过后 `renderAllFromState` 驱动正式元数据/对象/撤回/见证/关系/历史/目录 |
| ROOT-B-R1-03 / PY-H06（快照失败提交游标；空 Delta 吞差异） | 浏览器改用暂存状态，`/api/state` 读取失败/对拍差异均不提交游标；空增量仍全字段复核 |
| ROOT-B-R1-04 / PY-M02（首载未初始化历史选项） | `load("current")` 默认 + 有界 AsKnown 输入框 |
| ROOT-B-R1-05 / PY-H01..H05/H07/M01（坏持久值/坏参数仍成功） | Python reader 严格校验 generation/frontier/Delta 形状/连续性/末端/session 绑定；参数错误 400 InvalidQuery，存储损坏 503 |
| ROOT-B-R1-06（可达根损坏仍写入） | `verify_reachable_root` 在 accept/advance/recover 的同序边界核 index_frontier batch 存在 |
| native crash#7（DeliveryUnknown 无原身份查询；同 argv advance 重试产新 cut） | 新增 `query --identity-key` 只读查询；`advance` 无新输入（frontier==last_advance_frontier 且已有已发布代）幂等返回现有 cut |
| native crash#9（未发布 AsKnown2 返回混合载荷） | `frontier_at_generation` 缺 delta 行 → `Unavailable`（exit 1 / 503），不回退 -1 |
| L-1/L-2（无新输入 advance 推进；负 as_of/after） | 幂等 no-op + `--as-of`/`--after-generation` 要求 >= 0 |

## 0b. 改动面（只改本票模块）

`rust/src/bin/s_structure_session.rs`、`s_session/s_readonly_server.py`、`s_session/browser/index.html`、
`s_session/launch_s.sh`（epoch 透传）。未改 `formal/`、`inclusion.rs`、`local_shape.rs`、`signed-catalog.json`、
`profiles/*`。未撤销他人编辑；未 push/merge/main/gh 写/harvest/真实经济外效。

## 1. 逐验收条件证据（AC1..AC8，按原票顺序）

### AC1 —— 正式 launcher + 真实输入链（TOP → 更正 RISING → 新 TOP）
- launcher（`--testonly`，epoch 透传）端到端 exit 0：init(116/wal/synchronous=2/3.53.2)→accept(7)→advance(5 对象)→只读外壳→`/api/snapshot` gen1/5 对象。
- 三段轨迹（同 launcher 的 init/accept/advance 命令）：`accept in1`(10000/11000/10500)→advance→`TOP@[0,1,2]`；`accept in2`(e2 rev2=11500, supersedes_revision=1)→advance→`RISING@[0,1,2]` 活动 + 撤回 `TOP@[0,1,2]`；`accept in3`(e3=10800)→advance→`RISING`+`TOP@[1,2,3]`。
- 晚到臂（e0..e3 先到后更正）仍 4 个有效源坐标；更正先 S 本地 Begin 再解释（after_begin 杀点实测 advance_state=begun:*），撤回非前端/独立解释器补造。

### AC2 —— revision/窗口身份/结构 revision 分立 + 幂等/冲突/不复活
- `raw_events` 主键 `(identity_key, revision)`；revision/object_id/object_revision 分列；wire 规范十进制字符串。
- 同内容重投→replay 原 receipt；同 revision 异内容→IdentityConflict；新 revision→supersedes/replaces；旧版重放→replay 不复活；更低 revision 晚到→InvalidDomain 零写入；换源坐标→IdentityConflict 零写入。

### AC3 —— 持久 StructureDelta + 浏览器原子应用 + 同 cut 逐字段相等
- `structure_deltas` 每代一行（session/generation/catalog_revision/base_cut/next_cut/seq_range/input_frontier/
  index_frontier/catalog_run_status/catalog_evidence + 完整 upserts/withdrawals/replaces/witnesses/relations/
  observations/raw_history_added）；对象/边/端点同事务原子应用；见证/关系/观察 published_generation 首获知固定。
- 同源对拍（Node 复刻浏览器 applyStream+全字段比较，真实 API 响应）：从 after_generation=0 逐代应用后与
  `/api/snapshot` 的对象/撤回/见证/关系/观察/源历史全字段逐字段相等 → **PASS**（缺右邻 cut → TOP → 撤回 RISING → 新 TOP，4 代）。
- 重复投递不重生效；乱序/断档/超前游标显式 Gap+rebuild_cut；空增量仍全字段复核不吞差异。

### AC4 —— 真实 SIGKILL 三点同 DB 恢复
三点均从「已发布旧 TOP」开始（先 accept e0/e1/e2 并 advance 成 gen1 TOP，再接纳更正 e2 rev2，再对更正 advance 发真实
`SIGKILL`（`wait_exit=-9`，`/proc/<pid>/cmdline` 核 s_structure_session+advance+db），再以正式入口开同一 DB：
- ① Begin 后：`begun:*`/gen1(旧 TOP 已发布)/batches1/cut-1 → recover --new-epoch 2 清门换代 → advance --writer-epoch 2 → RISING 活动 + TOP 撤回。
- ② 完整 batch 后 Commit 前：`begun:*`/gen1/batches2(新增不可达 batch)/cut-1 → recover → advance → RISING 活动 + TOP 撤回（不可达 batch 未冒充已发布 cut）。
- ③ Commit 后回执前：`idle`/gen2/cut-2/RISING 活动+TOP 撤回 → 更正输入 replay 原 receipt；recover 后 advance 幂等返回 gen2（不产 gen3）。
- 恢复读集：重开同 DB 读原接纳序/身份、未决 Begin、可达根、writer 代际；未决门保持关闭（旧 epoch advance 被拒）；无幽灵引用、不漏已提交 cut。

### AC5 —— DeliveryUnknown 原身份 Query + 旧 epoch 拒绝 + 幂等重试
- 新增只读 `query --db --identity-key`：返回该业务身份全部修订/receipt/seq/source_coord/supersedes 及
  `latest_published`（e2 → 2 记录，latest rev2，已发布 true）；NoRecord 不自动新建动作。
- 同 argv advance 重试（无新输入）幂等返回现有 cut（`idempotent:true`，不产新 generation）。
- `recover` 严格单调换代（new_epoch≤cur 拒绝 exit 1）；换代后旧 epoch `accept`/`advance` 均 StaleWriter 拒绝（零写入）；`writer_epoch_history` 记录与 generation 同序。

### AC6 —— AsKnown / RecomputedWithRevision + 缺右邻不提前
- 缺右邻（两点）→ 无对象 + insufficient_knowledge（不提前 TOP）。
- AsKnown gen1：头 generation=1/cut-1/index_frontier=gen1 批次/CC-006 run=not_run、raw_history 2、无撤回元数据；AsKnown gen2 回看旧 TOP 为 active（未来撤回元数据不倒填）；AsKnown gen3 含 replaces+supersedes。
- 未发布 cut（as-of 99）→ `Unavailable`（exit 1 / HTTP 503），不返回混合/空历史；晚到更正不回写旧 first_known；当前集合移除不删历史。

### AC7 —— E/B/X 未启动 + StorageUnavailable
- 全程只启动 S（一次性命令）与只读外壳；未建 E/B/X 库、未加载经济政策；scope economic=not_started。
- S 库头损坏 → accept/advance/meta exit 1（不静默空库）；可达根 batch 被删 → accept/advance exit 1 StorageUnavailable；Python 损坏库 /api/* 503。
- 平台前件：Linux 实测 sqlite 3.53.2、wal、synchronous=2(FULL)、platform=linux；macOS fullfsync/GUI 未验（交根）。

### AC8 —— 证据保留 + 检查 + 未验
- 检查：fmt 0、check 0、clippy 本片 0 命中、bin 测试 22 passed（A 原 9 + B 13）、`--lib local_shape` 4 passed。
- 证据：命令/退出码、输入/构建/目录 hash、PID/信号/退出/重启/持久状态、API 前后、Node 全字段对拍、杀点输入（/tmp/s_b_*，工作草稿证据不入仓）。

## 2. 17 细项绑定（结论：本片范围内均已完成；未销项见下）

| 细项 | 证据（真入口/命令/退出码/输出） |
|---|---|
| A-CC-006-03 | 三段修订轨迹 + 三点真杀同 DB 恢复（wait_exit=-9）；同一 local_shape 核 oracle 4 passed；未写第二判定器 |
| A-CC-006-04 | 正式更正产生撤回/RISING/后续 TOP；对象四比较/三见证/input_refs/源修订链可查（CLI+HTTP；GUI DOM 交根） |
| A-ST-044-02 | 持久 Delta 原子应用对象/边/端点/见证；Node 全字段对拍 = 同 cut Snapshot PASS |
| A-ST-044-03 | 三点恢复按原游标接回 + AsKnown 逐代过滤；C 完整分页/保留/过期压力矩阵留 TB-01-C |
| A-ES-15-01 | E/B/X 未启动 S 修订/历史/恢复独立成功；结构 frontier 不冒充经济 ack（economic not_started） |
| A-ES-15-02 | Begin/batch/Commit/DeliveryUnknown/恢复真实迁移与拒绝有前后持久证据；旧 epoch 拒绝 exit 1 |
| A-RA-04-01 | 旧 TOP first_known_generation=1、原身份/证据/撤回史保留；本片不销经济计划/成交责任分派 |
| A-RA-10-01 | 同 cut Snapshot/Delta/AsKnown 字段一致（Node PASS）；Recomputed 另具名绑定最新 input/rule 版本 |
| A-I-02-01 | 经济未启动 S 成功；S 自身损坏 StorageUnavailable 且停写（exit 1 / HTTP 503），不声称全域故障乘积 |
| A-OB-004-01 | raw revision/object_id/object_revision 分立；更正撤旧/替代链；旧版不复活 |
| A-OB-008-01 | Delta 重复/乱序/断连/空增量处理 + 原子应用 + 同 cut 全字段比较（PASS） |
| A-OB-009-01 | 缺右邻不提前 TOP；晚到更正不回写旧 first_known |
| A-OB-014-01 | 无经济事件不补造历史经济应用；未发/可能已发/已成交责任本片不销项 |
| A-OB-015-01 | 同一 session/generation；delta gen 严格 +1、乱序/超前游标显式 Gap；换代后旧 epoch 拒绝 |
| A-OB-016-01 | 撤回断连、真实恢复后收敛到同 cut Snapshot（Node PASS）；浏览器 DOM 渲染交根 |
| A-OB-017-01 | 对照本域对象/边/事件/身份/获知史；无未来偷看、无 bars 造经济恢复 |
| A-OB-020-01 | 每项保存构建/输入/平台/故障/PID/信号/退出/重启/DB/API 证据（/tmp/s_b_*；GUI 未验 NOT_RUN） |

## 3. 检查与命令（exit 码）

| 命令 | exit |
|---|---|
| cargo fmt -- --check | 0 |
| cargo check --features s_session --bin s_structure_session | 0 |
| cargo clippy --features s_session --bin s_structure_session | 0（本片 0 命中） |
| cargo test --features s_session --bin s_structure_session --jobs 1 | 0（22 passed） |
| cargo test --lib local_shape | 0（4 passed，A oracle） |
| ./s_session/launch_s.sh --testonly … | 0 |
| /tmp/s_b_kill_test.py（三点 SIGKILL+恢复，从已发布旧 TOP 起） | 0（wait_exit=-9） |
| /tmp/s_b_node_verify.mjs（全字段对拍） | 0（PASS） |
| 拒绝路径（旧 epoch/坏 as_of/未发布 cut/可达根损坏） | 1 / HTTP 400/503 |

平台：Linux aarch64；SQLite 3.53.2（bundled）；rustc 1.97.1。

## 4. 哈希

- SPEC 清单（票面给定）：`38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`。
- `signed-catalog.json`：`938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`（与 A 一致）。
- TestOnly profile：`2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc`（与 A 一致）。
- 本片四文件（本轮修复后）：
  - `rust/src/bin/s_structure_session.rs`：`2e06b83eb220934aba6f88f3be31ea876fe21b63984ce0a78dc162549dfdeb9e`
  - `s_session/s_readonly_server.py`：`8024b01ca38215a23357afd8354c76b392565c895bac472269617fb531addef1`
  - `s_session/browser/index.html`：`bbc86d0158bfebc5a9f5e73070f99a19f89532c4a35f28f477d076c7ca0c5084`
  - `s_session/launch_s.sh`：`65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d`（epoch 透传，本轮未再改）

## 5. NOT_RUN / 未验（照实，交根）

1. 真实浏览器 GUI DOM 渲染与 AsKnown/Recomputed/Delta 交互（容器无 chromium/firefox）。已交付可运行
   index.html（node --check 通过；applyStream+全字段比较逻辑由 Node 复刻对真实 API PASS）；实际 DOM 渲染 NOT_RUN。
   恢复命令：`./s_session/launch_s.sh --db <db> --port 8787` 后打开 `http://127.0.0.1:8787/`。
2. macOS native 构建 / fullfsync / launcher .dylib（本容器只实测 Linux aarch64）。
3. >2^53 精确整数经真实浏览器 DOM 渲染（Rust CLI + HTTP 已实测十进制字符串往返；DOM 未验）。
4. 全量 cargo test / 全历史重放（有界验证优先）。
5. 分页/保留/过期压力矩阵、跨域经济 AsKnown、经济故障隔离（TB-01-C 与后续 TB）。
6. A-OB-014-01/RA-04 经济计划/成交责任分派、I-02/RA-10 全域故障分区、A-ST-044-01 高级扩展关系（后续 TB）。

## 6. 待根验项（不代独评）

1. 真实 GUI 浏览器渲染与交互（恢复命令见 §5）。
2. macOS native/fullfsync 前件。
3. 独立评审复评（本报告非独评；保留原两份 review.md/json 由独评会话另行更新）。
4. main 合入、真实外效、未决 G/FU（本片未授权）。
