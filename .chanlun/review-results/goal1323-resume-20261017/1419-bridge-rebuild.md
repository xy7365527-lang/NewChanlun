# #1419 结构桥复审接替：局部修复与闭环范围

结论：**结构桥仍未完成，不得生产接线或验收。** 本次只完成同次第二种游标输入修正，并把下阶段的实际接口缺口核到了源码。没有用 `Unsupported` 恒失败替代实现，没有改预算、切换判据、启用旧核兜底，也没有改三个禁区文件。

工作树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1419`。

## 本次实际修改和验证

唯一修改的源码为 `rust/src/bin/s_session_v3/content_structure.rs`：

- `StructureInput::second_store`（第 23–25 行）借用产生 `segment_facts` 的 parser 同次返回的 `SecondFoldStore`。
- 删除 `build_structure` 内新建 store 和事后 `restore_cursors`；第 60 行直接传入借用。恢复应在 parser 折叠之前完成，投影器没有恢复/推进权。
- 文件头改正“已审接线”的误导性表述，明确当前旧 JSON 桥和身份闭包缺陷。所有原有 codec 修复与三项测试保留。
- 单文件 `rustfmt --edition 2021` 成功；独立 `git diff --check -- rust/src/bin/s_session_v3/content_structure.rs` exit 0。

遵守禁止运行 cargo 的约束，没有编译或执行测试。新增字段只提供同次借用契约，**并不能用 Rust 生命周期证明调用者没有混配两个 parser 结果**；同次性还需生产接线和行为测试证实。全仓相关符号文本检索仅发现此文件的结构定义和函数定义，无调用点或输入构造点，不能宣称生产失败已修复。

图工具先查询 `NewChanlun` 得到未索引，再查询现有 `Users-silencehan-Projects-NewChanlun-rust` 得到相关符号零命中；因此对指定工作树回退源码读取，没有把另一树的图当本树现状。

## 已核事实及真正缺口

1. `parser/mod.rs:226` **已有** `parse_layer_with_group_evidence_store(state, config, store) -> (ParseLayer, GroupParseEvidence, SecondFoldStore)`。无需扩展 parser 或改段判据。实际生成器 `s_session_v3/generation.rs:689` 仍调用不返回 store 的 `parse_layer_with_group_evidence`。最小正确顺序是：读取已授权上一代 C/D version → 恢复检查点 → 调现有 `_store` 接口一次 → 同次事实与返回 store 一起交结构构造器。不能结构层重解析一次来补游标。

2. `tb02c.rs:176–272` 已从同次候选、笔事实得到真实实体、序列和段；旧 `add` 返回 obj-，嵌套 `sequence_object_id`、`second_sequence_object_id`、关系见证及 change 载荷会引用该返回值。新构造应先产真实 obj3 序列，再写引用它们的段，再写关系/change；**逐层按新内容重算身份**，不能对象构完后全局字符串替换。普通关系的 `subject/object` 常是实体 id，不能把它们一律改成 obj3。必须按字段语义区分实体引用和对象引用。

3. E/F 输入不是完成段枚举索引。`tb02e.rs:136` 的旧 `units_from_segments` 还核候选排序、成员笔极值、连续性、方向、与 `l0_units(layer.segments)` 的逐位一致，保留至多一个进行中单元。它消费完整 CC-013 JSON，不能直接作新生产入口。应将这些既有输入合同检查提取为接收 typed 行的共享核，由旧 JSON 适配与新同次事实适配共用；不是删去检查后直接拿 `l0_units`。typed 行的 `object_id` 来自本次直接生成的 CC-013；sources 来自相应笔端的真实 source_coords。进行中单元也必须接入。

4. E/F 判据已有共享核，不需要写另一套中枢算法：`tb02e.rs:394` 调 `scan_center_facts_full(&units, center_from_segments)`；`tb02f.rs:143` / `:206` 消费这同一次扫描的 `tested_positions` / `pair_facts`。新内容投影必须共用这一份扫描结果，并保持完成/进行中边界及已裁范围。把现成 E/F JSON 输出再拆成 CValue 不符合合同。

5. 本树生成输入只有 B 的 prior：`generation.rs:82–116` 的 `GenerationInput`、`BoundGenerationInput` 字段 `prior: content_bi::PriorState`；`content_bi.rs:749` 只含 `bi_records` 和 `version_record`。`object_index.rs:2030` 的 `prior_records` 和 `prepared.rs:593、703` 也只交 B 的根。结构桥输入虽有任意 `&[Built]`，**生产接入处没有交付 C/D/E/F 上一代已授权根**。单文件修改不能完成恢复、修订及撤回链。

6. 域不能局部硬编码：`s_structure_session.rs:102、107` 分别声明带 segment-second/2 的 RULE_REVISION 和只含 new-bi/2 的 V3_RULE_REVISION；`generation.rs:311–315` 按 V3 选目录，`tb02a_facts.rs:917–932` 只有 NewBi/SegmentSecond 两档。直接将对象工厂改成 V3 或直接改 V3 常量，都不能证明 E/F 目录/恢复兼容。正确构造器应借用实际 `GenerationRuntime` 的 producer，域统一由主控裁定并同步完整写者合同。

7. `content_producer.rs:1032` 共享 `build_object` 可直接接 CValue 值边；不需再实现 obj3 配方。其 `request_value` 在 `:1165` 私有，通用工厂返回 request=None。如 C/D/E/F 既有请求合同要求主体请求，应暴露共享请求构造口，接已生成 obj3 主体及真实原因/来源；不复制四轴规则、不把空原因当默认满足。还需生成器将请求纳入 CC-054（目前 CC-054 在 B 构造之前完成）。

## 最小具体文件面与接口

这里区分“独立构造器可在单文件继续实现”和“生产闭环必需跨文件”。**不能将工期或代码量冒称为技术上单文件不可能。** 直接构造 C/D/E/F 本身可在本文件写出；但当前完整目标包括生产、恢复、目录和增量域，上述真实接口缺口必须同步解决。

| 文件/归属 | 必须提供的接口或改动 | 不允许的替代 |
|---|---|---|
| `s_session_v3/content_structure.rs`（构造器工位） | `StructureEvidence` 借用同次段/笔事实、层结果、store；`StructurePrior` 持已核 C/D/E/F 活跃根及版本根；`build_structure(producer, store, evidence, prior, context)` 从共享事实直接写边；输出对象、真实 unit 映射、关系、请求/原因 | tb02c/e/f 完整 JSON → typed_from_json；合成身份/来源 |
| `s_session_v2/tb02e.rs`（共享输入合同工位） | 把 `units_from_segments` 的 typed 行校验提取成共享函数，旧 JSON adapter 与新事实 adapter 同调；扫描结果单次交 E/F | 将与塔逐位比较或 developing 输入删掉 |
| `s_session_v2/content_producer.rs`（共享值边工位，按请求合同需要） | 暴露带主体的共享请求构造器；实际已绑定 producer 提供来源数组/coords 根和唯一 obj3 工厂 | 再造 request 判据或单文件硬编码新域 |
| `s_session_v3/generation.rs`（根接线工位） | 输入增加 StructurePrior；parser 改用现成 `_store`；只执行一次 parser/中心扫描；传 runtime producer；结构对象/关系/请求入本次生成态；CC-054 在全部请求收集后构造；目录投影读真实结构小路径 | 调旧完整构造器救场；独立第二次 parser |
| `s_session_v3/object_index.rs`（根授权工位） | 在既有活跃独立索引上提供 C/D/E/F 记录根和版本根选择；保持 B 原接口 | 扫历史 JSON 拼任意 prior；把内容自洽当发布态授权 |
| `s_session_v3/prepared.rs`（StageOwner 持有人，**本代理不写**） | 在 full/bound 两入口把同一已审索引选择的 StructurePrior 传入 generation，恢复前可读；沿既有事务/资源所有权 | 绕开 StageOwner 或从未提交态恢复 |
| `s_structure_session.rs`、`s_session_v3/mod.rs`、`s_session_v2/tb02a_facts.rs`（根合同工位） | 同步规则域、初始化/写者接纳、完整 C/D/E/F 目录与旧域只读边界；先核既有 #1419/#1424 裁定，不擅造修订字符串 | 只改 manifest/单个常量冒充全域一致 |

无需改 `construction_state.rs`、`native.rs`、`scratch.rs`。也无需改 parser 判据或 SecondFoldStore 内部恢复机制。`tb02c.rs`/`tb02f.rs` 的旧 JSON 输出可仅留有界诊断对照；若需抽取其中非判据的身份/生命周期共享 helper，先报具体 helper 范围，不能通过改旧 Builder 的返回身份把混合图搬过去。

## 可独立推进的分割与验收

- 构造工位：实现 C/D typed 投影及逐层 obj3 身份闭包，再接 E/F；只消费事实，不运行第二套判据。先冻结 `StructurePrior` 与产物接口，再并行根接线。
- 输入合同工位：提取并对拍旧/新 typed 单元合同核，覆盖成员笔极值、非连续、方向不符、真实来源、进行中单元。共享核完成后 E/F 接相同扫描事实。
- 根接线工位：处理 prior 授权、恢复顺序、producer/版本和目录，禁区仍由原持有人负责；不能在构造器完成前宣称可接线。
- 独审/验证：保留三 codec 测试；增加第二种真实候选同次 store 成功、错配 store 拒绝、跨 cut/重启 checkpoint 连续；按 schema 枚举全部对象引用并核本批/授权 prior 闭包；旧 JSON 仅做有界逻辑对照且显式允许对象身份版本差异；双跑逐位、篡改/缺来源拒绝、目录及 CC-054 请求完备性。最后再 source-build/远端 CI，不把 fmt 当运行通过。

本报告是未完成范围交接，非生产许可。#1419/#1323/#1279 均不得据此关闭。
