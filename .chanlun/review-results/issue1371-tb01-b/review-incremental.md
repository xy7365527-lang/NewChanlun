# #1371 TB-01-B 独立评审·增量复审（R1 → 根修复候选 0298cf42）

> 编号声明：本提交中的 #1371 仅指 GitHub Issue。
> 本报告是原 R1 独评 session 的增量复审。固定范围 BASE `2212eba31e0b7419907e622e79f02f7c2cc37706`..REVIEW_HEAD `0298cf429804c611d6ee5f691a1f36ee2a720fba`；
> R1 源 head `8c868f968c5f937870a4305dca05a0e0162b5131`、R1 报告 tip `6039b83f6a1730daeb0d557f3bb712d8c31c92bd`。
> 原 `review.md`/`review.json`（绑定 8c868f96，tip 6039b83f6a）逐字保留，本报告为增量复审。

## 0. 结论

**verdict：INCOMPLETE**（根评审 REQUEST_CHANGES 的 6 项 ROOT + 3 项 native crash + Python 专审 7 HIGH/2 MEDIUM + R1 的 L-1/L-2 全部复核完毕：**所有 H/M 已修复并在 Linux 实测通过**；残留 1 项 L 防御缺口；真实浏览器 GUI 与 macOS native/fullfsync 两项必需验证在本容器不可达、尚未接回，故不得整体 PASS）。

- 已修且 Linux 实测：历史 cut 元数据/目录证据不可变、对象 batch_id/published_generation 首发布稳定、current snapshot 以已发布前沿为界、未发布 cut 显式 Unavailable、幂等 advance 不产新 cut、DeliveryUnknown 原身份只读 query、可达根损坏阻断 accept/advance/recover、reader 严格校验坏 generation/frontier/Delta 形状/连续性/末端/session 绑定、URL 解码 + 400/503 分界。
- 残留：读端 `read_delta` 对持久 delta 嵌套字段的精确整数只做形状校验、未做规范字符串投影（详见新发现 L-NEW-1，不阻塞）。
- NOT_RUN 交根：真实浏览器 GUI DOM 渲染与 Delta/AsKnown 交互；macOS native 构建与 fullfsync 前件。

## 1. 差异与范围

- `git rev-parse HEAD` = `0298cf429804c611d6ee5f691a1f36ee2a720fba`（= REVIEW_HEAD）；工作树干净；分支 `codex/1323-tb01-b-1371`。
- `git log 6039b83f6a..0298cf42`：单提交 `0298cf4298 fix(structure): #1371——历史cut不可变/可达根写前件/幂等advance与原身份Query`。
- 该提交改 6 文件：`rust/src/bin/s_structure_session.rs`(+472/-?)、`s_session/s_readonly_server.py`、`s_session/browser/index.html`、`launch_s.sh` 未再改（epoch 透传保持 65cb1552）、作者 IMPLEMENTATION-REPORT.{md,json}、roster。
- 生产文件 hash（与作者报告逐字一致）：rust `2e06b83e…9e`、py `8024b01c…f1`、html `bbc86d01…84`、sh `65cb1552…1d`（未变）。
- 未触碰：`formal/`、`inclusion.rs`、`local_shape.rs`、`signed-catalog.json`、`profiles/*`（A oracle 同 hash 复用）。

## 2. 检查运行（本容器实际执行，exit 码）

| 命令 | exit | 结果 |
|---|---|---|
| `cargo build --features s_session --bin s_structure_session` | 0 | 二进制 SHA256 `e500f13f002c5521dba20daff852401588606e67a4405163ae2a29cfab838854` |
| `cargo fmt -- --check` | 0 | 0 差异 |
| `cargo check --features s_session --bin s_structure_session` | 0 | 0 error |
| `cargo clippy --features s_session --bin s_structure_session` | 0 | 本片文件 0 命中 |
| `cargo test --features s_session --bin s_structure_session --jobs 1` | 0 | 22 passed（A 9 + B 13；新增 4：幂等 advance / 未发布 cut 拒绝 / current 排除未发布修订 / 可达根损坏阻断写入） |
| `cargo test --lib local_shape` | 0 | 4 passed（A oracle） |

运行时：sqlite 3.53.2（bundled）、wal、synchronous=2(FULL)、platform=linux。

## 3. R1 findings 复核（L-1/L-2）

| id | 结论 | 证据 |
|---|---|---|
| L-1 无新输入重复 advance 推进 generation | **已修复** | `cmd_advance:1178` 幂等门 `frontier == last_frontier && cur_gen >= 1` → 返回 `{idempotent:true, generation:1}` 不产新 cut；实测连续 advance exit 0 且 generation 保持 1；`advance_without_new_input_is_idempotent` 测试通过 |
| L-2 负 as_of/after_generation 接受 | **已修复** | Rust `--as-of`/`--after-generation` 拒绝负值（`InvalidDomain`，exit 1）；Python `_parse_nonneg` 拒绝负值（400 InvalidQuery）；实测 `snapshot --as-of -1` exit 1、`/api/state?as_of=-1` 400 |

## 4. 根评审 ROOT-B-R1-01..06 复核

| id | 结论 | 证据（Linux 实测） |
|---|---|---|
| ROOT-B-R1-01（HIGH，AsKnown 带未来元数据/目录证据/对象 batch/pg 前移） | **已修复** | Rust `read_snapshot_in_tx`/`read_catalog_in_tx` 按 as_of 从 `structure_deltas` 取该代历史头（generation/cut/index_frontier/catalog_revision）与 `catalog_run_status`/`catalog_evidence`；对象 upsert `ON CONFLICT DO NOTHING` + delta upsert 沿用既有 first_known/batch/published。实测：追加第四点后 current RISING `published_generation=2`、`batch_id=gen2 批次`（不前移）；`AsKnown gen2` RISING pg=2 batch=gen2、撤旧 TOP pg=1 batch=gen1；`catalog --as-of 2` CC-006 evidence.batch_id=gen2 批次、`--as-of 0` not_run/{} |
| ROOT-B-R1-02（HIGH，Delta 未驱动正式渲染） | **已修复（代码）** | `applyStream` 验证通过后 `renderAllFromState(state)` 驱动正式元数据/对象/撤回/见证/关系/历史/目录渲染。真实 DOM 渲染 NOT_RUN（无浏览器） |
| ROOT-B-R1-03（HIGH，快照失败提交游标、空 Delta 吞差异） | **已修复（代码）** | `applyStream` 用 tmp 暂存，`/api/state` 读取失败或对拍差异均不提交游标；空 delta 仍取 `/api/state` 全字段复核；代际不一致报「游标未提交」。真实 HTTP 传输故障 GUI 复现 NOT_RUN |
| ROOT-B-R1-04（MEDIUM，首次历史选择未初始化） | **已修复（代码）** | `asof-input` 初始 value="0"；`load("current")` 显式初始化；btn-asof 校验 `/^\d+$/`。node 逻辑验证 historyMode=current |
| ROOT-B-R1-05（HIGH，坏 generation/Delta 缺口/坏 frontier 仍成功） | **已修复** | Python `_canonical_gen`（generation 缺失/损坏→ValueError）、`_validate_delta_shape`（upserts 非数组→ValueError）、完整连续性 `gens==expected`（删中洞/尾洞→显式 gap+rebuild_cut）、`_frontier_at_generation`（delta 行缺失→Unavailable，不默认 -1）、`_frontier_i64`（文本/浮点 frontier→ValueError）、session 绑定（after>current→gap）。实测全部触发正确 |
| ROOT-B-R1-06（HIGH，可达根损坏仍写入） | **已修复** | `verify_reachable_root:326` 在 accept/advance/recover 三写入口调用。实测删除 index_frontier 指向 batch 后 accept/advance/recover 均 exit 1 `StorageUnavailable`，meta generation/advance_state/writer_epoch 不变（无部分换代） |

## 5. native crash 3 项复核

| id | 结论 | 证据 |
|---|---|---|
| DeliveryUnknown 无原身份查询 + 同 argv 重试多 cut | **已修复** | 新增 `cmd_query:2522`（`query --identity-key` 只读）；`cmd_advance` 幂等门。实测：`query` e2 返回 3 修订 + `latest_published`；after_commit SIGKILL 后同 argv advance 重试 exit 0 `idempotent:true`，generation 保持 1 |
| 默认 Snapshot 混入未发布接纳修订 | **已修复** | current raw_history 以 `last_advance_frontier`（已发布前沿）为界。实测：accept e2 rev3 未 advance，`snapshot` raw_history 仍 5 行、不含 rev3；`current_snapshot_excludes_unpublished_accepted_revision` 测试通过 |
| 未发布 AsKnown2 返回混合载荷 | **已修复** | `frontier_at_generation:2255` delta 缺失→`None`→`Unavailable`。实测 `snapshot --as-of 5` exit 1 `Unavailable：…尚未发布`；`as_of_unpublished_cut_is_rejected` 测试通过 |

## 6. Python 专审 7 HIGH / 2 MEDIUM 复核

| id | 结论 | 证据 |
|---|---|---|
| PY-DRAFT-H01（generation 回退 0） | **已修复** | `_canonical_gen:71`；实测 'broken'/缺失 → ValueError（HTTP 503） |
| PY-DRAFT-H02（Gap 只验首条） | **已修复** | `read_delta:481-486` 完整连续 + 末端；实测删中洞/尾洞均 gap |
| PY-DRAFT-H03（游标不绑定会话） | **已修复** | after>current → gap `cursor_ahead_or_session_rebuilt`；行 sid≠meta.session_id → ValueError |
| PY-DRAFT-H04（Delta 嵌套类型损坏） | **已修复** | `_validate_delta_shape:426`；实测 upserts='not-an-array' → ValueError |
| PY-DRAFT-H05（Delta 整数未投影 + Number 回转） | **部分修复** | 浏览器改用 BigInt（`bigAddOne/bigLe/bigEq`、`lastAppliedGen` 字符串、slot BigInt 排序）、AsKnown 有界 input。读端 delta 嵌套字段整数投影未做（见新发现 L-NEW-1）。生产 Rust 路径已全字符串化（实测 production delta wire 整数均为 str） |
| PY-DRAFT-H06（快照失败提交游标） | **已修复（代码）** | tmp 状态 + 验证通过才提交；空 delta 也取 `/api/state` 全字段对拍 |
| PY-DRAFT-M01（URL 解码 + 存储故障混用） | **已修复** | `parse_qsl`+`urlsplit`+`InvalidQuery`；实测 `as%5Fof=0`/`as_of=%30` 正确解析、重复参数/坏值 400 InvalidQuery、未发布 503 |
| PY-DRAFT-M02（首载未初始化） | **已修复** | `load("current")`；node 验证 historyMode=current |
| PY-8C-H07（AsKnown frontier 未校验/缺失回退） | **已修复** | `_frontier_i64:85` + `_frontier_at_generation`；实测文本/浮点 frontier → ValueError、delta 行缺失 → Unavailable |

## 7. 新发现（本轮）

- **L-NEW-1（低，残留防御缺口，不阻塞）**：读端 `read_delta` 对持久 delta 嵌套字段只做形状校验（list/dict/object_id str），未对 `upserts[].object_revision/window_start/window_mid/window_end/first_known_generation/published_generation/withdrawn_generation`、`seq_range.from/to` 等声明精确整数字段做规范字符串校验/投影。
  - 位置：`s_session/s_readonly_server.py:426-445`（`_validate_delta_shape`）。
  - 触发：持久 `structure_deltas.delta_json` 被人工注入 JSON number（如 `object_revision:9007199254740993`）时，`/api/delta` 仍 200 透传 number（实测）；生产 Rust 路径不会产生（已实测 production wire 整数全为字符串）。
  - 后果：损坏持久值未在 HTTP 层 503 拒绝；但浏览器 `canonicalFieldCompare` 的类型比较会报差异（不会静默 PASS），且有 BigInt 防 >2^53 回转，故不产生错误结论。
  - 最小修复：`_validate_delta_shape`（或独立投影）对声明整数字段校验 TEXT/规范十进制字符串，坏值 503。

## 8. 逐 AC（按原票 8AC 顺序）与 17 细项

| AC | 结论 | 证据 |
|---|---|---|
| AC1 launcher+真实输入链 | PASS(Linux) | launcher `--testonly` exit 0；三段轨迹 + 晚到臂实测；更正先 Begin 再解释 |
| AC2 revision/身份/结构 revision 分立 | PASS | replay/IdentityConflict/supersedes/replaces/旧版不复活/换源坐标拒绝 CLI 实测 |
| AC3 Delta+浏览器原子应用+同 cut 逐字段相等 | PASS(Linux 数据面) | 独立 Python 复刻 applyStream 全字段对拍 PASS；重复/乱序/断连/空增量显式 Gap；真实浏览器 DOM NOT_RUN |
| AC4 三点 SIGKILL 同 DB 恢复 | PASS | 三点真实 SIGKILL（wait status=9，/proc cmdline 核）→ 正式入口恢复；不可达 batch 不冒充已发布 cut |
| AC5 DeliveryUnknown+旧 epoch 拒绝+幂等 | PASS | `query --identity-key` 只读 + `latest_published`；同 argv advance 幂等；recover 单调换代 + 旧 epoch StaleWriter 零写入 |
| AC6 AsKnown/Recomputed+缺右邻不提前 | PASS(Linux) | 历史头不可变（gen/cut/index/catalog/CC-006 evidence 按代）；未发布 cut Unavailable；缺右邻无 TOP；晚到不回写 first_known |
| AC7 E/B/X 未启动+StorageUnavailable | PASS(Linux) | 只启动 S+只读外壳；库头损坏 exit 1 + 可达根损坏阻断写入 + 503 |
| AC8 证据保留+检查+未验 | PASS(Linux) | 本评审独立保留命令/exit/PID/信号/重启/DB/API 读数；GUI/macOS NOT_RUN |

17 细项：A-CC-006-03/04、A-ST-044-02/03、A-ES-15-01/02、A-RA-04-01、A-RA-10-01、A-I-02-01、A-OB-004/008/009/014/015/016/017/020 均在本片受测域内 Linux 实测通过（对象/边/事件/身份/获知史对照、Delta 原子应用、同 cut 全字段一致、真杀恢复、DeliveryUnknown 原身份、缺右邻/晚到不回写、E/B/X 未启动、S 损坏停写、旧代际隔离、证据留痕）；浏览器 DOM 渲染项（A-CC-006-04、A-OB-016-01 的 GUI 部分）与 A-OB-020-01 的 GUI 证据 NOT_RUN；经济责任分派（A-RA-04-01 后半）、全域故障分区（A-I-02-01 后半）、完整经济 AsKnown（A-RA-10-01 后半）依票面不销项。

## 9. NOT_RUN / 未验（交根）

1. 真实浏览器 GUI DOM 渲染与 AsKnown/Recomputed/Delta 交互（AC3/AC6 的浏览器必需成功；容器无 chromium/firefox）。恢复命令：`./s_session/launch_s.sh --db <db> --port 8787` 后开 `http://127.0.0.1:8787/`。
2. macOS native 构建 / fullfsync / launcher .dylib（C09.2 平台前件）。
3. >2^53 精确整数经真实浏览器 DOM 渲染（CLI/HTTP 已实测字符串往返；DOM 未验）。
4. 全量 cargo test / 全历史重放（有界验证优先）。
5. 分页/保留/过期压力矩阵、跨域经济 AsKnown、经济故障隔离（TB-01-C 与后续 TB）。
6. A-OB-014-01/RA-04 经济计划/成交责任分派、I-02/RA-10 全域故障分区、A-ST-044-01 高级扩展关系（后续 TB）。

## 10. 结论与剩余根验项

- R1 findings 与根评审全部 H/M 已修复并经 Linux 实测；残留 1 L 防御缺口（读端 delta 嵌套整数投影）。
- 剩余根验项：①真实浏览器 GUI 渲染与 Delta/AsKnown 交互；②macOS native 构建与 fullfsync；③main 合入、真实外效、未决 G/FU（本片未授权）。
- 本报告绑定源码 head `0298cf429804c611d6ee5f691a1f36ee2a720fba`；随后 docs commit 不改绑定。
