# #1419 raw/driver v4 定向修复交付

四项剩余缺陷 F1/F2/F5/F8 已实改，22 项纯 Python 内存/fake 定向测试通过，保存 88 条逐例观察。本结论是实施工蜂的候选验证，**不是整票通过，也不是独立复审通过**。原正式独审 c48b9620 于 2026-10-18T05:22:43.914Z 真实 code 0 退出，其裁决仍是 CHANGES_REQUIRED；完整原件保存在 `V3-INDEPENDENT-REVIEW.reference.json`。

工作草稿绑定开放票 #1419；父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者 `/root/manage_1419_sandcastle`。本工蜂为 Sandcastle 外部 Codex，真实 session_id 以运行时事件登记为准，不声明原生子代理注册。接手 HEAD 为 `4161ebfe6d1a11b93cccf1de1a8f134c6c7d2c6a`，接手工作树 clean；本轮没有提交，候选由 `MANIFEST.json` 的文件哈希及 `fixes-v4/changes.patch` 识别。旧 v3 清单及旧报告原字节保留，新的协议候选是 `fixes-v4/EXACT-RUN-PROPOSAL.json`（schema `/4`）。

## 修复及实测

| 项 | 变更及判据 | 具名证据 |
|---|---|---|
| F1 | 受测 kind 只读公共 `payload.data`；要求 payload、data 均为对象。公共入口出现内部 `payload_json` 一律拒绝（即使与 payload 相等），没有缺字段回退或内部记录入口。 | 15 字段公共形状正控得到 10 段、2 中枢、1 相邻对；仅改公共 geometric_end.price/group_anchor 均在合法重哈希后拒绝；公共错/内部对和公共对/内部错两种冲突均拒绝。 |
| F2 | 同一 WorkBudget 从初始化、输入迭代、重复节点、hex 扫描/解码、hash、头校验一路覆盖投影编码、分页收集/队列分配、decode/plain/records。先查截止并预留，再工作；拒绝不超记 used，collector 后续调用不重置、不加帽。 | 20,000 重复节点、帽 1：input_calls=0、hash_calls=0、used=1、deadline_calls=2；已到期：前三项均 0、deadline_calls=1。Collector 帽 10：snapshot_calls=1、content_calls=0、hash_calls=0、used=10、deadline_calls=9；已到期时 snapshot 也为 0。 |
| F5 | 每个入池节点（包括重复和未引用）都核合法 tag、对应长度、U64 域和分支计数。允许的头形状逐项来自固定 JS 参考；未实例化 template 仍不能成为已解码公共值。 | 合法 hash 的 ff、截断头和已知 tag 非法形状共 13 个负控拒绝；14 类合法头正控通过。原 catalog/catalog_evidence/record 元数据九负控继续通过。 |
| F8 | bind_named_submissions 深复制完整正式 accepted_cut；watch 每代核 input_frontier 以及 generation、structure_cut、index_frontier、seq_range、catalog 状态/证据，核会话、catalog_revision、scope，并保留 profile/rule/history 身份及跨代一致性检查。 | generation 140/frontier 999 对 watch frontier 139 拒绝；切面身份负控拒绝；1/65/65 跨 epoch 正控得到 139/204/269，即使受影响 seq_range.to 为 7 也不替代 input_frontier。冻结原请求事件数绑定及深复制通过。 |

逐项机器矩阵见 `FIX-MATRIX.json`；原输出为 `tests.acceptance.stdout`，原观察和当时实际执行的 Python 源哈希为 `CHECK-OUTPUT.json`。公共正控是有限静态模型，外层其余字段只补正式形状，不证明真实 lifecycle 值或正式生产对象身份。

工作计量是逻辑单位：每次访问/迭代及主要分配预留，字节处理按最多 4 KiB 块计费，字符串按每字符最多 4 字节预留；排序按键数量和位长预留。它不是 CPU 指令、OS 内存、磁盘或同步 IO 硬帽。Graph 汇集跨页证明时允许相同节点重复，重复也完整收费；这不是对 JS 单页 Graph.create “禁止重复 root”的等价声明。所有入池头形状与 JS 的对应检查一致；未引用节点的子图完整语义未另作证明。

## 预算、空输入与分块

空图也收费且查截止；同一预算下分块空图不能零费绕过。空页集合缺闭包节点被拒，一次性 pages 迭代器被拒，避免第二次遍历跳过锚/内容核验。大 hex 在 hash 前拒绝。16 层重复子图建图用 513、decode 后累计 729，plain 在原 1000 帽处拒绝，截止沿用同一回调；records 路径允许更早在同帽内拒绝。

原 fake 收集正控每个小页都携带全图。收费补齐后，该原始输入在 46 次调用、used=1,000,000 时正确拒绝，未上调原帽。受影响测试现同时保留这一负控和节点分块的正控：同一内容/分页语义、完整节点集合、完整 qcut/内容闭包，182 次调用，used=593,060，帽仍 1,000,000。该正控不宣称每页独立闭包。

回包返回后的原件写盘、hash 和解析也先预留工作；若预留失败就不继续处理该回包。exchange 的限读和硬截止仍由未来外层承担，本轮没有真实网络传输。

## 验证与原件沿用

实际最终命令（在仓根）：

```sh
python3 -B .chanlun/review-results/issue1419-raw-oracle-v2-20261018/fixes-v4/run_checks.py
```

结果 `Ran 22 tests in 16.609s / OK`；7 项受影响既有 API/codec/fake 回归、1 项原 F5 元数据检查、14 项本轮测试。没有运行测试模块的全量几何入口或真实 supervisor；没有发信号。新测试反例经历的真实失败输出 `tests.initial.stdout`、`tests.repaired.stdout`、`tests.final.stdout`、`tests.regression.stdout` 全部保留。部分中间失败来自断言误把几何字段错误当子字段错误、空图工作假设、records 在建图阶段更早耗尽，以及旧 fake 全图重复的预算拒绝；均写实保留，不充当最终候选证据。`tests.final2.stdout` 和 `tests.regression2.stdout` 为随后定向通过输出；最终候选统一以 `tests.acceptance.stdout` 与 CHECK-OUTPUT 的源身份为准，中间日志不自称冻结源码的完整证据。

F3 已在公共字段上重新执行 8 种 kind × 5 类非法载荷（缺失/null/字符串/缺 data/data=null）全集负控。F4 的 domain_fields 源字节未改，原独审结论复用；F6 supervisor、F7 raw_oracle 源字节未改，保留其原独审结果，不重跑真实 toy 或无关几何。raw、expected、独立账簿、定义引用与所有旧报告/失败原件均按 v3 manifest 核字节；沿用已核 139 前缀、35 分型、34 笔、10 完成段、2 中枢、16 格证据，没有重造 raw/expected。

## 精确参考与交付范围

`STATIC-WIRE-SOURCES.json` 固定 Git commit/blob、全文件 bytes/SHA256、摘录行号、UTF-8 字节半开区间和摘录哈希：

- `2694afcf147f3ba4114fde386470886c2428efc0:rust/src/bin/s_session_v3/object_records.rs`：102–118 公共 15 字段；794 发布 payload；70–100 区分内部 payload_json；769–829 为 typed_wire。
- `4161ebfe6d1a11b93cccf1de1a8f134c6c7d2c6a:s_session/browser/s-content-v3.js`：97–103 read64；648–681 headerFromRaw；746–766 Graph.create 逐节点检查。
- 同基线 `s_session/browser/q3-consumer.js`：42–60 FIXED 字段；1628–1656 comparePublication 对应关系。

复用管理目录既有 `XERJ-ATTEMPTS.json` 的失败原件和 `REFERENCE-HITS.json` 的固定响应，精确来源与字节见 `REUSED-EVIDENCE.json`。这不是本轮成功在线检索；未联网、重装服务或重新索引。没有可调用图谱工具，代码定位使用限定路径/固定 Git 对象。

本轮已有文件只改 driver 的 content_closure.py、driver_v2.py、fixed_cut_driver.py、test_v2.py，以及两个可再生根清单；新增实体均在 fixes-v4。`SCOPE.json` 由基线树与当前文件枚举得出。`changes.patch` 是相对固定基线的完整任务差异，排除自身、MANIFEST.json 和 SHA256SUMS 以避免循环；后两者由 seal_v4.py 再生。管理者负责接收、冻结和另会话独审。

## 仍未验证或授权

`authorization=false`、`execution_authorized=false`，source/tree/binary/frozen rule 仍为 `PENDING_SOURCE`，独审仍 `PENDING_INDEPENDENT_REVIEW_OF_V4_DIFF`。known_at、AsKnown、entity_revision、raw_history、withdrawn_objects、真实首次可见与完整来源接线仍 UNVERIFIED，静态结果不泛称 MATCH。

市场源认证、原量、完整历史、全级递归/下游消费、真实 S3/Q、正式双跑、持久恢复、浏览器、本票非空程序 full-group 一致性、真实进程组归属/owner 死亡和 OS 硬帽均 NOT_RUN。本轮不改变任何整票、父域或资源准入结论。未运行 cargo/rustc/Lean/Long/产品服务/真实库；未读凭据，未认领/评论/提交/push/merge/切分支/修改 Git 配置，也未开启其他工蜂。
