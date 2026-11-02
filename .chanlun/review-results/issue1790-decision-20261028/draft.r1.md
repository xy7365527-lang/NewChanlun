# #1790 起草：订正 ADR 0054——中段出生不使两序分叉，验收命题改为「出生构成窗口序后缀」

- 票：#1790（T 类，#1323 Notes N-1 预授权；分级见 `/Volumes/AgentStorage/decisions/1790/classify.json`）
- 起草：/root/draft1790_v2（手动起草；decision-claimer 自动考古两次输出无法解析，`decisions/1790/summary.json` phase=error，其 raw 考古 Q1/Q2 结论与本稿一致）
- 基线：main `df0ec357ab`（含 ADR 0054，`fd02ae41c4` 入）；PR #1780 head `a485194947`（OPEN，`codex/1491-segb-28p`）。下文 `cp` / `gen` / `oi` / `inc` 指该 head 上的 `rust/src/bin/s_session_v2/content_producer.rs`、`rust/src/bin/s_session_v3/generation.rs`、`rust/src/bin/s_session_v3/object_index.rs`、`rust/src/theta_v0/parser/inclusion.rs`。

## 结论

**前提成立，已证明。** 在现行身份配方下，每代新出生的被跟踪分型在窗口序上构成后缀；由此按代归纳，活集合的首获知代随窗口起点单调不减，出生序 `(first_known_generation ↑, window[0] ↑)` 与窗口序在可达输入上逐位相同。ADR 0054 裁定 4、两个分叉例、Δ_R 段「两序互有胜负」推导与验收 2 ① 均以「旧分型 Y 晚于新出生 X」为前提，该前提不可达，须订正。窗口序为正本的结论不变：否决出生序的理由（读真实首获知代须依赖发布史、生成层恒写本代、冷算不可复现）不依赖分叉是否发生。

载体选 **ADR 0054 追加「修订（#1790）」段**，不开 ADR 0056（理由见「订正条文草稿」首段）。

## 1. 问题

ADR 0054（main `docs/adr/0054-development-batch-entries-window-order.md`）：

- `:15-18` 背景称两序在「中段出生」「原位重生」两类可达输入上分叉，例 1 `[A′, C, B]` vs `[A′, B, C]` 假定 B 保留旧 object_id；例 2 `[A″, C, B]` vs `[B, C, A″]` 假定 C、B 不随 A 重生。
- `:25` 裁定 4「中段出生或非尾分型重生的代上两序不同」。
- `:31` Δ_R 段「中段出生时出生序少约 n−p 个内部节点，原位重生时窗口序少约 n 个」。
- `:65` 验收 2 ① 要求夹具里存在新出生 X、旧分型 Y 且 X.`window[0]` < Y.`window[0]`。

PR #1780 head `a485194947` 的锁 `bounded_layout_cc007_entries_window_order_on_mid_birth`（`gen:5582-5676`）断言的是 ① 的否命题，CI run 40663443503（headSha = `a4851949…`，completed success，本轮复核）三例通过；独审 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1780-INDEPENDENT-REVIEW-A485.md:8-10,43-55` 据此 BLOCK，要求回正本订正。

## 2. 权威依据（考古证据）

### 2.1 身份配方把分型的整条来源前缀链写进 object_id

| # | 位置 | 事实 |
|---|---|---|
| E1 | `cp:491-493`、`cp:1224-1258` | CC-006 形态 `obj3` = 十二字段身份，含 `input_refs`（值边）、三组窗口坐标、高低价 |
| E2 | `cp:2455-2490` | `input_refs` = 三组 `{merged_index, raw_refs(成员), dependency_refs((dir ∪ conf) \ 成员)}`；`sources = group_sources_coords(right)`（`:2456`），三组并集 = 右组来源链（`cp:520-525`） |
| E3 | `cp:916-929`、`cp:2719-2760` | `raw_refs` / `dependency_refs` 的叶是按坐标登记的八字段源引用（含 revision / seq 等）；某坐标被修订则其叶换值，含该坐标的数组根随之改变 |
| E4 | `cp:3405-3428` | `group_sources_coords`：有封口证据取 `conf.coords`，否则 `dir.coords ++ member_tail`；`dir.coords` = 「处理到本组锚为止的完整前缀」 |
| E5 | `inc:1170-1188` | 证据链构造：新证据 = 旧证据坐标 ++ 前组尾链，再接 incoming；首次建立方向时取首组成员链（锚 ++ 组尾）。即证据链从首根 raw 起连续不缺 |
| E6 | `cp:2170-2172`、`cp:1208-1220` | CC-007 分型对象身份为泛型六字段，`payload` 内嵌 `shape_object_id`；bounded 布局下来源只取基础来源 = 右组来源链（`cp:2150-2156`） |
| E7 | `cp:6582` `derived_coords_match_materialized_union_for_every_prefix` | 逐前缀对拍锁：「三连窗并集 == 右组 sources」，来源链逐级嵌套 `sources_i ⊆ sources_{i+1}`（`cp:522-525`） |

### 2.2 出生序的定义与生成层事实

| # | 位置 | 事实 |
|---|---|---|
| E8 | ADR 0037 `:12` §一 | 快照 `objects` 按 (first_known_generation ↑, 同代内原键) |
| E9 | `oi:404-410`、`oi:449-458` | 形态槽同代内原键 = `window[0]`；公开主序键 = (首获知代, order_key) |
| E10 | `cp:1120`、`oi:502-505` | 生成层 typed 记录 `first_known_generation` 恒写本代；真实首获知代只在 object_index |

### 2.3 实证

| # | 位置 | 事实 |
|---|---|---|
| E11 | `gen:5592-5676` | 三例（insert@85、revise b9、revise b8）：每例有中段出生（`:5641-5644`），全部旧分型窗口早于最早新出生（`:5646-5654`），entries 严格递增且至少一根同含新旧分型（`:5655-5673`），增量 ≡ 冷算（`:5636-5638`） |
| E12 | CI run 40663443503 | head `a485194947` 全部通过（含 rust-test-s-session 4 shard） |
| E13 | 独审 A485 `:29`、`:41` | 复核 E2/E4 推理成立；负控「改成出生序锁应失败」做不出来，因两序在夹具上逐字节相同 |

Lean：`formal/` 中无 `group_sources` / `development_batch` 相关定义（`git grep -il "group_sources\|development_batch" df0ec357ab -- formal` 零命中），本订正不涉形式化层。

### 2.4 证明

记第 g 代被跟踪分型 F 的来源链 chain(F) = 其右组 `group_sources_coords`，末端坐标 e(F) = max chain(F)。

1. **链是完整前缀**（E4、E5）：chain(F) = 第 g 代有序 raw 坐标中从首根到 e(F) 的全部坐标。
2. **末端随窗口单调**（E7）：窗口起点更小的分型，其右组在前，chain 被包含，e 不大于后者。
3. **id 绑定整条链**（E1–E3、E6）：chain(F) 内任一坐标新增或修订，F 的 CC-006 与 CC-007 object_id 都变。
4. **包含处理是因果的**（`inc:1040` 起逐根左到右处理；在案锁「增量 ≡ 冷算」）：坐标 < c 的 raw 不变，则只依赖该前缀的分型逐字节不变。

设 c = 第 g 代与第 g−1 代 raw 首个不同坐标（插入或修订；纯追加时 c 大于全部旧坐标）。

- 旧分型 Y 在第 g 代保留原 id ⟹ e(Y) < c（否则由 1，c ∈ chain(Y)，由 3 换 id）。
- 新出生 X ⟹ e(X) ≥ c（否则 X 全部输入落在未变前缀，由 4 在第 g−1 代已以同 id 存在）。
- 于是 e(Y) < c ≤ e(X)，由 2 得 Y.`window[0]` < X.`window[0]`（窗口起点按构造唯一，ADR 0054 裁定 1）。

**每代出生构成窗口序后缀。** 再按代归纳：若活集合中有 Y1.`window[0]` < Y2.`window[0]` 而首获知代 Y2 < Y1，则在 Y1 出生的那一代 Y2 已在且存活、窗口更晚，违背该代后缀性。故首获知代随窗口单调不减，出生序 = 窗口序。ADR 0054 两个分叉例在可达输入上分别变为 `[A′, C, B′]`、`[A″, C′, B′]`，两序相同。

证明的承重前提只有 1 和 3：**身份配方含从首根 raw 起的完整来源前缀链**。这正是退场条件要盯的东西。

## 3. 候选

- **A（推荐）**：ADR 0054 追加修订段，撤回分叉前提，验收 ① 改为后缀命题，结论不变。
- B：另立 ADR 0056 取代 ADR 0054 裁定 4 与验收 2。被否：本仓同一决策的事实订正一律在原 ADR 追加「修订」段（先例 ADR 0005 `:32`、0022 `:91`、0026 `:52`、0032 `:39`、0015 `:190`）；结论未变，拆到两份文件会让读者在 0054 读到被推翻的分叉例却无指引。
- C：维持 ADR 0054 原文，只改 #1780 注释。被否：#793 规定口径分歧不得只存在于注释，独审 A485 `:10` 已据此 BLOCK。

号位核查：main `docs/adr/` 最大号 0055；`/Volumes/AgentStorage/decisions/adr-reservations.json` 最大登记 55（#1786）。若主控改走候选 B，下一可用号为 **0056**（本稿只建议、未登记）。

## 4. 裁定（订正条文草稿）

### 4.1 ADR 0054 头部

三行头第二行改为：

```
> 最后裁定：#1790（2026-10-28，订正 #1783 的分叉前提与验收命题；#1323 Notes N-1 预授权 T 类判定）
```

其余正文（`:1-74`）不改，由下面的修订段覆盖。

### 4.2 追加段（置于 ADR 0054 文末）

```markdown
## 修订（[#1790](https://github.com/xy7365527-lang/NewChanlun/issues/1790)，2026-10-28）：两序在可达输入上恒等，验收命题改为「出生构成窗口序后缀」

**结论不变**：entries 仍按所引分型 `payload.window[0]` 严格升序（裁定 1–3 不动），出生序仍被否。本段只订正分叉前提及由它推出的段落。

**订正的事实**：分型身份含右组来源前缀链。CC-006 形态 `obj3` 的十二字段含 `input_refs`，其三组 `raw_refs` 与 `dependency_refs` 之并 = 右组 `group_sources_coords`（`content_producer.rs:2456`、`:520-525`）；该链是从首根 raw 起处理到组锚（有封口则到封口边界）的完整前缀（`content_producer.rs:3405-3428`、`theta_v0/parser/inclusion.rs:1170-1188`），叶为含 revision 的源引用；CC-007 分型身份经载荷 `shape_object_id` 继承之。设 c 为本代与上代 raw 首个不同坐标：保留旧 id 的分型来源链末端 < c，新出生分型来源链末端 ≥ c，来源链随窗口逐级嵌套（`derived_coords_match_materialized_union_for_every_prefix` 锁定），故**每代新出生分型在窗口序上构成后缀**；按代归纳，首获知代随窗口起点单调不减，出生序与窗口序在可达输入上逐位相同。行号基线 PR #1780 head `a485194947`。

**撤回与改写**：

1. 背景 `:15-18` 两个分叉例作废：中段出生例中 B 随前缀链变化重生为 B′，两序均为 `[A′, C, B′]`；原位重生例中 A 之后的 C、B 同代重生，两序均为 `[A″, C′, B′]`。
2. 裁定 4 改为：**任何代上两序相同；窗口序是规范定义，出生序不作判据。**
3. 理由「Δ_R 不受影响」段中「中段出生时出生序少约 n−p 个内部节点，原位重生时窗口序少约 n 个」撤回：两序给出同一 entries，Δ_R 逐位相同；「上界门不随两序选择变化」照旧成立。
4. 被否「出生序」的理由不变，并补一句：两序在可达域内值相同，出生序被否在于它的**算法**须读 object_index 的真实首获知代（生成层恒写本代，`content_producer.rs:1120`），使载荷计算依赖发布史；窗口序只读当前 raw。

**验收 2 改写**（取代原 `:65` 第 2 条；第 1、3 条不变）：

2. 锁 `bounded_layout_cc007_entries_window_order_on_mid_birth`（`generation.rs` 测试模块）在至少三个夹具（中段插入、非尾 bar 修订各至少一例）上断言：① 第 g 代存在中段出生，即新出生分型中至少一个窗口早于第 g−1 代最后一个分型；② **出生构成窗口序后缀**：凡 object_id 在第 g−1 代已存在的分型，其 `window[0]` 严格小于本代全部新出生分型的 `window[0]`；③ 每根 bar 的 entries 按 `window[0]` 严格递增，且至少一根同含新旧分型；④ 每代增量 ≡ 冷算逐字节相同。不读生成态 `first_known_generation`。`cargo test --locked --features s_session --bin s_structure_session bounded_layout_cc007` 全部通过。原名 `…_diverges_from_birth_order` 不再使用。

**退场条件补充**（与原退场条件并列，触发即重开本裁定，不自动改 entries 次序）：

- 身份配方变更使来源前缀链不再完整进入分型 object_id（例如 `dependency_refs` 截短为有界证据窗、`group_sources_coords` 改为非前缀构造、或 CC-007 身份不再继承 `shape_object_id`），致使第 ② 条锁失败、出现非后缀出生。

**依据**：#1790 resolution；草稿 `/Volumes/AgentStorage/decisions/1790-manual/draft.md`；实证 CI run 40663443503（head `a485194947`）与独审 `1780-INDEPENDENT-REVIEW-A485.md`。
```

## 5. 受影响代码清单

全部在 PR #1780 head `a485194947` 上，由 #1491 / PR #1780 owner 执行；无逻辑改动。

| 侧 | 位置 | 现状 | 改动 |
|---|---|---|---|
| Rust 锁 | `gen:5592` `bounded_layout_cc007_entries_window_order_on_mid_birth` | 名与断言已是后缀命题 | **保留原名**；不改为 `…_diverges_from_birth_order` |
| Rust 锁文档 | `gen:5582` | 「ADR 0054（#1783 验收 5）中段出生锁」 | 改指「ADR 0054 #1790 修订·验收 2」 |
| Rust 锁文档 | `gen:5587` | 「本条若失败，说明身份配方变了，须回 #1783 复核」 | 改为「…触发 ADR 0054 #1790 修订段退场条件，重开 #1790」 |
| Rust 断言 | `gen:5641-5654`、`:5655-5673` | 对应修订验收 ②①③ | 不改 |
| Rust 文档 | `gen:2314-2318` `development_batches` | 已写「两序仍一致……判据只认窗口序」，指 `ADR 0054 / #1783` | 指针补 `#1790`（如「ADR 0054 / #1783，#1790 修订」）；顺手把 `:2317` 超长行折行 |
| Rust 注释 | `gen:5252`、`cp:1475` | 「按分型窗口序（ADR 0054 / #1783）」 | 不改（结论未变，指针仍有效） |
| JS | `s_session/browser/q3-typed.js:759`、`tb01c-client.js:3362` | 同上 | 不改；`:760` / `:3363` 判据不变 |
| Python | `s_session/` entries 读口（#1491 欠项） | 未实现 | 按 ADR 0054 原要求，不变 |
| Lean | — | — | 无 |
| 文档 | `docs/adr/0054-development-batch-entries-window-order.md` | — | 头部「最后裁定」行 + 文末修订段（本稿 §4），落 `decision/1790` 分支 |
| 不改 | `CONTEXT.md:170`、ADR 0037、`decisions/1783-manual/*` | — | `CONTEXT.md` 未写分叉；推论层原件按 ADR 0012 不追溯改 |

## 6. 验收清单

1. `git grep -n "## 修订（\[#1790\]" -- docs/adr/0054-development-batch-entries-window-order.md` 一行命中；`git grep -n "最后裁定：#1790" -- docs/adr/0054-development-batch-entries-window-order.md` 一行命中。
2. #1780 对齐后 head：`git grep -n "diverges_from_birth_order" -- rust/src` 为空；`git grep -c "#1790" -- rust/src/bin/s_session_v3/generation.rs` ≥ 2；`git grep -n "M-2 待裁定" -- rust/src/bin s_session` 为空。
3. 同 head：`cargo test --locked --features s_session --bin s_structure_session bounded_layout_cc007` 全部通过（CI rust-test-s-session 各 shard 绿即可）。
4. 同 head：`node --test s_session/tests/test_tb02b_layout_cc007.cjs` 通过，`node s_session/tests/build_tb01c_browser.cjs --check` 退出码 0（只改 Rust 注释时，沿用 run 40663443503 的结果）。
5. 非起草者独审 `review.json` PASS 后落正本 PR；#1790 发 resolution 评论，首行「本条依 #1323 Notes N-1 预授权判定」。

## 7. 受影响票与 blocking 边

- #1790 → #1491：建议接 blocking（若主控尚未接）。PR #1780 以 a485194947 的两提交作为「落实 ADR 0054」合入前，须先合本修订（独审 A485 `:55`）。
- #1783 已关，不重开；其结论由本修订续正。#1491 → #1481、#1491 → #1643 不变。
- 原生 parent 记账同 #1783：在 #1323 `## Decisions-so-far` 加一行，不伪造挂接。

## 8. 授权

`.sandcastle/docs/graph-engineering.md:90` N-1 技术类决策预授权：T 类代理起草、独审 PASS 后直接裁定。本修订只订正事实前提与验收命题，不改结论、不涉政策参数；不授权合 main、关闭 #1491 或改 PR #1780。

## 引用自证

本轮实读：`gh issue view 1790 --comments`；`git show df0ec357ab:docs/adr/0054-…md`（全文 `:1-74`）、`0037-…md:10-22`、`.sandcastle/docs/graph-engineering.md`（`:28-58`、`:77-78`、`:90`）；`git show a485194947:` 下 `cp` `:480-560`、`:910-960`、`:1195-1300`、`:2150-2185`、`:2440-2530`、`:2700-2760`、`:3395-3450`，`gen` `:2305-2325`、`:5225-5275`、`:5578-5690`，`oi` `:395-490`，`inc` `:1040-1192`；`gh run view 40663443503`；`gh pr view 1780`（head `a4851949…` OPEN）；独审 A485 全文；`adr-reservations.json`。

## 考古对照

| 点 | 问题 | 类 | 查询 | 命中 / 处置 |
|---|---|---|---|---|
| Q1 | 分型 object_id 是否含完整来源前缀链 | A | `grep -n group_sources_coords` cp；读 inc 证据构造；`gh issue list --state all --search group_sources_coords` | E1–E7；tracker 仅 #1790 自身。沿用代码事实 |
| Q2 | 「旧 Y 晚于新 X」是否可达 | A | §2.4 证明；CI run 40663443503 | 不可达；订正裁定 4 与例 |
| Q3 | 否决出生序的理由是否仍成立 | A | ADR 0054 `:30`、`cp:1120`、`oi:502-505` | 成立，补「否的是算法不是值」 |
| Q4 | 订正载体：新号还是修订段 | A | `git grep -n -E "^## .*(修订\|订正)" -- docs/adr` | 先例五处，选修订段；备选号 0056 |
| Q5 | Δ_R 段推导 | A | ADR 0054 `:31`、ADR 0037 §四 | 两序同值，撤回差额推导，门结论不变 |
| Q6 | #1780 锁名与注释 | A | `grep -n "1783\|ADR 0054"` 于 gen/cp/JS | 见 §5 |
| Q7 | 既有窗口序/出生序讨论 | A | `gh issue list --state all --search "出生序 窗口序"`；`git grep -l 窗口序 -- analysis docs .chanlun` | #1783（已关）、#1491；推论层无独立主张 |
| Q8 | Lean 是否有对应定义 | A | `git grep -il "group_sources\|development_batch" -- formal` | 零命中，无 |

无 B 类（无待确认默认值）、无 C 类待决点、无事实缺口票。
