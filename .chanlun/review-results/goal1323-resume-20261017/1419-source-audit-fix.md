# #1419 来源审计接线候选

2026-10-17；工作树 `resume-1419`。本报告是绑定 #1419 的工作草稿，不代表已编译或验收。

## 改动

- `source_audit.rs`：真实冻结 rule 沿完整路径和单对象同核传递，修复 HEAD 已有未定义 rule；只在 V3_STRUCTURE_RULE_REVISION 精确允许 tb02c/e/f 的已声明 kind。通用 raw 原件、input_refs/source_coords 相等、规范 fact_key 和 obj3 身份核仍先执行。
- `source_structure.rs`（新增，作为 source_audit 子模块注册）：精确 schema/semantic/policy/view_role/七字段头、generation 与槽位核；实体槽位、关系三元槽位、change 三元槽位及 after/object_id 对应。三个版本根必须唯一且覆盖当前全原始账。跨对象核覆盖当前 sequence_object_id、second_sequence_object_id、unit_object_id、witness_object_id、change object_id；历史 before/previous 不冒作当前引用。核段序列角色、实体绑定、单元对象类型和实体、关系见证类型/来源包含、change.after 原件相等。
- `source_pending.rs`：新结构请求走具名四轴核，绑定规范对象 fact_key、当前 subject obj3、来源、固定 structure-classification/1、精确 CC/axis/path。直接读发布事实核 COMPLETE/OUTSIDE_DOMAIN/证据缺失/计算截断，不重跑判据；false vector 仍完整，≥9 pair 仍缺域证据。按每个对象可枚举的具名请求集合数量补遗漏门，不能靠删除请求逃过。历史域的非步骤请求规则不扩大。
- `source_contributions.rs`：沿 self.rule 补 Native 同核参数；StepContribution 不再把 equal_is_evidence 硬编码 false。复用 ids→objects 做定位，新增结构对象边逆索引和仅三个版本根的索引并纳入 retained_bytes；改动/移除对象使相应结构边及请求主体失效，改动/移除请求重核所涉及主体的具名请求完整性。只访问真实变化和其反向依赖，不调用全量 audit_generated/audit_full 兜底。
- `source_delta.rs`：测试用旧贡献适配调用点仅新增 self.rule 参数。

## 验证

- 以上五个文件 `rustfmt --edition 2021 --config skip_children=true` 及 `rustfmt --check`：exit 0。
- 四个已跟踪文件 `git diff --check`：exit 0。新增 source_structure.rs 由 rustfmt 检查。
- 新增五项测试源码：精确域与槽位；已算 false 向量与 direction 域外；主体/槽位/来源/轴错绑；pending pair 四轴；悬空或旧身份子边。正控均同时保留。
- **NOT_RUN**：未执行 cargo/rustc 或任何 Rust 测试；单文件格式检查不能证明类型检查或运行通过。未访问正式服务、HTTP、外盘；未 commit/push。

## 审查重点与边界

- 此核审的是原件、身份、来源和请求对事实的对应关系，不重新证明全部 C/D/E/F 分类语义；分类器同核及独立 oracle 门仍由原实现/测试承担。
- Native 新逆索引和请求遗漏门目前只有静态检查，仍需独立工程审查，以及带真实 owner 的增量正负控和资源测试。它们沿已有 native 容器授信入口；不声称解决 #1488 已知 retain 生命周期问题。
- 没有改变 generation/content_structure/content_producer；没有合入、关票或改写全图验收结论。

## SHA256

- `source_audit.rs`：`fc7620de0ae1ace4caeabf3b9eb926dbc7e708f1875178141968c4c56fbb2b62`
- `source_pending.rs`：`ec80d4e7784e23ffa87fcb30e54467bcac7941f1bff8dea064c3af63ed707633`
- `source_contributions.rs`：`e03a4acfc1d9be6ea47053d3c80408c1537ac9f128b9bdc3bf211df81e7766c3`
- `source_structure.rs`：`171201dc519b0646caf465478a2389cbe73ed6b19baf0c27ad9d29c9b0d2b671`
- `source_delta.rs`：`3569fa3d20307c971eeb989146d99a21f1ff155763b89708c8af1ccbbd04a149`
