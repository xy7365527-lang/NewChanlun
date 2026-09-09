> 阅读版：仅转换链接；原始独评字节与 SHA 见验收索引。

# #1370 TB-01-A Python/Rust wire R6 独立增量复核

结论：**PASS（本轮范围），0H / 0M / 0L，213/213 通过**。R5 的目录正常读取回归和坏 metadata 漏检两个 M 均由本轮实际 HTTP/native 反例复核关闭。R5 208/213 的历史结果保留，不覆盖为成功；本报告不关闭 #1370、TB-01 或 #1323。

固定提交 `76a641606d7097292fff2dc21f58c6156ae38916`。Python SHA-256 `63203e75dbf58a3d59ed5b0fe7a15d5eb1101be9e19945b4d47ad00f0e3e95a9`；Rust `1820c9a464df80b22fe50233905fba5fc959d50936000fd800f6e198493508be`；私有 binary `b9e35a9200eb391e8ab5902c2034d601bdacd507de7c85623fa2f1919b584766`。全部 408 个构建源清单项逐一与固定 git blob 哈希相符，执行前后私有 binary 哈希相同。[来源](../payload/inputs/python-r6-evidence/SOURCE.json)与[构建绑定](../payload/inputs/python-r6-evidence/native-build-binding.json)已封存。

## R5 两个 M 的实际关闭证据

**PY-R5-M01 已关闭。** 以固定 signed-catalog.json 调用本机 native `init`，真实写入全部 116 项（82 array / 34 object）。Rust `catalog` exit 0，HTTP `/api/catalog`、`/api/snapshot`、`/api/state` 均 200。对 Rust catalog、HTTP catalog 与 state.catalog 的每个 ID，逐值核 id/kind/title/domain/branches 的完整嵌套值、catalog_revision、初始实现/证明/运行状态、空 evidence 及计数，全部相等。此处是初始库，计数为 implemented=0、not_implemented=116、run=0、not_run=116。另有对象 branches 正向控制与标量/null/坏 JSON 拒绝检查。[完整目录结果](../payload/inputs/python-r6-evidence/full_catalog_probe.json)；[实际 init 库的审计副本](../UNBUNDLED-REFERENCES.md#ref-9)。

**PY-R5-M02 已关闭。** 正常 Rust schema 下插入的额外 meta BLOB 值、NULL key、BLOB key 三组，四个 HTTP API 现均返回结构化 503，detail 明确要求键和值为文本；Rust `meta/catalog/snapshot` 均 exit 1。读取前后 canonical 字节哈希保持不变，错误请求后 HTTP 服务仍能继续响应。真正缺少 scope 的对照仍两端成功返回 `{}`。[115 项合同结果](../payload/inputs/python-r6-evidence/contract_probe.json)。

## 已执行检查

所有命令由固定 Python 3.11.15 执行，使用本工位私有源码/二进制、临时 SQLite 和 localhost 随机端口。HTTP 请求经过实际 TCP 与固定 Handler，Node 为 v22.23.1；Python 查询 SQLite 为 3.53.1，native init 实读 writer SQLite 为 3.53.2，二者没有混报。

| 脚本 | exit | 通过/总数 |
|---|---:|---:|
| `probe.py` | 0 | 44/44 |
| `semantic_probe.py` | 0 | 7/7 |
| `type_probe.py` | 0 | 10/10 |
| `wire_probe.py` | 0 | 27/27 |
| `json_corruption_probe.py` | 0 | 7/7 |
| `supplemental_probe.py` | 0 | 2/2 |
| `contract_probe.py` | 0 | 115/115 |
| `full_catalog_probe.py` | 0 | 1/1 |

旧 97 项全部重跑通过；新增合同 115 项与全目录 1 项也通过。与 R5 的 213 个检查名和数量逐项相同，原先 5 个失败断言现均通过。原有 11 个 WAL SELECT 交错位置同一 cut 检查通过；i64 边界、大于 2^53 的 HTTP→Node→Rust 精确字符串对拍、非空 observation 窗口、坏整数/JSON/嵌套结构与只读路径检查均保留。143 处记录的 canonical 哈希检查全部未变。

完整目录原检查在本轮增强：除核存储值，还核三个真实读取结果的全部 116 项值；未删或放宽负向断言。R5 正向夹具已订正的整数/成员合同原样复用，具体变化见[夹具与运行适配记录](../payload/inputs/python-r6-evidence/fixture-adaptations.json)。R5 27 个文件的执行前后 SHA 全部一致。202 个临时 HTTP 实例使用 61374–62229 端口，未占用 18770–18773；上下文管理器均关闭并 join，所有 runner exit 0，临时目录已清空。

Python 语法编译检查通过；ruff/mypy/pylint/black 模块不可用，未报这些工具通过。本轮没有重编或复跑根已通过的 9 项 bin/fmt/clippy/build，也没有执行实际 GUI。多数字段破坏测试使用合成持久夹具，证明读取/wire/隔离合同；完整目录使用实际 native init。这些结果不替代结构计算、耐久发布顺序、正式 Prime 独审或整票验收。完整目录的逐值保真限现有查询投影及 branches 嵌套内容，不额外声称导出 acceptance_mode/axis_ids/criterion。

只写新证据及本报告，未改 Q/N、仓库、根库、GitHub 或服务。逐命令 argv、退出码、原始 stdout/stderr、结果哈希在 [R6 JSON 报告](../payload/inputs/TB01-A-PYTHON-R6-REVIEW.json)及[命令收据](../UNBUNDLED-REFERENCES.md#ref-10)中。
