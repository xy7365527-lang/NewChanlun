# #1370 TB-01-A：Prime L2–L4 合同处置补核

结论：**L2/L3 未见本片合同冲突；L4 所举混合输入不足以成立 M，但坐标显示和验收统计的边界必须保留。新增 0H / 0M。** 结论由合同和固定源码得出，不继承评审的 Low 等级。本报告不处置 H1、L1，不替代整票验收。

评审文件固定在 `f2213dd33fcfc8f7e200bca58b391c2671268339`，其实际代码对象是 `e79c74a17a800e660b515e1f88fc9a5706826b42`；已核该评审提交中的 S 源与 e79 逐字相同。签字合同固定在 `efc1ddc4d6c015f4f8c6d44f904d704a88308b46`。本轮实时读取 [#1370](https://github.com/xy7365527-lang/NewChanlun/issues/1370)，正文与既有票面一致（updatedAt `2026-09-09T05:44:45Z`）。来源 SHA-256、逐项定位和静态样例在 [JSON 证据](/tmp/newchanlun-1323-publication-20260909/inputs/TB01-A-REVIEW-LOW-DISPOSITION.json)。

## L2：两个原始修订列相等，不等于丢失结构对象修订

[#1370 AC2/AC5](https://github.com/xy7365527-lang/NewChanlun/issues/1370) 要求保留事件身份/revision，并让“输入 revision、结构对象身份和对象 revision 分别绑定”；[精确合同 §4](https://github.com/xy7365527-lang/NewChanlun/blob/efc1ddc4d6c015f4f8c6d44f904d704a88308b46/.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md#L45) 要求 `input_refs` 与 `object_revision`。这些条款没有设立独立的“输入档案级 revision”。

当前 [INSERT](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L580) 的 `input_revision` 和 `revision` 都取源事件 `e.revision`，是本片输入模型的别名。它们被分别保留在 [raw 查询](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L635)及 [input_refs](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L1031)；[对象](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L754)另有内容寻址 `object_id` 和 `object_revision=1`。因此源事件 revision=7 时，结构对象 revision 仍可为 1，不会互相覆盖。模式版本 `schema_revision`、输入档案 `input_file_hash` 也不能改称对象 revision。

处置：A 不需要为了两列同值新造一个修订域或改 schema。真正的源事件新 revision、更正幂等/冲突、supersedes/replaces、旧对象撤回与历史由 [TB-01-B #1371 AC1/AC2](https://github.com/xy7365527-lang/NewChanlun/issues/1371) 承接；A 当前同身份异内容仍报 IdentityConflict，不声称这条更正链已经交付。

## L3：parser 原始 K 序号与来源坐标各有字段

[#1370 AC2](https://github.com/xy7365527-lang/NewChanlun/issues/1370) 同时要求接纳序和源坐标；[合同公共信封](https://github.com/xy7365527-lang/NewChanlun/blob/efc1ddc4d6c015f4f8c6d44f904d704a88308b46/.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md#L9)要求结构坐标与来源游标分字段、不同来源数字不可比较；[SPEC C03.1](https://github.com/xy7365527-lang/NewChanlun/blob/efc1ddc4d6c015f4f8c6d44f904d704a88308b46/.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/SPEC.md#L253)也把原始坐标与逻辑输入顺序分列。因此没有 `seq == source_coord` 的合同前提，评审把两者不相等一概称为域外的说法应订正。

[接纳](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L505)按输入事件顺序对新身份分配持久递增 seq，把输入 `e.seq` 另存 source_coord；[推进](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L635)严格 ORDER BY seq。此 adapter 用该 seq 构造 [Bar.source_index](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L924)；parser 中它指[原始 K 序号](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/theta_v0/types.rs#L11)，合并时保留[组内首根序号](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/theta_v0/parser/inclusion.rs#L165)。[组成员查回](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L955)又用同一 seq，最终见证/input_refs 同时保留 seq 与 source_coord，浏览器也[分别标明二者](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/s_session/browser/index.html#L134)。

静态可核对例：价格 10000/11000/10500 的三个新事件，source_coord 分别为 9007199254740993/994/995；S seq 和 Bar.source_index 分别为 0/1/2，merged 窗口为 0/1/2，仍分类 TOP，原三个源坐标完整留在各自原始引用。此处没有把源坐标值写进结构坐标，也没有按源游标跨来源排序。本轮没有运行该样例。

处置：没有源成员错配的最小反例，不应直接把 Bar.source_index 改成 source_coord。消费方若要来源游标应使用 source_coord；TB-02 承接完整输入投影/包含组锚域，TB-01-B 承接修订历史。本片结论不证明这些下游域已通过。

## L4：原始窗口失败与合并窗口成立可以同时为真

[#1370 AC3](https://github.com/xy7365527-lang/NewChanlun/issues/1370) 明确要求“不满足域的输入另报，不能制造第五个 Other 分型”；[已签 CC-006](https://github.com/xy7365527-lang/NewChanlun/blob/efc1ddc4d6c015f4f8c6d44f904d704a88308b46/.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/inputs/SPEC-COVERAGE-INPUT.md#L182)的量化对象是“顺序标准 K 三元组”，局部域为相邻无包含。两处判断是否冲突，要核它们具体作用的三元组。

固定源码对评审例 10000/10500/10500/11000 的静态推导为：

| 层次 | 三元组／成员 | 结果 |
|---|---|---|
| 原始窗口 | raw[0,1,2]、raw[1,2,3] | 两份 adjacent_inclusion，保留原始价格 |
| 合并窗口 | 10000/10500/11000；raw 成员 [0]/[1,2]/[3] | merged[0,1,2] 为 RISING |

[raw 观察](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L971)与 [merged 分类](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L1007)分别保存。首两点已经严格建立 UP，合并不依赖初始化默认；保留首根是[组锚定义](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/theta_v0/parser/inclusion.rs#L165)，两根同价原始成员都在见证中，未选择某根作为同价极值端点。因此该例没有证明消费 [G-001/G-002 未定实例](https://github.com/xy7365527-lang/NewChanlun/blob/efc1ddc4d6c015f4f8c6d44f904d704a88308b46/.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/SPEC.md#L274)，也不是同一个判断的宽严兜底。

仍须明写三项边界：

- 观察 window_* 是 raw 下标，对象 window_* 是 merged 下标。[对象标题已写 merged](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/s_session/browser/index.html#L162)，但[域警告只有“窗口”](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/s_session/browser/index.html#L185)，没有显式 raw 标签，也未展开 detail.raw_prices。这是当前显示精度缺口，不能在报告里称坐标空间已全部明确。
- [catalog.evidence.tested_domain](https://github.com/xy7365527-lang/NewChanlun/blob/e79c74a17a800e660b515e1f88fc9a5706826b42/rust/src/bin/s_structure_session.rs#L1267) 是固定声明；classified_objects/windows_total 是本批 merged 统计，代码没有逐批验证“整个原始输入无包含”。run 仅表示产生过分类实例，proof_status 仍为 not_proved。这个字段组合不能作为实际输入已满足声明受测域的自动证明。
- 混合例只能用于域观察和原始映射检查；**不得计入 TB-01-A 的无包含成功样本，不销项 G-001/G-002 或完整包含域。** 这些完整成功义务仍由 TB-02 承接。坐标标签与逐批域声明可以进一步澄清，但所举例没有要求删除合法 merged 对象或更换判据的证据。

本轮只读取固定源码、签字合同和票面，并吸收独立 L4 静态复核；没有编译、native/HTTP/浏览器/数据库运行验证，没有修改仓库、进程、服务或 GitHub。
