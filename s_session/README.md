# 正式结构会话 S（#1370 TB-01-A）

正式会话 launcher → 独立 Rust S（唯一结构核，独立 SQLite/WAL 单写者）→ 只读查询/浏览器链。
受测域：具名 TestOnly 原始逐笔事件一对一精确退化 OHLC（O=H=L=C）的 `CC-006/local_shape` 四分支。

## 快速开始

```bash
# 1. 构建（pyo3 cdylib 需可链接 libpython3.11，launcher 已处理 PYO3_PYTHON）
./s_session/launch_s.sh --build --db /tmp/s_session_testonly.sqlite --port 8787

# 2. 浏览器 / API
open http://127.0.0.1:8787/
curl http://127.0.0.1:8787/api/catalog
curl http://127.0.0.1:8787/api/snapshot

# 3. 停止
./s_session/launch_s.sh stop --port 8787
```

## 目录

- `launch_s.sh`：正式 launcher（init → accept → advance → 只读查询外壳）。
- `s_readonly_server.py`：只读查询外壳（独立只读进程，S 自有 SQLite `mode=ro`，只 SELECT）。
- `browser/index.html`：只读浏览器（vanilla JS，读取同源 API，精确整数按字符串呈现）。
- `catalog/signed-catalog.json`：已签目录（由签署字节 `SPEC-COVERAGE-INPUT.json` 派生，116 项）。
- `profiles/testonly_tick_1_1_ohlc.json`：具名 TestOnly profile（固定单位/时钟，非产品默认）。
- `inputs/`：原始逐笔档案（四分支主流、追加流、>2^53 往返、冲突）。

## 接口

`rust/target/debug/s_structure_session`（`cargo build --features s_session --bin s_structure_session`）：

- `init --db <路径> --session <id> --catalog <catalog.json>`
- `accept --db <路径> --input <输入.json> --profile <profile.json>`（S.AcceptInput；同身份同内容→replay，异内容→IdentityConflict）
- `advance --db <路径>`（S.Advance；Begin→同次 Rust `ParseLayerIncr`→CC-006→Commit）
- `catalog --db <路径>`（S.ReadCatalog）
- `snapshot --db <路径>`（S.Snapshot）
- `meta --db <路径>`（WAL/FULL/SQLite 版本/平台前件核）

不启动 E/B/X、不建其库、不加载经济政策；结构 scope `CompleteCut`，经济 scope `not_started`。
