**PR #1449 R13 Python 独立限定审查：PASS_BOUNDED_PYTHON_REVIEW**

最终冻结的两份 Python 文件未发现阻断性问题。先前测试连接关闭、失败记录问题由主控修正，已复核。生产字段域、SQL绑定、结构读入口及错误处理语义符合本次未来索引拒绝目标；旧版负控会失败，新版五臂回归通过。本结论不替代全产品增量独评、真实HTTP重新验收或合入批准。

**绑定输入**

- 工作树：`/Users/silencehan/Projects/NewChanlun-1371-codex-r13`；基准：`750f933bddffc8251a880555b9beae2cdce75690`。
- `s_session/s_readonly_server.py`：57,819字节；SHA256 `26bc65bc7550fe3929dcabe451f6567d4107801b5d007e7eaf76b60598a67d6d`。
- `s_session/tests/r13_integrity.py`：6,647字节；SHA256 `f6b5d87a2caa06474c2b23355fb9fc555e2cc1c40523c248fcd9f745d271a6e5`。
- 两文件均与 `r13-build/SOURCE.json` 当前冻结记录相等；本工位最终实算并完成AST语法核验。

原生 `python-reviewer` 子派发在本工位仅尝试一次，被运行时 `agent thread limit reached` 拒绝；没有再试或使用Prime。本工位按Python specialist同等标准完成只读安全、类型、资源和测试审查。图查询返回当前项目未索引，故使用固定差异、精确源码及AST调用关系，不用其他checkout图冒充当前源码。

**生产实现审查**

`_verify_published_index_generations`（reader:322–337）检查六个字段：objects的first_known/published/withdrawn，以及witnesses/relations/observations的published。必填列只允许SQLite存储类型integer且在1..G；nullable withdrawn额外允许NULL，非NULL仍必须同域。`typeof(NULL) != integer` 能拒必填NULL；G=0时任何四表发布行均不合法。它只检查发布索引，不误拒已接纳但尚未发布的raw_events或未Commit的batches。

SQL的表名和谓词全部来自函数内固定tuple；动态G通过`?1`绑定，没有把请求参数或数据内容插入SQL标识符。`SELECT EXISTS`总有一行标量结果；表缺失或SQL异常沿既有错误处理返回不可用，不会默认为正常。

新门在 `_validate_required_meta` 得到合法当前G之后、逐cut过滤之前执行（reader:340–345），因此能看见原先可能被历史cut过滤隐藏的未来行。nullable生命周期的先后关系仍由既有完整载荷/对象生命周期校验承担，新门没有放宽这些规则。

| 读入口 | 路径核验 | 本次证据 |
|---|---|---|
| `/api/state` | read_state → read_catalog/read_snapshot → reachable_root → 新门 | 正式Python read_state回归 + 静态路由 |
| `/api/catalog` | read_catalog → reachable_root → 新门 | 由read_state间接执行 + 静态路由 |
| `/api/snapshot` | read_snapshot → reachable_root → 新门 | 由read_state间接执行 + 静态路由 |
| `/api/delta` | read_delta → reachable_root → 新门 | 静态调用链；本工位未重跑HTTP |
| `/api/meta` | 既有meta_dict原始诊断 | 不查四张发布索引，不纳入此次结构读取保证 |

HTTP `_read_only`（reader:1126起）对新ValueError转StorageUnavailable/503，先rollback且finally显式close；正常路径commit。这里核的是静态映射，不声称本工位重新运行了HTTP请求。新增函数不创建资源或副作用。

**测试真实性、资源与失败证据**

测试由正式writer的init/accept/advance产生4代seed，含真实对象、witness、relation和observation，再通过SQLite backup复制。变异SQL从固定表清单和正式schema取列，仅克隆一条已有行，改变ID及代际字段（test:85–106）；`rowcount == 1`防止空表导致伪测试通过。没有手编整库或用假成功响应替代writer。

已只读检查RED的实际数据库：seed为4代，objects=2、witnesses=6、relations=10、observations=1；future-0增加一条对象，其他表行数不变。克隆行与原行仅object_id、first_known_generation、published_generation、withdrawn_generation不同；first_known/published=5，当前G仍为4。

`closing`覆盖dump/read/backup连接（test:53–61、:90）；backup后变异INSERT另用`with conn`明确提交或回滚（:92）。返回的read_state是已经构造完成的数据，不携带关闭连接后的游标。CLI用参数数组、无shell、60秒超时；值和标识符属于具名本地测试输入，没有跨权限输入边界。

每次Python读记录db、cut、异常类型和消息，失败重新抛出（:57–67）。seed及后续测试从:70起处于try；失败对象和已有calls/reads由finally写入RESULT（:126–132）。输出目录新建且`exist_ok=False`，不会悄悄覆盖上次证据。参数解析、输出目录创建和reader模块导入在该记录范围之前；这些前件失败不保证生成RESULT，报告不扩大承诺。

五个future分支包含active和inactive对象克隆。inactive克隆同时具有未来first_known/published及更晚withdrawn，因此是“完全隐藏的未来行”场景，不冒充独立nullable withdrawn字段锁。已有对象仅wg越界的旧版可拒负控另属原证据，不与这五臂混计。

**复核的验证结果**

本工位独立从reader AST提取固定SQL谓词，在内存SQLite无亲和列上核6字段×10存储值（NULL、-1、0、1、G、G+1、TEXT、REAL、BLOB）共60例；另4表的G=0空表/有发布行8例。**68通过、0失败**。这只证明谓词存储类型与边界真值，不替代正式库或产品入口验收。

| 主控执行工件 | 本工位只读核验 |
|---|---|
| `work/R13-INTEGRITY-RED/RESULT.json` | 旧R12 reader；9次CLI前置成功、6次健康读成功；future-0 current仍返回成功，测试明确失败“objects: Python 未拒绝未来索引，cut=None”。负控只运行到第一失败，不声称五臂旧版全遍历。 |
| `work/R13-INTEGRITY-GREEN/RESULT.json` | 新reader + R13 binary；5分支全部passed，failure=null；39次CLI＝9成功前置+30预期拒绝；21次Python读＝6健康+15预期ValueError。 |
| `r13-build/PYTHON-GREEN.log` | PASS日志与RESULT一致；主控报告实际进程exit=0。 |

GREEN的15次Python失败消息全部明确来自新“1..=4已发布代”检查，而非其他无关ValueError。30次CLI失败全部含StorageUnavailable，分成15个snapshot和15个accept/advance/recover。测试对每分支三写口逐次比较完整SQL iterdump，共15次断言通过；它证明逻辑持久事实不变，不等同SQLite/WAL物理字节零变化。

RED结果SHA256：`7406c99b51ab25db7b8dc496b9e84db680db77483bf613646b6dfdf36440bfd1`。

GREEN结果SHA256：`6cb655f1aca57474adee23f058c84d63e549592955627b5e7ed09d608ec0a8e0`。

本工位未重跑产品CLI/HTTP、全套测试或扫描器，未修改源码、Git或GitHub。只写本审查MD/JSON；R13其余Rust、浏览器和新反例由主控及原独评独立接续。
