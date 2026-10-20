# 代码审查报告：#1419 来源审计返修 v2

日期：2026-10-17。绑定 #1419 的工作草稿；工作树 `.sandcastle/worktrees/resume-1419`。复审 `1419-source-audit-repair-v2.md` 的五文件冻结候选与原 `1419-source-kernel-review.md` 两项 HIGH。本报告是本次唯一写入；源码未改。

## 概念层质询结果

**限定范围通过。** 沿用初审已经回查的教义正本与封存 SPEC：语义状态与扫描覆盖正交、具名请求四轴及当前结构关系/变更的身份不混。返修没有新增分类器，没有改变事实代际含义，没有以宽松口径接管严格判定失败。关系核只投影已发布事实，未重算几何、种子或分类。

位置关系保留 witness 的 `center_revision` 有生产依据：`content_structure.rs:1998-2015` 在位置几何未变时保留旧 position 数据，`source_structure.rs:1121-1125` 与该真实 witness 修订核对，而不擅自强迫它等于目标中心最新修订。pair 的 a/b_revision 则来自当前双中心，依原生产合同核对。两者没有混为一条无依据的版本规则。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | 本次范围未发现 |
| HIGH | 0 | 原两项已静态消除 |
| MEDIUM | 0 | 未新增确定缺陷 |
| LOW | 0 | 未报风格项 |

结论：**PASS，仅返修源码的限定静态范围。所有 Rust 编译与测试 NOT_RUN。** 本报告撤销初审两项 HIGH 的静态阻断，不替代真实发布、Native owner、历史/恢复或资源验收。

### HIGH-1 复核：两个代际已分开，并覆盖缓存命中

- full 在 `source_audit.rs:2245-2249` 从当前 state 顶层 `generation` 读取；Native 在 `source_contributions.rs:2100-2105` 从当前 `SourceMaterial.root` 读取；二者沿 `audit_object_record_with_slots` 把同一值交给 `audit_payload`。
- `source_structure.rs:123-137` 检查 change 槽位规范整数与 `data.known_at.generation` 均等于当前 cut 代际。头部 `object_generation` 保留 fact_generation，不再被错误拿来比较。
- Native 将 change 索引列入保留费；删除时移除索引，插入时加入；`source_contributions.rs:1242-1261` 每代重核活跃 change，即使旧对象内容根未变也不能跨代沿缓存免审。
- 已复读真实生成链 `generation.rs:1058-1073/1110-1141`：当前对象由本次 shape+typed 组成，`objects_seq.sync` 同步本次列表；没有把持久 object_index 的全部历史 change 并回本次材料。因此当前 cut 的 change 约束成立；读取历史 cut 时仍使用历史根本身的代际。
- `source_delta.rs` 的 test-only 局部核调用也补了真实 rule/generation。该旧测试辅助增量器不等同 Native 生产贡献状态；本次不据它宣称完整新域增量验收。

原反例 `fact_generation=1 / generation=28` 现在静态满足代际门；同一 change 用可信 generation=29 审查会失败。没有通过删除校验来修复。

### HIGH-2 复核：当前端点与见证投影已绑定

`source_structure.rs:969-1177` 按实际十个关系分支核验，而非只加关系名白名单：

- C derived_from 同时绑定当前序列 obj3、段实体及段修订；C member_of 绑定当前笔与当前序列，按 stroke_refs/raw_elements 的同一下标核成员，来源取该笔两个端点来源并集。
- E attempted_from 绑定 attempt 与首单位；member_of 绑定 center.members、状态与版本；connects_left/right 绑定 assignment.unit 与两侧当前中心，核对应中心修订。
- F position_of/against 绑定 position 的单位/中心、witness 修订与相应来源；pair_from/to 绑定 pair 两侧实体、pair 自身修订及双方显式中心修订。
- 未声明关系、错误 witness kind、存在但错绑端点、悬空实体及错误版本都有明确失败路径。原“修改 target 并同步 fact_key/obj3”反例会被当前实体查找或 witness 投影拒绝。
- `source_structure.rs:318-356` 为非空 after 补目标 kind、entity_id、当前实体与 obj3 一致及 after 原件相等；withdrawn 核 before 实体、空 after/object_id 与当前实体缺席。原“after 真实但记给另一实体”反例已被拒绝。

full 的当前实体索引在 `:375-384` 建立并拒重复。Native 的 `structure_entities` / `structure_entity_dependents` 在原对象移除时撤销旧登记、通知引用方，在新对象写入时登记并通知；`references` 的 obj3 反向依赖仍保留。新增实体/版本/change 索引均列入 retained_bytes。对登记的依赖，未发现明显漏删或“只标脏却不调用共享 audit_links”的遗漏；未见调用 full audit 作为增量兜底。

本结论是现有关系对象的绑定完整性复核，不扩大成“全对象/全部关系已证明没有遗漏”。例如新增实体依赖登记本身不是整个结构语义或关系全集完备性的证明，后者仍依赖全域生成/验收合同。

## 测试源码真实性与剩余验证门

作者所述新增六项测试源码实际存在（加上原五项）；本次只读，**均未执行**：

1. `source_structure.rs:1281` 覆盖十个实际关系分支的正控，并逐分支改变目标、关系名、版本、witness；调用真实共享 audit_links。fixture 的来源均为空，不能据此证明非空来源投影正确。
2. `:1393` 保留 witness、删除 center 后调用共享核，确有拒绝断言。
3. `:1414` 的 Native 测试调用 `source_contributions.rs:2257` 的组件辅助器；实际使用 Map/Tree、NativeObjects、生产 dirty_from 与共享核。**没有调用 SourceMaterial/owner 的完整 audit_native_bound，也没有调用生产 remove_object/object_phase 来驱动整个删除流程**，作者报告已诚实声明此边界。
4. `:1432` 的 C/E/F change 代际正负控确用 header=1、event=28，并试 1/29 和过期 known_at。
5. `:1484` 保持真实 after 字节，改变 change 实体，核通过与失败两侧。
6. `:1518` 从封存 TB02D raw-ledger 读取实际 bars，经真实 parser 与 build_cd 发出 change；`seen>0` 排除空真，继而把真实 prior 传回工厂暖续到 29，并断言旧 change 不复制。该测试核的是 change payload 代际，未调用 full/native source audit；原始 source refs 用测试绑定，不能称为完整原账来源验收。

以上测试比只改字符串更有针对性，但后续获准环境仍须运行：真实生成 A…F 材料的 full/native 同核、非空来源的十关系正负控、完整 owner 的保留旧 change/删除端点/替换端点版本增量、发布/查询/恢复，以及资源帽内原成功合同。不能把本报告和这些尚未执行的测试源码当作已跑通过。

## 检查与冻结指纹

五文件现值与作者冻结清单一致。四个已跟踪来源文件 `git diff --check` exit 0。没有 cargo/rustc、正式服务、HTTP、浏览器、外盘或重型回放；未 commit/push/合入/关票。单文件 rustfmt 的作者结果未当作编译证据。

| 文件（`rust/src/bin/s_session_v3/`） | SHA256 |
|---|---|
| source_audit.rs | 0c43214deaa9fa49184fb3bec6d484cf15ee9bee36dbbbbf22c56a7fe6fcad73 |
| source_structure.rs | 5dd619d794f97a06800d4b92210f253c7afc0ddad84ae80b83c3516685255111 |
| source_pending.rs | ec80d4e7784e23ffa87fcb30e54467bcac7941f1bff8dea064c3af63ed707633 |
| source_contributions.rs | 507a66ef81681f02f7ae580fac2493488b54a6de218b42e88b5e224a83c307ce |
| source_delta.rs | f5975d169300e27d4e094454736dd64f71aef5897631f1c54d16133a08f4b810 |

`source_pending.rs` 与初审哈希相同，具名四轴请求逻辑未改。本复审不撤销运行验证、全历史回放和 #1323/#1279 的剩余验收门。
