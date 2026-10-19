# #1394 S16 全正文 fixture 独立预运行审查

**结论：PASS（仅预运行门）。** 允许 Root 使用冻结的普通 `python3 run.py` 执行一次正式 S16 Q21；此结论不声称新 HTTP、全正文或独立 oracle 已实跑成功。运行后必须同时核 `RESULT.json`、`CLEANUP.json`、`ORACLE-REPORT.json`、原始 HTTP gzip 和三份源码 SHA，任何失败均不得沿用旧证据补成 PASS。

## 冻结对象与范围

- Root HEAD `8fb179091a81e274214056603171b759decfdd08`；Rust tree `7196a77fef56cf87cbe91dc4b135326a142e2395`。
- `run.py` SHA-256 `c69fd8ba741e77fc56abb32576b3487ce70372275d5a9b762d83247d2bd1ce63`。
- `collect.js` SHA-256 `9ed938af7d220dbffe242abd5f213a20362cb12d53ed26073ed1713c8e76b8fb`。
- `independent_oracle.py` SHA-256 `bb3a6655adab81f97c66491fde964b1e4b117b6f532842e8f82c95efda3e9c4f`。
- Q21 原件 SHA-256 `50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1`。脚本固定当前 release binary `c098bf76…`、Rust tree、候选补丁 `677164…`，并为四个具名输入持久化原件与 owned 副本 SHA。无 Client 安装、物理 allocation、全 9011、#1460 或 tracker 操作。

## 规范与目标核对

- `run.py` 在启动 Q 前核源 DB/WAL/SHM 无持有者、两次稳定 SHA，复制到独立 owned COW 三件套，只 SQLite-open 副本；全 schema、24 表、48,403 行、96,397 BLOB 单元与冻结 fullDB 逐类型规范对拍，并重算 47,671 个 native F。`python -O` 被入口硬拒绝。
- 16 页 Snapshot 的 offset、token 交接、页级 metadata、行拼接、472 条及六族计数逐项检查，再重算 projection root。`collect.js` 用现有 Reader/GraphView 完整取值和逐响应 proof；存全部 Content 原始 request/response wire gzip、请求指标、472 个正文摘要，断流保存部分 wire 且失败，流写入和 flush 失败退出非零。
- 独立 Python oracle 不导入 JS/生产 codec，从冻结 native 图还原精确 JSON number token、JSON_TEXT、SEQ 和字符串 escaping；把 472 条记录、raw revisions、source refs/coords、OHLC、witness bars/shape、116 条 SQL catalog 和 axes 与传输图逐字段对照。Q 临时节点单独验 F、原生重合字节及从独立投影根可达性；全部 HTTP range 的 path/unit/offset/next_token、items、envelope 与 raw SHA 再独立核。
- 收尾对同一 Q/Node Popen 等待终态，复核端口、owned 源语义和原件三件套；任何 Node/Q/Controller/端口或清理收据错误最终优先 `FAIL_CLEANUP`。Q 超时不重启，资源原值不扩大。

## 本次证据与边界

`git diff -- '*.py'` 无受跟踪 Python 改动，审查对象是上述未跟踪私有 fixture。Python 两文件 AST、Node `--check` 通过；ruff、mypy、pylint、black、bandit 本机不可用。独立 oracle 纯函数反例核了 bool/number、number/string、JSON_TEXT/string 分离、重复键与 NaN 拒绝及转义。以旧正式 S16 16 页响应和冻结 fullDB 做**只读** native/source 与投影映射回放，472 条通过，耗时约 2 秒。此回放不是新 binary 的 HTTP 结果。

**剩余运行门**：在运行前重核上述 SHA、原源无持有者和可用磁盘；仅 Root 发起一次 `python3 /Users/silencehan/Projects/NewChanlun-1394-content-v3/.scratch/issue1394-q3-full-body-20261001/run.py`。若它通过，再以同一次 raw 产物执行 oracle `--actual` 并核报告。未取得这两项真实 PASS 前，全正文／来源闭包状态仍是 `UNVERIFIED`；Client 物理 allocation 必须保持 `UNMEASURED`，不能填 0。
