# #1370 TB-01-A 独立评审·增量复核报告（H-1 修复后，HEAD `76a641606d`）

> 复核对象：`codex/1323-tb01-a-1370` 自上次 FAIL 评审（`review-final.md/json`，HEAD `e79c74a17a`，
> commit `f2213dd33f`）之后的增量变更；固定待审 HEAD=`76a641606d7097292fff2dc21f58c6156ae38916`。
> 原精确基线 `efc1ddc4d6c015f4f8c6d44f904d704a88308b46`、范围授权与 9 AC 不变。
> 评审身份：同一独立评审上下文（会话 01a08556），只读；未修改生产源码/CI/其他报告。
> 方法：`git diff e79c74a17a...HEAD` 逐行核生产差异；自行重核「8 旧测试 + 耐久写路径逐字未变」；实跑第 9 项
> 真实 116 目录测试、完整目录成功路径（init 已签目录 → catalog CLI → /api/catalog → /api/state）、
> 坏 branches/scalar/null 拒绝、Python meta BLOB/NULL key/value 拒绝。不重复无关全历史重放。
> **结论：H-1 修复成立，AC6/AC7 必需成功路径恢复（受测域内）。** 无新 H/M；上一轮 L-1（README）已修，
> L-2/L-3/L-4 维持。未接回的 macOS/GUI 外部证据仍不预写通过。

## 0. 生产差异核（e79c74a17a...HEAD，不采信根说明，自行逐行核）

`git diff --stat`：`rust/src/bin/s_structure_session.rs`（71 行，7 个 hunk）、`s_session/s_readonly_server.py`
（4 行）、`s_session/README.md`（2 行）；`signed-catalog.json` **未改**。新增 `CATALOG-REPAIR-REPORT.md` 为根
工作草稿。差异仅四类，与根说明一致：

1. **Rust `JsonShape` 枚举**（`s_structure_session.rs:1433-1448`）：原 `json_shape(text, is_array: bool)` 改为
   `json_shape(text, shape: JsonShape)`，`JsonShape::{Array, Object, CatalogBranches}`；`CatalogBranches`
   仅 `branches` 允许 `array || object`，`Array`/`Object` 语义与旧 `is_array` 完全一致。所有调用点逐一核：
   `branches`→`CatalogBranches`（:1490）、`evidence`→`Object`（:1494）、`comparisons`→`Array`（:1534）、
   `input_refs`→`Array`（:1536）、`raw_bars`→`Array`（:1559）、`detail`→`Object`（:1595）、
   `scope`→`Object`（`read_scope` :1466-1470）。其余列仍严格单一形状。
2. **新第 9 项测试** `full_signed_catalog_preserves_both_declared_branch_shapes`（`s_structure_session.rs:1783-1818`）：
   init 后读真实已签目录，断言 116 项、82 数组 + 34 对象、逐项 `branches`/`title`/`domain` 与已签源同值、
   初始 `implementation_status=not_implemented`，并断言 `null`/`true`/`1`/`"text"`/`{` 五类坏值对
   `CatalogBranches` 均 `is_err()`。
3. **Python**（`s_readonly_server.py:41-44` `meta_dict` 增「键值必须都是文本」检查；`:121` `branches` 改为
   `_json_field(branches_json, (list, dict))`）。
4. **README**（`:35`）accept 示例补 `--profile <profile.json>`。

**耐久写路径逐字核**：对 `make_token/is_canonical_integer/parse_canonical_i64/canonical_event_content/
identity_key/open_db/verify_writer_epoch/cmd_init/cmd_accept/read_raw_events/build_object_and_witnesses/
cancel_own_begin/verify_begin_owner/verify_batch_bytes/persist_batch_before_publish/cmd_advance/
canonical_json/num_to_str/project_input_refs/project_raw_bars/cmd_catalog/cmd_snapshot/cmd_meta/run/main`
等函数在 e79 与 HEAD 两端按函数体 sha256 比对，**全部 IDENTICAL**；`git diff` 的 7 个 hunk 全部落在读路径
（json_shape/json_column/read_catalog/read_snapshot）与新测试，无任何写路径改动。8 项旧测试函数体未改。

## 1. 实跑证据（本人本轮实际执行）

- `cargo build --features s_session --bin s_structure_session` → exit 0。
- `cargo test --locked --features s_session --bin s_structure_session --jobs 1` → **9 passed / 0 failed**
  （8 旧 + 第 9 新 `full_signed_catalog_preserves_both_declared_branch_shapes`，已 `--list` 核对 9 名）。
- `cargo fmt -- --check` → 0；`cargo check --features s_session --bin s_structure_session` → 0；
  `cargo clippy --features s_session --bin s_structure_session` → 本片文件 0 命中。
- launcher 端到端（`--testonly --db /tmp/s_incr.sqlite --port 8796`）：init 116 项（wal/synchronous=2/
  sqlite 3.53.2）→ accept 7 → advance gen=1/5 objects → 只读进程 → stop 退出 0。
- **Rust `catalog` CLI**：116 项、`counts={implemented:1,not_implemented:115,run:1,not_run:115}`、
  `branches` 形状 82 数组 + 34 对象、CC-006 目录项含 evidence（batch_id/structure_cut/rule_revision/
  classified_objects=5 等），退出码 0。
- **HTTP**：`GET /api/catalog`、`/api/state`、`/api/snapshot`、`/api/meta` 全部 200；`/api/state`（浏览器
  单请求入口）返回 `ok:true`，`cut.generation/structure_cut/catalog_revision` 与 catalog/snapshot 三者
  相等（同 cut），catalog 116 项 + `counts={implemented:1,not_implemented:115}`，snapshot 5 objects/
  15 witnesses/20 relations；`branches` 形状 82/34。
- 浏览器 HTML `GET /` 200（14456 bytes），`load()` 仅调 `/api/state`（`index.html:231`）。
- **坏 branches 不伪 200**：对副本库把 `catalog.branches_json` 改为 `null`/`true`/`1`/`"text"`/`{`，
  Rust `catalog` 均 exit 1（「持久 JSON 形状不符」/「解析失败」），不返回成功空目录。
- **其余 JSON 列仍严格**（Python 函数级）：`comparisons` 传对象 → ValueError（须 list）；`evidence`/`detail`
  传数组 → ValueError（须 dict）。
- **Python meta 文本检查**：`meta_dict` 对 NULL 值、BLOB 值、BLOB 键均抛 ValueError（转 503）；
  正常文本键值通过；`scope` 缺失仍 `{}`、损坏/数组抛错。Rust 侧对 BLOB meta 值 `catalog`/`snapshot`/`meta`
  均 exit 1（`Invalid column type Blob`），两端一致。

## 2. H/M/L 清单

- **H：无。** 原 H-1（目录 branches 强制数组破坏 ReadCatalog/浏览器）已修：branches 联合 array|object，
  其余列严格不变；catalog CLI 与 `/api/catalog`、`/api/state` 恢复 116 项；坏值仍拒绝。
- **M：无。**
- **L-1（已修）**：README accept 示例缺 `--profile`（`s_session/README.md:35`）→ 已补。
- **L-2（维持，非阻断）**：`input_revision` 与 `revision` 两列均取 `e.revision`
  （`s_structure_session.rs:583`/`:591`），语义冗余；函数体与 e79 逐字未变，具体依据不变。
- **L-3（维持，域外观察）**：`Bar.source_index` 取接纳序 `seq` 而非 `source_coord`（`:937`），受测域
  两者相等无缺陷；逐字未变。
- **L-4（维持，域外观察）**：混合原始包含输入会同时产 merged 分类对象 + raw 域不满足观察（`:971` 起），
  受测域（严格无包含）不触发；逐字未变。

## 3. 复用的上一轮结论（源逐字未变，不重跑全片）

上一轮（`review-final.md`）AC1/AC2/AC3/AC4/AC5/AC8/AC9 的必需成功路径已实跑通过；本轮核对这 7 AC 对应的
全部生产函数与测试在 e79→HEAD 逐字未变，且 9 项 bin 测试（含 complete_batch 先持久后发布、cancellation
零清理、stale epoch、profile 重放、同价域观察+损坏 JSON 拒绝、wire 整数拒绝/投影）全部实跑通过，可安全
复用其通过结论。本轮额外跑通了 AC6/AC7 的完整目录成功路径。

## 4. 未接回 / 未验（不预写通过）

- 根在 macOS 的 native 构建、独立 HTTP/Node、持久顺序与**真实浏览器 GUI** 外部证据尚未接回；本容器不声称
  macOS fullfsync/GUI 通过。修复后 GUI 恢复命令：`./s_session/launch_s.sh --db <db> --port 8787` 后打开
  `http://127.0.0.1:8787/`。
- 全量 `cargo test`（有界验证优先，未重复无关全历史）。
- 跨进程杀/重启、真杀恢复、修订/Delta/Gap/背压/分页（TB-01-B/C、TB-05/07，本片不承担）。

## 5. 结论

受测域（具名 TestOnly 逐笔一对一退化 OHLC、相邻严格无包含、无同价端点竞争的 CC-006/local_shape）内，
原 FAIL 的阻断缺陷 H-1 已按最小修法正确修复且经本人实跑确认：完整目录成功路径（init 已签目录 → Rust
`catalog` CLI → `/api/catalog` → `/api/state` 浏览器单请求同 cut）恢复 116 项并给出
`counts={implemented:1,not_implemented:115}`；坏 branches/scalar/null 不伪 200；其余 JSON 列严格形状不变；
Python meta 文本检查与 Rust 对齐。**受测域内 9 AC 的必需成功路径现均成立**，无新增 H/M。macOS/GUI 等外部
证据仍待根接回，本报告不据此预先宣布全票验收，也不关闭 #1370/TB-01/SPEC #1340/图 #1323、不启用经济/交易、
不批准 main 合入或真实外效。

<promise>COMPLETE</promise>
