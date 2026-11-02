# #1790 考古（手动）

> **命令正本 = `archaeology.json` 各 query 的 `command` 字段。** 本 md 表格里的 `\|` 是 Markdown 表格转义，复跑时请改成 `|`，或者直接取 json 里的命令。主控 2026-10-28 用脚本复跑了 json 里全部 17 条 git 命令，命中数逐条与 `hits` 一致。


[grilling] 订正 ADR 0054：中段出生不造成两序分叉（身份含前缀链），验收命题改为“出生构成窗口序后缀”

基线：main `df0ec357ab`，PR #1780 head `a485194947`。命令均在仓库根实跑（2026-10-28），命中数 = 输出行数；`git grep <commit>` 不依赖检出状态，可原样复跑。claimer 自动考古（`decisions/1790/archaeology.json`，parsed=false）作废，由本件取代。

归类：A 已裁 8 · B 0 · C 0。A 类直接沿用；无默认值待确认，无待决点，无缺口票。

## Q1 [A] 分型 object_id（CC-006 obj3 与 CC-007 泛型身份）是否绑定从首根 raw 起的完整来源前缀链，且链上任一坐标新增或修订都会换 id？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | group_sources_coords | `gh issue list --repo xy7365527-lang/NewChanlun --state all --search 'group_sources_coords' --json number -q '.[].number'` | 1（仅 #1790 自身） |
| repo | group_sources_coords（cp） | `git grep -n 'group_sources_coords' a485194947 -- rust/src/bin/s_session_v2/content_producer.rs` | 12 |
| repo | RawRefSpec/Leaves/bind_source | `git grep -n -E 'struct RawRefSpec\|fn to_shape_value\|enum Leaves\|ShapeSourceRef\(\|pub\(super\) fn bind_source\|self.shape_ref_arrays.invalidate' a485194947 -- rust/src/bin/s_session_v2/content_producer.rs` | 10 |
| repo | 身份配方与 bounded 分支 | `git grep -n -E 'fn shape_identity_scoped\|\("input_refs", V::Edge\|fn typed_identity_scoped\|"shape_object_id".to_string\|let all_sources = if bounded\|let units_array = if bounded\|let session_arrays = if bounded' a485194947 -- rust/src/bin/s_session_v2/content_producer.rs` | 8 |
| repo | inclusion seed/establish/new_group/merge 证据链 | `git grep -n -E 'action: "seed"\|首次 establish_direction\|let base = match &previous_coords\|merge：改末组\|let tail = previous_tail.append_greater\|违反即降级\|self.unsettled = true' a485194947 -- rust/src/theta_v0/parser/inclusion.rs` | 8 |
| repo | 逐前缀对拍锁 | `git grep -n -E 'fn derived_coords_match_materialized_union_for_every_prefix\|sources_i ⊆' a485194947 -- rust/src/bin/s_session_v2/content_producer.rs` | 2 |

命中：
- `rust/src/bin/s_session_v2/content_producer.rs:106-214`：RawRefSpec 八字段（identity_key/payload_hash/receipt_id/event_id/input_revision/revision/seq/source_coord）；to_value / to_shape_value 两种叶编码
- `rust/src/bin/s_session_v2/content_producer.rs:404-444`：Leaves::SourceRef / ShapeSourceRef：源引用数组的叶 = 坐标当前登记版本的八字段值根
- `rust/src/bin/s_session_v2/content_producer.rs:800-842`：bind_source：坐标版本变化即换叶根并 invalidate 该坐标（ref_arrays 与 shape_ref_arrays）
- `rust/src/bin/s_session_v2/content_producer.rs:1249-1272`：CC-006 十二字段身份含 ("input_refs", V::Edge(part.input_refs.digest))
- `rust/src/bin/s_session_v2/content_producer.rs:2455-2490`：input_refs = 三组 raw_refs(成员) + dependency_refs；sources = group_sources_coords(right)
- `rust/src/bin/s_session_v2/content_producer.rs:3405-3429`：group_sources_coords：conf→conf.coords；dir→dir.coords ++ member_tail；皆无→members
- `rust/src/theta_v0/parser/inclusion.rs:1056-1066`：共享拼接前提：坐标严格递增，违反即降级到全量入口
- `rust/src/theta_v0/parser/inclusion.rs:1084-1120`：seed：首根 raw 原样成第 0 组，members=[s0]，tail 空
- `rust/src/theta_v0/parser/inclusion.rs:1122-1143`：未建方向前遇包含即 unsettled 永久未定（不出分型）
- `rust/src/theta_v0/parser/inclusion.rs:1170-1188`：new_group/establish：base = 旧证据 ++ 前组尾（首次取 锚 ++ 组尾），再 append incoming
- `rust/src/theta_v0/parser/inclusion.rs:1247-1322`：merge：不换证据指向，组尾 append_greater(incoming)
- `rust/src/bin/s_session_v2/content_producer.rs:2117-2130,2150-2156,2217-2241,2254-2266`：bounded：分型载荷不含 subsequent_development，来源 = shape.sources，不走共享整段数组
- `rust/src/bin/s_session_v2/content_producer.rs:6582`：derived_coords_match_materialized_union_for_every_prefix：三连窗并集 == 右组 sources

- 出处：代码事实，PR #1780 head a485194947
- 推理：seed 使链从首根 raw 起；establish 取首组成员链 ++ incoming；new_group 取旧证据 ++ 前组尾 ++ incoming；merge 只扩组尾。归纳：在非退化域（坐标严格递增、初始方向已定）内，dir.coords = 首根到本组锚的全部坐标，conf.coords = 首根到封口边界的全部坐标，dir ++ tail = 首根到末组末成员。叶为含 revision/seq 的八字段，bind_source 版本变化即换叶。故链上任一坐标新增或修订都换 CC-006 id，CC-007 经 shape_object_id 与 input_refs 继承。
- 处置：沿用：身份含完整来源前缀链

## Q2 [A] ADR 0054 裁定 4/验收 2 ① 所需的「旧分型 Y 窗口晚于新出生 X」是否可达？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| repo | 中段出生锁断言 | `git grep -n -E 'fn bounded_layout_cc007_entries_window_order_on_mid_birth\|不得晚于新出生分型\|须在中段出生新分型\|须有一根 bar 的 entries 同含新旧分型' a485194947 -- rust/src/bin/s_session_v3/generation.rs` | 4 |
| ci | run 40663443503 | `gh run view 40663443503 --repo xy7365527-lang/NewChanlun --json headSha,conclusion -q '.headSha+" "+.conclusion'` | 1 |
| repo | ADR 0054 分叉条件 | `git grep -n -E '出生序使载荷依赖发布史\|分叉条件\|中段出生时出生序少约' df0ec357ab -- docs/adr/0054-development-batch-entries-window-order.md` | 4 |

命中：
- `rust/src/bin/s_session_v3/generation.rs:5592-5680`：三例 insert@85 / revise b9 / revise b8；:5649 中段出生、:5656 旧分型不得晚于新出生、:5672 entries 严格递增、:5679 同含新旧
- `CI run 40663443503`：a4851949472a592fca308575dcaca573886622d4 success
- `docs/adr/0054-development-batch-entries-window-order.md:25`：4. 分叉条件：…中段出生或非尾分型重生的代上两序不同
- `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1780-INDEPENDENT-REVIEW-A485.md:29,41,47-49`：推理成立；负控做不出来；分叉不发生

- 出处：draft.md §2.4 证明 + CI 实证
- 推理：由 Q1：保留旧 id ⟹ e(Y)<c；新出生 ⟹ e(X)≥c；链随窗口嵌套 ⟹ Y.window[0]<X.window[0]。不可达。
- 处置：订正裁定 4、两分叉例与验收 2 ①
- 冲突：ADR 0054:15-18,25,65 与代码事实冲突（本票订正）

## Q3 [A] 否决出生序的理由在两序恒等后是否仍成立？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| repo | 生成层首获知代恒写本代 | `git grep -n -E '"first_known_generation".to_string\(\), number\(generation\)' a485194947 -- rust/src/bin/s_session_v2/content_producer.rs` | 2 |
| repo | object_index 序键与本代校验 | 见下方命令 Q3-repo | 7 |

Q3-repo 命令（表格中 `|` 须转义，故单列；原样可复跑）：
```
git grep -n -E 'fn order_key|Slot::Shape\(w\) => OrderKey::Shape|fn public_rank|first_known_generation")\? != g' a485194947 -- rust/src/bin/s_session_v3/object_index.rs
```


命中：
- `rust/src/bin/s_session_v2/content_producer.rs:1120`：("first_known_generation".to_string(), number(generation)?)
- `rust/src/bin/s_session_v3/object_index.rs:404-410`：Slot::Shape(w) => OrderKey::Shape(window[0])
- `rust/src/bin/s_session_v3/object_index.rs:451-458`：公开主序键 = (首获知代, order_key)
- `rust/src/bin/s_session_v3/object_index.rs:502-505`：生成候选 first_known_generation != g 即拒收
- `docs/adr/0054-development-batch-entries-window-order.md:30`：出生序使载荷依赖发布史

- 出处：ADR 0054:30 + 代码
- 推理：值相同不改变算法依赖：出生序须读 object_index 真实首获知代，生成层不具备；窗口序只读当前 raw。
- 处置：沿用否决，补「否的是算法」一句

## Q4 [A] 订正载体：ADR 0054 修订段还是新 ADR 0056？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| repo | 既有修订/订正段先例 | `git grep -n -E '^## (修订\|订正\|补充)\|^## .*订正' df0ec357ab -- docs/adr` | 9 |
| repo | ADR 最大号与预留 | `git ls-tree --name-only df0ec357ab docs/adr/ \| tail -2; python3 -c "import json;print(max(json.load(open('/Volumes/AgentStorage/decisions/adr-reservations.json')).values()))"` | 3 |

命中：
- `docs/adr/0005-nest-import-ban-clause9-ruling.md:32`：## 修订（#1061…）
- `docs/adr/0022-acceptance-criterion-parameters.md:91`：## 修订（#1677…）
- `docs/adr/0026-scan-unify-defense-lean-mirror.md:52`：## 补充（#1060…）
- `docs/adr/0032-fill-allocation-policy.md:39`：## 修订（#1406…）
- `docs/adr/0015-cost-trajectory-acceptance-metric.md:190`：## 订正（#937…）
- `docs/adr/0055-itemized-old-pool-settlement.md`：main 最大号 0055
- `/Volumes/AgentStorage/decisions/adr-reservations.json`：最大登记 55（#1786）

- 出处：仓内先例
- 推理：同一决策的事实订正一律追加修订段；结论未变，另立 ADR 会让 0054 留着已推翻例子而无指引。
- 处置：ADR 0054 追加修订段；备选号 0056（未登记）

## Q5 [A] ADR 0054 Δ_R 段「两序互有胜负」推导是否随之撤回，上界门结论是否变？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| repo | Δ_R/上界 | `git grep -n -E 'Δ_R\|上界' df0ec357ab -- docs/adr/0054-development-batch-entries-window-order.md` | 3 |

命中：
- `docs/adr/0054-development-batch-entries-window-order.md:31`：中段出生时出生序少约 n−p 个内部节点，原位重生时窗口序少约 n 个…上界门不随两序选择变化
- `docs/adr/0054-development-batch-entries-window-order.md:44`：退场：上界门超界且归因于 entries M 树子树重写

- 出处：ADR 0054:31
- 推理：两序给出同一 entries，节点差额为 0；门结论原本就不依赖差额。
- 处置：撤回差额推导，保留门结论

## Q6 [A] PR #1780 head 上须对齐的锁名与注释有哪些？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| repo | 1783/ADR 0054 指针 | `git grep -n -E '1783\|ADR 0054' a485194947 -- rust/src/bin/s_session_v3/generation.rs rust/src/bin/s_session_v2/content_producer.rs s_session/browser/q3-typed.js s_session/browser/tb01c-client.js` | 7 |
| repo | 旧锁名 | `git grep -n 'diverges_from_birth_order' a485194947 -- rust/src` | 0 |

命中：
- `rust/src/bin/s_session_v3/generation.rs:2315`：…ADR 0054 / #1783…两序仍一致
- `rust/src/bin/s_session_v3/generation.rs:5582`：ADR 0054（#1783 验收 5）中段出生锁
- `rust/src/bin/s_session_v3/generation.rs:5587`：须回 #1783 复核
- `rust/src/bin/s_session_v3/generation.rs:5252`：entries 按分型窗口序（ADR 0054 / #1783）
- `rust/src/bin/s_session_v2/content_producer.rs:1475`：按分型窗口序，ADR 0054 / #1783
- `s_session/browser/q3-typed.js:759`：ADR 0054 / #1783
- `s_session/browser/tb01c-client.js:3362`：ADR 0054 / #1783

- 出处：代码
- 推理：锁名已是后缀命题，保留；5582/5587/2315 改指 #1790；其余指针仍有效。
- 处置：见 draft §5

## Q7 [A] tracker 与推论层是否已有「两序分叉」或中段出生的既有主张？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 出生序 窗口序 | `gh issue list --repo xy7365527-lang/NewChanlun --state all --search '出生序 窗口序' --json number -q '.[].number'` | 30（默认上限 30；直接相关为 #1783（已关）、#1491、#1790） |
| tracker | 中段出生 | `gh issue list --repo xy7365527-lang/NewChanlun --state all --search '中段出生' --json number -q '.[].number'` | 30（默认上限 30；首位 #1790，余为关键词泛命中） |
| tracker | #1783 状态 | `gh issue view 1783 --repo xy7365527-lang/NewChanlun --json state -q .state` | 1 |
| repo | 推论层窗口序/出生序 | `git grep -l -E '窗口序\|出生序' df0ec357ab -- analysis docs .chanlun CONTEXT.md` | 14 |
| repo | 推论层「两序分叉」主张 | `git grep -n -E '(窗口序\|出生序).{0,40}(分叉\|不同\|两序)\|两序' df0ec357ab -- analysis docs .chanlun CONTEXT.md` | 13（ADR 0054 六行、CONTEXT.md:176 一行，其余六行为无关「两序列/两序 fold」） |

命中：
- `CONTEXT.md:176`：entries 按所引分型 window[0]…即窗口序，不按出生序（#1783，ADR 0054；纯追加时两序相同）。
- `#1783`：CLOSED，ADR 0054 来源票
- `#1491`：实施票，PR #1780
- `.chanlun/review-results/issue1323-grilling-20261013/1489-AFFECTED-CODE.md`：出生序累计数组的影响清单（不涉 entries 分叉）

- 出处：tracker + 推论层
- 推理：分叉主张在 ADR 0054 本身；正本层 CONTEXT.md:176 括注「纯追加时两序相同」隐含非纯追加时可不同，随本订正改写；推论层无独立主张。
- 处置：订正 ADR 0054；CONTEXT.md:176 括注改为「两序在可达输入上恒等，#1790」
- 冲突：CONTEXT.md:176 括注措辞与订正后事实不一致

## Q8 [A] Lean 形式化层是否有对应定义需同步？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| repo | formal | `git grep -il -E 'group_sources\|development_batch' df0ec357ab -- formal` | 0 |

命中：
- （无）

- 出处：零命中
- 推理：布局编码层，无形式化对象。
- 处置：Lean 无改动
