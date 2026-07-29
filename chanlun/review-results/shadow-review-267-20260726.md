# 影子评审报告：#267 held 腿占位元素角色输入重建（commit `8bc7c67e98`）

- **评审票**：#284（对应实装票 #267）
- **评审者**：独立影子评审（新上下文 Opus，未参与 #267 实装，禁自评资格成立）
- **评审对象**：`8bc7c67e98`，`rust/src/theta_v0/strategy/coverage.rs` 单文件 +230/−0
- **worktree**：`/tmp/kimi-nest-mainline`，HEAD = `ecb8923c19`（#267 已是 HEAD 祖先）
- **日期**：2026-07-26

## 0. 结论

**无 HIGH，不建议 reopen #267。** 两处占位 push 的角色输入重建口径正确、与 #247 前例一致、
测试只测外部行为且防真空、生产对拍照实（含 BTC 转差）。

浮出 **1 条 MED**（覆盖不完全 + 声明过强，同根）与 **2 条 LOW**。MED 不是回归——命中时行为
= 修复前行为，且两窗口未观测到；但它使本票「同类位点全闭合」的字面验收留下缺角，且 helper doc
的「**恒**被 AncOK 剪除」在 #267 语境下不成立（在 #247 语境下成立）。

## 1. 复跑（本上下文独立执行）

| 项 | 命令 | 结果 |
|---|---|---|
| 全量 | `cargo test --release --lib` | **1834 passed / 1 failed / 132 ignored**，唯一失败 = `classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#110 在案，未碰未归因） |
| 定向 | `cargo test --release --lib held_leg` | **4 passed / 0 failed** |
| diff 面 | `git show 8bc7c67e98 --stat` | `coverage.rs` 单文件 +230/−0，无越界 |

工作区 `git status` ≈370 个 ` D`（未 checkout 的删除态），未触碰；本报告为唯一新增文件。

## 2. Spec 轴（票 #267 票体 + resolution + #247 前例 `b57da4bde4` + `mutex-domain-loadbearing-20260725.md`）

| # | 票面要求 | 判定 | 行号证据 |
|---|---|---|---|
| S1 | 先做核实（RED 前置）：占位元素是否真到达角色计算 | **PASS** | `held_leg_live_detached_placeholder_...`（`5044` 前的 `4988`）先断言 `next_active` 含该 id（承重前提），再断言 `sep_legs` 的 `role_v`/`q_units`（角色被消费）。承重结论由测试坐实非静态推断，符合票面「先写测试坐实结论」 |
| S2 | 用已知 `parent_id`(=`leg.op_parent`) 重建 `parent` 索引 + `attached_dir` | **PASS** | `rebuild_placeholder_parent_attached`（`2175-2194`）：`work.set_parent_attached(idx, pidx, work[pidx].eps)` |
| S3 | σ_{p(g)} = 父元素 eps，与 `push_element_tree`/#247 同口径 | **PASS** | `2190`（`let p_eps = work[pidx].eps`）逐字同 #247 修复段（`b57da4bde4` 的 `restore` 循环后 fixup）；`push_element_tree`（`456`）压子元素同样传 `Some(父eps)` |
| S4 | 三级解析序同 #247：`id_idx`(base) → `overlay_seen`(candidate+restore) → `raw` 扫 | **PASS** | `2183-2188`，与 `b57da4bde4` fixup 段逐字同构；base 优先亦与 `ancestor_close_by_id` 的 `lookup`（`869-871`）precedence 一致 |
| S5 | 父不可解析时**不伪造**，且明确该情形语义、与「防御分支」区分 | **PARTIAL（MED-1）** | 不伪造已兑现（`2185-2193` 无 else 分支）。但「恒被 AncOK 剪除」（`2303`/`2335`/`4912` + helper doc `2168-2172`）**在 #267 语境下不成立**，详见 §4 |
| S6 | ∂（`parent_id=None`）保持 None/None | **PASS** | `2183` `if let Some(pid)`；测试 `5124` 断言 `role_v==Ambient`、`q_units==600`、`parent_id==None` |
| S7 | TDD：先 RED 后 GREEN | **PASS（静态核对，未执行回退）** | 两条承重测试的 RED 值写死在 doc（`Ambient/600/p̃=300`、`Ambient/600/p̃=0`），与 GREEN 断言（`ShortDiff/100/800`、`ShortDiff/300/300`）逐项互斥 ⟹ 回退实装必红。**本评审只读，未实际执行「删 helper 复跑」的回退验证**——判定依据是断言值互斥性的静态核对 |
| S8 | 声明锚：检查 #247 的 10 处锚是否覆盖本路径，未覆盖则补 | **PASS** | 占位元素是 `(A_t∖𝒟_x)` 项内 Stale 持仓腿的物化（`2294-2301`/`2322-2333` 的 `id: leg.id` 携持仓身份），**非**第三来源 `RegistryRestore`；不补新锚成立。HEAD `ecb8923c19` 的 #266 订正（`:18` 头表归属 + `element_depth` 链域）已把 #267 引入的「parent 可指 overlay 段」写进 `element_depth` doc（`1582`），声明面闭合 |
| S9 | `cargo test --release --lib` 不退化 | **PASS** | 见 §1（票面基线 1817/1 → commit 时点 1825/1 → HEAD 1834/1，增量全来自后续票，唯一失败恒为 #110） |
| S10 | 行为改变影响 p̃/腿集合 ⟹ 改动前后对拍证据，翻负照实 | **PASS** | `restore-chain-quantification-20260726.md` 三臂对拍（B=`b57da4bde4` 仅 #247，C=HEAD）：OKLO `+1144.000` 减亏、**BTC 切片 `−422.310` 转差已照实报**（:29/:39）；触发面/∂/声部身份 0 变化。commit message 只报 OKLO，报告补 BTC——诚实通道完整，无声明膨胀 |
| S11 | 改 rust/src ⟹ 配影子评审票（新上下文禁自评） | **PASS** | #284 建票，本报告即产出 |

## 3. Standards 轴

| # | 标准 | 判定 | 证据 |
|---|---|---|---|
| T1 | 090 严格性：声明与实际一致 | **FAIL ×1（MED-1）** | 「**恒**被 AncOK 剪除」（`2303`/`2335`/`2168-2172`/`4912`）是全称断言，存在反例（§4）。其余声明（∂ 语义、三级解析序、σ_p 口径、不伪造）逐条与实装相符 |
| T2 | 090：不留半成品（完整解决） | **PARTIAL（MED-1）** | 立即式修补（push 点即调 helper）对「父在本轮更晚 push」不闭合；#247 前例用的是**循环后统一 fixup**，正是为规避同一时序问题（`b57da4bde4` 注释：「push 子元素时真父 idx 通常尚未知 ⟹ 统一在循环结束后修补」）。#267 未沿用该形状 |
| T3 | no-patch：非防御兜底 | **PASS** | helper 无 else 兜底、无 `unwrap_or` 造值；ancok probe 计数未新增伪路径；两分支语义在 doc 中明确区分「正确语义」vs「防御分支」 |
| T4 | helper 三级解析正确性 | **PASS** | `id_idx` 覆盖 base 树前缀（`2247-2250` 由 `TreeCache` 或现建，`tree_end==candidate_start==base_len`）；`overlay_seen` 覆盖 candidate 段 + restore push（`2255-2261` + `restore` 内 `or_insert`）；`raw` 扫兜底先前 push 的占位（占位从不写 `overlay_seen`，doc `2166-2167` 已如实说明）。三级并集 ⊇ raw ⟹ 「不可解析 ⟹ 当时不在 raw」成立（但 ≠「最终不在 raw」，见 §4） |
| T5 | 调用点无自指 | **PASS** | `rebuild(...)` 在 `raw.push(idx)` **之前**（`2304-2308` / `2336-2340`）⟹ 占位不会把自己解析成父 |
| T6 | `set_parent_attached` 写入域安全 | **PASS** | 占位经 `work.push` 入 overlay（`ElementView::push` `246-251`）⟹ `debug_assert!(idx >= base.len())`（`255-257`）恒真 |
| T7 | 测试只测外部行为 | **PASS** | 4 测试全部只调 `coverage_step_from_buckets_sep` 公共出口并断言 `(next_active, p_tilde, sep_legs)`；无一处读私有字段/内部索引 |
| T8 | 测试防真空 | **PASS** | 每条承重测试三段断言（存活 → 角色 → p̃），且带对照锚（`5029-5033` 断言 restore 链父腿 `FollowParent/300` 不受本修复影响）。两条边界测试（`5094`/`5124`）自陈「修复前后行为一致」= 语义锚而非 RED→GREEN，未冒充承重证据——诚实 |
| T9 | 测试覆盖 | **LOW-1** | 未覆盖：(a) §4 的时序场景；(b) 父经 `overlay_seen`（candidate 段）解析的路径（现有两测试分别走 restore-overlay 与 base-`id_idx`） |
| T10 | doc 与实装一致 | **PASS（除 T1）** | helper doc（`2154-2173`）的解析序、边界语义、消费链（`element_depth`/V/G）与实装逐条对应；`element_depth` 链域已由 #266 在 HEAD 订正 |
| T11 | 不可变性（`coding-style.md`） | **LOW-2** | `set_parent_attached` 就地 mutate；与 `ElementView` 既有 push/mutate 惯用法一致，同 #247 评审判定，不计缺陷 |
| T12 | 文件规模 | **NOTE** | `coverage.rs` 已 >5000 行，远超 `coding-style.md` 的 800 行上限。非本 commit 引入（本 commit +230），但每次同类追加都在加深该偏离——建议在 #59 收尾时挂拆分票 |

## 4. MED-1（唯一实质发现）：立即式修补的时序缺角 + 「恒」字过强

### 缺陷陈述

helper doc（`2168-2172`）与两处调用点注释（`2303`/`2335`）声明：

> 父无法解析 ⟹ 保持 None/None，**不伪造**——占位元素的 parent_id 不在 raw ⟹
> `ancestor_close_by_id`（AncOK）**恒**剪除 ⟹ 不进 next_idx，到不了角色计算。

该推理把**两个不同时刻**当成同一时刻：

- 「不可解析」判定发生在**占位 push 的当轮**（`raw` 只含此前迭代的元素）；
- 「AncOK 剪除」判定发生在**整个 `prev_active` 循环 + open 循环之后**（`2417` `ancestor_close_by_id(&work, &raw)`，`raw` 已终态）。

`ancestors_by_id_lookup`（`780-793`）先 `chain.push(pid)` 再 lookup，成员判据是**终态 `raw_ids`**
（`849-850`）。故只要父在**更晚**的迭代中被物化进 `raw`，占位元素就**存活**，而它的
`parent`/`attached_dir` 仍是 `None/None` —— 正是本票要消除的角色输入丢失。

### 反例（结构可达，两窗口未观测）

**前提：父与子都是 Stale**（父若 Exact 则恒在 base 树 ⟹ `id_idx` 必命中，无问题），**且父在
`prev_active` 中排在子之后**。

排序可达性：open 候选先入 `raw`（`2396-2398`），其父链经 `restore_ancestor_chain_from_registry`
在**其后**追加（`2405-2412`）；`next_active` = `next_idx` 序 = `raw` 序（`2444-2445`）⟹
下一 bar 的 `prev_active` 中**子在父前**。

两条命中路径：

1. **LivePresent 臂**（`2288-2309`，无 restore）：子的 `op_parent` 此刻既不在 base 树（父 Stale）、
   不在 `overlay_seen`（占位从不写入）、也不在 `raw`（父尚未迭代到）⟹ 保持 None/None；父在后续
   迭代 push 入 `raw` ⟹ 子的 `parent_id ∈ raw_ids` ⟹ AncOK **保留** ⟹ `depth=0`/`V=Ambient`。
2. **LiveDetached 臂 + registry 断链**（`2310-2341`）：`restore` 因 `restore_break_registry_lost`
   未物化父；但该父腿本身若在后续迭代经 `Closed|Invalidated ∧ is_boundary_root` 臂（`2345-2360`）
   或另一占位臂进入 `raw`，同样存活且 None/None。

命中后的行为 = **修复前行为**（`depth=0`/`Ambient`/`SameLevel`），**不是回归**，但本票的
「同类位点闭合」字面未达成。

### 观测面

`restore-chain-quantification-20260726.md` 无针对 #267 占位「未解析」的 probe 计数（现有
`germ_patched`/未解析列统计的是 #247 restore push）。OKLO `state_live_present=1`、
`state_live_detached=12`，复合事件概率低，两窗口未见异常——故判 MED 非 HIGH。

### 建议（不在本评审执行）

1. 改为 **#247 同形状的循环后统一 fixup**：在 `prev_active` 循环中把占位 idx 收进
   `placeholders: Vec<usize>`，在 `2415`（AncOK 之前）统一跑一遍 `rebuild_...`。时序无关，且
   与 #247 口径真正对齐。届时「恒剪除」可退化为「解析失败 ⟹ 父确实从未物化 ⟹ AncOK 剪除」，
   全称断言才成立。
2. 在此之前，把 `2303`/`2335`/`2168-2172`/`4912` 的「**恒**被 AncOK 剪除」改为
   「**当父在本 bar 从未物化进 raw 时**被 AncOK 剪除」（090 声明与实际一致）。
3. 补 1 条测试：`prev_active = [子(Stale/LivePresent), 父(Stale/LivePresent)]`，断言子存活且
   `role_v == ShortDiff`（现实装下应红）。
4. 加 probe 计数 `placeholder_parent_unresolved`，让该路径在生产窗口可观测。

## 5. 分级汇总

| 级 | 编号 | 内容 | 处置 |
|---|---|---|---|
| — | 无 HIGH | — | **不 reopen #267** |
| MED | MED-1 | 立即式修补的时序缺角 + 「恒被 AncOK 剪除」全称断言有反例（§4） | 新挂 follow-up 票（建议并入 #59）：改循环后统一 fixup + 措辞订正 + 1 测试 + 1 probe |
| LOW | LOW-1 | 未覆盖 §4 时序场景 与 `overlay_seen`(candidate 段) 解析路径 | 随 MED-1 一并补 |
| LOW | LOW-2 | `set_parent_attached` 就地 mutate 偏离 `coding-style` 不可变原则 | 不建议改（与本文件既有设计一致，同 #247 评审判定） |
| NOTE | — | `coverage.rs` >5000 行，超 800 行上限 | #59 收尾挂拆分票 |

## 6. 结果包六要素

1. **结论**：#267 实装通过（无 HIGH）；1 MED（时序缺角 + 声明过强）、2 LOW、1 NOTE。复跑
   1834/1（#110 在案）+ held_leg 4/4，diff 面 = coverage.rs 单文件 +230/−0。
2. **定义依据**：#267 票体核实/实装/验收四条；#247 前例 `b57da4bde4`（σ_{p(g)}=父 eps、三级解析、
   不伪造、循环后 fixup）；`mutex-domain-loadbearing-20260725.md`（A 类实装缺口裁定）；
   `.claude/rules/no-patch-mentality.md` 090（声明与实际一致 + 不留半成品）；
   `.claude/rules/common/coding-style.md`（不可变、文件规模）。
3. **边界条件（结论何时翻转）**：
   (a) 若 §4 的时序场景在生产窗口被 probe 观测到非零触发 ⟹ MED-1 升 HIGH，#267 需 reopen；
   (b) 若 `ancestors_by_id_lookup`（`785-792`）改为**跳过**不可解析 pid，则「不伪造」的正当性
   基座失效，所有断链占位都会带 None/None 进角色计算 ⟹ MED-1 直接升 HIGH；
   (c) 若 `raw` 的组装顺序改为「父恒先于子」（拓扑序 push），MED-1 自动消解为 LOW；
   (d) 若 `element_depth`/任何消费者引入「parent 必在 base 段」的运行时假设，#266 已订正的
   MED-3 会复活为 HIGH（#267 使 parent 可指 overlay 段）。
4. **下游推论**：#267 已使 OKLO/BTC 两窗 `stream_hash` 改变（`restore-chain-quantification`
   :26/:30）⟹ 依赖旧口径的 π ledger / 声部统计需按 HEAD 重算；MED-1 若修复，占位腿角色会再变一次
   ⟹ 需第三轮对拍，不能与 #267 对拍复用。
5. **谱系引用**：`no-patch-mentality` 090（严格性语法规则：声明膨胀 / 半成品）；
   `formalization-validity-domain` 231 号（有效域 ≠ 定义域——本评审 §4 正是「全称声明的有效域
   小于其定义域」的又一例：「恒剪除」在 #247 restore 语境成立，被搬到 #267 占位语境后有效域缩小
   而声明未缩）。#247 影子评审（`shadow-review-247-20260726.md` S4）在其语境下判 PASS，与本判定
   不矛盾——两者语境不同，S4 的前提是「断链 pid 从不被任何路径物化」。
6. **影响声明**：本评审**只读**，未改任何实装/测试/spec 文件；唯一写入 = 本报告
   `chanlun/review-results/shadow-review-267-20260726.md`。未触碰工作区 ≈370 个 ` D` 条目，
   未修 #110。
