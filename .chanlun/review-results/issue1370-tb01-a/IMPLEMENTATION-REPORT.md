# #1370 TB-01-A 实施报告（正式结构会话贯通只读观察）

> 票：GitHub Issue #1370（TB-01-A 内部执行单元，SPEC #1340 / 图 #1323 / SPEC #1339 R2）。
> 实施分支：`codex/1323-tb01-a-1370`；精确基线：`efc1ddc4d6c015f4f8c6d44f904d704a88308b46`。
> 编号声明：本提交中的 #1370 仅指 GitHub Issue。
> 本报告是工作草稿（活期绑定票），不是独立评审，也不以自审 PASS 冒充独评。

## 0. 结论（一句话）

已交付可实际启动/停止的正式 S 会话入口（Rust 唯一结构核，独立 SQLite/WAL 单写者）与只读查询/浏览器链：
原始输入 → S.AcceptInput → 本地 Begin → 现役 Rust `ParseLayerIncr` 同次真实计算 CC-006 `local_shape`
四分支 → 独立持久 Commit（内容寻址批次 + 对象/关系/修订/索引可达根）→ S.ReadCatalog / S.Snapshot →
只读查询外壳（独立只读进程）→ 浏览器。E/B/X 不启动、不建其库、不加载经济政策，仍继续接纳追加输入。

受测域固定为具名 TestOnly 原始逐笔事件一对一精确退化 OHLC（O=H=L=C）、相邻严格无包含的
`CC-006/local_shape`。只证明受测域与协议，不把分型窗口等同于笔/线段/中枢，不以单 CC 通过宣布全 62 CC。
真实 GUI 浏览器渲染与 macOS fullfsync 前件未验（见 §8 NOT_RUN，含恢复命令）。

## 0a. 代码初审 / Python R1 复核的修复记录（已并入本提交）

首次实施被调度超时中断后，冻结工作树经过代码初审（5H+5M）与 Python 固定副本 R1 复核（2M）。
本提交已按有界修复方向逐项处理，摘要如下（验证证据见 `AC-REPORT.json`）：

| 缺陷 | 修复 |
|---|---|
| H1 Snapshot/catalog 跨 cut 混读 | Rust `catalog/snapshot` 单读事务；Python 显式 `BEGIN` 读事务 + 浏览器单请求 `/api/state`；确定性 WAL 交错复现快照一致 |
| H2 判定前无持久 Begin / writer_epoch 未校验 | `advance` 先短写事务核 `writer_epoch`/`advance_state`、持久 Begin/门/输入前沿，Commit 核 Begin/epoch/前沿仍有效后发布；并发 advance 恰一个胜出 |
| H3 目录状态无条件标成功、知识不足/域不满足被丢弃 | 实现/证明/运行分列且绑定 evidence；`insufficient_knowledge`/`domain_not_satisfied` 作为正式 `observations` 持久发布；无实例不标 run、不伪造已证 |
| H4 launcher 重启删除权威库 / init 重置已有 cut | launcher 默认恢复已有库（不删）；`init` 拒绝已存在目标；显式 `--reset` 才删除重建 |
| H5 内容寻址批次不含完整对象/见证/输入身份 | batch 封存完整 raw_events/objects/witnesses/relations/observations + profile/规则绑定；object_id 含 input_refs 绑定输入身份；对象身份与 object_revision 分列 |
| M1 成交量被替换、规范整数无准入核验 | accept 加载并绑定具名 profile；volume_unit='1' 精确拒绝非 1；`parse_canonical_i64` 拒绝 `+`/空白/前导零 |
| M2 见证丢接纳序/源修订、浏览器 seq undefined | witness raw_bars 与 input_refs 含完整身份（seq/source_coord/revision/received_at/receipt_id/identity_key 等），浏览器按契约字段显示 |
| M3 查询失败先呈空实例、前端不检 HTTP/业务成功 | 浏览器校验 HTTP/业务状态与 cut 一致后才渲染；错误清楚显示、不覆盖为成功空集 |
| M4 launcher 默认 TestOnly、profile 非产品默认未生效 | 正式入口要求显式 `--input`+`--profile`（`--testonly` 为显式选择）；profile 随 session 绑定 |
| M5 launcher 硬编码容器 Python 路径、仅凭可执行位复用二进制 | 从候选 Python 的 sysconfig LIBDIR/LDLIBRARY 核真实 `.so/.dylib`；产物路径随 `CARGO_TARGET_DIR` 解析，stamp 与产物同目录；显式 `--bin` 只核本平台、不自动构建 |
| PY-H01 未编码 SQLite URI 开错库 | `Path.resolve().as_uri() + '?mode=ro'` + `PRAGMA query_only=ON`；`?`/`#`/`%` 路径复现通过 |
| PY-M01 / R1-M01 查询/解码错误越界断连、坏 scope 被吞 | 读取/解码/序列化/编码全在请求边界内；坏 scope/BLOB → 结构化 503；孤立 surrogate 无损转义不裸断连 |

## 1. 最小改动面（只改本片模块）

| 文件 | 改动 |
|---|---|
| `rust/src/theta_v0/parser/inclusion.rs` | `contains`/`strict_dir` 由私有改 `pub(crate)`（CC-006 复用同一判断，不另起第二查法；返回类型 `MergeDir`→公开 `Direction`，语义不变） |
| `rust/src/theta_v0/classifier/local_shape.rs`（新） | CC-006 `local_shape` 四分支分区 + 见证（两次方向 + 四个严格比较）+ 滑窗 + 单测 oracle |
| `rust/src/theta_v0/classifier/mod.rs` | 注册 `pub mod local_shape;` |
| `rust/src/bin/s_structure_session.rs`（新） | S 正式结构会话二进制（init/accept/advance/catalog/snapshot/meta），独立 SQLite/WAL 单写者 |
| `rust/Cargo.toml` + `rust/Cargo.lock` | `rusqlite 0.40 (bundled)`（optional，`s_session` feature 门控）+ bin 声明 `required-features=["s_session"]` |
| `s_session/`（新） | 正式 launcher `launch_s.sh`、只读查询外壳 `s_readonly_server.py`、浏览器 `browser/index.html`、已签目录 `catalog/signed-catalog.json`、TestOnly profile、输入档案 |

未撤销其他编辑；未切 main、未 push、未 merge、未 gh 写/关票、未改全局服务/生产账户凭据、未运行真实交易。

## 2. 逐验收条件证据（AC1..AC9）

### AC1 —— 可实际启动/停止的正式 S 会话入口和只读查询/浏览器链

- 正式 launcher：`./s_session/launch_s.sh`（可复制命令），链为
  `init → accept（S.AcceptInput）→ advance（Begin→同次 Rust parser→Commit）→ 只读查询外壳 → 浏览器`。
- 实跑（退出码 0，截断输出）：
  ```
  $ ./s_session/launch_s.sh --db /tmp/s_session_launch.sqlite --port 8799
  [launcher] 1/5 S init（独立 SQLite/WAL 单写者）
  {"catalog_items":116,"journal_mode":"wal","ok":true,"platform":"linux","session_id":"s-session-testonly-001","sqlite_version":"3.53.2","synchronous":2}
  [launcher] 2/5 S.AcceptInput ...（7 条 accepted）
  [launcher] 3/5 S.Advance ... {"generation":1,"merged_bars":7,"objects_published":5,"structure_cut":"cut-1"}
  [launcher] 4/5 启动只读查询外壳（独立只读进程，端口 8799）
  ```
- 停止：`./s_session/launch_s.sh stop --port 8799`（退出码 0，kill 只读进程 + 删 PID 文件）。
- 同源 API / 浏览器链（curl 证据）：
  - `curl http://127.0.0.1:8799/api/snapshot` → `{"generation":"1","objects":5,...}`。
  - `curl http://127.0.0.1:8799/api/catalog` → `{"items":116,"counts":{"implemented":1,"not_implemented":115}}`。
  - `curl http://127.0.0.1:8799/` → `<title>S 正式结构会话 · 只读观察（#1370 TB-01-A）</title>`。
- 非 schema/enum/测试/手填 StructureRecord：结构对象由 `ParseLayerIncr` 同次真实计算产生，不是手填。

### AC2 —— 原始事件携带完整字段 + 一对一 O=H=L=C + TestOnly profile

- 输入 schema（`RawInputFile`/`RawEvent`）：`schema_revision`、`session_id`、`source_namespace`、
  `source_epoch`、`instrument`、`profile`、每事件 `event_id`、`revision`、`seq`（源坐标）、
  `received_at`、`raw_text`、`price`、`timestamp`、`volume`。全部持久入 `raw_events` 表（含
  `source_coord`、`payload_hash`、接纳序 `seq`、`receipt_id`）。
- 一对一：`advance` 对每事件构造 `Bar{open=high=low=close=price}` 逐根 `ParseLayerIncr::append`；
  7 事件 → 7 merged bars（无折叠），见证 `raw_bars` 每 merged 组恰 1 根原始成员——不聚合、不重采样、
  不丢顺序/单位/修订。
- 具名 TestOnly profile：`s_session/profiles/testonly_tick_1_1_ohlc.json`
  （单位=tick、price_scale=exact_integer_decimal_string、clock=fixed_seq_clock、one_to_one_ohlc、
  `not_product_default: true`），profile 由 accept 输出回显，不成为产品默认。

### AC3 —— 独立 oracle 对拍真实 Rust 四分支

- 单测 `local_shape::tests::cc006_four_branch_oracle` 用**独立硬编码期望值**（票面映射，非从 Rust 代码
  反推）对拍 `classify_local_shape`：
  - 10000/10500/11000 → RISING；10000/11000/10500 → TOP；
  - 11000/10000/10500 → BOTTOM；11000/10500/10000 → FALLING。
- 实测 `cargo test --lib local_shape` → **4 passed**（含 `cc006_four_branch_oracle`、
  `cc006_equal_is_domain_not_satisfied`、`cc006_inclusion_is_domain_not_satisfied`、
  `cc006_sliding_window_count`）。
- 会话级见证：7 事件流 [10000,10500,11000,10500,10000,10500,11000] 的 snapshot 5 个对象 =
  RISING, TOP, FALLING, BOTTOM, RISING；每个对象携带两次方向（dir_ab/dir_bc）、四个严格比较
  （axis/pair/prev/cur/strict_up/held，全部 held=true）、三根 raw/merged 见证（slot 0/1/2）。
- 前两点缺右邻 = 知识不足、不满足域（含/等值）另报 `DomainNotSatisfied`，不造第五 Other 分型
  （`cc006_equal_is_domain_not_satisfied`、`cc006_inclusion_is_domain_not_satisfied` 单测锁定）。

### AC4 —— S 自有 SQLite/WAL 单写者 + 原子 Commit

- S 自己的数据库文件（`--db`），不复用 `trading_system/persistence/database.py` 共库。
- `init` 设置 `PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;`（macOS 另开 fullfsync，Linux 不声称）。
- 实际读数（`init`/`meta` 输出）：`sqlite_version=3.53.2`（≥ C09 要求的 3.51.3 WAL-reset 修复，来自
  `libsqlite3-sys 0.38.2` bundled 源码，锁实际依赖版本）、`journal_mode=wal`、`synchronous=2`(FULL)、
  `platform=linux`。
- 原子 Commit：`advance` 在**同一事务**先写不可变批次（`batches`，内容寻址 `batch_id=sha256(规范字节)`），
  再发布 `objects`/`witnesses`/`relations` 与索引可达根（`meta.generation`/`structure_cut`/`index_frontier`）
  ——崩溃遗留不可达批次可回收；读端只读已提交 cut（只读进程 `mode=ro`，只 SELECT）。

### AC5 —— 重放/冲突 + 精确整数 + payload_hash 绑定规范字节

- 同身份同内容重放：重跑 `accept --input cc006_four_branch.json` → 全部 7 条 `status=replay`
  （返回原 `receipt_id`）。
- 同身份异内容：`bigint_identity_conflict.json`（`bigint-a` 改价）→ `status=IdentityConflict`
  （原 `receipt_id` 与新 `payload_hash` 分列，不覆盖）。
- 身份/修订分离：业务身份键 `(source_namespace|source_epoch|instrument|event_id)`；`input_revision`
  与对象 `object_revision`（=1，不可变批次）分列；结构 `object_id` 由对象内容寻址（sha256 规范字节）。
- 精确整数：价格/时间戳/成交量在 wire 上为**规范十进制字符串**（TEXT 存储 + JSON 输出），Rust 侧仅
  在计算边界解析为 `i64`；>2^53 往返反例：`9007199254740993`(=2^53+1) 与 `9007199254740991`(=2^53-1)
  经 accept→advance→snapshot 原样往返（snapshot 见证 `raw_bars.price` 逐字一致），不依赖 JS Number /
  浮点 epsilon。
- `payload_hash = sha256(规范事件内容字节)`（固定键序紧凑 JSON），不散列任意 pretty-print。

### AC6 —— ReadCatalog 全目录 + 未实现不隐藏

- `catalog`（S.ReadCatalog）从已签目录（`s_session/catalog/signed-catalog.json`，由签署字节
  `SPEC-COVERAGE-INPUT.json` 派生）列出**全部 116 项**（62 CC / 44 ST / 10 LC），每项
  `implementation_status`/`proof_status`/`run_status`。
- 未实现不隐藏：`counts={"implemented":1,"not_implemented":115,"run":1,"not_run":115}`；CC-006 =
  implemented/not_proved/run，其余 115 项 = not_implemented/not_proved/not_run。
  **证明状态诚实**：本会话不执行 oracle（`cargo test --lib local_shape` 是独立构建期证据），故
  session 的 `proof_status=not_proved`、不伪造已证；oracle 执行证据见本目录 `AC-REPORT.json` AC3。
  CC-006 的 evidence 绑定 batch_id/structure_cut/profile 哈希/rule_revision/实例计数与 oracle 引用。
- 浏览器按 scope 从目录进入本域对象：目录表 + 对象/见证可展开；未实现、域不适用、知识不足、当前
  无实例分别显示（snapshot 的 `observations` 数组分别承载 insufficient_knowledge / domain_not_satisfied，
  无实例才是「当前无 CC-006 实例（实例集合为空，不是 PASS）」）。
- 未触发中枢/线段/BSP 不标已验收：CC-001..005、CC-007..062、ST/LC 全部 not_implemented。

### AC7 —— Snapshot 完整字段 + 浏览器读同一 cut

- `snapshot`（S.Snapshot）返回 `session_id`、`generation`、`structure_cut`、`catalog_revision`、
  `scope`、`index_frontier` 与完整 `objects`/`witnesses`/`relations`（实测 5 对象 / 15 见证 / 20 关系）。
- 浏览器读同一持久 cut：只读进程 `mode=ro` 查询同一 SQLite，按服务返回值呈现
  （分支/方向/比较/raw-merged 见证），不按价格或 marker 补判结构、不选择操作级别、不推算经济资格
  （页面正文明示，且渲染逻辑无任何价格 Number 换算）。

### AC8 —— E/B/X 不启动仍继续推进

- launcher 只启动 S（写命令，一次性）与只读查询外壳；不启动 E/B/X、不建其库、不加载经济政策。
- 继续接纳追加：`accept append_more.json`（3 条 accepted）→ `advance` → `generation=3`/`structure_cut=cut-3`
  （raw_events 13 → 11 窗口）。
- `scope={"structure":"CompleteCut","economic":"not_started"}`：结构 scope 可 CompleteCut，经济 scope
  明确未启动/缺失；不等待 JoinCut/经济 ack/全局序号；不称整个系统当前完整。

### AC9 —— 证据保留 + 检查运行 + 未验项照实

- 保留：launcher 命令、输入/TestOnly profile/schema（`s_session/inputs`、`profiles`）、规则/目录
  （`signed-catalog.json`，SHA 见 §6）、构建（Cargo.lock 钉 rusqlite 0.40.2/libsqlite3-sys 0.38.2 =
  SQLite 3.53.2）、profile 哈希、原始/API/浏览器证据（本报告 + curl 输出）。
- Rust 检查（本片改动）：
  - `cargo check --all-targets`（default）→ 通过（退出码 0）。
  - `cargo check --features s_session --bin s_structure_session` → 通过。
  - `cargo fmt -- --check` → 通过（已 fmt 我的文件）。
  - `cargo clippy --features s_session --bin s_structure_session` → 本片零新增 warning
    （lib 的 321 条为既有，非本片引入；`s_structure_session`/`local_shape` 零命中）。
  - `cargo test --lib local_shape` → 4 passed。
  - `cargo build --features s_session --bin s_structure_session` → 成功（需 `PYO3_PYTHON` 指向
    uv python，因 pyo3 cdylib 链接 libpython3.11）。
- 前端检查：浏览器为单文件 vanilla JS（无构建），`curl /` 返回 200 且 HTML 语法可解析；未运行
  TypeScript/Vite 构建（本片未改 `frontend/`，不在本片改动面）。

## 3. 已执行命令与退出码（摘要）

| 命令 | 退出码 |
|---|---|
| `cargo check --all-targets`（default） | 0 |
| `cargo check --features s_session --bin s_structure_session` | 0 |
| `cargo build --features s_session --bin s_structure_session`（PYO3_PYTHON=uv python） | 0 |
| `cargo fmt -- --check` | 0 |
| `cargo clippy --features s_session --bin s_structure_session` | 0（lib 既有 warning 不计） |
| `cargo test --lib local_shape` | 0（4 passed） |
| `./s_session/launch_s.sh --db /tmp/s_session_launch.sqlite --port 8799` | 0 |
| `curl http://127.0.0.1:8799/api/snapshot` | 0 |
| `curl http://127.0.0.1:8799/api/catalog` | 0 |
| `curl http://127.0.0.1:8799/` | 0 |
| `./s_session/launch_s.sh stop --port 8799` | 0 |

平台：Linux（aarch64）容器；SQLite 3.53.2（bundled，libsqlite3-sys 0.38.2）。

## 4. 构建/运行前置（可复制）

```bash
# 构建（pyo3 cdylib 需可链接 libpython3.11；指向 uv python）
cd rust
PYO3_PYTHON=/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 \
  cargo build --features s_session --bin s_structure_session

# 运行期（S 二进制动态链接 libpython3.11.so.1.0）
export LD_LIBRARY_PATH=/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/lib

# 正式 launcher（已内置上述 env 处理）
cd ..
./s_session/launch_s.sh --db /tmp/s_session_testonly.sqlite --port 8787
```

## 5. 四分支 oracle 与 session 实测对照

| 输入（三根价格） | 期望（独立定义） | 会话对象 branch | 单测 |
|---|---|---|---|
| 10000 / 10500 / 11000 | RISING | RISING | ✓ |
| 10000 / 11000 / 10500 | TOP | TOP | ✓ |
| 11000 / 10000 / 10500 | BOTTOM | BOTTOM | ✓ |
| 11000 / 10500 / 10000 | FALLING | FALLING | ✓ |

## 6. 哈希

- SPEC 清单 SHA256（票面给定）：`38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`。
- 已签目录源 `SPEC-COVERAGE-INPUT.json`：`76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c`。
- 派生 `signed-catalog.json`：`938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`。
- TestOnly profile：`2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc`。
- `cc006_four_branch.json`：`7bb94ef11a40dd32bf4f05c1daa6173ce448afafe560f69298c0e83b6533b60a`。

## 7. 局部成功门与证明/销项范围

- 本域不含 G-001 初始化包含 / G-002 同价极值选择；超域输入准确留缺项（DomainNotSatisfied /
  InsufficientKnowledge），不用默认 UP/同价留早补证。
- 正式持久化（WAL/FULL + 原子 Commit）、精确 wire（十进制字符串 + sha256 规范字节）与同次 Rust
  原始投影（ParseLayerIncr + merged 组映射）已实证。
- 只证明受测域与协议；分型窗口 ≠ 笔/线段/中枢；不以单 CC 通过宣布全 62 CC。
- A-ST-003-01 的非退化 high-only 反例不在该 tick 域，完整分型输入域证明仍由 TB-02 承担。
- RA-09/I-02 仅承接结构独立与缺经济域分栏子义务；无经济故障、分区、旧 cut/consumed_structure 或
  交易行为验收。TB-01 其余纵片、TB-02..TB-10、62 CC/44 ST/10 LC 完整构造、真实主线/独评义务未闭合。

## 8. NOT_RUN / 未验（照实）

- **真实 GUI 浏览器渲染**：headless 容器无 chromium/firefox。已交付 `browser/index.html`（vanilla JS，
  读取同源 API）并用 `curl` 证明 HTML/API 链可达；实际浏览器渲染**未验**。恢复命令：
  `./s_session/launch_s.sh --db <db> --port 8787` 后在任意浏览器打开 `http://127.0.0.1:8787/`。
- **macOS 平台（根接）**：macOS fullfsync 前件**未测**（代码仅在 `#[cfg(target_os="macos")]` 开启，
  不声称已测）；launcher 在真实 macOS 的默认 Python 发现（`.dylib`）与构建产物平台核**未在本容器验**，
  launcher 已改为按 sysconfig LIBDIR/LDLIBRARY 核真实 `.so/.dylib`，由根在真实 macOS 验收。
- **SQLite 网络文件系统约束**：本片在本地 `/tmp` 文件系统验证，不承诺 NFS/WAL 语义。
- **跨进程杀/重启、双跑、故障注入**：属 TB-05/07（本片不承担）；本片只做了 accept→advance 追加推进
  （generation 递增）与只读进程独立读取。
- **全量 Rust 测试套件**：只跑 `cargo test --lib local_shape`（本片改动面）；全量 `cargo test` 未跑
  （本片未改其它模块，避免长时全量重放，见「有界验证优先」）。
- 代码/接口存在不等验收通过；未跑项如上，不捏造通过。

## 9. 待根验项（不代独评）

1. 真实 GUI 浏览器渲染与交互（恢复命令见 §8）。
2. 独立评审会话（本报告非独评）。
3. main 合入、真实外效、未决 G/FU（本片未授权）。
4. TB-01 父票汇总验收与其余纵片（TB-02..TB-10）闭合。
