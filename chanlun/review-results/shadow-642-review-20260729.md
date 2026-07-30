# 影子评审 #692 — #642 coverage 域 12 提交语义重放（双轴）

评审面：`git diff 08585763ae^1 08585763ae`（merge `08585763ae`，7 commit）。
工位：`/private/tmp/wt-692`（分支 `shadow/642-review`，main 尖端 `23a1869849`）。
对照面：`/private/tmp/kimi-nest-mainline`（封存只读，本评审全程未写入）。
评审器与交付方无重叠上下文（新会话，非自评）。

## 判词

| 轴 | 判词 |
|---|---|
| **Standards**（仓内编码纪律：关票门、语义重放留痕三段式、测试纪律） | **PASS WITH CONDITIONS** |
| **Spec**（票面 12 件表 + 验收三条） | **PASS WITH CONDITIONS** |

发现分级：**HIGH ×1、MED ×3、LOW ×4**。

放行条件（两轴合并）：
1. HIGH-1 必须先处置——补移植 kimi `#446` 的 restore 侧候选段过滤，或用运行时反例坐实其在 main 侧不可达并写入留痕；
2. MED-1 补 `#512` 防线的反例红测（或显式声明「新增 release 防线零见证」并降级该防线的可信度声明）；
3. MED-2/MED-3 是文档与代码互相矛盾，改文档即可，但必须改——两处都是「读者据此判断已有防线/无接线面」的承重声明。

## 已实测核销（本评审独立复跑，非采信申报）

| 项 | 申报 | 实测 | 结论 |
|---|---|---|---|
| 基线计数 | 2550/0/138 | `08585763ae^1`（`e75b341e76`）`cargo test --lib` → **2550 passed / 0 failed / 138 ignored** | 逐位一致 |
| 终态计数 | 2557/0/138 | `08585763ae` → **2557 passed / 0 failed / 138 ignored** | 逐位一致 |
| 净增 7 | #315+1/#350+1/#310+4/#346-347+1 | diff 内新增 `#[test]` **恰 7 个**，名单与申报逐条对应 | 一致 |
| 逐 commit 可编译（关票门第 4 子句） | 「前六件可编译」 | 六件逐个 `cargo test --lib --no-run` 全部 `Finished`（`b80e2ac52e`/`4419710e2a`/`203c0d8030`/`ff9db6fbca`/`6c70adce51`/`67f0f2254e`）；`890f75cf09` 纯文档 | 属实 |
| 工位尖端计数 | — | 本工位（main 尖端）2573/0/138，差 +16 来自 #642 之后并入的票，非本票 | 无新增红 |

编译告警核查：`unused variable: reg`（`ancok_tests.rs:121`/`step_tests.rs:91`）、`standard_p_star`（`fill.rs:3960`）均为存量，不在本次 diff 触及文件内，不计入发现。

对拍核销属实的三件（Spec 轴第 2 点）：
- **#247**（`b57da4bde4`）：main 侧 `held.rs:11-12/168-174` 有三来源 `∪RegistryRestore` 声明；Lean 侧 `ActiveSet.lean:311-312` 与 kimi 该提交的注释段落逐字同源，且 main 多出 `§5′`（`ℛ_x=∅` 有效域限定，`ActiveSet.lean:374-401`）——**比 kimi 更详尽**，核销属实。
- **#267**（`8bc7c67e98`）：main `held_stale_reregister_idx` 的 `#247 C2` 角色重建（`held.rs:375-399`）在本票之前已在场，核销属实。
- **#266**（`ecb8923c19`）：MED-3（链域声明）有等价表述（`ancok.rs` 的 `element_depth_fuel_exhausted` doc「#247 缺口二回填首次让 restore 元素的 parent 可指向 overlay」）；MED-1 未落，见 LOW-3。
- **#359 判「不适用」**：kimi 该提交清理的 22 个死重导出逐个对应其自身 `59248b301b` 拆分边界，main 拆分独立且边界不同，无语义内容可移植——核销属实。
- **#346/#347 的 E 项自我登记**：实测 `ancok.rs` 的 `ancestors_by_id_lookup`（`while let Some(pid)` 上溯，`ancok.rs:162`）确无自环排除守卫——报告「已核实但未修复」如实。

---

## HIGH-1 — `#446` 的 restore 侧候选段过滤未移植，且未作差异声明；可构造 raw 同 ElementId 双槽

**文件锚**：`rust/src/theta_v0/strategy/coverage/held.rs:320`
**对照锚**：kimi `rust/src/theta_v0/strategy/coverage/held.rs`（restore 复用判定带 `.filter(|&idx| idx >= overlay_cand_end)`，参数 `overlay_cand_end` doc「★#446：初始候选段终点。restore 只能复用 base 或此边界之后的持久 overlay；候选拷贝的方向/坐标/parent_id 不是 registry 持久身份」）
**留痕锚**：`chanlun/review-results/issue642-coverage-replay-20260729.md:29-32`（条目 1 的「未移植」声明）

kimi `f838540eff`（#446）的 commit message 列了三项修复：①open 候选按 ElementId 注册（同向首现去重／反向逐对湮灭）；②**restore 仅复用树前缀／持久 overlay、不复用候选段拷贝**；③held 复用 restore 槽位时恢复 held 权威方向／坐标／op_parent。

main 侧重放 `b80e2ac52e` 只落了「探针计数 + wverify 逐窗断言」，另三项判为「已有等价机制覆盖」。核查结果：
- ①属实（main 有 `open_candidates_same_carrier_id_reverse_pair_annihilates`，`held_tests_1.rs:484`）；
- ③如实声明为未移植，且等价论证成立（main 的 held 复用分支「不重写属性」，因 main restore 的 push 属性直接取自 registry 条目，`held.rs:364-365`）；
- ②**既未移植，也未在 commit message 与留痕报告中出现**——条目 1 的等价论证通篇只讲 held 侧（`held_stale_reregister_idx`／`op_parent` 重解析），对 restore 侧这一面零字。

main 侧现状（`held.rs:316-324`）：

```rust
if let Some(&existing_idx) = id_idx.get(&pid).or_else(|| overlay_seen.get(&pid)) {
    raw.push(existing_idx);
    cur = work[existing_idx].parent_id; // 沿已有元素的结构父链上溯
    continue;
}
```

`overlay_seen` 在进入 held 循环前已把**整个候选段**登记（`step.rs:129-135`，`for i in candidate_start..work.len()`），而 `overlay_cand_end` 恰是候选段终点（`step.rs:140`）。main 自己在 `step.rs:136-139` 与 `held.rs:369-373` 完整论证过为什么候选拷贝不可复用：「候选 eps 是信号方向（可与持仓反向）、lambda==rho 点元素、parent_id 非本腿 op_parent；复用会让 `element_as_leg` 采纳候选属性（持仓方向静默翻转、I4 op_parent 失真）」。该门禁只施加在 held 侧（`held.rs:420-421` 的 `Some(&existing) if existing >= overlay_cand_end`），restore 侧同一张 `overlay_seen`、同一风险，**无门禁**。

**失败场景（静态路径推导）**：本 bar 存在两条持仓腿 A、B 与一个 open 候选，B 的 ElementId 同时是 A 的操作祖先链上一环、也是该候选的 carrier（即「同 carrier 反手／加仓」——main 已有 `held_leg_id_hits_candidate_copy_keeps_held_identity`（`held_tests_1.rs:637`）专测 held 侧的同一碰撞，可达性同级）。held 循环先处理 A：`restore_ancestor_chain_from_registry` 上溯到 B 的 pid，此时候选段元素在 `work` 但**尚未进 `raw`**（候选推送在 open 循环，位于 held 循环之后，`step.rs` 约 340-396），故 `already_in_raw` 不命中 → 命中 `overlay_seen[B]`=候选段 idx → `raw.push(候选段 idx)`，并以**候选的 `parent_id`**（非持久 `structural_parent_id`）继续上溯。随后 held 循环处理 B：`held_stale_reregister_idx` 见 `existing < overlay_cand_end` → 走新 push 分支，push B 的持仓身份元素并入 `raw`。此时 `raw` 内出现同一 ElementId 的两个 idx（候选段槽 + held 槽）。

三重后果：
1. **祖先链走错**：`cur = work[候选段 idx].parent_id` 与持久 `structural_parent_id` 不同（main 自己 `held.rs:371` 断言「parent_id 非本腿 op_parent」）⟹ 上溯分支偏离，可能提前收敛（候选 `parent_id=None` 时直接终止）⟹ 真祖先未物化 ⟹ AncOK 误剪／误留；
2. **角色输入失真**：raw 内代表该祖先的是候选拷贝，`element_as_leg` 采纳候选 eps／λ==ρ ⟹ σ_p／depth 权重错 ⟹ `p̃` 偏（正是 #247/#446 反复处理的那一类数值污染）；
3. **`#512` 防线被自己打穿**：若两槽的祖先链都齐、都通过 AncOK，`next_idx` 就含重复 ElementId ⟹ `67f0f2254e` 新落的 `panic!`（`step.rs:521`，release/debug 均炸）在生产跑批中触发 —— 这条防线本意是「重复按构造不可能」，而缺失的正是使其成立的那条构造性前提。

**严谨性标注**：**PLAUSIBLE，非 CONFIRMED**。本评审只做静态路径推导与代码锚定，未构造运行时反例（评审器不改交付方代码）。但 kimi 侧对同一位点明确落了过滤、main 侧对同一风险在姊妹路径上落了门禁而此处没有——这两点使「主张不可达」成为交付方的举证责任，而目前留痕报告连这一面都未提及。

---

## MED-1 — `#512` 新落的 release fail-loud 防线零测试覆盖，且未声明测试未移植

**文件锚**：`rust/src/theta_v0/strategy/coverage/step.rs:504-527`（新 `panic!` 块）
**对照锚**：kimi `rust/src/theta_v0/strategy/coverage/step_tests.rs:249` `duplicate_active_id_panics_with_id_indices_and_sources_in_release`（`catch_unwind` + 三段 payload 断言：重复 ID 文本、`idx 0(boundary-root-retain)`、`idx 1(boundary-root-retain)`）
**留痕锚**：`chanlun/review-results/issue642-coverage-replay-20260729.md:164-178`（条目 12）

`67f0f2254e` 的 diff **只改 `step.rs`**（55+/19−，零测试文件）。kimi `8d8895c652` 的 commit message 明写「反例红测能咬（修前返回双腿 p_tilde=1200）」，其 `step_tests.rs` 的 `catch_unwind` 测试是该防线唯一见证；main 侧未移植，且条目 12 的差异声明只列了「golden v2 schema／T3 报告／Spec MED 路径订正」这些域外产物，**未提及这条 coverage 域内、与所落代码直接配对的回归测试**。

`67f0f2254e` 的 commit message 自承「panic 未在任何既有测试路径触发，符合预期」——把「无测试覆盖」写成了预期符合项。后果：`panic!` 的 `source()` 闭包（`step.rs:512-520`，三段 `candidate_start`／`overlay_cand_end`／`appended_source` 分类）以及 `appended_source` 的 5 个标记点（`step.rs:223/247/256/278/391`）全部零执行路径见证——若归类逻辑写错（例如 `appended_source` 漏标某条 push 路径而落进 `post-overlay-unregistered`），要等生产 panic 时才发现，而那正是最需要准确来源的时刻。

---

## MED-2 — 留痕报告与生产 doc 断言 main 侧「接线点完全不存在」，与代码不符（字段名亦写错）

**文件锚**：`rust/src/theta_v0/strategy/level_order.rs:384`（`pub cap_narrowed_levels: Vec<u32>`）
**声明锚**：`chanlun/review-results/issue642-coverage-replay-20260729.md:66-69`、`:182-186`（停手项 1）；`rust/src/theta_v0/strategy/coverage/sizing.rs`（`clamp_levels_to_weighted_cap` doc 的 `#642 语义重放偏差照实声明`段）

两处声明的原文是：「该函数、以及文档提到的 `plan_level_gated_order`/`LevelOrderPlan.capped_levels` **从未在代码中实现**（纯文档承诺）」／「main 侧 `level_order.rs` 文档描述的接线点（`capped_levels` 字段、两处施加点差集）**目前均不存在于代码**」。

实测：
- `fn plan_level_gated_order` 确不存在（`grep -rn "fn plan_level_gated_order" rust/src/` 零命中）——这一半属实；
- 字段名不是 `capped_levels` 而是 **`cap_narrowed_levels`**，且**存在**（`level_order.rs:384`），并有完整消费链：`LevelOrderStats::n_cap_narrowed`（`level_order.rs:205/640-641`）、逐级 sparsity 判据 `plan.cap_narrowed_levels.contains(&lvl)`（`level_order.rs:593`）、m8 报表列（`backtest/wverify_run/m8.rs:624/668/675`、`report.rs:376-381`）；唯一填入者缺位（`plan_gated` 恒置空表，`level_order.rs:545-553`）。

为何是 MED 而非 LOW：这条声明是「#310 接线为何可以不做」的全部理由（停手项 1 据此建议另开票 #693）。真实形态不是「零实现、需从头设计接口」，而是「字段／统计／报表／判据四层俱在，缺一个生产者」——接线成本与风险评估被这条错误陈述系统性放大。附带效应：`n_cap_narrowed` 恒 0 使 m8.rs:617-675 自己写下的「非平凡性」判据处于平凡通过态（该文件已如实注明「帽关时不断言」，故不另计发现）。错误陈述同时写进了生产代码 doc（`sizing.rs`），不止在报告里。

---

## MED-3 — `#512` 之后 `wverify_run.rs` 的逐窗硬断言成为不可达断言，注释宣称的「降级为纯观测」并未执行

**文件锚**：`rust/src/theta_v0/backtest/wverify_run.rs:1295-1306`（`assert_eq!(duplicate_id_violations, 0, ...)` 与报表留痕行仍在原样）
**声明锚**：`rust/src/theta_v0/strategy/coverage/step.rs:503`（「m8 wverify_run 的窗口后置断言相应降级为纯观测，防线单一化」）；`67f0f2254e` commit message 同句

`#512` 把唯一性检查前移到 `next_idx` 且改 `panic!` 后，任何一次违规都在 coverage 内部当场炸；`ancok_probe` 的 `duplicate_active_id_violations` 只可能在 panic 展开后被 `catch_unwind` 读到。因此 `wverify_run.rs:1300` 的逐窗 `assert_eq!(…, 0)` **永远不可能观测到非零**——它成了恒真断言、零鉴别力。

问题不在于留着它（无害），而在于三处声明与之矛盾：
1. `step.rs:503` 与 commit message 都声称该断言「已降级为纯观测」，实际代码未动，仍是 `assert_eq!`；
2. `wverify_run.rs:1295-1296` 的注释仍以「`debug_assert!` 在 release 被编译消除，此计数不依赖构建 profile」为存在理由——该理由已被 `#512` 取代；
3. 逐窗写入 m8 报表的留痕行 `<!-- #446 {tag}: duplicate_active_id_violations=0（逐窗硬断言，见 stderr） -->`（`wverify_run.rs:1304-1306`）会让报表读者认为存在一道**独立的窗口级防线**，实则与 coverage 内部 panic 是同一道防线的下游影子。

---

## LOW-1 — `#310`「与 kimi 同名函数逐字等价」缺时点限定

**锚**：`rust/src/theta_v0/strategy/coverage/sizing.rs`（`clamp_levels_to_weighted_cap` doc）、留痕报告 `:71-73`

实测对照：与 kimi **`7d8b45be70` 时点**的函数体逐字等价（`git show 7d8b45be70:rust/src/theta_v0/strategy/coverage.rs` 的 `clamp_levels_to_weighted_cap` 无 assert、无残差桶分支，与 main 落地体一致）——这一层属实。但与对照面**尖端**不等价：kimi 现版含 `assert!(level_weights_sum_le_one(risk), …)`（release 生效的配置校验）与 `LEVEL_ACCOUNT_RESIDUAL` 残差桶跳过，两者均由 `19aea33a26`（#351）加入。报告条目 5 已如实声明 #351 未做，故整体不构成隐瞒；但「逐字等价」写在生产 doc 里而不带时点，后续读者按对照面尖端核对会判为不符。附带：kimi 侧该函数 5 个单测，main 移植 4 个，未移植的第 5 个（`clamp_levels_to_weighted_cap_rejects_misconfigured_weights_over_one`）正是 #351 的测试，与声明自洽。

## LOW-2 — `#315` 的探针合并未声明，且新 doc 引用了 main 侧不存在的字段名

**锚**：`rust/src/theta_v0/strategy/coverage/ancok.rs:58`（doc 引用 `placeholder_parent_unresolved`）；`held.rs:481-484`（unresolved 分支 bump `restore_parent_unresolved`）

kimi `2ca040d9fa`（#315）新增了**独立**探针 `AncokProbe::placeholder_parent_unresolved`（held 占位统一 fixup 时点父不可解析），与 restore 链的 unresolved 分开计。main 重放把两者合并进既有 `restore_parent_unresolved`——这与 main 修复前的既有口径自洽（main 原来就在 `held_stale_reregister_idx` 里 bump 同一计数），是一个可接受的选择，但留痕报告条目 3 未声明这一差异，探针可分辨性的下降（held 占位 vs restore 链无法分离读数）无处可查。

同时 `ancok.rs:58` 的新 doc 写「`restore_parent_unresolved`/`placeholder_parent_unresolved` 命中的 idx 中…」——`placeholder_parent_unresolved` 在 main 侧**不存在**（全仓仅此一处 doc 提及），是从 kimi 侧带过来的悬空引用。

次要（同批，不另计）：原 restore 内部的 `restore_parent_rebound` bump 在 `if let Some(e) = work.overlay_mut(i)` **内部**，新 `resolve_pending_parent_fixups`（`held.rs:453-488`，bump 在 `:479`）把 bump 移到 `overlay_mut` 之外——`overlay_mut` 返回 `None` 时（理论上不可达，pending 内均为 overlay 段新 push）会计一次未实际写入的 rebound。

## LOW-3 — `#266` MED-1（头表第三来源归属）在 main 侧未落，核销理由的「已包含等价表述」只覆盖其中一条

**锚**：`rust/src/theta_v0/strategy/coverage/mod.rs:18`；留痕报告 `:111-115`

kimi `ecb8923c19` 两行订正中：MED-3（`element_depth` 链域声明）有等价表述（见上文「已实测核销」），核销属实；MED-1 针对的是头表第 2 行「把第三来源归给 `active_set_step`」这一自相矛盾——main 侧同一张头表同一行现为 `| 2 活动集 | M16 … AncOK[(A_t∖D_t)∪B_t] | active_set_step（先关后开 + 祖先闭合）|`，既未标第三来源、也未区分「M16 理想式原语」与真生产路径（`coverage_step_from_buckets_sep` 从未出现在头表）。实质内容确已在 `held.rs:11-12/168-174` 声明（故为 LOW、非 MED），但报告「已包含等价或更详尽的表述…无需移植」对这一条不成立。

## LOW-4 — 关票声明中「分段测试计数在留痕报告」指针指错

**锚**：#642 关票评论「逐 commit 可编译性=前六件可编译（分段测试零新增红在留痕报告）」；`chanlun/review-results/issue642-coverage-replay-20260729.md:10-17`

留痕报告的基线／终态表只有两行（2550 / 2557），无逐 commit 计数；分段计数（2551 / 2552 / 2556 / 2557 / 2557）分散在各 commit message 里，`b80e2ac52e` 未报计数。可编译性本身已实测属实（六件全 `Finished`），故仅为指针问题；但关票门第 3 子句要求「每一条声明在关票时都已为真」，一条指向不存在内容的指针使该声明不可当场核对。

---

## 未发现问题的核查项（备查）

- `resolve_pending_parent_fixups` 生产调用点唯一（`step.rs:404-405`），位于两个物化循环之后、AncOK 之前；两个 push 点（restore `held.rs:352`、held 占位 `held.rs:435`）汇入同一累加器，4 处生产传参（`step.rs:220/244/253/388`）全部传同一 `pending_parent_fixup`，无「push 后无人 resolve」的泄漏路径。
- `#350` 把 restore 的立即修补整段移出后，3 处既有单测（`held_tests_1.rs:97/140-144/409-413`）改为显式补跑生产 `resolve_pending_parent_fixups`，未复制解析逻辑——测试与生产同源，符合仓内纪律。
- `#315`／`#350` 两个移植回归测试都带 RED／GREEN 双向数值论证（`held_tests_2.rs:621-690`、`:691-769`），且 `#281` 的 `ShortDiff`→`ReverseOpen` 更名差异在测试 doc 中显式标注。
- `#346/#347` 的交叉核对（`step.rs:488-496`）只读 probe、不改控制流；`#358` 的「恒为 0 加非环形限定」确已写入 `ancok.rs:57-63`。
- `level_cap`／`clamp_levels_to_weighted_cap` 未接生产（`enforce_level_cap` default=false，`config.rs:229`），零行为改变声明成立。
- 碰撞面声明（留痕报告 `:192-199`）核对无误：本次 diff 全部落在 coverage/ 七文件 + `wverify_run.rs`，未触 #668/#671/#676 声明面。

## 一句话小结

P0 三修复的两个时序孔（#315/#350）重放得扎实——延后 fixup 的形状、三级解析的 raw 扫兜底、RED/GREEN 双向数值论证都到位，逐 commit 可编译与 2550→2557 的计数在本评审独立复跑下逐位吻合；真正的问题集中在 #446：它的三项修复里，「restore 不复用候选段拷贝」这一项既没移植、也没进差异声明，而 main 自己在姊妹路径上写满了「候选拷贝不可复用」的论证，缺的恰是让 #512 那句「重复按构造不可能」成立的构造性前提。其余三条 MED 都是同一类型的问题——声明与代码各说各话（防线声称已降级实则未改、接线面声称零实现实则四层俱在、新落 panic 声称符合预期实则零见证），单条都不重，但它们共同削弱的正是这份留痕报告最主要的产出：让后来者能只读声明就判断哪些面已闭合。
