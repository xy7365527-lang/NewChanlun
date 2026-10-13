# #1419 TB-02-E schema v1.2（冻结，2026-10-14：合同复核 PASS_SCHEMA_V1_1_FREEZE，`tb1419-review-r2/schema-v1.1/RESULT.json` 4726a10e…，冻结时按 E1–E4 改措辞，不改语义）

## v1.1 → v1.2 差异（冻结措辞，复核 E1–E4，不改语义）
1. E1：§6 C4 改为「`break_up ≠ breaks_upward(break_unit, ZD, ZG)`（按区间位置判，不读 `direction`）」；L0 向上突破的单元是下行单元，反之亦然，读 `direction` 会拒全部合法结算。
2. E2：§6 游标补「Q/JS 由 §3 推出期望游标四元组逐字段比对」，等待位置有进行中单元时必须用 `awaiting_unit_id` 编码（逆向条件写明）。
3. E3：§4「不出 `center_change`」改为「不出 `withdrawn` 类 `center_change`」，并写明该修订照常发 `upgrade_pending`。
4. E4：§6 补「非 UPGRADE 中枢的每个成员恰有一条指向该中枢的 `member_of` 边」。

## v1 → v1.1 差异（根裁定 `ROOT-RULING-1419-DJ2.md`；合同审 `tb1419-review-r2/schema-v1/REVIEW.md`）
1. B-S1：`seed_start_unit_id` 的「存在」包括进行中单元（与 `awaiting_unit_id`、`next_tested_unit` 同口径），连进行中单元也没有才为 null，此时起点 = 等待位置 − 2；位置约定措辞改为「本 cut 的完成单元序为空」「位置 −1」；新增七个「起点为进行中单元」切面的唯一编码表（§3）；§6 补游标必拒条目。
2. B-S2：`status = UPGRADE_PENDING_TB02H` 的中枢修订——`members` 照列；成员归属恒 `NOT_YET_ASSIGNED/UPGRADE_PENDING_TB02H`、`center_id = null`；本修订不发 `member_of` 边（较早修订的边按 AsKnown 保留）；§6 一致性改双向条款；W9 K8/K9 编码表（§3）。
3. C1–C7 写入 §3 对应条目（C7 写入 §8）。
4. 新增 §6 必拒：`break_up ≠ breaks_upward(break_unit, ZD, ZG)`（C4，按区间位置判，不读 `direction`）、`member_of` 边指向 UPGRADE 中枢或非成员。

## v0 → v1 差异（根裁定 `ROOT-RULING-1419-DJ.md`，sha256 830ef88e…）
1. 第 1 点：seed 尝试与中枢分成两个对象、slot 形式——采纳，无改动。
2. 第 2 点：`resume_anchor` 由单元序号文本改为实体锚 `{after_unit_id: <末成员 unit_id>, offset: "2"}`；`next_tested_unit` 改为 `unit_id | null`；`scan_cursor` 全部改用实体锚（`seed_start_unit_id`、`awaiting_unit_id`，或 `awaiting_after_unit_id` + `awaiting_offset`），不再出现任何单元序号。Q/JS 门校验 `after_unit_id == members[-1]`、`offset == "2"`。
3. 第 3 点：原因名采纳。`NOT_ALTERNATING` 留在枚举里；S 写者在 L0 遇到它按输入合同整 cut 失败、不发布带该原因的尝试；Q/JS 对出现 `NOT_ALTERNATING` 的 `s-center/1` 对象一律拒。
4. 第 4 点：`deferred_scope` 名单改为带实票号（TB-02-H = #1429 已有票，无需 `tb02h_unticketed`）；三轴 `not_implemented` 采纳。
5. 第 5 点：整 cut 失败采纳；新增失败收据格式（逐位不符位置）与注入测试义务（§1）。
6. §6「+1 复用」拒因改用 unit_id 表达。


> 工作草稿，绑定 #1419。作者工位 /root/tb1394_support_r4_review，2026-10-14。依据：PLAN §2.5/§2.6、ROOT-RULINGS-1419（D1–D11、D-G、D-I）、
> ROOT-RULINGS-1419-r2 第 6 条（R1–R6）、S0 oracle `worked-examples.json`（schema tb1419-s0-oracle/1）、TB-02-D 先例 `tb1414-schema.md`。
> 内核已在分支：`center.rs` seed_facts/CenterRoles（639c063769）、`center_scan.rs` 扫描事实与归属（bd1d3aa0ad）。本稿只定 wire 形状与 Q 门。

## 0 版本元组
- 新 tuple：`schema_revision = s-center/1`，`semantic_version = center-seed/1`，`policy_version = strict_seed_zn_outer_c_resume`，`view_role = main`。
- `RULE_REVISION = s2-axis-quantifiers+new-bi/1+segment-second/1+center-seed/1`。旧四条规则修订与 `s-segment/1`、`s-segment/2` 原样严格读取；旧规则 cut 不要求 CC-016/017/022。
- 所有 wire 整数仍为十进制文本；对象 id 机制不改（`tb02a_facts.rs:79-88`）。外层 payload 键与段族同形：`schema_revision, semantic_version, policy_version, view_role, object_generation, slot, data`。

## 1 输入合同（S 写者，fail-closed）
- 完成单元 = 同 cut `CC-013.segment` 且 `result ∈ {CASE_ONE, CASE_TWO}`，按段起笔序；区间 = `sort(stroke_refs[0].start_price, geometric_end.price)`，方向 = `construction.direction`。不用 stroke_refs 求段高低（PREP §3）。
- 完成单元必须首尾相连（前段末笔 + 1 = 后段起笔）且与同次 parser `layer.segments` 经 `segment_to_unit` 的结果逐位相等（区间、方向、个数）；任一不符 ⟹ 本 cut 提交失败，不发布任何 TB-02-E 对象。
- 至多一个尾随的未完成段（起笔 = 末个完成段末笔 + 1，`result ∉ {CASE_ONE, CASE_TWO}`）作进行中单元，记 `UNIT_NOT_COMPLETED`；完成单元之间夹非完成段 ⟹ 同上整 cut 失败。
- 扫描中任何 seed 尝试的 `exclusion_reason = NOT_ALTERNATING` ⟹ 同上整 cut 失败（L0 段恒交替，出现即输入不合同）。
- **失败收据**（D-J 第 5 点）：会话错误写 `TB02E_INPUT_CONTRACT`，并逐条列出不符位置，每条 `{check, position, expected, actual}`：`check ∈ {count, lo, hi, direction, contiguity, interior_incomplete, not_alternating}`，`position` = 完成单元序号与段实体 id（收据是诊断，不是 wire 对象，可用序号）。
- **注入测试义务**：注入一处区间/方向/个数不符，断言该 cut 提交被拒、零 TB-02-E 对象发布、前一 cut 的 TB-02-E 对象逐字节不变。
- 扫描 = `scan_center_facts(units, center_from_segments, 0, FactSink)`，与生产塔 L0 同一函数、同一单元列表（#804 L3 锁）。

## 2 Kinds（7 个，对应 TB-02-D 段族的 7 个）
| kind | slot | 何时发布 |
|---|---|---|
| `CC-016.center_seed_attempt` | `seed_attempt:<gen>:<u0 段锚>` | 扫描每次评估的三元组（含失败） |
| `CC-016.center` | `center:<gen>:<seed 首段锚>` | seed 成立即发布（不发 candidate，A-ST-016-02） |
| `CC-017.center_roles` | `roles:<center entity>` | 随中枢发布，成员变化即新修订 |
| `CC-022.unit_assignment` | `assign:<unit entity>` | 每个完成单元 + 至多一个进行中单元 |
| `CC-055.center_relation` | `[source, kind, target]` | `member_of`、`connects_left`、`connects_right`、`attempted_from` |
| `CC-055.center_change` | `[gen, entity, change]` | `formed / member_added / roles_updated / settled / upgrade_pending / withdrawn` |
| `CC-056.center_version` | `"TB-02-E"` | 每 cut 一个 |

单元以段实体 id 指称（`segment:<gen>:<anchor>`），并带同 cut 的 `CC-013` object_id；跨 cut 身份只认实体 id。

## 3 data 键（严格 keys，多一少一皆拒）
**公用**：`known_at = {generation, input_frontier, receipt_id, received_at, semantic_commit_ns}`（同段族）。`Unit = {unit_id, unit_object_id, direction UP/DOWN, lo, hi}`。`Extremum = {value, contributors[unit_id…升序], same_price_witness[…]}`，`same_price_witness` = contributors（≥2 时）否则 `[]`。

**center_seed_attempt**：`entity_id, units[3]:Unit, vector{completed, consecutive, same_level, alternating, strict: bool|null}, ZD:Extremum, ZG:Extremum, core_order ∈ {ZD<ZG, ZD=ZG, ZD>ZG}, result ∈ {FORMED, EXCLUDED}, exclusion_reason ∈ {null, NOT_ALTERNATING, TANGENT_ZD_EQ_ZG, INVERTED_ZD_GT_ZG}, center_id: null|string, known_at`。
约束：`strict` 为 null 当且仅当 `completed=false`（R5）；`strict ⟺ core_order=="ZD<ZG"`；`FORMED ⟺ 五位全真 ⟺ exclusion_reason=null ⟺ center_id≠null`；`EXCLUDED` 时原因与 `alternating`/`core_order` 一致（先交替后核心序）。
（C3）本片发布的尝试恒 `completed = consecutive = same_level = alternating = true`、`strict ≠ null`：三单元由输入合同保证完成、相连、同级（内核 `SeedFacts` 只带交替与核心序，另三位由 S 写者按构造置真），`NOT_ALTERNATING` 整 cut 失败不发布；故 `EXCLUDED` 的原因只会是 `TANGENT_ZD_EQ_ZG`（`ZD=ZG`）或 `INVERTED_ZD_GT_ZG`（`ZD>ZG`）。

**center**：`entity_id, entity_revision, seed_attempt_id, seed_units[3]:unit_id, zn_direction, members[unit_id…], zn_members[unit_id…], status ∈ {EXTENDING, SETTLED, UPGRADE_PENDING_TB02H}, settled bool, break_unit: null|unit_id, break_up bool, break_up_defined bool, resume_anchor: null|{after_unit_id, offset}, next_tested_unit: null|unit_id, roles_id, formed_known_at, settled_known_at: null|known_at`。
约束：`members` 连续、以 seed 三单元开头、全是完成单元；`settled ⟺ break_unit≠null ⟺ break_up_defined ⟺ resume_anchor≠null ⟺ settled_known_at≠null`；未结算 `break_up=false`（R4）；`resume_anchor = {after_unit_id: members[-1], offset: "2"}`（R8，D-J 第 2 点）；`next_tested_unit`：开放中枢下一个待测延伸的单元，存在（进行中单元）时为其 unit_id，否则 null，结算后恒 null；`UPGRADE_PENDING_TB02H` ⟺ 成员数 ≥9（R3：对象保留不撤回、不取代；状态与 settled 正交，结算后仍为 `UPGRADE_PENDING_TB02H`）。
（C1）成员 <9 时 `status = EXTENDING ⟺ ¬settled`、`status = SETTLED ⟺ settled`。（C4）`break_unit` = 同 cut 完成单元序中紧接 `members[-1]` 的单元；`break_up` 与内核 `center::breaks_upward` 一致：突破段 `lo > ZG ⟹ true`，`hi < ZD ⟹ false`。（C5）`zn_direction` = `seed_units[0]` 的方向；`zn_members` = `members` 中方向与之相等者（D-A）。Q/JS 复核 `Zn` 只用方向相等，不用位次奇偶（二者在 L0 恒等、在上级分歧，Q 侧不得长出第二判据）。
（B-S2，D-J2 第 2 点）`status = UPGRADE_PENDING_TB02H` 的修订：`members` 照列（原始延伸扫描，成员计数继续）；其成员的 `unit_assignment` 恒为 `NOT_YET_ASSIGNED/UPGRADE_PENDING_TB02H`、`center_id = null`；本修订不发 `member_of` 边（见 §4）。

**center_roles**：`entity_id, entity_revision, center_id, roles{CORE{ZD,ZG}, OUTER{DD,GG}, INNER_EXTREMA{D,G}}`，每个变量 `{value: int|null, contributors, same_price_witness, frozen bool, null_reason: null|UPGRADE_PENDING_TB02H}`；`previous: null|{entity_revision, roles}`，`known_at`。
约束：ZD/ZG `frozen=true` 且贡献只在 seed 三单元内；其余四个 `frozen=false` 且贡献只在 Zn 内；`DD≤ZD<ZG≤GG`；`D>G` 合法；六变量不合并、不以 D/G 替代核心；`UPGRADE_PENDING_TB02H` 时 DD/GG/D/G 的 `value=null` 且 `null_reason` 同名，ZD/ZG 照常（R3）。
（C2）`value = null ⟺ null_reason ≠ null`；null 时 `contributors = same_price_witness = []`；动态四变量同时有值或同时为 null；`DD≤ZD<ZG≤GG` 只对非 null 项判。

**unit_assignment**：`entity_id, unit: Unit|{unit_id, unit_object_id, completed:false}, status ∈ {CENTER_MEMBER, CONNECTOR, NOT_YET_ASSIGNED}, center_id, left_center_id, right_center_id, right_pending bool, reason, seed_from_self ∈ {FORMED, FAILED, NOT_EVALUATED}, known_at`。
三值恰一（指归属状态，不指关系边）：`CENTER_MEMBER` ⟹ 仅 `center_id`；`CONNECTOR` ⟹ `left_center_id` 必填，`right_center_id` 与 `right_pending` 二选一；`NOT_YET_ASSIGNED` ⟹ `reason ∈ {NO_LEFT_CENTER_IN_WINDOW, SEED_NOT_YET_EVALUABLE, UPGRADE_PENDING_TB02H, UNIT_NOT_COMPLETED}`。其余字段为 null/false。规则见 D9、D-G、D-I、R2、R3、R6（内核 `center_scan.rs` 模块文档）。`UNIT_NOT_COMPLETED` ⟺ 单元为进行中单元；（C6）进行中单元 `seed_from_self = NOT_EVALUATED`。

**center_version**：`input_frontier, known_at, scan_cursor, waiting_reasons, delivered_scope = "l0_strict_seed_zn_roles_c_resume_assignment", deferred_scope = ["global_G009_upper_levels_1429", "ge9_upgrade_1429", "third_class_and_break_1434", "expansion_newborn_1424", "connector_upper_construction_1429"]`。

`scan_cursor = {state, center_id, seed_start_unit_id, awaiting_unit_id, awaiting_after_unit_id, awaiting_offset}`（D-J 第 2 点，全部实体锚）。

位置约定：同 cut 完成单元按起笔序编位置 0..n−1，进行中单元（若有）位置为 n。「某位置上有单元」指该位置有完成单元或进行中单元。`after_unit_id` 恒为本 cut 末个完成单元（位置 n−1）；本 cut 的完成单元序为空时 `after_unit_id = null`、其位置记 −1。`offset` 为十进制文本、≥1，表示的位置 = after 位置 + offset。
- `EXTENDING`（末中枢开放）：`center_id` = 末中枢、`seed_start_unit_id = null`；等待位置 = 末成员位置 + 1（= n）：该位置有单元（进行中）⟹ `awaiting_unit_id` = 其 id，否则 `awaiting_after_unit_id = members[-1]`、`awaiting_offset = "1"`。与该 center 的 `next_tested_unit` 一致。
- `SEEKING_SEED`：`center_id = null`；seed 起点位置 s = 末个已结算中枢的续扫锚位置（无中枢时为 0）再按其后每个 EXCLUDED 尝试 +1；起点位置上有单元（完成或进行中）⟹ `seed_start_unit_id` 填其 id，连进行中单元也没有 ⟹ null，此时 s = 等待位置 − 2（D-J2 第 1 点）。等待位置 = s + 2（seed 第三单元）：该位置有单元（进行中）⟹ `awaiting_unit_id`，否则 `awaiting_after_unit_id` + `awaiting_offset`。
- `awaiting_unit_id` 与 `awaiting_offset` 恰一非 null；`awaiting_unit_id` 非 null 时必须是本 cut 的进行中单元，且 `awaiting_after_unit_id = null`。

七个「起点为进行中单元」切面的唯一编码（S0 oracle 切面，`uk` 指位置 k 的单元；D-J2 第 1 点）：

| 流/切面 | 完成单元 | 进行中 | `seed_start_unit_id` | `awaiting_unit_id` | `awaiting_after_unit_id` | `awaiting_offset` |
|---|---|---|---|---|---|---|
| UP K8、DOWN K8 | u0..u8 | u9 | u9 | null | u8 | "3" |
| E4 K4、E4_DOWN K4 | u0..u4 | u5 | u5 | null | u4 | "3" |
| E4 K10、E4_DOWN K10 | u0..u10 | u11 | u11 | null | u10 | "3" |
| LEAD K5 | u0..u5 | u6 | u6 | null | u5 | "3" |

W9 K8/K9（D-J2 第 2 点）：中枢 A `status = UPGRADE_PENDING_TB02H`，`members` = u0..u8（K8）／u0..u9（K9），`settled = false`，`next_tested_unit` = u9／u10；这些成员的归属全为 `NOT_YET_ASSIGNED/UPGRADE_PENDING_TB02H`、`center_id = null`；本修订无 `member_of` 边；游标 `EXTENDING`、`center_id` = A、`awaiting_unit_id` = u9／u10；角色 ZD/ZG = 20/35（照常），DD/GG/D/G 为 null 带 `UPGRADE_PENDING_TB02H`。

## 4 关系
`member_of(unit → center)`：非 `UPGRADE_PENDING_TB02H` 中枢的每个成员一条；`UPGRADE_PENDING_TB02H` 修订不发（D-J2 第 2 点）。边随 `center.entity_revision` 版本化：较早修订已发的 `member_of` 边按 AsKnown 保留——查询较早 cut 照旧可见，不发撤回声明（不出 `withdrawn` 类 `center_change`；本修订照常发 `upgrade_pending` 类 `center_change`）；它们在本 cut 当前态里不再出现，是 cut 级当前态的常规生命周期，不改写历史。`connects_left(unit → A)`、`connects_right(unit → B)`（B pending 时不发 right 边）；`attempted_from(seed_attempt → u0)`。`version = center.entity_revision`，`witness_object_id = 对应 center/assignment 对象`。

## 5 目录轴
`catalog_axes` 增 CC-016、CC-017、CC-022：`scope = TB-02-E`，三轴 `impl_status = not_implemented`（有限交付，`tb02a_facts.rs:493` 纪律），`delivered_subobligations = ["l0_strict_seed_vector_and_core_order", "zn_outer_inner_roles_with_contributors", "c_resume_assignment_and_connectors"]`，`deferred_subobligations` = §3 版本对象的 `deferred_scope`。`object_ids`：CC-016 = seed_attempt + center，CC-017 = center_roles，CC-022 = unit_assignment。

## 6 Q / JS 门（`validate_center_payload` + `validate_center_links`，Python 与 JS 逐条同结论同原因码）
必拒（每条以稳定原因码 `TB02E_<CODE>` 拒，Python 与 JS 对共享语料逐条对拍）：连接段作成员；B 的首成员是 A 末成员在同 cut 完成单元序中的下一个单元（+1 复用），或 B 首成员早于 A 的续扫锚位置；A 与 B 之间的单元不全是 `CONNECTOR(A→B)`，或续扫锚到 B 首成员之间的单元没有对应 EXCLUDED 尝试；`resume_anchor.after_unit_id ≠ members[-1]` 或 `offset ≠ "2"`；任何 `exclusion_reason = NOT_ALTERNATING` 的对象；D/G 冒充核心或角色缺一；贡献成员缺失或不在允许集；`candidate` 类 kind；成员不是同 cut 完成段或区间/方向与该 CC-013 不符；相切/倒置缺原因；`s-center/1` 对象出现在旧规则 cut 或混用旧 tuple；单元归属不恰一（归属状态，不指边）；未结算中枢 `break_up_defined=true`；`break_up ≠ breaks_upward(break_unit, ZD, ZG)`（C4，按区间位置判，不读 `direction`）。
一致性双向条款（D-J2 第 2 点）：对每个中枢 C、每个 u ∈ C.members，C 非 UPGRADE ⟹ u 的归属为 `CENTER_MEMBER(C)`；C 为 UPGRADE ⟹ u 的归属为 `NOT_YET_ASSIGNED/UPGRADE_PENDING_TB02H`。反向：任一 `CENTER_MEMBER(C)` 的单元必属 C.members；任一 `UPGRADE_PENDING_TB02H` 归属的单元必属某个 UPGRADE 中枢的 members。`member_of` 边只能从非 UPGRADE 中枢的成员指向该中枢；非 UPGRADE 中枢的每个成员恰有一条指向该中枢的 `member_of` 边（E4）。
游标（D-J2 第 1 点）：`state = EXTENDING ⟺ 末中枢开放`，且 `center_id` = 末中枢、`seed_start_unit_id = null`；`awaiting_unit_id` 与 `awaiting_offset` 恰一非 null；`awaiting_after_unit_id` = 本 cut 末个完成单元（无完成单元时 null）；`offset` = 等待位置 − after 位置（after 为 null 时以 −1 计）；EXTENDING 时等待位置 = `members[-1]` 位置 + 1 且与该 center 的 `next_tested_unit` 一致；SEEKING_SEED 时等待位置 = seed 起点位置 + 2，seed 起点 = 末个已结算中枢的续扫锚与其后每个 EXCLUDED 尝试 +1 推出的位置（无中枢、无尝试时为 0），`seed_start_unit_id` 与该位置上的单元一致（无单元时 null）；`awaiting_unit_id` 非 null 时必须是本 cut 的进行中单元。 Q/JS 由 §3 推出期望的游标四元组（`seed_start_unit_id`, `awaiting_unit_id`, `awaiting_after_unit_id`, `awaiting_offset`）逐字段比对；等待位置上有进行中单元 ⟺ 必须以 `awaiting_unit_id` 编码（E2）。

## 7 待拍板点的去向
v0 五点由根裁定 D-J 定案；v1 合同审两阻断（B-S1、B-S2）与 C1–C7 由 D-J2 定案（见文首）。复核范围：B-S1/B-S2 文字、§6 新条目、§3 七个切面与 W9 编码。

## 8 与 S0 oracle 的映射（C7）
oracle 切面的 `changes_from_previous` 含 `UNIT_COMPLETED`、`ASSIGNMENT_CHANGED`；wire 上 `center_change` 只覆盖中枢实体的六种变化（formed、member_added、roles_updated、settled、upgrade_pending、withdrawn）。单元完成表现为同 cut 新的 `CC-013.segment`（result 变为 `CASE_ONE/CASE_TWO`），归属变化表现为 `unit_assignment` 对象的新内容与新 `known_at`。运行验收对拍 oracle 时按此映射。oracle W9 早于 R3、把整组 `roles` 置 null；wire 按 R3 发 ZD/ZG、只把动态四变量置 null，对拍时按本 schema。
