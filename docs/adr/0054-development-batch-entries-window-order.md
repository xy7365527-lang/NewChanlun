# ADR 0054：单元批记录 entries 按分型窗口序（取代 ADR 0037 §二 一子句）

> 正本：本文件是 `CC-007.development_batch` 载荷 entries 次序的正本。
> 最后裁定：#1790（2026-10-28，订正 #1783 的分叉前提与验收命题；#1323 Notes N-1 预授权 T 类判定）
> 受影响代码清单：见本文「受影响代码清单」节（`generation.rs` / `content_producer.rs` 注释与分叉锁、`q3-typed.js` / `tb01c-client.js` 注释，实施归 #1491 / PR #1780 owner）

**状态**：已采纳，文书待 main 合入。只取代 [ADR 0037](0037-s-content-bounded-layout.md) §二（`:14`）中「entries 按分型出生序」一个子句；ADR 0037 原文不改，§一 出生序累计数组、§二 其余内容、§四 上界式、§五 布局组件、§八 验收全部不变。
**裁定依据**：#1783 resolution 评论。原件：草稿 `/Volumes/AgentStorage/decisions/1783-manual/draft.md`（头部记 28AC 落地修订），考古 `archaeology.md` / `archaeology.json`（同目录），独审 `review.json`（同目录，verdict PASS）与报告 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1783-INDEPENDENT-REVIEW-28AB.md`（0 CRITICAL / 0 HIGH；MEDIUM-1、LOW-1~4 已在草稿修订，语义不变）。
**行号基线**：main `5123c9e273`；PR #1780 head `e72cf569ba`（下文 `gen` / `cp` / `oi` 分别指该 head 上的 `rust/src/bin/s_session_v3/generation.rs`、`rust/src/bin/s_session_v2/content_producer.rs`、`rust/src/bin/s_session_v3/object_index.rs`）。

## 背景

ADR 0037 §二 写 `CC-007.development_batch` 的 entries「按分型出生序」。PR #1780（#1491）实现为分型窗口序：`gen:805-822` 按合并 bar 窗口起点升序遍历形态窗，`gen:879-881` 依此序收集 `developments`，`gen:2389-2393` 按此序组 entries；JS 读口 `s_session/browser/q3-typed.js:760`、`tb01c-client.js:3363` 核 `window[0]` 严格递增。独审 M-2（`1780-INDEPENDENT-REVIEW-7CF8.md:26`）要求落正本，实施方停下待裁（`issue1491-segb-28p/IMPL-REPORT.md:73-78`）。

两序在 raw 纯追加时相同，在两类可达输入上分叉：

- **中段出生**：第 1 代被跟踪分型 A、B（窗口起点 20、60）；第 2 代 raw 在坐标 45 出现一根 bar，生出窗口起点 40 的新分型 C。后续 bar 的批记录重建后，窗口序为 `[A′, C, B]`，出生序为 `[A′, B, C]`。
- **原位重生**：第 3 代 A 的基础来源被修订，A 换新 object_id A″（A″ 走 `gen:2356` `None` 分支判脏，旧 A 由 `gen:2362-2366`「上代有、本代无」循环判脏）。窗口序为 `[A″, C, B]`，出生序为 `[B, C, A″]`。

## 裁定

1. **entries 按所引分型对象 `payload.window[0]`（源坐标整数）严格升序。** 被跟踪分型的窗口起点按构造唯一（每个合并 bar 窗口起点至多一个形态）；读者见到相等或逆序即拒收，不设次级排序键，不设兜底（#799 / #804：只有一套判据）。
2. **取代范围**：只取代 ADR 0037 `:14`「entries 按分型出生序」一子句，以及复述它的 #1489 评论 Q1 ②、#1491 票面 `:17`、`.chanlun/review-results/issue1323-grilling-20261013/1489-AFFECTED-CODE.md:36` 三处同句；这三处原文不改，以本 ADR 为准。
3. **不升布局版本**：PR #1780 未合入 main，`bounded-layout/1` 尚无生产库发布过批记录；本条是该组件定义的补全，布局组件号 `bounded-layout/1` 不变，不开 `/2`。现行 main 与 PR #1780 head 的 `V3_RULE_REVISION` 均为 `"s2-axis-quantifiers+new-bi/2"`，尚不含 ADR 0037 §五 要求的 `+bounded-layout/1` 组件；生产串切换属 #1491 欠项，应与批记录同 PR 落地。
4. **分叉条件**：纯追加代两序相同；中段出生或非尾分型重生的代上两序不同，以窗口序为准。

### 理由

- **出生序的作用域只含快照累计数组。** ADR 0037 §一（`:12`）逐一列出 `objects`、`withdrawn_objects`、`witnesses` / `relations` / `observations`、`raw_history` 与 CC-054 `requests`，entries 不在列；其立论是尾部追加使 §5.2 M 树每代只新增 O(log n) 节点（CONTEXT.md「出生序累计数组」）。批记录载荷是不可变内容，脏 bar 整条重建（`gen:2375-2408`）、其余 bar 原样复用（`gen:2409-2430`），从不跨代追加，该立论没有作用对象。#1489 已登记生成态次序不改（`1489-AFFECTED-CODE.md:21`）。
- **出生序使载荷依赖发布史。** typed 对象身份六字段（`cp:1203-1220`）不含代；生成器给每条记录写 `first_known_generation = 本代`（`cp:1120`），`oi:502-505` 拒收非本代候选，真实首获知代只在 object_index。按出生序排 entries，同一 raw 状态经不同 cut 史会得到不同 object_id，在案锁「增量 ≡ 冷算」（#1491 `:27`、`gen:5154-5197`；ADR 0042 `:40`、`:89`）在可达夹具上必败。窗口序下 entries 只由当前 raw 决定，载荷与 object_id 是 raw 内容的函数。
- **Δ_R 不受影响。** 判脏只看坐标区间（`gen:2329-2373`），两序重建的批记录集合相同；差额只在单条记录内 entries 的 M 树子树复用（k = 严格小于 n 的最大 2 的幂，`rust/src/s_content.rs:387`）。中段出生时出生序少约 n−p 个内部节点，原位重生时窗口序少约 n 个，均不超过整条重写（≤ 2n−1），且只出现在含原始修订的代，落在 ADR 0037 §四「坐标 ≥ 修订点的派生记录」语义必要量内；纯追加代逐位相同。上界门不随两序选择变化。
- **§5.4 Delta 等式与 §5.5 Q 行序不涉**：前者作用于快照数组元素，后者在读取时按 record_order_key 重排（ADR 0037 `:12`）。

### 被否

- **出生序**：理由同上。entries 次序是布局编码取舍，不是 `[新缠论:选择]` 自由度，版本机制即 `bounded-layout/<n>`（ADR 0037 §五）；出生序违反在案锁「增量 ≡ 冷算」，属 [ADR 0041](0041-degrees-of-freedom-configurable-not-excluded.md) 裁定四所列「真正错误」（违反在案锁），照旧排除，不落可调字段。
- **回写 ADR 0037 原句**：正本不追溯改写，另立本 ADR 取代。

## 退场条件

任一触发即按 ADR 0037 `:20` 开 `bounded-layout/2` 改 entries 次序，旧库 AsKnown 保留、不回填：

- ADR 0037 `:31-32` 原有触发：APPROVED-SOURCE 明文规定 CC-007 载荷或数组次序，或 SPEC #1340 为「已实现后续发展」立独立轴或终止条件；
- #1491 / #1481 的上界门在 S86 逐代或 9011 选点出现超界代，且超界量经归因落在批记录 entries 的 M 树子树重写上（同代改尾追加即回界内）。

## 受影响代码清单

基线 `e72cf569ba`（PR #1780 head，落地时复核未变）；全部由 #1491 / PR #1780 owner 执行，无逻辑改动。

| 侧 | 位置 | 改动 |
|---|---|---|
| Rust | `rust/src/bin/s_session_v3/generation.rs:2314-2317` | `development_batches` 文档注释：「待裁定（#1491，PR #1780 独审 M-2）」改为「entries 按分型窗口序（ADR 0054 / #1783）」，保留分叉条件一句 |
| Rust | `rust/src/bin/s_session_v3/generation.rs:5252` | 测试注释「M-2 待裁定」改指 #1783 |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:1475` | `build_development_batch` 文档：「现为分型窗口序」改为「按分型窗口序（ADR 0054）」 |
| Rust | `rust/src/bin/s_session_v3/generation.rs` 测试模块（`:5561` 附近） | 新增分叉锁（见下「验收」第 2 条） |
| JS | `s_session/browser/q3-typed.js:759` | 注释改指 #1783；`:760` 判据与报错文案不变 |
| JS | `s_session/browser/tb01c-client.js:3362` | 同上；若由 `build_tb01c_browser.cjs` 生成，改源后重建，`--check` 通过 |
| Python | `s_session/` entries 读口（#1491「Python 侧」欠项，尚未实现） | 实现时按 `window[0]` 严格递增校验，不读 `first_known_generation` |
| Lean | — | 无 |
| 不改 | `layout.rs`、`object_index.rs`、`aux_index.rs`、`history.rs` 的出生序比较器 | 快照累计数组仍按 ADR 0037 §一 |

## 验收（落到 #1491 / PR #1780 对齐后的 head）

1. `git grep -n "M-2 待裁定" -- rust/src/bin s_session` 输出为空；`git grep -ln "#1783\|ADR 0054" -- rust/src/bin/s_session_v3/generation.rs rust/src/bin/s_session_v2/content_producer.rs s_session/browser/q3-typed.js s_session/browser/tb01c-client.js` 列出 4 个文件。
2. 分叉锁 `bounded_layout_cc007_entries_window_order_diverges_from_birth_order`（名可微调，须含 `bounded_layout_cc007`）在同一夹具内断言：① 至少一根 bar 的 entries 中存在分型 X、Y，X 的 object_id 不在第 g−1 代生成态 `objects` 中、在第 g 代中，Y 的 object_id 在第 g−1 代生成态已存在，且 X.`window[0]` < Y.`window[0]`（两序在此真实分叉；不读生成态 `first_known_generation`，它恒为本代）；② 该 bar entries 按 `window[0]` 严格递增；③ 该代增量 ≡ 冷算逐字节相同。`cargo test --locked --features s_session --bin s_structure_session bounded_layout_cc007` 全部通过。
3. `node --test s_session/tests/test_tb02b_layout_cc007.cjs` 全部通过，`node s_session/tests/build_tb01c_browser.cjs --check` 退出码 0。

## 受影响票与边

#1783 关票即解除 #1783 → #1491 blocking；#1491 其余 blocked_by 为 #1488（已关）。不新增边；#1491 → #1481、#1491 → #1643 不变（#1643 按坐标判脏，与 entries 次序无关）。#1783 原生 parent 为 #1340，记账在 #1323 `## Decisions-so-far`，不伪造原生挂接。

## 授权

`.sandcastle/docs/graph-engineering.md:90` N-1 技术类决策预授权（用户 2026-10-18）：T 类由代理起草、独审 PASS 后直接裁定。ADR 0037 本身即代理依用户 2026-10-13 授权裁定，本裁定在同一授权链上修订其一子句。本 ADR 不授权合 main、真实资金或关闭 #1491。

## 修订（[#1790](https://github.com/xy7365527-lang/NewChanlun/issues/1790)，2026-10-28）：两序在可达输入上恒等，验收命题改为「出生构成窗口序后缀」

**结论不变**：entries 仍按所引分型 `payload.window[0]` 严格升序（裁定 1–3 不动），出生序仍被否。本段只订正分叉前提及由它推出的段落。

**订正的事实**：分型身份含右组来源前缀链。CC-006 形态 `obj3` 的十二字段含 `input_refs`（`content_producer.rs:1249-1272`），其三组 `raw_refs` 与 `dependency_refs` 之并 = 右组 `group_sources_coords`（`:2456`、`:520-525`）；在分型存在的输入上（坐标严格递增、初始方向已定），该链是从首根 raw 起到组锚、封口边界或末根的完整前缀（`:3405-3429`；`theta_v0/parser/inclusion.rs` seed `:1084-1120`、establish/new_group `:1170-1188`、merge `:1247-1322`），叶为含 revision/seq 的八字段源引用（`content_producer.rs:106-214`、`:404-444`，`bind_source` `:804-842` 版本变化即换叶）；CC-007 分型身份经载荷 `shape_object_id` 继承之。设 c 为本代与上代 raw 首个不同坐标：保留旧 id 的分型来源链末端 < c，新出生分型来源链末端 ≥ c，来源链随窗口逐级嵌套（`derived_coords_match_materialized_union_for_every_prefix` 锁定），故**每代新出生分型在窗口序上构成后缀**；按代归纳，首获知代随窗口起点单调不减，出生序与窗口序在可达输入上逐位相同。行号基线 PR #1780 head `a485194947`。

**撤回与改写**：

1. 背景 `:15-18` 两个分叉例作废：中段出生例中 B 随前缀链变化重生为 B′，两序均为 `[A′, C, B′]`；原位重生例中 A 之后的 C、B 同代重生，两序均为 `[A″, C′, B′]`。
2. 裁定 4 改为：**任何代上两序相同；窗口序是规范定义，出生序不作判据。**
3. 理由「Δ_R 不受影响」段中「中段出生时出生序少约 n−p 个内部节点，原位重生时窗口序少约 n 个」撤回：两序给出同一 entries，Δ_R 逐位相同；「上界门不随两序选择变化」照旧成立。
4. 被否「出生序」的理由不变，并补一句：两序在可达域内值相同，出生序被否在于它的**算法**须读 object_index 的真实首获知代（生成层恒写本代，`content_producer.rs:1120`），使载荷计算依赖发布史；窗口序只读当前 raw。

**验收 2 改写**（取代原 `:65` 第 2 条；第 1、3 条不变）：

2. 锁 `bounded_layout_cc007_entries_window_order_on_mid_birth`（`generation.rs` 测试模块）在至少三个夹具（中段插入、非尾 bar 修订各至少一例）上断言：① 第 g 代存在中段出生，即新出生分型中至少一个窗口早于第 g−1 代最后一个分型；② **出生构成窗口序后缀**：凡 object_id 在第 g−1 代已存在的分型，其 `window[0]` 严格小于本代全部新出生分型的 `window[0]`；③ 每根 bar 的 entries 按 `window[0]` 严格递增，且至少一根同含新旧分型；④ 每代增量 ≡ 冷算逐字节相同。不读生成态 `first_known_generation`。`cargo test --locked --features s_session --bin s_structure_session bounded_layout_cc007` 全部通过。原名 `…_diverges_from_birth_order` 不再使用。

**退场条件补充**（与原退场条件并列，触发即重开本裁定，不自动改 entries 次序）：

- 身份配方变更使来源前缀链不再完整进入分型 object_id（例如 `dependency_refs` 截短为有界证据窗、`group_sources_coords` 改为非前缀构造、或 CC-007 身份不再继承 `shape_object_id`），致使第 ② 条锁失败、出现非后缀出生。

**依据**：#1790 resolution；草稿 `.chanlun/review-results/issue1790-decision-20261028/draft.md`（考古 `archaeology.md` / `archaeology.json` 同目录）；独审 reviewer gpt-6.1-sol 三轮（r1 BLOCK `review.r1.json`、r2 BLOCK `review.r2.json`、终审 PASS `.chanlun/review-results/issue1790-decision-20261028/review.json` / `review.md`）；实证 CI run 40663443503（head `a485194947`）与独审 `1780-INDEPENDENT-REVIEW-A485.md`。
