# #1419：结构生产根接线候选

状态：源码接线完成、独审与运行验证待办。未编译、未运行测试，不能称生产/重启/资源验收通过。

工作树 `/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1419`。源码归属仅以下 4 文件，未碰 StageOwner 控制流程及 construction_state/native/scratch。主控拥有规则常量/初始化/目录域，构造工位拥有 content_structure；本文件记录接线面。

## 实装

- `generation.rs:83` 新增 `StructurePriors`：已核活动段根、段版本根及 E/F 活动实体/版本根。
- `object_index.rs:2063` 新增 `structure_prior_records`：仅选未撤回的活动独立索引项；段版本、center 版本、pair 版本各自拒绝重复。E/F 选 seed attempt、center、roles、assignment、position、pair 及两份版本；不从任意历史 JSON 猜 prior。
- `prepared.rs:585/695` full 与 bound 入口均交同一 audited.objects 选出的 prior。总计新增 4 行，不调整 lease、材料签发、COMMIT、StageOwner 生命周期。
- `generation.rs:706` 仅主控明确的 `V3_STRUCTURE_RULE_REVISION` 启用 C/D/E/F：先从已授权段版本恢复 cursor，再仅一次 `parse_layer_with_group_evidence_store`，返回 layer/evidence/store 同次借给构造器。历史 A/B 域仍是原 parser 入口；这是域选择，不是新域失败后的兜底。
- `generation.rs:1034/1041` 同 runtime producer 直接 `build_cd` 后 `build_ef`；E/F 消费 C/D 本次真实段对象映射，单元合同复用已抽取的 typed 核。没有旧完整 JSON builder，没有生成合成身份。
- `content_bi.rs:140/1718` 按主控授权公开共享 fact_generation 与 known_value。后者参数从私有 Ledger 改为已有 BiLedgerView，内部立刻包装原 Ledger；B 旧调用改传 input.raw，算法原样。结构 known_at/fact_generation 与 B 共用同一判据。
- C/D/E/F 对象、关系、waiting_reasons 纳入本次生成；按已与构造工位冻结的 `StructureOutput.requests: Vec<Built>` 收集完整唯一请求集合（含一对象多子轴），不再从 `TypedObject.request` 重复收集。具名请求的实现与四轴合同由桥工位完成，本报告不宣称其验收通过。全部收齐之后才构造 CC-054，仍插回历史对象序中的原知识态槽位，保旧域序列兼容。
- `finish_axes` 按实际 meta.rule_revision 查主控已审表，未知域 Err；目录投影四类版本仅读取真实 data.waiting_reasons 小路径，不导出大 payload。

## 测试源码

新增 `structure_domain_wires_versions_knowledge_axes_and_rejects_foreign_prior`（generation.rs:1989）：

- 用现有原账测试输入进入真实新域 generator；要求段/center/pair 三版本确实存在、22 轴和新闭包有效。
- 每份结构版本的等待原因必须进入 CC-054；独立 runtime + 独立 MemoryNodes 同输入应产生同根。
- 从本次 TypedObject.record 抽出实际 prior，续跑与冷 runtime 重启应同根。
- 自身合法的 B 版本根混充段版本必须拒绝，不能进入历史核兜底。

这些是未执行的候选行为锁；小前缀并不证明存在非真空第二种候选或完整历史回放，相关实测仍归远端验证。没有靠测试文本宣称恢复已通过。

现有本文件/object_index 测试构造点加空 structure_prior。跨归属的 aux_index.rs:3273、catalog_evidence.rs:1256、source_audit.rs:2435/2729 已报主控统一补 `structure_prior: crate::v3::generation::StructurePriors::default()`。本文件中普通 rule="r" 测试已统一明确为 V3_RULE_REVISION；错误规则测试仍保留 wrong-rule/foreign-rule。外部跨代测试不能持续传空 prior，相关风险已报主控。

## 验证与边界

已执行 4 文件单独 rustfmt --check 与限定 git diff --check，均 exit 0。未 cargo、未服务、未外盘、未 commit/push。

新结构构造会增加生成期工作量；全 source 坐标 materialize、共享 fact_generation 的再次历史扫描尚未通过资源/性能验收，不改变 16/8MiB、stage、恢复或发布 guard。#1488 的另树改动以后需按字段合流，不直接整文件覆盖。

补丁 `1419-generation-wiring.patch` 为本 4 文件相对恢复 HEAD 差异。哈希（记录时）：

- `rust/src/bin/s_session_v3/generation.rs`：`027d9253747b75994c433e40f225c3936a803bcdb22985889074ce6e06eb01cd`
- `rust/src/bin/s_session_v3/object_index.rs`：`6dc03d398393c8f2705aae5815ad82fd2ae0c2fbe94f59a05ec811bbd36a2f87`
- `rust/src/bin/s_session_v3/prepared.rs`：`dc1cdab4f60bd804ebe679e006ff790a9ca253f0d4a29ba97b9f4ff73893b97a`
- `rust/src/bin/s_session_v2/content_bi.rs`：`b3ed67cce4f3c5a315ed69f76a03b441cc6dcab317abd4b16b1023b8a8855335`

此报告不关闭 #1419/#1323/#1279；主控应复核规则域、源审计/写者允许域、直接桥身份闭包、prior 完整性与远端编译/行为。
