# 代码审查报告：#1419 来源审计候选

日期：2026-10-17。绑定 #1419 的工作草稿。审阅树：`.sandcastle/worktrees/resume-1419`。只读源码；本报告是唯一写入。没有运行 cargo/rustc、正式服务、HTTP、浏览器、外盘或重型回放。

## 概念层质询结果

**限定范围通过。** 核对 `缠论知识库.md`、`definitions.yaml` 的来源地位，并以 `.chanlun/definitions/xianduan.md:199-208`、`.chanlun/definitions/zhongshu.md:198-236` 和封存 SPEC 的具名轴为准：`.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/inputs/SPEC-COVERAGE-INPUT.md:185-196/230-232`。候选消费既有事实，没有新增宽松分类器；false 条件向量、域外、缺证据与扫描计算未完成仍有区分。没有发现需要重新裁定定义的矛盾。

本次通过不覆盖分类器自身的全部语义、未实现轴或全图。以下是现有身份/来源合同的实现缺陷，不是提议更改教义。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | 未发现 |
| HIGH | 2 | 需要返修 |
| MEDIUM | 0 | 未另报 |
| LOW | 0 | 未报风格项 |

结论：**WARNING；当前来源候选不签静态通过，先修两项 HIGH。Rust 编译及测试全部 NOT_RUN。**

### HIGH-1：把输入事实代际当成变更发生代际，合法多代输出被拒

定位：`rust/src/bin/s_session_v3/source_structure.rs:122-128`，尤其 `:124-125`。

该分支强制 change 槽位首项等于 `payload.object_generation`。实际生产构造有两个明确不同的值：

- `content_structure.rs:387` 的头部 `object_generation = ctx.fact_generation`；
- 段变更 `content_structure.rs:975`、通用中心/对变更 `:1222` 的槽位首项为 `ctx.generation`；
- `generation.rs:1011-1023` 分别从原始事件推导事实代际、从当前提交取 generation；`content_bi.rs:140-157` 只在修订或乱序时增加 fact_generation，普通逐 bar 追加时它保持 1；
- 已有真实工厂测试 `content_structure.rs:2914-2919` 明确使用 `fact_generation=1, generation=28`。

因此一个合法 change 可以是 `slot=["28", entity, "formed"]`、头部 `object_generation="1"`，现代码必报“结构变更槽位不符”。所有 full/native 路径均调用此 payload 核，都会遭遇这个兼容性阻断。现有新增测试只用同为 1 的头/槽位，没有覆盖它。

必要修复：保留两个代际的原义。当前 material 的可信顶层 `generation` 是 `GenerationMeta.generation`（`generation.rs:1209`）；full 可从传入的 `state` 读取，Native 可从 `SourceMaterial` 当前 root 的同一字段读取，然后沿单对象同核传参。对本次新发 change，校验三元槽位首项（规范十进制）与 `data.known_at.generation` 及该当前代际相等；头部 `object_generation` 继续是事实代际。对象 record 没有简单的 `generation` 字段，现 `TYPED_SOURCE_FIELDS` 也只取七字段，不能误用 `first_known_generation`。若为历史 change 复用本核，必须传入其历史 cut 的可信代际，而不能拿最新视图代际覆盖历史。补真实工厂生成的 1/28 正控以及错绑事件代际负控，覆盖 formed/updated/withdrawn 和 C/E/F。禁止仅删掉校验，或为迎合新审计而把生产事实代际改成逐提交增长。

### HIGH-2：结构关系只核见证种类，没有核端点/关系与见证对应，悬空实体边可放行

定位：`source_structure.rs:106-121`、`:156-208`、`:296-317`；Native 依赖建立在 `source_contributions.rs:1120-1127`。

关系 payload 核只比较自身 `slot` 与自身 `source_id/relation_kind/target_id`，然后要求正数 version。跨对象核只要求 `witness_object_id` 存在、属于一组允许 kind，以及关系来源为见证来源子集；它不读取关系端点或关系名来核对该见证事实。`references()` 只收集 obj3 字段，不收集上述实体端点。

具体静态反例：取一个合法 `CC-055.segment_relation`，保留真实段 witness、正数 version 及全部来源；把 target 改为当前 cut 不存在的 `segment:missing`，同步修改三元槽位/fact_key 并用同一 producer 重算 obj3。也可把 relation_kind 换成根本未声明的字符串。通用 identity 核只能证明这些新字节自洽；新关系核的所有条件仍成立，而它与见证段的 `entity_id`、`sequence_id`、真实成员均不相符。请求完整性门也不会挡住，因为关系对象的 expected_requests 为 0。`source_audit.rs:2272` 之后没有第二道结构关系端点核。

这直接违反封存 SPEC `SPEC-COVERAGE-INPUT.md:116` 的 A-ST-044-02：“一批变更中的对象/边不混版本；已删除端点不能留下有效悬空边”。这是关系来源绑定校验，不要求重新运行分类判据。

必要修复：从当前已核 witness 数据投影该类型允许的关系三元组，并核端点、关系名、版本及来源的对应关系；对于只以实体名引用且不能由 witness 完整证明的端点，接当前实体索引并维护删除/修改逆依赖。不能只新增 relation_kind 白名单。full/native 使用同一核，补真关系正控、存在但错绑的端点、悬空端点、未声明关系、换 witness 和 Native 删除端点负控；新增实体索引仍须计入保留费。

实际投影对应建议（只读现有 witness，不重算分类器）：

| 关系 | witness 与可核字段 |
|---|---|
| C `derived_from` | `CC-013.segment`：source 是该段的 first/second sequence_id，target 是该段 entity_id（构造 `content_structure.rs:881-895`）；沿已核 sequence 对象落实当前端点。 |
| C `member_of` | 同一段 witness：target 是其对应 sequence_id，source 是该 sequence 的真实 raw_elements 对应笔实体；使用已发布 stroke_refs/序列事实，不做几何重判（`:898-923`）。 |
| E `attempted_from` | `CC-016.center_seed_attempt`：source=entity_id，target=units[0].unit_id，version=1（`:1519-1526`）。 |
| E `member_of` | `CC-016.center`：source∈members，target=entity_id，version=entity_revision；UPGRADE_PENDING 不发 member_of（`:1706-1716`）。 |
| E `connects_left/right` | `CC-022.unit_assignment`：source=unit.unit_id，target=left/right_center_id；版本取当前目标中心真实 entity_revision（`:1879-1889`）。 |
| F `position_of/against` | `CC-018.unit_core_position`：source=entity_id，target 分别为 unit.unit_id / center_id，version=center_revision（`:2037-2052`）。 |
| F `pair_from/to` | `CC-020.center_pair`：source=entity_id，target 分别为 a_center_id / b_center_id，version=entity_revision；两侧对象版本还应匹配 a_revision / b_revision（`:2109-2114/2228-2237`）。 |

同类缺口：change 分支 `:319-328` 只比 after 与目标 data 的 digest，未核 change 的 entity/kind 与目标 entity/kind 对应；返修时一起核定，避免“after 字节真实但给另一个实体记变更”。本项合并报告，不重复计数。

## 已核对但不夸大为运行通过的部分

- 真实 rule 已沿 full 单对象核及 Native 单对象核传递；`StepContribution::bind` 现在根据真实 rule 选择 equal_is_evidence。静态未发现这几处仍漏实参或使用未定义 rule；这不等于类型检查通过。
- CDEF kind 仅在新结构 rule 开放；七字段头、声明 schema/semantic/policy/view_role、fact_key 规范编码及 obj3 原件绑定均保留。旧域不会因这份白名单自动获得 CDEF。
- 三版本根逐 kind 唯一。来源先经严格升序与真实 raw 引用校验，所以此处 `sources.len()==raw.len()` 配合已有子集校验足以证明覆盖；不是无条件只计数。
- 请求完整性当前亦不只是裸计数：规范 request_id 全局唯一，fact_key/subject/source 绑定，允许 CC/axis/path 枚举受限；attempt 索引走 `parse_canonical_i64`，不能靠 `00`/`+0` 构造同一索引别名。对 n 次 attempt 的段，合法集合确为 `6+3n`；其余对象的合法请求空间也与 expected_requests 一致。本轮没有找到“替换缺失请求为另一合法唯一键”绕过。
- 对现已登记的 obj3 子边，删除/修改对象会通过 `structure_dependents` 重核引用方；主体修改会经 `request_subjects` 重核请求；删除/修改请求会触发 touched 主体完整性核。旧边先 unlink、新边再 link，未发现明显漏删。此结论限已登记字段，不能覆盖 HIGH-2 中根本未登记的实体端点。
- 未发现增量路径调用 full audit 兜底；新增索引已纳入 retained_bytes。未运行 owner/资源测试，不能签资源或增量复杂度验收。

## 验证及指纹

已读 staged/working diff；staged 为空。限定四个已跟踪来源文件 `git diff --check` exit 0。新增文件已逐段精读；未新增测试源码、未启动任何 Rust 检查。本报告反例为静态控制流推导，不冒称已经在 Rust 中复现。

| 文件（`rust/src/bin/s_session_v3/` 下） | SHA256 |
|---|---|
| source_audit.rs | fc7620de0ae1ace4caeabf3b9eb926dbc7e708f1875178141968c4c56fbb2b62 |
| source_structure.rs | 171201dc519b0646caf465478a2389cbe73ed6b19baf0c27ad9d29c9b0d2b671 |
| source_pending.rs | ec80d4e7784e23ffa87fcb30e54467bcac7941f1bff8dea064c3af63ed707633 |
| source_contributions.rs | e03a4acfc1d9be6ea47053d3c80408c1537ac9f128b9bdc3bf211df81e7766c3 |
| source_delta.rs | 3569fa3d20307c971eeb989146d99a21f1ff155763b89708c8af1ccbbd04a149 |

修复后须重新独审改变面，并在获准环境运行真实 full/native、发布/查询/恢复及资源验证。当前报告不改变 #1419/#1323/#1279 的验收状态。
