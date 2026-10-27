# #1419 参考检索

工作草稿。**本轮在线 XERJ 检索失败；下列命中来自已有响应原件，不能冒称本轮在线命中。** 没有重装服务、启动服务或重新索引。

先执行 `curl -fsS --max-time 8 'http://127.0.0.1:19200/_cat/indices?format=json'`，exit 7，输出 `Failed to connect to 127.0.0.1 port 19200`。随后三个 `_search` 的完整 argv、请求体、exit、stdout/stderr 已存 `XERJ-ATTEMPTS.json`：

| 索引 | query_string | 结果 |
|---|---|---|
| nc1419gate-docs | source_structure rule generation | exit 7，无响应 |
| nc1488generation-docs | Native CodecVec BoundGenerationInput | exit 7，无响应 |
| nc1488d1ref-docs | MemoryReservation try_grow | exit 7，无响应 |

运行时工具列表没有 `search_graph`、`trace_path`、`get_code_snippet`、`query_graph` 或 `get_architecture`。因此在上述实际失败后，代码定位回退到已知交接路径、Git 对象与限定路径的 `rg`；未将本地文件检索算作知识图谱成功。

## 已有项目与参考索引证据

`REFERENCE-HITS.json` 保存旧响应的精确路径、SHA256 与相关命中。旧响应没有保存原请求体的地方明确不还原请求体；本轮只按其 `_index`、`path/name` 筛选和阅读。

| 原件 | 旧结果中的相关命中 | 当前用途及源码核对 |
|---|---|---|
| `/Volumes/AgentStorage/issue1419-interface-fix-20261018/nc1419gate-docs-reference.json` | total 41，返回 3：`source_audit.rs::audit`、`source_structure.rs::payload`、`source_structure.rs::is_kind` | donor 的 is_kind 精确要求完整结构 rule 且属于 C/E/F KINDS；payload 用同 store 查 JsonText 内根。核对 donor 同名函数，未将种类集合当生产能力。 |
| `/Volumes/AgentStorage/issue1419-interface-fix-20261018/nc1488d1ref-docs-reference.json` | total 108，返回 3：`pool.rs::insufficient_capacity_err`、`mod.rs::MemoryLimit`、`mod.rs::new` | 只作预算拒绝/真实 reservation 的参考，不承担缠论语义证明，也不证明本实现已经满足资源帽。 |
| `/tmp/codecvec-xerj-search.json` | 原响应带查询 `reserve_native`，total 3，返回 2，`nc1488rawscratch-docs` 的 scratch.rs/work.rs | 核对 current `rust/src/s_content/capture_allocation/work.rs` 的 `CodecVec::try_reserve_native`、`try_into_native_text`，以及 `native_owned.rs` 的 NativeValue 持费生命周期。不能把 donor 普通 Vec 换个类型名便称已计费。 |
| `/tmp/1488-native-map-xerj-indices.json` | 旧列表包含 nc1419gate-docs、nc1488generation-docs、nc1488d1ref-docs | 只证明曾有这些索引；不证明当前服务可用。 |

## 实际源码依据

以下版本固定为 `5cb7253d46d973669e7d10cd3331e12c9ef313b1`（current）或 `634f62cddfaddf656a35ca9b9db20cb716b62bff`（donor）；精确 blob 和 SHA256 见两份来源 manifest。

- current `rust/src/theta_v0/parser/stroke.rs:14` 的 RULE 是 `new-bi-dual-coordinate/1`，`:23` 是 `Endpoint.raw_position`。donor 相同模块的 RULE 是 `/2`，端点用 `raw_first/raw_last`，并有 `PairFact.evidence`。
- current `rust/src/theta_v0/parser/segment.rs:159` 的 CONSTRUCTION_RULE 是 `segment-second/1`；donor `:160` 是 `/2`。current 不存在 `parser/second_fold.rs`，且没有 `parse_layer_with_group_evidence_store`。
- current 不存在 `classifier/center_scan.rs`、`s_session_v2/tb02e.rs`、`s_session_v2/tb02f.rs`。donor 的 `content_structure::build_ef` 经段输入合同消费该共享中心扫描核；这不是加三个 module 声明即可补足的接口。
- current `s_session_v3/generation.rs` 保留 `FixedRadix`、`CodecVec`、`NativeValue<String>` 与有准入的 push/extend。donor C…F 输出拥有普通 Vec、BTreeMap 和 CValue 树；其新增分配与持有期未在 current Native 账中建立对应。
- current `s_session_v3/prepared.rs` 在 `publication_read_lease` 内生成、计划对象与结束目录轴，然后才借实际事务材料；原件中的两个 lease 阶段必须保持。
- current `source_audit.rs:1771` 仍核 `s-new-bi/1`。仅应用 base→donor 的白名单删除，不会补入已经在 base 上存在、但 current 尚缺的 `/2` 和结构审计前置。

以上由本作者静态核对，**不是独立评审**。无 Rust 编译/测试或产品执行证据。旧 run39080728253 的四修复和三锁直接复用原已验范围，没有重跑或重新下载。
