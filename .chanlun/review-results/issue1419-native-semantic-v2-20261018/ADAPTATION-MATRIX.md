# #1419 Native 语义适配增补矩阵

> 工作草稿，绑定 #1419；本轮执行授权仅源码与静查，无编译、运行或合入授权。

前轮三方矩阵及 SHA 原件只读复用 `/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/run01-artifacts/`。固定基线 `5cb7253d46d973669e7d10cd3331e12c9ef313b1`；语义来源 `634f62cddfaddf656a35ca9b9db20cb716b62bff`；原差异基线 `38aeca9d4699742d40353cff7fe51d1935651176`；真实共同祖先 `9160fc9deec9bb8870c6e757583424b272cf3d69` 仅用于逐文件差异定位，不切换基线。

## 实施前确定的必要写集

| 文件 | 符号/用途 | 语义来源与资源处置 |
|---|---|---|
| `rust/src/theta_v0/parser/stroke.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/parser/second_fold.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/parser/mod.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/parser/segment.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/parser/feature_seq.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/parser/second_kind.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/config.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/classifier/center.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/classifier/center_scan.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/classifier/recursive_tower.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/classifier/mod.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/parser/stroke_tb02v_tests.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/parser/tb02w_tests.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/classifier/tests/center_scan_1419.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/theta_v0/classifier/tests/center_pair_1424.rs` | 笔段/第二序列或中心共享核与定向测试 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v2/content_bi.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v2/content_producer.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v2/tb02a_facts.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v2/tb02b.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v2/tb02c.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v2/tb02e.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v2/tb02f.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/generation.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/object_index.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/prepared.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/mod.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/aux_index.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/catalog_evidence.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/source_audit.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/source_contributions.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/source_pending.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/source_delta.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/source_structure.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/content_structure.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_session_v3/structure_value.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |
| `rust/src/bin/s_structure_session.rs` | Native 结构生成/身份/前代/审计/目录接线 | donor 同路径已裁语义；Native 变动逐项在实施结果补录 |

## 调用闭包与保护边界

- `generation` → `parse_layer_with_group_evidence_store` → `stroke /2`、`segment /2`、`feature_seq`、`second_fold`；沿用当前包含状态，不修改 inclusion.rs 初始化。
- `content_structure` → C/D 同次事实 → E/F 段单元合同 → `center_scan` → `recursive_tower::scan_center_facts` / `center::{SeedRule,CenterRoles,PairEndpoint,PairLeaf}`。不复制 61 bin/33 theta 整树。
- `tb02e/f` 只取新 S3 实际消费的合同、常量；S2 全树 builder 不进入 S3。必要共用核与纯对照测试按真实角色列明。
- `StructureOutput`、prior、请求键、codec 中间数组/文本须逐个准入并由 owner 持有许可；保现有 NativeValue、CodecVec、FixedRadix、retained Map/Tree、借用 reader 和两段 prepared lease。
- `content_bi` 的 /2 极值区及整区间成员、`tb02b` 规则分派、入口默认规则为与 #1393 的共享面；不纳 #1394 99e819，不借 #1488 其他树验收。
- 旧 /1、/2 按历史只读身份；当前新规则必须同时接构造和 payload/links/full、request exact、三族版本全 raw 来源。

## 调用闭包增补（实施前登记）

- `s_session_v3/audit_owner.rs::{prepare, recover}`：donor 的六行现行规则写门；否则空历史库可在生成之前接受新输入或恢复写者。只接版本拒绝，不触动 owner/Stage/租约实现。
- `s_session_v3/driver.rs`：donor 历史库只读与失败无副作用定向测试。
- `s_session_v3/object_records.rs`：仅 donor 的同价多根测试 oracle 更新，移除历史竞争窗跳过；不改对象记录核。
- `classifier/tests/mod.rs` 接入 donor 中心扫描/对核测试；BTC 段夹具按前轮 SHA 原字节恢复，仅为对照，非 raw 成功。
- `content_structure` 的段输入合同移到 Native 借用/持费实现；原 tb02e 全 JSON builder 不引入。`structure_value` 是每个真实 buffer 自带许可的局部中间表示，不改 codec 编码核。

## 细化：K1 线形边界（实施前增补）

`second_fold::{SecondFoldCursor::wire_view,SecondFoldStore::checkpoint_cursors}` 增加只借用的序列化视图与检查点迭代。原 `to_wire/checkpoint/from_wire` 和恢复重折叠判据保留；S3 投影改用借用视图，避免先建立新的 `Vec<serde_json::Value>` / 游标整树再计费。整数文本仍由内容层同一 WireSerializer 编码。parser 内部折叠状态与恢复重折叠是沿用 donor 的独立算法工作区，不作为 Native 输出或 source retained ledger 的 owner。

`classifier/tests/center_pair_1424.rs` 的两份编译期输入当前基线不存在；增补 donor 同路径 `rust/tests/fixtures/center_pair_1424_{lean,worked_examples}.json`，仅供既有对照测试。Lean 夹具是 donor 既存求值结果，不声称本轮执行 Lean。

`content_bi` 的 /2 新字段采用同一持费 `content_structure::fact`：端点、条件、成笔证据与确认先序列化成内容根，原 B 包装只接边。新增 raw_first/raw_last/evidence 不经普通 CValue 树分配；原无 scope 手写编码仅留测试对照。既有 B 的身份/版本包装债务不冒称在本票整体迁移完毕。

定向测试的 `include_str!` 闭包按实际编译期引用枚举；若基线缺少 donor 的 `tb02v_same_price` 夹具，只恢复被上述新增测试直接引用的 JSON，不搬全套实验目录。精确路径与 SHA 进入 `SOURCE-MANIFEST.json`。

## 最终逐文件核对

下表是实际差异；精确字节来源由 SOURCE-MANIFEST.json 定位，非整树覆盖。Rust 行号属于本次候选。

|文件|符号/入口|实际资源或合同变化|来源|
|---|---|---|---|
|`rust/src/bin/s_session_v2/content_bi.rs:8`|`ContentBi::build / structure_fact / known_root`|保 FixedRadix；/2 新事实走持费 serializer，成员到终点末根|current + donor 逐符号适配|
|`rust/src/bin/s_session_v2/content_producer.rs:6631`|`build_structure_object / explicit_request_root / group_reasons_admitted`|新增兼容字符串先 reserve_native；请求根借边、原因数组持费；同价非等待|current + donor 逐符号适配|
|`rust/src/bin/s_session_v2/tb02a_facts.rs:968`|`catalog_axes_admitted / CatalogAxisTable`|保持费 JsonOwner/CodecVec；具名轴、三族版本等待、deferred 常量栈视图|current + donor 逐符号适配|
|`rust/src/bin/s_session_v2/tb02b.rs:11`|`BiContract / contract_for_rule`|/1 与 /2 身份映射；无新增持久 owner|donor 原字节|
|`rust/src/bin/s_session_v2/tb02c.rs:22`|`round_budget / scan_cursors`|S2 与共享 parser K1 合同；非 Native 主生成入口|donor 原字节|
|`rust/src/bin/s_session_v2/tb02e.rs:32`|`SegUnit / UnitInputs`|只取 E 常量与 Native 段输入类型；不引入 donor 全 JSON builder|current + donor 逐符号适配|
|`rust/src/bin/s_session_v2/tb02f.rs:11`|`KINDS / DEFERRED_SCOPE / PENDING`|只取 F 字面合同，不引入另一条生成路径|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/audit_owner.rs:168`|`prepare / recover`|仅补旧规则拒写门；不改 owner/预算/Stage 协议|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/aux_index.rs:6`|`audit / plan`|配套结构规则与版本判定，保原持费索引|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/catalog_evidence.rs:23`|`audit / stage_current`|按规则选择目录域并核 evidence，保当前 store API|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/content_structure.rs:532`|`build_cd / segment_units / build_ef_units / fact`|输出/关系/请求/字段 Native owner；纯核事实经同一整数文本 serializer；不建 serde 全业务树|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/driver.rs:571`|`historical_bi_rule_is_readable_but_writer_and_recover_leave_it_unchanged`|仅定向历史拒写测试|donor 原字节|
|`rust/src/bin/s_session_v3/generation.rs:85`|`StructurePriors / generate_impl / ObjectBody::Structure`|同次事实构造；输出、请求、全源数组持费；知识态最后建；保缓存失效|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/mod.rs:125`|`require_current_writer_rule / content_structure / source_structure`|当前规则写门、模块接线；不改 Native 设施|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/object_index.rs:2109`|`StructurePriorRecords / structure_prior_records`|从已核活动索引选根；CodecVec 准入，版本唯一性拒绝|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/object_records.rs:1`|`parity tests`|仅更新多根同价测试 oracle，不改记录生产核|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/prepared.rs:679`|`generate_bound / generate_node_products`|两段 lease 保留；结构 prior 在真实 store 域内分配；prior 生命周期与 raw 分离|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/source_audit.rs:1645`|`audit_object_record_with_slots / structure_rule`|三域 kind/header/来源绑定；保 Native ref 和 reader|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/source_contributions.rs:1`|`Contributions / apply / NativeObjects`|结构实体/版本/依赖索引为 TextValue+Map/Tree retained owner，增删同时维护|current + donor 逐符号适配|
|`rust/src/bin/s_session_v3/source_delta.rs:35`|`Contributions`|兼容完整/对照差分的结构依赖核对；保既有 BusinessMap 准入|donor 原字节|
|`rust/src/bin/s_session_v3/source_pending.rs:649`|`audit_request / StepLookup`|精确具名请求、主体/槽/来源/axes 核验；无宽松兜底|donor 原字节|
|`rust/src/bin/s_session_v3/source_structure.rs:55`|`audit_payload / audit_links / audit_request / Lookup`|完整 donor 审计合同接当前 SourceFieldStore/CodecVec/TextValue；不是重新分类|donor 原字节|
|`rust/src/bin/s_session_v3/structure_value.rs:13`|`Text / Vec / Map / Value / put`|新局部容器，NativeValue/CodecVec 实际持费，插入与失败归还遵当前 API|本轮 Native 接口适配|
|`rust/src/bin/s_structure_session.rs:104`|`RULE_REVISION / V3_STRUCTURE_RULE_REVISION`|入口版本与S2 K1接线；不改外部运行路由|current + donor 逐符号适配|
|`rust/src/theta_v0/classifier/center.rs:514`|`SeedRule / CenterRoles / PairEndpoint / PairLeaf`|共享中心、角色、五叶判据；donor 原核|donor 原字节|
|`rust/src/theta_v0/classifier/center_scan.rs:121`|`scan_center_facts_full / CenterScanFacts`|E/F 只消费一次扫描；算法工作区沿 donor|donor 原字节|
|`rust/src/theta_v0/classifier/mod.rs:63`|`center_scan`|暴露已裁共享扫描模块|donor 原字节|
|`rust/src/theta_v0/classifier/recursive_tower.rs:509`|`scan_center_facts / detect_centers_windowed_resume`|中心扫描同核与恢复；没有独立宽松判断|donor 原字节|
|`rust/src/theta_v0/classifier/tests/center_pair_1424.rs:1`|`定向测试模块`|donor 语义锁，仅测试代码，无生产资源变化|donor 原字节|
|`rust/src/theta_v0/classifier/tests/center_scan_1419.rs:1`|`定向测试模块`|donor 语义锁，仅测试代码，无生产资源变化|donor 原字节|
|`rust/src/theta_v0/classifier/tests/mod.rs:13`|`center_scan_1419 / center_pair_1424`|仅挂定向测试模块|current + donor 逐符号适配|
|`rust/src/theta_v0/config.rs:39`|`ParseConfig`|参数说明同步 /2；无分配变化|donor 原字节|
|`rust/src/theta_v0/parser/feature_seq.rs:143`|`SECOND_CANDIDATE_RULE_REVISION / SecondTerminal`|第二种事实及有限扫描合同；donor 同核|donor 原字节|
|`rust/src/theta_v0/parser/mod.rs:226`|`parse_layer_with_group_evidence_store`|沿当前包含态取证，新增传递同次 K1 store；inclusion.rs 不动|donor 原字节|
|`rust/src/theta_v0/parser/second_fold.rs:19`|`SecondFoldCursor / SecondFoldStore / SecondFoldWire`|donor K1 恢复及步进；新增借用 wire 与 checkpoint 迭代无自有 JSON 树|current + donor 逐符号适配|
|`rust/src/theta_v0/parser/second_kind.rs:1`|`confirm_second_kind`|接共享第二序列核，不新增放宽分支|donor 原字节|
|`rust/src/theta_v0/parser/segment.rs:160`|`CONSTRUCTION_RULE / IncrSegments`|/2 第二种终止与恢复调度；donor 同核|donor 原字节|
|`rust/src/theta_v0/parser/stroke.rs:16`|`RULE / Endpoint / PairFact / pair`|/2 极值区与同价证据；donor 算法 DTO，非 Native 输出账|donor 原字节|
|`rust/src/theta_v0/parser/stroke_tb02v_tests.rs:1`|`定向测试模块`|donor 语义锁，仅测试代码，无生产资源变化|donor 原字节|
|`rust/src/theta_v0/parser/tb02w_tests.rs:1`|`定向测试模块`|donor 语义锁，仅测试代码，无生产资源变化|donor 原字节|
|`rust/tests/fixtures/center_pair_1424_lean.json:1`|`直接 include_str 夹具`|测试静态数据；不作 raw 成功证据|donor 原字节|
|`rust/tests/fixtures/center_pair_1424_worked_examples.json:1`|`直接 include_str 夹具`|测试静态数据；不作 raw 成功证据|donor 原字节|
|`rust/tests/fixtures/tb02v_same_price/real_C_bars14_40_multi_root_second_gate.json:1`|`直接 include_str 夹具`|测试静态数据；不作 raw 成功证据|donor 原字节|
|`rust/tests/fixtures/tb02v_same_price/syn2_multi_root_group.json:1`|`直接 include_str 夹具`|测试静态数据；不作 raw 成功证据|donor 原字节|
|`rust/tests/fixtures/tb02v_same_price/syn3_multi_root_replace.json:1`|`直接 include_str 夹具`|测试静态数据；不作 raw 成功证据|donor 原字节|
|`s_session/tests/fixtures/tb02e-real/btc-tail50k-first600.json:1`|`直接 include_str 夹具`|测试静态数据；不作 raw 成功证据|donor 原字节|

所有生产差异均留在本 checkout；最终收包前没有运行 cargo/rustc 或产品。SOURCE.patch 的反向 --check 成功仅证明差异可应用，不证明类型/行为正确。
