# #1424 TB-02-F schema v1（2026-10-14，s-center-pair/1）

> 工作草稿，绑定 #1424。作者工位 /root/tb1424_prep_r2。依据：根裁定 #1424 issuecomment-6160553615（六问全采纳）、prep
> `tb1424-prep-r2/CHANGE-POINTS.md` §D 草案、S1 内核（d001765909：`center::{center_pair_facts, unit_core_position}`、
> `center_scan::{pair_facts, tested_positions}`）、#1419 先例 `issue1419-tb02e-20261013/tb1419-schema.md` v1.3。
> 本稿只定 wire 形状与 Q 门；语义正本在 `.chanlun/definitions/zhongshu.md`。

## 草案 → v1 差异（实施中定案，均不改语义）
1. 变化种类增 `inputs_revised`：输入端点或连接段变了而叶、状态都没变（W12 K12：B 加成员）。草案只有 `formed / leaf_revised / pending_upgrade / withdrawn`，会让这类修订无名可发。
2. `pair_version` 增 `pair_count`、`pending_pairs`、`waiting_reasons`（同 #1419 `center_version` 的等待原因口径），供 Q 核计数。
3. `deferred_scope` 末项定为 `two_step_settlement_r5_1496`（根裁定第 6 问开 #1496）。
4. Q 门先绑 `inputs` 再判待定（§6）：待定与否由输入外缘是否为 null 推出，输入错只报 `PAIR_INPUTS`，不让派生判据替源头判据兜底。

## 0 版本元组
- 新 tuple：`schema_revision = s-center-pair/1`，`semantic_version = center-pair/1`，`policy_version = zn_outer_five_leaf_pending_ge9`，`view_role = main`。
- `RULE_REVISION = s2-axis-quantifiers+new-bi/1+segment-second/1+center-seed/1+center-pair/1`。`s-center/1` 七个 kind 在新规则下逐字节不变（根裁定第 5 问）；旧规则 cut 不要求 CC-018/020，Q 读旧规则 cut 照旧。
- wire 整数为十进制文本；外层 payload 键同段族与中枢族：`schema_revision, semantic_version, policy_version, view_role, object_generation, slot, data`。

## 1 输入（S 写者）
TB-02-F 不另扫描。`tb02e::build` 返回 `CenterBuild`（单元、扫描事实、窗口、中枢 id 与修订、获知时点），`tb02f::build` 吃同一次扫描：对走 `center_scan::pair_facts` → `center::center_pair_facts`，位置走 `center_scan::tested_positions` → `center::unit_core_position`。塔的 `classify_relation` 是 `center_pair_facts(..).leaf.relation()` 的投影，延伸循环读 `unit_core_position(..).closed_overlap()`：同一判定只有一份（#804）。输入不合同时整 cut 失败沿用 #1419 §1，本片不新增失败面。

## 2 Kinds（5 个）
| kind | slot | 何时发布 |
|---|---|---|
| `CC-018.unit_core_position` | `ucp:<中枢锚>:<单元锚>` | 每个中枢的延伸测试单元（seed 之后的成员 + 突破段），≥9 窗口照发 |
| `CC-020.center_pair` | `pair:<A 锚>:<B 锚>` | 本 cut 每对扫描相邻中枢一个，含待定对 |
| `CC-055.pair_relation` | `[source, kind, target]` | `pair_from(pair→A)`、`pair_to(pair→B)`、`position_of(ucp→unit)`、`position_against(ucp→center)` |
| `CC-055.pair_change` | `[gen, entity, change]` | `formed / leaf_revised / pending_upgrade / inputs_revised / withdrawn` |
| `CC-056.pair_version` | `"TB-02-F"` | 每 cut 一个（尚无中枢时照发，计数为 0） |

锚 = 实体 id 去掉前缀后的 `<代>:<锚>`：W1 的对是 `pair:1:10:1:46`，A 的首个延伸成员位置是 `ucp:1:10:1:42`。

## 3 data 键（严格 keys，多一少一皆拒）
**unit_core_position**：`entity_id, center_id, center_revision, unit{unit_id, unit_object_id, direction, lo, hi}, core{ZD, ZG}, position ∈ {STRICT_BELOW, LOW_TOUCH, INTERIOR_CONTACT, HIGH_TOUCH, STRICT_ABOVE}, closed_overlap, strict_leave, leave_side ∈ {UP, DOWN} | null, construction_role ∈ {EXTENSION_MEMBER, BREAK_UNIT, UPGRADE_PENDING_MEMBER}, destruction = "NOT_EVALUATED_DEFERRED_1434", known_at`。
约束：`core` 是该中枢的冻结核心（seed 三单元）；`closed_overlap ⟺ position ∈ 中间三叶 ⟺ ¬strict_leave`；`leave_side`：BELOW→DOWN、ABOVE→UP，按区间位置判，不读单元 `direction`（同 #1419 C4）；`BREAK_UNIT ⟺ strict_leave`，且该单元是该中枢的 `break_unit`；≥9 窗口的 seed 后成员记 `UPGRADE_PENDING_MEMBER`；离开不是破坏，`destruction` 恒为固定字面量。事实不变时整对象沿用前 cut（`center_revision`、`known_at` 不前移）。

**center_pair**：`entity_id, entity_revision, a_center_id, a_revision, b_center_id, b_revision, connector_units[unit_id…], status ∈ {EVALUATED, PENDING_UPGRADE_TB02H}, leaf, direction, provisional, inputs{A{ZD,ZG,DD,GG}, B{ZD,ZG,DD,GG}}, matrix, witnesses, realization, null_reason, previous, formed_known_at, known_at`。
- `inputs` 逐值等于同 cut `CC-017` 角色：核心 = 冻结 ZD/ZG，外缘 = Zn 外缘 DD/GG（R26，不取全单元外缘）。≥9 窗口那端的 DD/GG 为 null。
- `EVALUATED`：两端外缘齐全、`null_reason = null`；`leaf` 是五叶逐叶独立求值的唯一真值（`CORE_NOT_STRICTLY_SEPARATED / UP_NEWBIRTH / DOWN_NEWBIRTH / UP_EXPANSION / DOWN_EXPANSION`）；`direction`：新生、扩展取 UP/DOWN，首叶为 null；`matrix` 16 格，键 `"B.X?A.Y"`（X、Y ∈ DD/ZD/ZG/GG），值 `<`/`=`/`>`；`witnesses = {core_tangent, outer_touch, outer_containment, core_containment}`，包含取 `NONE / A_CONTAINS_B / B_CONTAINS_A / EQUAL`。
- `PENDING_UPGRADE_TB02H`（根裁定第 2 问）：A 或 B 为 ≥9 窗口 ⟺ 对待定；`leaf = direction = matrix = witnesses = null`，`null_reason = PENDING_UPGRADE_TB02H`。不照抄塔对 ≥9 窗口过渡态子中枢算出的关系：子中枢可越出 `DD≤ZD<ZG≤GG` 域（S1 发现：W9 子窗 u3..u5 有 DD=22>ZD=20，u9..u11 有 GG=33<ZG=35）。
- `provisional ⟺ B 未结算`；`connector_units` = A 末成员与 B 首成员之间的全部完成单元（至少有突破段）。
- `realization = {upper_center: "NOT_CONSTRUCTED_DEFERRED_1429", trend: "NOT_ASSERTED_DEFERRED_1426"}`：扩展、新生只是关系，本片不构造上级对象、不断言趋势已实现；出现 `level_lift` 一类键即拒。
- 修订：事实（除 `entity_revision, previous, formed_known_at, known_at` 外的全部键）不变则整对象沿用；变了则 `entity_revision + 1`、`previous = {entity_revision, status, leaf}`（首修订为 null）、`formed_known_at` 保持首次形成时点、`known_at` 取本 cut。

**pair_relation**：`source_id, relation_kind, target_id, version, witness_object_id`；`version` = 对的 `entity_revision` 或位置的 `center_revision`。

**pair_change**：`entity_id, change, before, after, known_at, object_id`。`formed`：`before = null`；`withdrawn`：`after = object_id = null`（前 cut 有、本 cut 无的对）；其余三种前后件都在。变化种类判序：首次 → `formed`；状态翻成待定 → `pending_upgrade`；叶变 → `leaf_revised`；否则 `inputs_revised`。`after` 必须逐值等于本 cut 当前对。

**pair_version**：`input_frontier, known_at, pair_count, pending_pairs, waiting_reasons, delivered_scope = "l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position", deferred_scope = ["upper_level_pairs_1429", "ge9_pairs_pending_1429", "upper_center_realization_1429", "trend_realization_1426", "first_retest_destruction_1434", "two_step_settlement_r5_1496"]`；`waiting_reasons = ["PENDING_UPGRADE_TB02H"]` 当且仅当 `pending_pairs > 0`。

## 4 关系
每个对一条 `pair_from`、一条 `pair_to`；每个位置一条 `position_of`、一条 `position_against`；`witness_object_id` = 发出该边的对象。边集合不多不少，版本号随对象修订。

## 5 目录轴
`catalog_axes` 增 CC-018、CC-020：`scope = TB-02-F`，`impl_status = not_implemented`（有限交付），`delivered_subobligations = ["l0_unit_core_position_five_leaves", "l0_adjacent_pair_five_leaves_zn_outer_with_full_matrix", "expansion_relation_separate_from_realization"]`，`deferred_subobligations` = §3 的 `deferred_scope`，等待原因取自 `pair_version`。`object_ids`：CC-018 = 全部 `unit_core_position`，CC-020 = 全部 `center_pair`。

## 6 Q / JS 门（`validate_pair_payload` + `validate_pair_rule` + `validate_pair_links`；整 cut 门 `validate_pair_cut` 先过 #1419 中枢整 cut 门）
Q 不信写者：五叶、矩阵、证据、位置都按定义重算，`inputs` 与同 cut CC-017 角色比对（角色值已由中枢门按 Zn 成员重算）。每条拒绝以 `TB02F_<CODE>:` 开头，Python 与 JS 对共享语料逐条比完整拒因（原因码与拒因文字同文）。

| 原因码 | 拒什么 |
|---|---|
| `SHAPE` / `VERSION` / `IDENTITY` / `SLOT` | 键缺多、tuple 不符、身份未绑定两中枢或中枢与单元、语义槽不符；`level_lift` 一类多余键 |
| `DOMAIN` | 输入越出 `DD≤ZD<ZG≤GG`，或五叶不恰一 |
| `LEAF` / `DIRECTION` / `MATRIX` / `WITNESS` | 叶、方向、16 格矩阵、相切/触及/包含证据与输入重算不符 |
| `PENDING` | 待定条件与 `status`、null 字段不一致；可判对缺外缘；有外缘却记待定 |
| `PAIR_INPUTS` | `inputs` 不等于同 cut 中枢角色（含照抄塔子中枢关系、改用全单元外缘） |
| `REALIZATION` / `DESTRUCTION` | 断言上级已构造、趋势已实现；把离开写成破坏 |
| `PROVISIONAL` / `CONNECTORS` / `REVISION` | 暂定与 B 结算不符；连接段不是两中枢之间全部完成单元；修订号、`previous` 链与中枢修订不符 |
| `PAIR_COUNT` / `ADJACENCY` | n 个中枢不是恰 n−1 个对；两端不是扫描相邻中枢 |
| `UCP_COUNT` / `UNIT_SOURCE` / `CORE` / `POSITION` / `ROLE` | 位置对象不恰覆盖延伸测试单元；单元与同 cut CC-013 不符；核心与角色不符；位置五叶、重叠/离开位不符；构造角色不符 |
| `EDGE` / `CHANGE` / `VERSION_COUNT` / `SCOPE` | 关系边缺多或版本不符；变化后件不是当前对、撤回的对仍在；版本对象个数与计数不符；交付范围不符 |
| `RULE_MIX` / `CANDIDATE_KIND` / `AXIS_BINDING` | 对族对象出现在旧规则 cut 或混用旧 tuple；未声明的 CC-018/CC-020 kind；目录证据未绑定实际对象 |

检查次序：candidate kind → 单对象 → 规则 → 跨对象（位置 → 对计数与相邻 → 逐对修订、连接段、输入、待定、暂定 → 边 → 变化 → 版本 → 目录轴）。

## 7 验证锚
- 共享语料 `s_session/tests/fixtures/tb02f-contract/corpus.json`（`generate-corpus.py` 按定义直算，复用 #1419 生成器的 `cut()`）：11 个合法 cut 覆盖五叶与待定，33 个反例覆盖 §6 全部原因码；`test_tb02f_contract.py` 逐条对拍 Python 与 JS。
- S→Q：`tb02f_writer_output_passes_python_pair_contract_on_all_prefix_cuts` 让写者在 11 条工作例流的每个前缀切面（逐 cut 以前 cut 为 previous）上的输出都过 Python `validate_pair_cut`。
- 内核与 oracle：`rust/tests/center_pair_1424.rs` 对 `oracle_pairs.py` 生成的 W0–W12 工作例逐例对拍。
