# #1370 TB-01-A 独立评审报告

> 评审对象：`codex/1323-tb01-a-1370` 相对精确基线 `efc1ddc4d6c015f4f8c6d44f904d704a88308b46` 的全部变更（`git diff BASE...HEAD`，2 个提交：`4379be6b51` + `ac2b59387b`）。
> 评审身份：独立评审会话，未继承实施者推理；只读评审，未修改生产代码。
> 评审方法：逐 AC 读当前代码 + 靶向真实验证（构建、单测、正式 launcher 端到端、只读 API/浏览器、并发、重放/冲突、>2^53 往返、域边界），不以实施自述或测试计数作结论。
> 结论：**PASS（受测域内）**。AC1..AC9 的必需成功路径全部经独立实跑证实；另列 6 项 L 观察与 3 项按票面范围明确推迟项（TB-05 崩溃恢复、历史 cut 导航、根接 macOS/真实 GUI）。最终 `<promise>COMPLETE</promise>`。

## 0. 验证环境

- 平台：Linux aarch64 容器；rustc 1.97.1；uv cpython-3.11.16-linux-aarch64-gnu。
- 构建：`cargo build --features s_session --bin s_structure_session` 退出 0（30.8s / 12.9s 增量）。
- 运行时需 `LD_LIBRARY_PATH=<uv python>/lib`（launcher 的 `configure_runtime` 已自动处理）。

## 1. 逐验收条件核验（AC1..AC9）

### AC1 —— 可实际启动/停止的正式 S 会话入口和只读查询/浏览器链 —— 通过

实跑（退出码均 0）：

```
./s_session/launch_s.sh --testonly --db /tmp/s1370_launch.sqlite --port 8799 --build
  → init(116 catalog items, wal, synchronous=2) → accept(7 accepted) → advance(gen=1, 5 objects)
  → 只读查询外壳(独立进程 PID 2107) → 5/5 完成
curl /api/snapshot → generation=1, objects=5, branches=[RISING,TOP,FALLING,BOTTOM,RISING]
curl /            → <title>S 正式结构会话 · 只读观察（#1370 TB-01-A）</title>
./s_session/launch_s.sh stop --port 8799 → 退出 0，按 PID 文件 + 进程身份核后 kill
```

结构对象由 `rust/src/bin/s_structure_session.rs:806-811` 逐事件构造 `Bar{open=high=low=close=price}`、`:815` 逐根 `ParseLayerIncr::append`、`:826` `classify_local_shape_sliding` 同次真实计算产生，非手填 StructureRecord、非 schema/enum/测试。launcher 链见 `s_session/launch_s.sh`（init→accept→advance→只读进程→浏览器）。

### AC2 —— 原始事件完整字段 + 一对一 O=H=L=C + TestOnly profile —— 通过

`raw_events` 表（`s_structure_session.rs:161-179`）持久 `identity_key/input_revision/payload_hash/receipt_id/seq/source_namespace/source_epoch/instrument/event_id/revision/received_at/raw_text/price/ts/volume/source_coord`。实跑 7 事件 → 7 merged bars（1:1 无折叠）；每对象 3 根 raw/merged 见证，见证 `raw_bars` 含完整身份（seq/source_coord/event_id/identity_key/revision/received_at/receipt_id/namespace/epoch/instrument/price/ts/volume）。

TestOnly profile（`s_session/profiles/testonly_tick_1_1_ohlc.json`：unit=tick、clock=fixed_seq_clock、one_to_one_ohlc、`not_product_default:true`）由 accept 显式加载绑定（`s_structure_session.rs:449-458`），输入 `profile` 必须等于 `--profile` 的 `profile_id`（`:459-464`）；`volume_unit='1'` 精确拒绝非 1（`:493-501`，实跑 volume='2' → exit 1 InvalidDomain）。

### AC3 —— 独立 oracle 对拍真实 Rust 四分支 + 见证 —— 通过

单测 `local_shape::tests::cc006_four_branch_oracle`（`rust/src/theta_v0/classifier/local_shape.rs:244-251`）用独立硬编码期望对拍 `classify_local_shape`，实跑 `cargo test --lib local_shape` → **4 passed**（四分支 oracle + equal/含包含域前件 + 滑窗计数）。`classify_local_shape` 复用 `parser::inclusion::contains/strict_dir`（`local_shape.rs:19,147-158`）——与现役 parser 的包含处理是同一判断，不另起第二查法（#804）。

会话级：7 事件流 [10000,10500,11000,10500,10000,10500,11000] → 5 对象 = RISING/TOP/FALLING/BOTTOM/RISING；每对象 dir_ab/dir_bc + 4 个严格比较（all held=true）+ 3 根 raw/merged 见证。边界实测：1 tick → 0 对象 + 1 `insufficient_knowledge`；[10000,10000,11000] → 0 对象 + 1 `domain_not_satisfied(adjacent_inclusion)`；不造第五 Other 分型（`local_shape.rs:110-120` 只报违反原因）。

### AC4 —— S 自有 SQLite/WAL 单写者 + 原子 Commit —— 通过

S 自己 `--db` 文件，不复用 `trading_system/persistence/database.py` 共库。实跑 init 读数：`sqlite_version=3.53.2`（libsqlite3-sys 0.38.2 bundled，≥ C09 要求的 3.51.3 WAL-reset 修复，`Cargo.lock` 钉 0.40.2/0.38.2）、`journal_mode=wal`、`synchronous=2`(FULL)、`platform=linux`。Linux 容器不声称 macOS fullfsync（`s_structure_session.rs:243-245` 仅 `#[cfg(target_os="macos")]`）。

原子 Commit：`advance` 先短写事务核 `writer_epoch`/`advance_state` 并持久 Begin/门/输入前沿（`:747-779`），再同次 Rust 计算，Commit 用 IMMEDIATE 事务核 Begin/epoch/generation/前沿未变后**同一事务**先 `INSERT batches`（`:1009-1013`，内容寻址 batch_id=sha256(规范字节)）再发布 objects/witnesses/relations/observations + meta 可达根（`:1160-1164`）+ 目录状态（`:1168-1191`）。读端只读已提交 cut：Rust `catalog/snapshot` 用 `unchecked_transaction` 单读事务（`:1368-1390`）；Python 只读进程 `mode=ro` + `query_only=ON`（`s_session/s_readonly_server.py:27-31`，实跑写被 `attempt to write a readonly database` 拒绝）。

并发实测：两个 `advance` 同时跑 → 退出码 1/0，恰一个胜出（gen+1），另一个 StaleWriter；且 batch_id 与串行重跑逐字节一致（`batch-cb49be6f…`，确定性内容寻址）。

### AC5 —— 重放/冲突 + 精确整数 + payload_hash 绑定规范字节 —— 通过

- 重放：重跑 accept 同一输入 → 全部 `status=replay`，返回原 `receipt_id`（实测 `rcpt-d31c5cebbfd2fff3` 等与原 accept 一致）。
- 冲突：同身份（`identity_key=(namespace|epoch|instrument|event_id)`，`:153-155`）异内容 → `status=IdentityConflict`，existing/new payload_hash 与 receipt_id 分列，不覆盖。
- 精确整数：wire 上规范十进制字符串（TEXT 存储 + JSON 字符串输出），Rust 仅计算边界 `parse_canonical_i64`（`:90-105`，拒绝 `+`/空白/前导零/`-0`，实跑 `+0010000` → exit 1）。>2^53 往返实测：`9007199254740993`(2^53+1)/`9007199254740991`(2^53-1)/`9007199254740990` 经 accept→advance→snapshot 逐字一致（witness raw_bars.price 与 comparisons.prev/cur 均为原字符串），不依赖 JS Number/f64。
- payload_hash：`canonical_event_content` 固定键序紧凑 JSON（`:137-147`），实测同内容不同键序/缩进的输入 hash 相同（`b7e0bbd4…`），同内容不同身份不撞收据（receipt 绑定 `ikey|payload_hash`）。

### AC6 —— ReadCatalog 全目录 + 未实现不隐藏 —— 通过

`catalog` 从 `s_session/catalog/signed-catalog.json`（116 项 = 62 CC + 44 ST + 10 LC，source_sha256=`76019aba…` 与 `SPEC-COVERAGE-INPUT.json` 实算一致）全部写入，未实现不隐藏。实跑 `counts={implemented:1,not_implemented:115,run:1,not_run:115}`；CC-006=`implemented/not_proved/run`（proof_status 诚实：session 不执行 oracle，证明状态不伪造；evidence 绑定 batch_id/structure_cut/profile 哈希/rule_revision/实例计数/oracle 引用，`:1169-1189`）。未触发中枢/线段/BSP 的 CC-001..005/007..062/ST/LC 均 `not_implemented`，不标已验收。浏览器对未实现/域不适用/知识不足/无实例分别显示（`s_session/browser/index.html:139-160,165-187`）。

### AC7 —— Snapshot 完整字段 + 浏览器读同一 cut —— 通过

`snapshot` 返回 session_id/generation/structure_cut/catalog_revision/scope/index_frontier/profile_id/profile_hash + objects/witnesses/relations/observations（实测 5 对象/15 见证/20 关系；追加后 8/24/32）。浏览器单请求 `/api/state` 在同一读事务内读 catalog+snapshot（`s_readonly_server.py:174-184`），并核 catalog/snapshot 的 cut 一致（`index.html:205-209`）；渲染对价格只 `textContent` 字符串呈现，无 Number/parseFloat 处理价格（`index.html` 注释与 `esc()`），不按价格/marker 补判结构、不选操作级别、不推算经济资格。

### AC8 —— E/B/X 不启动仍继续推进 —— 通过

launcher 只启动 S（一次性写命令）+ 只读查询外壳；不启动 E/B/X、不建其库、不加载经济政策。`scope={"structure":"CompleteCut","economic":"not_started"}`（`:335-339`）。追加实测：accept `append_more.json`(3 accepted) → advance → gen=2/cut-2/merged_bars=10/8 对象（重启恢复不删库，`:285-287` 库存在即跳过 init；重放=replay）。不等待 JoinCut/经济 ack/全局序号，不称整个系统当前完整。

### AC9 —— 证据保留 + 检查运行 + 未验项照实 —— 通过

保留：launcher 命令、inputs/profiles/schema、signed-catalog.json、Cargo.lock 钉 rusqlite 0.40.2/libsqlite3-sys 0.38.2（SQLite 3.53.2）、profile/输入文件哈希（实算与报告一致：catalog `938b0ef5…`、profile `2cd50e43…`、cc006 `7bb94ef1…`）。Rust 检查独立复跑：`cargo check --all-targets`(default) 退出 0、`cargo fmt --check` 通过、`cargo clippy --features s_session --bin s_structure_session` 本片文件（s_structure_session.rs/local_shape.rs）0 命中、`cargo test --lib local_shape` 4 passed、`cargo test --lib inclusion` 24 passed/1 ignored(ES 数据，既有)。前端为单文件 vanilla JS（无 TS/Vite 构建，本片未改 `frontend/`），`curl /` 200 且标题正确。未跑项照实：真实 GUI 浏览器渲染、macOS fullfsync/launcher 真实运行、全量 cargo test（有界验证优先）——均如实标注 NOT_RUN，不捏造通过。

## 2. 深入专项审（票面点名）

- **同次 Rust 计算 / 无第二判定**：`advance` 直接调 `theta_v0::parser::ParseLayerIncr::append`（`:815`，现役 parser 增量入口）与 `classifier::local_shape::classify_local_shape_sliding`（`:826`）；CC-006 的方向/非包含判定复用 `inclusion::strict_dir/contains`（`local_shape.rs:19,147-158`），无第二份生产 Python 判定器，不调用 ThetaPiStream 资金推进冒充 S。仓库 grep 无既有 RISING/TOP/BOTTOM/FALLING 四分支实现，`local_shape` 非重复判据。
- **精确身份/字节/哈希**：业务身份键 `(namespace|epoch|instrument|event_id)`；`payload_hash` 绑定固定键序紧凑字节（实跑对键序/缩进不敏感）；object_id 由含 input_refs/比较/规则/profile 的对象规范字节 sha256 内容寻址（`:684-697`）；batch_id/收据/关系/见证 id 同源内容寻址；`input_revision` 与 `object_revision(=1 不可变批次)` 分列。
- **SQLite 版本/FULL 与原子可达 cut**：3.53.2 ≥ 3.51.3；WAL+synchronous=FULL；Commit 先持久不可变批次再同事务发布对象/关系/修订/索引根；读端只读已提交 cut（只读进程 mode=ro + query_only + 显式 BEGIN）。
- **真正只读进程**：`s_readonly_server.py` 为独立进程，`Path.resolve().as_uri()+"?mode=ro"` + `PRAGMA query_only=ON`，只 SELECT；实测写被拒绝。
- **目录未实现状态**：116 项全列出，未实现条目不隐藏，CC-006 状态与 cut 同事务生效。
- **客户端整数与同源证据**：价格/时间戳/成交量全程规范十进制字符串，>2^53 往返反例逐字一致；浏览器 textContent 字符串呈现。
- **无经济配置实际推进**：不启动/不建 E/B/X 库、不加载经济政策，追加输入继续推进 cut（gen 1→2 实跑），经济 scope 明确 not_started。

## 3. 缺陷清单（H/M/L）

未发现 H（高危）缺陷。以下均为 L（低），且 1-6 属本片受测域外或纯展示层，不阻断本片必需成功：

- **L-1（展示层）**：`s_session/browser/index.html:155` 读 `o.detail.merged_bars`，但 insufficient_knowledge 的 detail 实为 `{"raw_events":N}`（`s_structure_session.rs:845`），浏览器对知识不足会显示 `merged_bars=undefined`。改：显示 `raw_events` 或不显示该字段。
- **L-2（历史导航）**：`advance` 每次 `DELETE FROM objects/witnesses/relations/observations`（`s_structure_session.rs:1014-1020`）再重插当前 cut；历史 cut 的结构化对象仅存于 `batches.canonical_bytes`（不透明 BLOB），不可按行查询/导航。受测域内对象内容寻址稳定故无可观察破裂；结构化历史导航属后续切片。
- **L-3（坐标语义）**：`Bar.source_index` 取接纳序 `seq`（`s_structure_session.rs:795`）而非 `source_coord`。受测域两者相等（seq "0..9" == 接纳序），无可观察缺陷；若源坐标≠接纳序，parser 的合并组锚（按 source_index）会偏离源坐标语义。source_coord 已单独在见证保留，身份未丢。
- **L-4（域边界双报）**：混合原始包含输入（如 [10000,10500,10500,11000]）会同时产 1 个 merged 窗口分类对象 + 2 个原始窗口 domain_not_satisfied 观察（raw_violations 循环 `:829-848` 与 wins 循环 `:856+` 索引空间不同）。受测域（严格无包含）不触发；行为可辩护但易误读。
- **L-5（读命令开库）**：Rust `catalog/snapshot/meta` 经 `open_db`（`:238-247`）以读写模式打开并执行 `PRAGMA journal_mode=WAL; synchronous=FULL`（init 后为 no-op）。外部真正只读进程（Python server）正确 mode=ro；S 侧读命令属 S 自身域，无功能性写，仅为卫生注意。
- **L-6（输入字段冗余）**：`raw_events` 同时存 `input_revision` 与 `revision` 两列且均取 `e.revision`（`:469-473,532,536`），语义冗余易混淆。

### 按票面范围明确推迟（非本片缺陷，如实记录）

- **TB-05**：Begin 后进程被杀/崩溃恢复（`advance_state="begun:…"` 残留 → StaleWriter，`:751-754`；仅 `reset` 删除重建可重开，`:1448-1452`）。接口合同「中断保留 Begin、恢复仍闭门」本片做到「保留+闭门」，但「从 Begin 正确续算」属 TB-05 故障恢复，票面与本片报告均如实声明。
- **历史 cut 导航 / 重连续接 / delta 游标**：属 TB-07 / AT-04，本片 snapshot 只读当前 cut。
- **根接**：真实 GUI 浏览器渲染、macOS fullfsync 与 macOS launcher 真实运行——本容器无浏览器、Linux 不声称 macOS fullfsync，恢复命令已列于实施报告 §8。

## 4. 未验平台与未跑项

- macOS：fullfsync、launcher `.dylib` 发现与构建产物平台核（`launch_s.sh` 已按 sysconfig 处理，未在真实 macOS 验）。
- 真实 GUI 浏览器渲染（HTML/API 链已 curl 验证，DOM 渲染未验）。
- 全量 `cargo test`（有界验证优先，本片改动面已单测）。
- 网络文件系统上的 WAL 语义（本地 /tmp 验证，不承诺 NFS）。

## 5. 评审结论

受测域（具名 TestOnly 逐笔一对一退化 OHLC、相邻严格无包含、无同价端点竞争的 CC-006/local_shape）内，AC1..AC9 必需成功路径全部经独立实跑证实：真实 launcher → 现役 Rust parser 同次四分支 → 独立 SQLite/WAL/FULL 原子 Commit → 只读进程/浏览器 → E/B/X 缺失下继续推进；精确整数、身份/字节/哈希、重放/冲突、目录未实现状态均实证。无高危缺陷；6 项 L 观察与 3 项按票面明确推迟项不阻断本片。**判定：PASS（受测域内）**，最小修复建议见 L-1..L-6（可选，均非必需成功项）。

<promise>COMPLETE</promise>
