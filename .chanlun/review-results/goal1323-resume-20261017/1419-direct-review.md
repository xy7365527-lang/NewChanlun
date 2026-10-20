# 代码审查报告：#1419 直接内容构造与 CC-054 窄合同

审查树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1419`。只读审查，未运行 cargo、未修改源码。作者与其他代理仍在工作，以下绑定精确快照，不覆盖后续编辑。

- 主审文件 `rust/src/bin/s_session_v3/content_structure.rs` SHA256 `a312d2592d1888f815925fbf1e675a7262b032f74986bbfbbef6adfc3a380281`，保留在本报告同目录 `1419-direct-review-snapshot.rs`。
- 关联读取 `rust/src/bin/s_session_v2/tb02e.rs` SHA256 `9701f93d114eb62a8321cb558b6117a50b6653a0c81d7d7666fe5aedef369b60`。
- 接线读取 `rust/src/bin/s_session_v3/generation.rs` SHA256 `eea2f7e0f1894162dcdaad639ef2f8e255f523324d6ea788c65818627ef22b18`。
- 本轮 `git diff --check` 通过；这不是编译、测试或运行验收。

## 概念层质询结果

**发现一项明确合同缺口，结论 FAIL。** 旧 codec 桥的身份、同次游标和完整 JSON 转图缺陷已被此版本实质改写；不能沿用旧报告逐条判失败。仍未满足的具体项目如下。

### HIGH：真实 C/D/E/F 分类求值没有逐请求四轴知识记录

依据是本地已签附件 `.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.md:228`：CC-054 的对象为“一个具体分类请求及它所指结果版本的知识/计算状态”，四轴 input_quality/applicability/computation/validity 必须同时输出；不同原因可以多项。它并没有要求每个 typed object 都生成 request。

分类请求存在的直接依据及应绑定的对象：

| 已签行号 | 轴 | 当前实际输出 |
|---|---|---|
| 185 | CC-011 length / construction_conditions / direction | seed/seed_candidate 的条件向量与候选方向；须按各子轴的适用前件分别描述 |
| 187 | CC-013 semantic_termination_result | 每个 segment 终结求值；CASE_ONE/CASE_TWO 与未满足证据不能共用默认状态 |
| 188 | CC-014 known_fact_vector / scan_coverage | 候选 fact_vector 与第二序列 coverage；资源不足不能冒充结构否定 |
| 190 | CC-016 seed_condition_vector / core_order | 每个 center_seed_attempt 的向量与核心比较 |
| 192 | CC-018 unit_core_position | 每个真实 `(unit, center)` 位置比较 |
| 194 | CC-020 center_pair_development | 每个相邻中枢对，包括 PendingUpgrade |
| 196 | CC-022 assignment_status | 每个固定读法单元的归属结果，完成前件必须区分 |

代码证据：`content_structure.rs:235-274` 的 emit 只调用共享 build_object；该共享函数最终 `content_producer.rs:1022` 固定 request=None。当前文件没有知识 request 构造/赋值。`generation.rs:1051-1058` 仅收集 `Some(request)`，所以以上实际产生的分类求值不会进入 CC-054 的 requests。全局 waiting_reasons 被收集，只证明原因串没有丢；它不能指出哪个具体请求、哪一结果版本的哪个轴处于何状态。

最小反例：三个完整连续、方向交替单元形成 strict seed，程序输出 CC-016 seed_condition_vector/core_order；或者存在一对已核相邻中心而一端≥9，程序输出 CC-020 PendingUpgrade。对应 CC-054 requests 中均无这个求值主体及四轴，而已有 A 请求照常存在。单靠记录有 payload/result/status 不能满足另一个知识乘积轴合同。

最小修改方向：明确构造上述实际分类请求的 request key、结果版本/subject_id、真实四轴及源坐标，并直接接内容边；数据、适用域、计算完成、有效性分别从可核事实取得，不能“有 reason 就全写 incomplete”。一个对象可承载多个请求，单个 `TypedObject.request: Option` 可能不足，应使用明确的请求集合或一个请求承载有名子轴，按现行请求合同确定，不以默认值糊过去。

**不得盲用 A 工厂 `request_value`**：`content_producer.rs:1173-1220` 只识别三个 A 原因来判断输入不足，任何其他原因统一改成 DOMAIN_PROOF_MISSING / INCOMPLETE。对已完成且明确排除的 seed、对尚未完成单元、对预算受限扫描、对≥9待实现分类，其四轴不能通过该规则推导。此函数是A合同实现，不是一般知识推理器。

### 撤回旧报告过宽判断

旧报告“工厂 request=None 因而 C/D/E/F 请求未闭环”没有明确请求域，容易被读成每个对象都必须 request，现明确撤回该笼统判断，并以上述逐轴证据替代。CC-012（186）、CC-017（191）、CC-055/056（229-230）明定角色全集/公式/关系/身份不变量，不能仅因存在对象就强造分类请求；它们的实际等待和未支持范围仍应准确公开。旧 builder 没有请求也不能否决更高层已签 CC-054 义务。

## 已核通过的概念部分（静态限定）

1. **同次游标因果已修正。** `StructureEvidence:20-25` 借用 segment/stroke facts 和 SecondFoldStore；`generation.rs:711-717` 在同一 parser 调用前恢复并保留返回 store，build_cd 从它查 cursor。缺候选游标返回错误，没有宽松重判。新 CD 测试故意换空 store，要求 IdentityConflict，并要求真实第二种候选计数>0。
2. **直接内容构造路径成立。** emit/fact 使用 CV/共享 Built，旧完整对象 JSON 转换助手已限制 cfg(test)。WireSerializer 把整数转成十进制文本，保留 bool/null/序列/枚举形状，与 tb02c::wire 的现行整数事实语义一致；浮点拒绝，不假称支持任意原始数字 token。checkpoint 还原仅处理 parser 自有 wire，不把完整业务对象再变回旧 JSON builder。
3. **对象身份闭包已有实质修复。** 第一/第二序列对象先构造并取真实 obj3，再进入 segment；关系见证、change after/object_id 使用该次真实返回身份。EF 的 unit_object_id 来自 CD SegmentObject 映射。不存在旧桥的 obj-segment-i 或 obj-/obj3- 混接。外部 bi/entity 关系属于实体域，不要求每个字符串都等于对象 ID。
4. **真实来源与段区间。** stroke_refs 从同次笔端 source_coords 收集并排序去重；emit 通过 producer 已登记源版本取 refs。`segment_units:1044` 依据同次 candidate 构造成员笔价格并调用共享 units_from_segment_rows，后者按成员极值而非几何端价或 observed_through 全集取区间，再与同次塔 L0 对拍；不是直接给循环位置 i 充来源。现有新增未登记来源反例有明确失败断言，未执行。
5. **prior 的内容验证和授权边界有分工。** verify_prior:125-170 检查节点、kind、六字段身份、schema/semantic/policy/view、fact_key-slot 与实体名；read_prior 拒绝重复实体和缺版本根。EfPrior 限定允许kind和双版本根。它们自身不证明历史可用性；该授权由 prepared 使用 audited.objects，object_index::structure_prior_records 仅选未撤回条目并拒多份版本来承担。不得把孤立 Merkle 自洽当作发布授权。这里未发现可证明的绕过，但真实 restart/corruption 路径尚未执行。
6. **E/F 判据同源。** build_ef_units:1120 仅执行一次 scan_center_facts_full，F 的 tested_positions/pair_facts 消费同一 facts。Zn 外缘取共享 CenterRoles/window_center；≥9 隐去动态四变量并给 null_reason，不发普通 member_of；pair 显式 PendingUpgrade，矩阵/叶不伪造。与 tb02e/f 及 .chanlun/definitions/zhongshu.md 的受限过渡合同一致；它不意味着 #1429 已完成。
7. **修订和时点投影与旧 oracle 大体一致。** 逐字段 Built 摘要对比、排除既定簿记字段、首次formed及settled时点保留、pair leaf/input变更及撤回路径均存在。此次静态核查未发现>80%确信的额外行为回归；这不是完整等价证明。

## 验证缺口及审查停止点

现有 CD oracle 测试覆盖真实冻结输入、二序列非空、同对象重建身份稳定与错误store；EF前缀测试覆盖有界构造、pending翻转、旧时点和witness闭包。它们尚未执行。EF测试直接使用显式外部CD输入，不能替代实际parser→CD→segment_units→EF的完整链；现有保留历史断言也不等同冷重启验证。主控规则目录/新writer域另有并行审查，本报告不重复授予通过。

## 工程层审查

按概念层先行纪律，因具体 CC-054 合同缺口，停止工程质量评分。结论 **FAIL（上述缺口未闭环），已修正部分可保留为中间成果；不关闭 #1419/#1323/#1279。**
