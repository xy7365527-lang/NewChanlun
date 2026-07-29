# 影子评审：#365 小修批八条（MED-B 真 assert + 声明域订正 + 防平凡化，commit `9391f25530`）

- 票：#375（parent #59）；对象：`9391f25530`（`fill.rs` +72 / `runner.rs` +9 / `level_order.rs` +35，共 +104/−12）
- 评审者：独立新上下文（claude opus，**非**实装 lineage）；不采信 resolution 自述，逐条读源码复核
- 工作区：`/tmp/nc-review-365`，detached HEAD=`9391f25530`，开评/收评 `git status` 均干净（中途一次临时探针已按备份还原，见「实测证据」）
- 复跑：`cargo test --release --lib` → **1920 passed / 1 failed**（`classifier::signal::extract_signals_bit_exact_digest_guard`，#115 线在案），与 resolution 声明逐值一致；定向 `level_cap_reclamp_tests`（5/5）、`lee_m4`（2/2）、`lee_m3`（6/6）全绿

## 裁决

**八条全部落地、无功能性缺陷，可收口；但 090「声明与实际一一对应」上留了两处未做全（MED×2 / LOW×3）。**

两处 MED 都是本批**自己的主题**（订正声明域、删无据承诺）在同一文件内漏了一处，不是新引入的错误，也不影响运行行为——建议同批补齐（各一处 doc 改动），不必单开票。

## Spec 轴

| 核对点 | 判定 | 证据 |
|---|---|---|
| ① MED-B 真 assert：release 非零执行 + 构造/非构造分界 | **PASS**（实证） | `fill.rs:725-738` 已是 `assert!`（论证段 `:719-724`）。非平凡执行实测：release 下 `lee_m4_cap_on_...`（`enforce_level_cap=true`，9000 决策点）跑通，我加探针读出 `n_cap_narrowed=159`、`n_rescaled=3` ⟹ `!enforce_level_cap` 短路项为假、`all(...)` 逐级 cap 比较真被求值 159+ 次。分界论证成立：`Σq ≡ t_lee'` 由 `order_units = Σ(targets−planned)` 构造性给出（`plan_gated` 两分支我独立推导过，与 #356 影子评审一致）；cap 护栏依赖「clamp 是最后一个改写 `targets` 的算子」这一接线事实（`fill.rs:664-690` 之后无二次改写，核实过）⟹ 非构造性，升真 assert 正确 |
| ② MED-2 声明域限定双处一致、未引入新裁剪语义 | **PASS** | 生产绑定处 `fill.rs:702-712` 与测试 doc `fill.rs:3098-3101` 双处均限定「真实级别桶」。残差桶前提核实无误：`attribute_total` 仅在 `b_sum==0 ∧ total≠0` 走残差桶（`level_attrib.rs:58-67`，`b_sum==total` 的恒等分支优先），`clamp_levels_to_weighted_cap` 对该桶原样透传（`sizing.rs:132-134`），生产护栏同款豁免 ⟹ 上界回落账户层，登记属实。diff 内无任何裁剪逻辑改动（只增注释） |
| ③ MED-3 文档锚 ↔ `plan_gated` 实际计算一致 | **部分 PASS** | 计算侧核实：`struct_gap = Σgated_basis − target_total`（`level_order.rs:529`），`target_total` 入参即 `t_lee_raw`（`fill.rs:646` 传入 `p_star_lee.round()`）⟹ 锚 `t_lee_raw` 属实，「reclamp 后不重算」属实（`..plan` 带过，`fill.rs:674/684`）。字段 doc（`level_order.rs:370-383`）与 `plan_gated` doc（`:520-524`）订正正确。**但漏一处，见 MED-1** |
| ④ MED-4 钉死值 == 当前真值 | **PASS** | `assert_eq!(targets_sum, cap_0 + cap_1)`（=60），`fill.rs:3175-3182`；release + debug 定向跑绿 ⟹ 真值即 60。旧 `< 100` 的平凡化风险论证成立（合计压到 0 也满足） |
| ⑤ LOW-2 与生产护栏同口径 | **PASS** | 测试 `fill.rs:3158-3172` 用 `level_cap(lvl, base_units, &risk).floor().max(0.0) as i64`，与生产护栏 `fill.rs:725-738` 及施加点 `sizing.rs:135-137` **逐字同式**；残差桶 `continue` 豁免与生产 `lvl == LEVEL_ACCOUNT_RESIDUAL ||` 同一口径 |
| ⑥ 条 6 端到端断言真实执行（非空转） | **PASS（当下）**，见 LOW-1 | 探针实测该 fixture：`n_levels_off_clock_delta=150`、`unexplained=0`、`n_cap_narrowed=159`、`n_decisions=9000` ⟹ 断言有真实内容，非平凡通过。缺**钉死的**非平凡前置 |
| ⑦ 两 assert 契约表述准确 | **PASS** | `clamp_levels_to_weighted_cap` 是逐级 `map`（`sizing.rs:129-139`）⟹「保序等长」属实；`zip` 对不等长静默截断、尾部被裁级别漏出本表 ⟹ 逐级判据误计为未解释违例——方向推导正确（漏出即 `contains` 为假即计 unexplained）。表述无夸大 |
| ⑧ 充要性证明 ⟸/⟹ 两向 + L0/有效域标注 | **PASS**，措辞见 LOW-3 | ⟹（必要）逐句核实成立：`regate` 对无 tick 级别取 `skeleton` 的 planned 前值（`level_order.rs:489-503`）⟹ `gated_ℓ = planned_ℓ`；`!rescaled ⟺ b_sum==total` 恒等分支（`level_attrib.rs:58-60` 返回 `(basis,false,false)`，另两分支 `rescaled=true`）⟹ 逆否成立。有效域标注（L0 / 恒等分支 / 严格小于消费者定义域）符合 `formalization-validity-domain` |

## Standards 轴

| 核对点 | 判定 | 证据 |
|---|---|---|
| 注释密度与票面偏离登记照实 | **PASS** | +104 行中实质代码约 25 行、注释/doc 约 79 行（≈76%）。密度高，但与本仓周边文风一致（`fill.rs`/`level_order.rs` 全文即此密度），且本批八条中三条（②③⑧）票面要求的产出**本身就是文档**，不构成过度展开。commit body 逐条对应票面八条，偏离（超票面一行）明写 |
| 超票面一行（等长 `debug_assert_eq`→真 assert）处置 | **合理但论证不自洽** | 处置本身可辩护（与保序断言同源同形失效，只升一半确实是半成品），登记也照实；但与条 1 自立的分界判据冲突，见 LOW-2 |
| 不可变性 / 命名 / 错误处理 | PASS | 无新 mutation；新增全为 assert 与 doc；`filter` 闭包内加 assert 不改变迭代语义（谓词返回值仍为 `b.1 != a.1`） |
| 行为等价性 | PASS | default（`enforce_level_cap=false`）路径：新 assert 短路为真、`levels_narrowed_by_cap` 契约恒真 ⟹ 零行为变化；基线 1920/1 与父 commit 逐值相同 |

## 问题

**MED-1｜条 3 的锚定订正漏了 `LevelOrderStats::max_abs_struct_gap` 字段 doc——而那是 #362 MED-3 点名两处之一**
`level_order.rs:206-212` 仍逐字写「`max |Σ_ℓ basis^gated_ℓ − T_lee|`……级别结构目标与**账户层投影后物理目标**的分歧幅度」，既未订正为 `t_lee_raw`，也无指向新订正段的交叉引用。而在帽 binding 的决策点上，「账户层投影后物理目标」已是二次裁剪后的 `t_lee'`，本读数量的不是它——这正是 #362 MED-3 的原指控（原文点名 `level_order.rs:194-196/311-312` 两处，本批只订正了后者所在的 `LevelOrderPlan::struct_gap` 与 `plan_gated`）。`max_abs_struct_gap` 是 stats 侧的**对外读数**，消费者（`runner.rs` 验收、`bin/theta_overlay.rs` 打印）先读到的就是这一处。
建议：该字段 doc 补一句锚 `t_lee_raw` + 链到 `LevelOrderPlan::struct_gap`。（模块头 `:55`/`:118`/`:166` 同款措辞属 M2 历史文案，可不动，但若一并处理更彻底。）

**MED-2｜条 8「删承诺」不彻底：下游字段 doc 仍承诺「升序」，正是本条注释自己反对的「换个位置再声明一遍」**
`union_sorted_levels` 的 doc 已按本条删掉输入升序前置，并明写「同理不承诺**输出**升序……写进 doc 就是把删掉的膨胀换个位置再声明一遍」（`fill.rs:561-571`）。但它的唯一去处 `LevelOrderPlan::cap_narrowed_levels` 字段 doc 首行仍是「被 M4 级别帽真实裁剪过的级别（**升序**、去重……）」（`level_order.rs:332`）。该承诺同样无校验、无消费者（唯一消费者 `observe_decision` 用 `contains`，`level_order.rs:578`），且其生产者已显式不保证顺序 ⟹ 与本条要修的是同一个声明膨胀，只是位置在下游。
实际取值当下确为升序（`levels_narrowed_by_cap` 保持输入序，而 `gated`/`targets` 由 `merge_levels`/`attribute_total` 维持 level 升序），但那是**未声明的实现巧合**，正是 090 要求删掉的那类。建议：字段 doc 同样删「升序」二字（改「去重，顺序不承诺」）。

**LOW-1｜条 6 的端到端断言没有钉死的逐级非平凡前置——与同批条 4「防平凡化」不同待遇**
`n_cap_narrowed` 的字段 doc 自己写着这条警告：「`per_level_sparsity_has_no_unexplained_violation()` 在 `n_cap_narrowed==0` 时可能只是『帽从未 binding』的平凡通过」（`level_order.rs:200-202`）。新增断言（`runner.rs:3038-3043`）未配任何逐级前置。已有的 bar 级前置 `n_orders_off_structural_clock > 0` **不蕴含**逐级非零：`has_structural()==false` 时 `ticked_levels` 仍可非空（风控钟点不计入 structural，`level_clock.rs:177/187`），此时 off-clock 的级别可全部被 tick 覆盖。
当下不成立空转（我实测 `n_levels_off_clock_delta=150 > 0`、`n_cap_narrowed=159 > 0`），故不影响本批验收；但上游漂移后可静默转平凡绿——这正是条 4 对 `targets_sum < 100` 的同一指控。建议补一行 `assert!(s.n_cap_narrowed > 0, ...)`（或 `n_levels_off_clock_delta > 0`）。顺带：`n_cap_narrowed` 字段 doc 的「跑批验收侧尚未接入本读数」在本批后仍然成立（runner 接的是 unexplained 而非 `n_cap_narrowed`），措辞无需改。

**LOW-2｜条 1 与条 7 的 assert 分级判据不自洽（注释里的分界 ≠ 实际采用的分界）**
条 1 立的分界是「构造性恒等 ⟹ `debug_assert` 足够；非构造性 ⟹ 真 assert」（`fill.rs:719-724`）。条 7 升级的两条（等长、保序）在票面与注释里都被明确称为 clamp 的**构造性契约**，且其调用点就在同一函数内、实参就是 `clamp(plan.targets)` 本身（`fill.rs:667-672`）⟹ 按条 1 的判据应留 `debug_assert`。实际取真 assert，理由是「后续接线可变」——但那与条 1 给 cap 护栏的理由（依赖接线事实）是**同一类**理由，而不是「构造 vs 非构造」。
即：真正在起作用的分界是「同函数内构造性 vs 跨接线可破」，注释写的却是「构造 vs 非构造」。处置本身我不反对（代价 O(级别数)/bar，可忽略），但按 090 应把条 1 的分界措辞订正为实际判据，否则下一个人按注释推导会得出与本批相反的结论。

**LOW-3｜条 8 的 ⟸ 方向书写弱于其结论所需（结论正确，措辞欠一步）**
⟸ 项写作「ℓ 被帽裁 ⟹ `targets_ℓ` 被改写离开 `planned_ℓ` ⟹ `Δq_ℓ` **可**非零」，而下一行的结论是 ⟺（等价），需要的是「**必**非零」。该更强的结论实际可证：无 tick ⟹ `gated_ℓ = planned_ℓ`（`regate`）；恒等分支 ⟹ clamp 前 `targets_ℓ = gated_ℓ`；clamp 改写它 ⟹ `targets_ℓ ≠ planned_ℓ` ⟹ `Δq_ℓ ≠ 0`。把「可」改成「必」并补这一句即可闭合（顺带可注明：第二施加点在恒等分支下因 clamp 幂等而不可能触发，故 ⟸ 只需对第一施加点论证）。

## 实测证据（我方独立跑，不采信自述）

1. `cargo test --release --lib`：`1920 passed; 1 failed; 134 ignored`，唯一失败 = `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（摘要不符，#115 线在案），与 resolution 声明一致。
2. 定向：`level_cap_reclamp_tests` 5/5、`lee_m4` 2/2（`LEE_M4_SPARSITY off_clock=145 explained=145`；`LEE_M4_CAP baseline 8055/48 | capped 761/169`）、`lee_m3` 6/6（`LEE_M3_PERLEVEL off_clock_delta=1 unexplained=0`）。
3. 逐级读数探针（临时插 `eprintln`，跑后由备份 `cp` 还原，`git status` 复检干净，无 git mutation）：cap-on 9000-bar fixture 上 `perlevel_delta=150 unexplained=0 n_cap_narrowed=159 n_rescaled=3 n_decisions=9000` ⟹ ①的 release 执行与⑥的非空转均坐实。

## 结果包六要素

1. **结论**：#365 八条全部落地、无功能缺陷，release 基线不破；两处 MED（条 3 漏订一处 stats doc、条 8 删承诺未覆盖下游字段 doc）+ 三处 LOW（条 6 缺非平凡前置、条 1/7 分界论证不自洽、条 8 ⟸ 措辞弱）。
2. **定义依据**：`.claude/rules/no-patch-mentality.md` 090「声明与实际一一对应 / 禁声明膨胀」；`formalization-validity-domain` L0 与有效域标注；`level_order.rs:156-161`「恒等证据须 release 非平凡可读」（#289 纪律）；#365 票体八条 + #375 核对点。
3. **边界条件**：MED-1/MED-2 的判定会翻转，若能证明 `max_abs_struct_gap` 的消费者只在 cap-off 路径读、且 `cap_narrowed_levels` 的升序被某处校验或消费——我全仓 grep 未见（消费者仅 `contains` 与 `is_empty`）。LOW-1 会翻转，若能证明 `has_structural()==false ⟹ ticked_levels 为空`（源码不支持：风控钟点非 structural 但计入 ticked）。①的 PASS 会翻转，若 cap-on 测试从 release 套件中被移出（届时 assert 回到零执行）。
4. **下游推论**：本批不改变任何数值行为（default 路径逐字节退化、cap-on 路径只增断言），M0–M8 既有基线与 #135 口径不受影响；`struct_gap` 的锚定语义澄清后，任何以该读数做「帽吃掉多少」归因的分析须知它不含二次裁剪量（那部分在 `cap_narrowed_levels`）。
5. **谱系引用**：090（严格性/声明膨胀）、231/`formalization-validity-domain`（有效域 ≠ 定义域，本批条 8 标注符合）、#289→#356→#362→#369 的「release 可读证据」链条。未发现需新开谱系的概念分离。
6. **影响声明**：本次评审为只读，未改动仓库任何文件；唯一写入 = 本报告。评审期间在 `/tmp/nc-review-365` 插入过一行临时 `eprintln` 探针并已按备份还原（`git status` 干净）。
