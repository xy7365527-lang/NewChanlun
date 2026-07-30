# Issue #642 — 12 提交语义重放留痕报告（coverage 域）

工位：`/private/tmp/wt-642`（分支 `ticket-642`，自 main 尖端切出）。
参照面：`/private/tmp/kimi-nest-mainline`（只读，未做任何写操作）。
移植方式：语义重放（非 cherry-pick）——两侧 coverage 模块组织不同（kimi 侧单文件
`strategy/coverage.rs`；main 侧已拆分 `strategy/coverage/{mod,step,held,element,
ancok,leg,sizing,role,compose,shadow}.rs`），逐提交读懂原 diff 语义后在 main 侧对应
位点等价重落。

## 基线 / 终态

| | passed | failed | ignored |
|---|---|---|---|
| 基线（main 尖端 `cargo test --lib`，本次会话实测） | 2550 | 0 | 138 |
| 终态（本报告落盘时） | 2557 | 0 | 138 |

净增 7（#315 +1、#350 +1、#310 +4、#346/#347+#358 +1），零新增红。

## 逐件对照

### 1. #446 f838540eff — 活动集同 ElementId 双计修复 [P0][已完成]

main commit：`b80e2ac52e`

main 侧 restore/held/open 三处注册路径已通过自有 #216/#247 谱系独立实现按 ID 判重
（`held_stale_reregister_idx`、`overlay_cand_end` 候选段隔离），但 `next_active` 唯一性
守卫此前只是 `debug_assert!`（release 编译消除）——kimi 侧 #446 诊断的"生产 release 静默
双计"风险在 main 侧同样成立。补上 `AncokProbe::duplicate_active_id_violations` 计数
（release/debug 都累计）+ `wverify_run.rs` 逐窗断言。**未移植** kimi 侧 `element.rs::
replace_overlay`/held.rs 覆盖 overlay 属性的机制——main 的 `held_stale_reregister_idx`
复用分支已按 `leg.op_parent` 重新解析 `parent`/`attached_dir`（#247 C2 角色输入重建），
不存在 kimi 侧"候选 snapshot 覆盖 held 权威属性"的风险面，判定为已有等价机制覆盖。

（本项后续被 #512 进一步加固为 release/debug 均 panic，见条目 12。）

### 2. #350 760c3520c5 — restore fixup 时序孔 [P0][已完成]

main commit：`203c0d8030`

kimi 侧原提交建立在其 #315（见条目 3）之上，在 monolithic `coverage.rs` 里操作。main 侧
`restore_ancestor_chain_from_registry` 原在函数内自行立即修补（只用本次调用局部状态）。
移除该函数内的立即修补循环，改为把新 push 的 idx 汇入调用方 `pending_parent_fixup`
累加器（与 #315 共用），2 处生产调用点改传该累加器，3 处既有单测改为调用后显式
`resolve_pending_parent_fixups` 补跑。移植回归测试
`restore_chain_ancestor_unresolved_when_shared_ancestor_only_materializes_via_later_boundary_root_push`
（坐实 raw 扫兜底在"父经非 restore 更晚直接 push 物化"场景下仍能补上连接）。

### 3. #315 2ca040d9fa — held 腿占位修补改统一 fixup 形状 [P0][已完成]

main commit：`4419710e2a`

kimi 侧此为 #350 的前置（先在此提交建立 pending 累加器机制，#350 再扩展到 restore 链）。
按 ticket 编号顺序（P0 三修复的第 3 件）单独提交：`held_stale_reregister_idx` 新 push
分支不再立即解析 `parent`/`attached_dir`，改延后到两个物化循环结束、AncOK 判定前统一
`resolve_pending_parent_fixups`。移植回归测试
`held_leg_placeholder_parent_materializes_in_later_iteration`（子在 `prev_active` 中先于
父处理，立即式会误留 `Ambient`/depth=0/q=600；统一 fixup 后正确解析 `ReverseOpen`（main
侧 #281 已将 `ShortDiff` 更名为 `ReverseOpen`，语义不变）/depth=1/q=300）。

### 4. #310 7d8b45be70 — LEE M4 级别 sizing/risk [P1][部分完成]

main commit：`ff9db6fbca`

**对拍发现**：main 侧已有 `strategy/level_risk.rs`（`level_weight`/`level_weights_sum_le_one`）
+ `RiskConfig.{level_weights, enforce_level_cap}` 字段（此前会话已落地），且
`level_order.rs` 文档已多处预写"M4 级别帽实际施加点 = `super::coverage::
clamp_levels_to_weighted_cap`"——但该函数、以及文档提到的 `plan_level_gated_order`/
`LevelOrderPlan.capped_levels` 从未在代码中实现（纯文档承诺，源自 main 自己更晚的
#355/#363/#369/#376 谱系）。

已完成：在 `sizing.rs` 补齐核心数学原语 `level_cap`（协变分解 cap_ℓ=w_ℓ·γ̄·U_ℓ）+
`clamp_levels_to_weighted_cap`（逐级 clamp），语义与 kimi 侧同名函数逐字等价，移植 4
个单测（协变分解守恒/协变缩放/表外级别帽=0/clamp 只裁越界级别）。

**未完成（如实声明）**：kimi 侧 `fill.rs::plan_level_gated_order` 调用点（把本函数接进
`LevelOrderLedger::regate` 输出）未接线。main 侧该接线点目前完全不存在，且它绑定的是
main 自己尚未兑现的文档承诺（#355/#363/#369/#376），要接线需要先决定是否、如何兑现那些
承诺——超出本票"12 提交语义重放"的范围，标记为独立跟进项。两函数当前 `#[allow(dead_code)]`
且未被生产路径调用，`enforce_level_cap` default=false ⟹ 零行为改变。

> **订正（#714 MED-2，2026-07-29，同步 #693；只追加订正，不改写原证词）**：本条目原文
> 「该函数、以及文档提到的 `plan_level_gated_order`/`LevelOrderPlan.capped_levels`
> **从未在代码中实现**（纯文档承诺）」与「main 侧该接线点目前**完全不存在**」均不确，且
> 字段名写错。实测：字段名是 **`cap_narrowed_levels`**（非 `capped_levels`），且该字段
> **存在**（`level_order.rs:384`），并有完整消费链——统计 `LevelOrderStats::n_cap_narrowed`
> （`level_order.rs:205/640-641`）、逐级 sparsity 判据 `plan.cap_narrowed_levels.contains(&lvl)`
> （`level_order.rs:593`）、m8 报表列（`backtest/wverify_run/m8.rs:624/668/675`、
> `report.rs:376-381`）。真实缺口只是**唯一填入者缺位**：`plan_gated`（`level_order.rs:
> 545-553`）恒把该字段置空表，从未调用 `clamp_levels_to_weighted_cap` 填入实际裁剪结果。
>
> 接线成本因此不是「零实现、需从头设计接口」，而是「字段/统计/报表/判据四层俱在，缺一个
> 生产者」——原表述系统性放大了接线成本与风险评估。是否、如何补上这个生产者仍需先对齐
> main 自己 #355/#363/#369/#376 谱系写下的既有接口形状，本订正不改变「留作独立跟进项」的
> 结论，只订正对现状的描述。生产代码 doc（`coverage/sizing.rs::clamp_levels_to_weighted_cap`）
> 同一处错误陈述已同步订正。

### 5. #351 19aea33a26 — M4 级别帽四 MED 补课 [P1][未完成，阻塞于条目 4]

**未做**。整个提交内容（Σw 校验接线、`plan_gated` 归因缩放后二次 clamp、`cap_narrowed`
计数、first-observation 断言强化）建立在 #310 的 `fill.rs`/`runner.rs` 实际接线之上——
条目 4 已如实声明该接线未做，故 #351 无接线对象可补课。kimi 原提交自身也承认"浮出未裁决
（另票）：二次裁剪只改内部归因账本，下单量仍走未二次裁剪路径——物理约束未真正兑现"，
即便在 kimi 侧这条线也未完全闭环。留作与条目 4 接线一并处理的后续项。

### 6. #247 b57da4bde4 — registry 恢复祖先角色输入重建 [P1][对拍完成，无需新增]

**对拍结论：main 侧自有 #247 三提交已完整覆盖，语义等价（部分更优）。**

kimi 此提交的核心内容——`restore_ancestor_chain_from_registry` 循环后统一修补新 push
元素的 `parent=Some(父idx)`/`attached_dir=Some(父eps)`，断链不伪造（AncOK 恒剪除），∂
根保持 None/None——在本会话开始处理条目 1-3 之前，main 侧就已经独立实现（`held.rs` 中
大量 `#247 缺口二`/`#247 C1/C2/C3` 文档锚点，且实现比 kimi 此处的初版更进一步：main 已经
把角色重建同步应用到 held 腿占位自身，即 kimi 侧 #267/#371 才补上的内容）。

Lean 侧对拍：`formal/Origin/ActiveSet.lean`/`AncestorClosure.lean` 已含 `∪RegistryRestore`
第三来源声明（`A_{t+1} = AncOK[(A_t∖𝒟_x^†)∪ℬ_x∪ℛ_x]`），与 kimi 此提交对 Lean 侧的纯注释
补充逐字一致。无需移植。

### 7. #267 8bc7c67e98 — held 腿占位元素角色输入重建 [P2][对拍完成，无需新增]

kimi 此提交把 #247 的角色重建从"仅恢复链元素"扩展到"held 腿占位自身"。main 侧
`held_stale_reregister_idx` 在本会话 #315/#350 之前就已经内建这一层（op_parent 解析
parent/attached_dir），即 main 早于本次移植就已站在 kimi #267 之后的位置。本会话的
#315 修复进一步把它从"立即解析"升级为"统一延后 fixup"（时序孔修复，kimi 侧对应
#350 才做到）。无需额外移植。

### 8. #266 ecb8923c19 — 头表第三来源归属 + element_depth 链域声明 [P2][对拍完成，无需新增]

纯文档订正（2 行，零行为变化）。main 侧对应文档（`coverage/held.rs`/`ancok.rs` 中
`#247`/`element_depth` 相关 doc）已包含等价或更详尽的表述（"parent 链不再恒在 base"、
"∪RegistryRestore 归生产路径"等）。无需移植。

### 9. #346/#347 c3cd34bcea — 合成数据波形/真结构护栏/config 私有化/probe 交叉核对/环形守卫 [P2][部分完成]

main commit：`6c70adce51`

已完成 D 部分：新增 `AncokProbe::placeholder_pruned_by_ancok` 计数，
`resolve_pending_parent_fixups` 改为返回未解析 idx 列表，`step.rs` 在 `next_idx` 算出后
交叉核对（不在 `next_idx` 即被剪除 ⟹ 计入该 probe），使"unresolved 可与 AncOK 剪除计数
交叉核对"这条此前无对应实现的文档声明变得可执行。新增回归测试
`placeholder_pruned_by_ancok_cross_check_matches_unresolved_on_true_lost_chain`。

**未做（A/B/C/E 部分，如实声明）**：
- A（`incremental.rs` 合成数据波形订正）、B（`OwnedIncrementalClassifier.bar_count()`
  真结构锁步护栏）、C（`ThetaCore.config` 私有化）——均是与 coverage 域正交的独立
  bug-fix，触及 `backtest/incremental.rs`、`classifier/streaming.rs`、
  `nautilus/strategy.rs`，不在本票"coverage 域 12 提交"范围内，未评估 main 侧对应
  文件现状，留作独立跟进项。
- E（`ancestors_by_id_lookup` 环形守卫）——main 侧该函数（`ancok.rs:145`）目前**没有**
  `r != idx` 自环排除守卫（我在 #350/#347 的 `resolve_pending_parent_fixups` 里加的
  自环守卫是另一处、独立的函数）。已核实但未修复：`ancestors_by_id_lookup` 标注为
  "非生产入场，GAP-5 note"（M16 区间递归原语对齐用，非 `coverage_step_from_buckets_sep`
  生产路径消费），风险面小于生产路径，但如实登记此处环形数据下确实可能死循环，留作
  独立跟进项。

### 10. #358 ea027130f7 — 评审浮出小修批 [P2][已随条目 9 一并完成]

- "「恒为 0」加非环形限定"：已在新增的 `placeholder_pruned_by_ancok` doc 中写入
  （"非环形 parent_id 数据下该差值结构性恒为 0"）。
- "删测试 helper 委托壳"：main 侧本就没有 kimi 那个"只差一个 s"的委托壳
  `resolve_pending_parent_fixup`（单数）——`held_tests_1.rs` 三处调用点本次移植时
  直接调用生产版 `resolve_pending_parent_fixups`（复数），未产生需要删除的委托壳。
- "assert 上移防 wrap"：kimi 侧该修复位于 `runner.rs:5105-5115`（m8 报表打印）—— main
  侧 `wverify_run.rs` 目前没有等价的 `pruned <= unresolved` 断言（本次 #446 只加了
  `duplicate_id_violations==0` 断言），无对应代码可修，未做。

### 11. #359 28773fbc99 — 删 22 死重导出 + build_tree_id_index 归位破环 [P2][不适用]

**判定：不适用于 main 侧。** 该提交是 kimi 把 `coverage.rs` 单文件拆分为 7 模块
（`59248b301b`）之后，针对**那次拆分产生的**22 个死重导出做的清理——名单（`raw_active_set`/
`ancestor_close`/`push_element_tree`/`operation_role_indexed`/`theta_dir_slot`/
`feasible_lex_candidates` 等）逐一对应 kimi 自己的模块边界。main 侧模块拆分在本票开始前
就已独立存在、边界不同，从未有这些具体的死重导出（例如 main 的 `build_tree_id_index`
已经在 `element.rs` 且未重复导出）。没有语义内容可移植，只是 kimi 自己重构历史的收尾。

值得记录的交叉验证：该提交明确把 `level_cap`/`clamp_levels_to_weighted_cap`
（条目 4）列入"评审列 30 名单中不是死导出"的 8 个例外——因为 kimi 侧它们被 `fill.rs`
消费。这佐证了条目 4 中"这两个函数本该被接线消费，只是 main 侧接线未做"的判断。

### 12. #512 8d8895c652 — release 防线升 fail-loud + golden v2 [P2][coverage 部分已完成]

main commit：`67f0f2254e`

kimi 此提交是对自己 #446 的返修（#511 评审打回：debug_assert-only 防线在 release 下
仍会静默放行双计）。本次移植时序上 #446（条目 1）先于 #512 处理，故 #446 落地时用的是
`debug_assert!`+计数的中间态；发现条目 12 后立即用 #512 的设计回头加固：唯一性检查从
`next_active_idx`（gross cap 之后）前移到 `next_idx`（AncOK 刚产出、`strategy_target_legs`
之前），`debug_assert!` 升级为 `panic!`（release/debug 都炸），新增 `appended_source`
显式来源标注（5 个 push 点）替代原来只能判断"是不是 restore"的 `HashSet`。

**未移植的 coverage 域外部分**：golden v2 schema（`scripts/check_armR_trades_digest.py`
+ `scripts/tests/`）、T3 §15.6/§15.7 报告 canonical 订正、Spec MED-1/MED-2 报告路径
订正——均是 #481/#511 评审链的报告/脚本产物，不属于"coverage 域代码"范围，且依赖 main
侧尚不存在的那条报告链（`treasury-reverify-*`），未移植。

## 停手 / 存疑项汇总

1. **#310/#351 的 fill.rs/runner.rs 实际接线**（M4 级别帽从"数学原语存在"到"生产路径真
   实裁剪下单量"）——main 侧现状是大量文档承诺 + 零实现，接线前需要先确认是否要一并
   兑现 main 自己 #355/#363/#369/#376 谱系写下的接口形状（`plan_level_gated_order`、
   `LevelOrderPlan.capped_levels` 字段），这是一个比"移植 kimi 的 12 个提交"更大的决策，
   建议另开票处理。
2. **#346/#347 的 A/B/C/E 四个子部分**——A/B/C 触及 coverage 域外文件，E
   （`ancestors_by_id_lookup` 环形守卫）虽在 coverage 域内但该函数非生产路径消费，
   风险面已如实登记，留作独立跟进。
3. **#358 的 runner.rs assert-上移防 wrap**——main 侧无对应代码，无法评估是否需要。

## 碰撞检查

本次改动全部落在 `rust/src/theta_v0/strategy/coverage/{ancok,held,step,sizing,
sizing_tests_2,held_tests_1,held_tests_2}.rs` 与 `rust/src/theta_v0/backtest/
wverify_run.rs`，未触碰 `#668`（classifier: cand_event/chain_cert/recursive_tower）、
`#671`（level_view_store + 探针 bin）、`#676`（chain_cert 测试/battery）声明的碰撞面；
`coverage/mod.rs` 仅追加两个 `use` 项（`resolve_pending_parent_fixups`），未改动既有
导出。
