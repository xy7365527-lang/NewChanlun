> 阅读版：仅转换链接；原始独评字节与 SHA 见验收索引。

# #1370 TB-01-A Python R5 与 Rust wire 独立复核

结论：**0H / 2M，仍待修复**。R4 的三个旧 M 已修复；本轮发现完整已签目录正常读取回归，以及 Python 对未投影坏 meta 行的漏检。不能封为最终通过，不能关闭 #1370、TB-01 或父图 #1323。

固定提交 `e79c74a17a800e660b515e1f88fc9a5706826b42`，源码由 `git show` 导出。Python SHA256 `4c05effb0229bbb7167c69c155561bdd7730e30729a93def49785efb7f0519f6`；Rust `c376159b6fb0a9542fc00cccd9061cfccd94080fd0b44ce53d0cb124ad7c9841`。原生 binary 从根 native-r4 构建绑定复制到本工位目录。

实际 binary SHA256：`85f247650531d2ec807f56f36e1c379d0ed34b4906d455108129737bc5b0769f`，运行前后均核对；本轮未重编。

[MEDIUM] PY-R5-M01：branches 被收窄为 array，完整已签目录正常读取失败

File: `s_session/s_readonly_server.py:119`；`rust/src/bin/s_structure_session.rs:1478`。

Issue: 固定提交内的 `s_session/catalog/signed-catalog.json` 有 **116 项：82 个 array branches，34 个 object branches**。以该完整源文件调用本机 Rust `init` 写入自建正式库，exit 0，所有 116 项 branches 与源逐值相同；随后 Rust `catalog` exit 1，HTTP `/api/catalog` 与 `/api/state` 均 503，`/api/snapshot` 200。object 型 generator branches 是已签合法输入，不能按损坏拒绝。

Fix: branches 专门接受 array 或 object，继续拒绝 scalar/null/坏 JSON；不改已签目录、不重裁语义。补完整 116 项目录的同值读取测试。其他列按各自已声明形状检查。

证据：`python-r5/full_catalog_probe.py/json`；源目录 SHA256 `938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`。根在真实 launcher 验收先发现，本工位随后用冻结文件和自建库独立复现。

[MEDIUM] PY-R5-M02：Python 漏检未投影坏 meta 行，与 Rust typed error 不一致

File: `s_session/s_readonly_server.py:40`。

Issue: 正常 schema 可存的额外 meta BLOB 值、NULL key、BLOB key 共 3 组，Rust `meta/catalog/snapshot` 全部 exit 1，Python `/api/catalog/snapshot/state` 全部仍 HTTP 200。NULL key 的 `/api/meta` 也返回 200，被 JSON 序列化成 `"null"` 键。Python 未核 meta 行类型，坏行不在投影字段时被静默忽略。

Fix: `meta_dict` 收集每行时检查 key/value 均为 str，失败抛 ValueError 并进入统一 503。真正 scope 缺项仍保留 `{}`。

证据：`python-r5/contract_probe.json` 的 `extra_meta_blob`、`null_meta_key`、`blob_meta_key`。已知 generation BLOB 对照已两端正确拒绝。此为 Python 既有相邻漏检，本轮 Rust 收集器已严格拒绝。

## 已核通过与验证口径

R4 三个旧 M 的反例均已修复；重放 97 项通过。为了让原通用 HTTP 正向夹具符合新增类型合同，revision/window/slot/index 改为整数，input_refs/raw_json 补齐既定成员字段，cut 标记仍覆盖对应表。整数对拍的合成 catalog 暂用合法 array branches，以隔离整数路径；这不能证明 object branches 或全目录可用。对象型 branches 的合法性由新增检查和完整已签目录独立覆盖，结果如 M01。所有夹具调整记录于 `fixture-adaptations.json`，R4 证据保留。

新增检查覆盖 SQL 整数类型、i64 上下界与越界、嵌套缺字段/错误结构、JSON 文本/容器/非有限值、Rust meta typed 错误、真正缺失 scope。合法 i64 值经真实 HTTP → Node 与 Rust CLI 精确一致；非空 observation 窗口精确，合法 null 保留；真正 scope 缺项两端均 `{}`。SQL NOT NULL 阻止插入的反例按数据库约束通过计，没有改 schema 绕过约束后冒称正常 schema 可达。

| 命令（python-r5 目录） | exit | 通过/总数 |
|---|---:|---:|
| `python3 probe.py` | 0 | 44/44 |
| `python3 semantic_probe.py` | 0 | 7/7 |
| `python3 type_probe.py` | 0 | 10/10 |
| `python3 wire_probe.py` | 0 | 27/27 |
| `python3 json_corruption_probe.py` | 0 | 7/7 |
| `python3 supplemental_probe.py` | 0 | 2/2 |
| `python3 contract_probe.py` | 1 | 111/115 |
| `python3 full_catalog_probe.py` | 1 | 0/1 |

合计 **208/213**；5 个失败断言归于上述 2 个 M。首次 contract 探针曾误把 object branches 列为负向，该轮原始输出保留为 `contract_probe-array-assumption.json`，不计入当前总数；已按实际已签合同订正并重跑。

内部 canonical/identity/read_raw/build_object、SCHEMA 与 batch 内容/哈希构造 8 项源码相对 754 逐字相同；读取探针不改既存 canonical_bytes 哈希。新 batch 先独立持久再 publish 协议归耐久性工位；未用本报告替代其验收。Python 编译检查通过；ruff/mypy/pylint/black 的 CLI 和模块均不可用，未报静态工具通过。

本轮只写独占 R5 报告及临时证据，保留 R4；未改仓/GitHub/根库，未占用 18770–18773，未重编 binary。
