# #1370 TB-01-A 独立评审报告（最终轮，追加修复后，HEAD `e79c74a17a`）

> 评审对象：`codex/1323-tb01-a-1370` 相对精确基线 `efc1ddc4d6c015f4f8c6d44f904d704a88308b46` 的全部变更
> （`git diff BASE...HEAD`；6 个提交，HEAD=`e79c74a17a800e660b515e1f88fc9a5706826b42`）。
> 评审身份：独立评审会话，未继承实施者推理；只读评审，未修改生产源码/CI/根报告。
> 方法：先完整读取本票 `REPAIR-REPORT.md` 与 `ROOT-REPAIR-REPORT.md`，再以最终源码独立核全部 9 AC 与适用受测域；
> 逐条实跑（构建、8 项 bin 回归、lib oracle、launcher 端到端、只读 API、重放/冲突、>2^53 往返、域边界、
> 持久顺序、竞态交错、epoch/代际零清理），不以实施自述或测试计数作结论。
> **结论：FAIL（受测域内）。** AC6/AC7 的必需成功路径（ReadCatalog 全目录列表、浏览器按同一切面呈现）被
> 最终提交 `e79c74a17a` 的「拒绝损坏读取」改动破坏：目录 `branches` 字段被强制要求为数组，而已签目录
> 116 项中有 34 项的 `branches` 是对象（非分区/描述轴的合法表示），导致 `catalog` 读命令与
> `/api/catalog`、`/api/state`（浏览器单请求入口）全部失败。其余 AC1/AC2/AC3/AC4/AC5/AC8/AC9 的必需成功
> 路径在本容器独立实跑通过。旧 `f635` 报告 PASS 不继承；根在 macOS 的并行 HTTP/Node/持久顺序/真实浏览器
> 外部证据尚未接回，本报告不据此预先宣布全票验收。

## 0. 验证环境

- 平台：Linux aarch64 容器（`uname -m`=aarch64，`uname -s`=Linux）；rustc/cargo 1.97.1；
  Python 3.11（uv cpython-3.11.16-linux-aarch64-gnu，含 libpython3.11.so）。
- 构建：`cargo build --features s_session --bin s_structure_session` 退出 0。
- 运行需 `LD_LIBRARY_PATH=<uv python>/lib`（launcher 的 `configure_runtime` 已自动处理）。
- SQLite 实际读数（init 输出 + Cargo.lock）：`sqlite_version=3.53.2`（libsqlite3-sys 0.38.2 bundled，
  ≥ C09.2 要求的 3.51.3 WAL-reset 修复）、`journal_mode=wal`、`synchronous=2`(FULL)、`platform=linux`。
  本容器不声称 macOS fullfsync（代码仅 `#[cfg(target_os="macos")]`，见 `s_structure_session.rs:246-248`）。

## 1. 逐验收条件核验（AC1..AC9）

### AC1 —— 可实际启动/停止的正式 S 会话入口与只读查询/浏览器链 —— 通过（本容器实跑）

`./s_session/launch_s.sh --testonly --db /tmp/s_review_1370.sqlite --port 8797 --build` 退出 0，
链为 `init(116 items, wal, synchronous=2, sqlite 3.53.2) → accept(7 accepted) → advance(gen=1, 5 objects)
→ 只读查询外壳(独立进程 pid=902) → 5/5 完成`；`stop --port 8797` 退出 0，进程消失。
结构对象由 `s_structure_session.rs:928-953` 逐事件构造 `Bar{open=high=low=close=price}`、
`:947` 逐根 `ParseLayerIncr::append`、`:953` `classify_local_shape_sliding` 同次真实计算产生，
非手填 StructureRecord、非 schema/enum/测试。注意：浏览器 HTML 可由 `GET /` 200 取到，但浏览器
`load()` 单请求 `/api/state` 当前 503（见 H-1），页面呈现「读取失败」——入口链存在，但浏览器端未达可用。

### AC2 —— 原始事件完整字段 + 一对一 O=H=L=C + TestOnly profile —— 通过

`raw_events` 表（`s_structure_session.rs:164-181`）持久 identity_key/input_revision/payload_hash/
receipt_id/seq(接纳序)/source_namespace/source_epoch/instrument/event_id/revision/received_at/raw_text/
price/ts/volume/source_coord。实跑 7 事件 → 7 merged bars（无折叠），追加后 10 事件 → 10 merged；
每见证 `raw_bars` 恰 1 根原始成员、`merged_source_index` 覆盖 0..9 连续（1:1，不聚合/不重采样/不丢序）。
TestOnly profile（`s_session/profiles/testonly_tick_1_1_ohlc.json`：unit=tick、clock=fixed_seq_clock、
one_to_one_ohlc、`not_product_default:true`）由 accept 显式加载绑定；`volume_unit='1'` 精确拒绝非 1。

### AC3 —— 独立 oracle 对拍真实 Rust 四分支 + 见证 —— 通过

单测 `local_shape::tests::cc006_four_branch_oracle`（`local_shape.rs:263-268`）用独立硬编码期望值
对拍 `classify_local_shape`；`cargo test --lib local_shape` → 4 passed。
会话级 snapshot 实跑：7 事件流 → 5 对象 = RISING/TOP/FALLING/BOTTOM/RISING，每对象两次方向
（dir_ab/dir_bc）+ 4 个严格比较（全部 held=true）+ 3 根 raw/merged 见证（slot 0/1/2）；15 见证/20 关系。
边界实测：1 tick → 0 对象 + 1 `insufficient_knowledge`；`[10000,10000,10000]` → 0 对象 + 1
`domain_not_satisfied(adjacent_inclusion)`；不造第五 Other 分型（`local_shape.rs:110-123`）。

### AC4 —— S 自有 SQLite/WAL 单写者 + 完整批次先持久后发布原子 cut —— 通过

S 自己 `--db` 文件，不复用 `trading_system/persistence/database.py` 共库；WAL+synchronous=FULL 实读
（3.53.2 / wal / synchronous=2）。持久顺序：`advance` 先短写事务核 `writer_epoch`/`advance_state` 持久
Begin/门/输入前沿（`:887-919`），同次 Rust 计算后 `persist_batch_before_publish` 在**独立有限事务**先
INSERT 完整内容寻址批次并 `verify_batch_bytes`（`:854-878`、`:1128`），提交成功后才进入结构发布事务
（`:1133-1301`）核 Begin/epoch/generation/前沿后同一事务发布 objects/witnesses/relations/observations +
meta 可达根（generation/structure_cut/index_frontier，`:1263-1265`）+ 目录状态（`:1268-1298`）。
bin 测试 `complete_batch_is_visible_before_reachable_root_and_never_overwritten` 实跑通过：
独立只读连接可读批次而根仍 `cut-0`/`index_frontier=""`；同 ID 异字节与字节被篡改均拒绝。读端只读已提交
cut：Rust `catalog/snapshot` 用 `unchecked_transaction` 单读事务；Python `mode=ro`+`query_only=ON`，
写被 `attempt to write a readonly database` 拒绝（实跑）。竞态交错实跑：advance A 在 Begin 后与 accept
交错，A 退出 1 且「已正常取消并释放推进权」、`advance_state=idle`、generation 不变；后续 advance B
退出 0（generation=1）。

### AC5 —— 重放/冲突 + 精确整数 + payload_hash 绑定规范字节 —— 通过

- 重放：重跑 `accept` 同一输入 → 全部 `replay` 返回原 receipt_id（实跑 `rcpt-d31c5cebbfd2fff3` 等一致）。
- 冲突：同身份（`(namespace|epoch|instrument|event_id)`，`:151-153`）异内容 → `IdentityConflict`，
  existing/new payload_hash 与 receipt_id 分列，不覆盖（实跑）。
- 精确整数：wire 投影字段（accept.results[].seq、advance.generation、snapshot.objects[].object_revision/
  window_*/input_refs[].merged_index/raw_refs[].seq、witnesses[].slot/merged_source_index/raw_bars[].seq、
  observations 窗口坐标）均为规范十进制字符串（实跑逐字段 `str`）。>2^53 往返反例实跑：
  `9007199254740993`/`9007199254740991`/`9007199254740990` 经 accept→advance→snapshot 逐字一致
  （witness raw_bars.price 与 comparisons.prev/cur 均为原字符串），不依赖 JS Number/f64。
  `parse_canonical_i64`（`:96-104`）拒绝 `+`/空白/前导零/`-0`；`num_to_str`（`:1389-1393`）拒绝浮点/
  布尔/空值/文本。
- payload_hash：`canonical_event_content` 固定键序（`:136-148`）；同内容不同键序/缩进不敏感（结构体
  反序列化 + 固定键序字符串），hash 绑定规范字节。
- profile 重放不改已公布 cut 来源：同 profile_id 异字节 accept → `IdentityConflict`，`meta.profile_hash`
  前后不变（实跑）。

### AC6 —— ReadCatalog 全目录 + 未实现不隐藏 —— **失败（H-1）**

`catalog` 表 init 时写入全部 116 项（`:353-419`），但读取路径被最终提交破坏：`read_catalog_in_tx`
用 `json_column(r, 4, true)` 强制 `branches` 为数组（`:1478`），而已签目录 116 项中 34 项的 `branches`
是对象（非分区/描述轴），读第 1 个 dict 项即抛「持久 JSON 形状不符」。实跑：Rust `catalog` 子命令
exit 1（`collect catalog 失败：Conversion error from type Text at index: 4`）；Python
`GET /api/catalog` 503、`GET /api/state` 503。目录当前无法列出任何条目，未实现条目更无法展示——AC6 不成立。

### AC7 —— Snapshot 完整字段 + 浏览器读同一 cut —— **失败（H-1）**

`snapshot`（Rust 与 `/api/snapshot`）本身实跑通过：返回 session_id/generation/structure_cut/
catalog_revision/scope/index_frontier/profile_id/profile_hash + objects/witnesses/relations/observations
（5 对象/15 见证/20 关系；追加后 8/24/32）。但浏览器单请求 `/api/state`（`index.html:231`）因同目录读取
回归返回 503，`load()` 进入错误分支（`index.html:245-255`）呈现「读取失败」，无法按服务返回值渲染对象/
关系/见证。AC7 的浏览器必需成功路径不成立。浏览器错误处理本身诚实（不伪造成功空集）。

### AC8 —— E/B/X 不启动仍继续推进 —— 通过

launcher 只启动 S（一次性写命令）+ 只读查询外壳；不启动 E/B/X、不建其库、不加载经济政策。
`scope={"structure":"CompleteCut","economic":"not_started"}`。追加实测：`accept append_more.json`(3)
→ `advance` → generation=2、raw=10、merged=10、windows=8、objects=8；重启恢复不删库（库存在即跳过
init，`:208-213`）；不等待 JoinCut/经济 ack/全局序号，不称整个系统当前完整。

### AC9 —— 证据保留 + 检查运行 + 未验项照实 —— 通过（但 catalog/浏览器两项因 H-1 转为失败路径）

保留：launcher 命令、inputs/profiles/schema、signed-catalog.json、Cargo.lock 钉 rusqlite 0.40.2/
libsqlite3-sys 0.38.2（SQLite 3.53.2）、profile/输入哈希。Rust 检查独立复跑：`cargo fmt -- --check` 0；
`cargo check --all-targets`（default）0；`cargo check --features s_session --bin s_structure_session` 0；
`cargo clippy --features s_session --bin s_structure_session` 0（本片文件 0 命中，lib 既有 warning 不计）；
`cargo test --locked --features s_session --bin s_structure_session --jobs 1` → **8 passed**；
`cargo test --lib local_shape` → 4 passed；`cargo test --lib inclusion` → 24 passed/1 ignored（既有）。
未跑项照实见 §4。

## 2. 缺陷清单

### H-1（阻塞，AC6/AC7 必需成功路径破坏）：目录 `branches` 形状校验把 34 项合法对象误判为损坏

- **文件/行号**：
  - `rust/src/bin/s_structure_session.rs:1478`（`"branches": json_column(r, 4, true)?`）+ 辅助
    `json_shape` `:1433-1441`（`is_array` 时强制 `value.is_array()`）；
  - `s_session/s_readonly_server.py:119`（`"branches": _json_field(branches_json, list)`）+ 辅助
    `_json_field` `:60-67`。
- **真实反例（实跑）**：已签目录 `s_session/catalog/signed-catalog.json` 的 116 项中，`branches` 为
  **对象**的有 34 项（CC-002/003/007/008/009/011/013/014/016/019/021/024/027/028/031/032/034/035/
  038/039/041/043/044/045/047/048/050/052/054/055/056/057/059/062）。这些是「非分区/描述轴/生成器/
  并存义务」的合法表示，不是损坏数据。最终提交 `e79c74a17a` 把 `branches` 从「宽松解析」改成「强制
  数组」，`read_catalog_in_tx` 与 `read_catalog` 读第 1 个 dict 项即失败。
  - Rust：`s_structure_session catalog --db <库>` → exit 1，
    `{"error":"collect catalog 失败：Conversion error from type Text at index: 4, 持久 JSON 形状不符"}`。
  - Python：`GET /api/catalog` → 503 `{"error":"StorageUnavailable","detail":"持久 JSON 形状不符"}`；
    `GET /api/state` → 503（浏览器 `load()` 单请求入口失败，页面只显示「读取失败」）。
- **根因**：目录 `branches` 字段天然异构（82 数组 + 34 对象），不能用单一数组形状校验；
  需要「拒绝损坏 JSON/标量」而非「拒绝对象」。
- **最小修法**：`branches` 读取接受 array 或 object 两种容器（仍拒绝 null/string/number/bool 及坏 JSON）。
  Rust 侧给 `json_shape` 增加「容器（array-or-object）」模式（或单独 `json_shape_container`），
  `:1478` 改用该模式；Python 侧 `_json_field(branches_json, (list, dict))`（或专用
  `_json_container_field`）。保持 `evidence`(dict)、`comparisons`/`input_refs`/`raw_bars`(list)、
  `detail`(dict) 的现有严格校验不变（这些列确实只应是单一形状）。修复后应实跑
  `catalog` 命令与 `/api/catalog`、`/api/state` 返回 116 项 + `counts={implemented:1,not_implemented:115}`
  且浏览器可渲染。

### L-1（文档，可忽略）：`s_session/README.md:35` accept 示例缺 `--profile`

CLI 实际要求 `--profile`（`cmd_accept` 缺参报错、launcher 总是传），README 接口示例写成
`accept --db <路径> --input <输入.json>`，与 `usage()`（`:1647`）不一致。

### L-2（冗余，可忽略）：`input_revision` 与 `revision` 两列均取 `e.revision`

`s_structure_session.rs:583` 与 `:591` 都把 `RawEvent.revision` 写入两列；本 schema 的 `RawEvent`
只有单一 `revision` 字段，无输入档案级独立修订，故两列语义冗余（不阻断受测域）。

### L-3（域外观察）：`Bar.source_index` 取接纳序 `seq` 而非 `source_coord`

`s_structure_session.rs:937` 用 `ev["seq"]`（接纳序）构造 `Bar.source_index`。受测域内
seq(0..9)==source_coord(0..9) 相等，无可观察缺陷；`source_coord` 已单独在见证保留。若源坐标≠接纳序，
parser 合并组锚（按 source_index）会偏离源坐标语义——属受测域外，记录不阻断。

### L-4（域外观察）：混合原始包含输入会同时产 merged 分类对象 + 原始窗口 domain_not_satisfied 观察

`s_structure_session.rs:971-978`（raw 级循环）与 `:1007-1092`（merged 级循环）索引空间不同；
混合输入（如 `[10000,10500,10500,11000]`）可同时产出 1 个 merged 分类 + 2 个 raw 域不满足观察。
受测域（严格无包含）不触发；去重键（`:1099-1106`）已避免同窗口同因重复。

## 3. 深入专项审（票面点名）

- **同次 Rust 计算 / 无第二判定**：`advance` 直接调 `theta_v0::parser::ParseLayerIncr::append`
  （`:947`，现役 parser 增量入口）与 `classifier::local_shape::classify_local_shape_sliding`（`:953`）；
  CC-006 的「非包含/方向」复用 `inclusion::contains/strict_dir`（`local_shape.rs:20,150-157`），
  是现役 parser 同一份判断（`inclusion.rs` 由 `fn` 改 `pub(crate)`，语义未变），无第二份生产 Python
  判定器，不调用 ThetaPiStream 资金推进冒充 S。仓库 grep 无既有四分支实现，`local_shape` 非重复判据。
- **精确身份/字节/哈希**：业务身份键 `(namespace|epoch|instrument|event_id)`（`:151-153`）；
  `payload_hash=sha256(固定键序规范字节)`（`:136-148`）；object_id 由含 input_refs/比较/规则/profile
  的对象规范字节 sha256 内容寻址（`:695-712`）；batch_id/收据/关系/见证 id 同源内容寻址；
  `input_revision` 与 `object_revision(=1 不可变批次)` 分列。
- **SQLite 版本/FULL 与原子可达 cut**：3.53.2 ≥ 3.51.3；WAL+synchronous=FULL；完整批次先独立持久可读，
  再短事务 CAS 发布对象/关系/修订/索引根；读端只读已提交 cut（mode=ro + query_only + 显式 BEGIN）。
  正常前沿变化条件取消自身（`cancel_own_begin` CAS 核 epoch/token/gen/frontier/generation，
  `:774-816`），epoch/token/gen 任一不符即零清理（实跑 `cancellation_preserves_foreign_begin...` 通过）。
- **真正只读进程**：`s_readonly_server.py` 独立进程，`Path.as_uri()+"?mode=ro"` + `PRAGMA query_only=ON`，
  只 SELECT；写被拒绝（实跑）。Rust 读命令 `open_db` 以读写模式打开并执行 WAL/FULL PRAGMA（init 后
  no-op），属 S 自身域，无功能性写——卫生注意（沿用旧 L-5）。
- **目录未实现状态**：init 把 116 项全部写入、status 初始 not_implemented（`:353-419`）；CC-006 在
  advance Commit 同事务置 implemented/not_proved/run 并绑定 evidence（`:1268-1298`）。但读取端当前被
  H-1 阻断，无法列出（见 H-1）。
- **客户端整数与同源证据**：价格/时间戳/成交量/接纳序/源锚全程规范十进制字符串，>2^53 往返反例实跑
  逐字一致；浏览器对价格只 `textContent` 字符串呈现（`index.html:94-95`），无 Number/parseFloat 处理。
- **无经济配置实际推进**：不启动/不建 E/B/X 库、不加载经济政策，追加输入继续推进 cut（gen 1→2 实跑），
  经济 scope 明确 not_started，不等待经济 ack/全局序号。

## 4. 未验平台与未跑项（NOT_RUN，照实）

- macOS：fullfsync、launcher `.dylib` 发现与构建产物平台核（代码仅 macOS 开启，本容器不声称已测）。
- 真实 GUI 浏览器渲染（HTML/API 链已 curl 验证；且当前 `/api/state` 503，浏览器无法渲染——修复 H-1 后
  需重跑）。恢复命令：`./s_session/launch_s.sh --db <db> --port 8787` 后打开 `http://127.0.0.1:8787/`。
- 全量 `cargo test`（有界验证优先，本片改动面已单测）。
- 跨进程杀/重启、真杀恢复、修订/Delta/Gap/背压/分页（票面明确由 TB-01-B/C、TB-05/07 承接，本片不承担）。
- 根在 macOS 的独立 HTTP/Node、持久顺序与真实浏览器外部证据尚未接回，本报告不据此预先宣布全票验收。

## 5. 评审结论

受测域（具名 TestOnly 逐笔一对一退化 OHLC、相邻严格无包含、无同价端点竞争的 CC-006/local_shape）内，
AC1/AC2/AC3/AC4/AC5/AC8/AC9 的必需成功路径经本容器独立实跑证实；但 AC6（ReadCatalog 全目录列表）与
AC7（浏览器读同一切面呈现）被最终提交 `e79c74a17a` 的目录 `branches` 形状校验回归破坏，两 AC 的必需
成功路径当前**不成立**（`catalog` 读命令 exit 1、`/api/catalog` 与 `/api/state` 503、浏览器只显示
「读取失败」）。按票面「未达必需成功即 FAIL 并列最小修复」，本片判定 **FAIL**，最小修复见 H-1
（branches 读取接受 array-or-object，其余列严格校验不变），修复后须实跑 catalog/API/浏览器重验 AC6/AC7。

本报告不关闭 #1370、TB-01、SPEC #1340 或图 #1323；不启用经济/交易，不批准 main 合入或真实外效。

<promise>COMPLETE</promise>
