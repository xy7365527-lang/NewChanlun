# #398 coverage.rs 拆分——逐块去向表（票面 AC2 交付物）

> 票面 AC2 原话：「main 上原 coverage.rs 的全部改动（含 main 侧后来新增的那 1527 行）
> **无一丢失——逐一列出去向**」。本表即该要求的交付物：主控可**看见**每一块的落点，
> 而不是相信一句「都保全了」。

- 源：`rust/src/theta_v0/strategy/coverage.rs` @ `b7dc9b544d`（8436 行）
- 结果：`rust/src/theta_v0/strategy/coverage/` 21 个文件
- 分块：**255 块**（顶层条目 + 其前导文档/属性），逐块落点唯一，**未落点 0 块**
- 生成方式：逐行分块解析 → 与新树按归一化内容匹配定位 → 与 `git blame` 归因合并

## 一、按目标文件汇总

| 目标文件 | 块数 | 行数 |
|---|---:|---:|
| `ancok.rs` | 10 | 197 |
| `ancok_tests.rs` | 11 | 326 |
| `compose.rs` | 10 | 478 |
| `compose_tests_1.rs` | 8 | 482 |
| `compose_tests_2.rs` | 11 | 450 |
| `element.rs` | 18 | 620 |
| `element_tests.rs` | 17 | 345 |
| `held.rs` | 7 | 460 |
| `held_tests_1.rs` | 12 | 655 |
| `held_tests_2.rs` | 13 | 617 |
| `leg.rs` | 13 | 490 |
| `leg_tests.rs` | 17 | 375 |
| `mod.rs` | 2 | 13 |
| `role.rs` | 19 | 495 |
| `role_tests.rs` | 5 | 131 |
| `sizing.rs` | 16 | 404 |
| `sizing_tests_1.rs` | 18 | 447 |
| `sizing_tests_2.rs` | 10 | 412 |
| `step.rs` | 4 | 508 |
| `step_tests.rs` | 10 | 185 |
| `test_support.rs` | 24 | 276 |
| **合计** | **255** | **8366** |

## 二、main 侧 24 个 commit 的引入内容 → 落点

拆分基点（与 #359 分支的共同祖先）= `7e6bf7c7ea`，彼时 coverage.rs 5760 行。
此后 main 侧下列 commit 改动了该文件（`git blame` 归因，按引入行数降序）：

| commit | 主题 | 引入行数 | 落到哪些文件 |
|---|---|---:|---|
| `0feed5e429` | fix(coverage): #247 影子评审三条关票条件——element_depth 环路硬门 + 持仓腿角色重建 | 284 | `held_tests_1.rs`(174), `held.rs`(68), `leg.rs`(30), `ancok.rs`(10), `step.rs`(2) |
| `2459cd9524` | #233 实装方向翻转显式化为声部终结事件（父翻向=父终结，蓝图两步形） | 267 | `held_tests_2.rs`(241), `step.rs`(21), `held.rs`(3), `ancok.rs`(2) |
| `0dd83531bf` | #216 修复活动集注册：held Stale 重注册复用 restore idx + open 候选按 id 判重/反 | 260 | `held_tests_1.rs`(171), `step.rs`(53), `held.rs`(36) |
| `69834d17cd` | #237 修复断言门按方向拆查（该级该方向——卖批查多侧/买批查空侧） | 253 | `compose_tests_1.rs`(209), `compose.rs`(23), `test_support.rs`(21) |
| `1447773411` | #201 解释器阶段 B：trace/裁决层统一（显式 Hold，schema 冻结） | 248 | `compose_tests_2.rs`(169), `compose.rs`(76), `sizing_tests_1.rs`(3) |
| `bc7b8c26dd` | fix(coverage): #247 registry 恢复祖先——角色输入重建 + 穷尽性声明补第三来源 | 230 | `held_tests_1.rs`(143), `held.rs`(64), `ancok.rs`(11), `element.rs`(7), `step.rs`(3), `leg.rs`(2) |
| `34fed43877` | refactor(theta_v0): #283 词汇对齐——ShortDiff→ReverseOpen（修4「次级别首 | 210 | `ancok_tests.rs`(36), `leg_tests.rs`(31), `role.rs`(25), `leg.rs`(19), `role_tests.rs`(18), `compose.rs`(13), `sizing_tests_2.rs`(12), `held.rs`(10), `compose_tests_2.rs`(10), `step.rs`(9), `compose_tests_1.rs`(8), `sizing_tests_1.rs`(7), `held_tests_2.rs`(5), `element.rs`(3), `held_tests_1.rs`(2), `element_tests.rs`(1), `step_tests.rs`(1) |
| `d7fa0e01a5` | #226 修复 open 父注入 restore 复活当 bar 被关父：ℬ_x 段关闭种子遇链即中断（S3 子树清仓收 | 189 | `held_tests_2.rs`(144), `step.rs`(21), `held.rs`(19), `ancok.rs`(4), `held_tests_1.rs`(1) |
| `5f151b3b75` | #183 归一：T4 子树清仓镜像函数接线进生产 π loop | 161 | `held_tests_2.rs`(74), `step.rs`(71), `ancok.rs`(6), `leg.rs`(6), `held.rs`(2), `held_tests_1.rs`(2) |
| `60a7eeeddc` | #202 解释器阶段 C：仅替换 P2/P3（本级证书平仓由 channel 解释器承担） | 157 | `sizing_tests_1.rs`(130), `compose.rs`(27) |
| `d83bd9422f` | #146 T2：按级别消费已确认出场证书 | 142 | `sizing_tests_1.rs`(113), `test_support.rs`(25), `compose.rs`(4) |
| `eb5cc9b4a4` | #145 T1：typed 出场裁决前移到组合层决策点（StepTrace.closed 三元化） | 134 | `compose_tests_1.rs`(83), `compose.rs`(25), `test_support.rs`(14), `compose_tests_2.rs`(10), `sizing_tests_1.rs`(2) |
| `09e4307239` | fix(strategy): #269 翻向守卫事件化——事件谓词（registry首见方向≠当前树方向⟺树段方向冲突） | 127 | `held_tests_2.rs`(93), `held.rs`(23), `step.rs`(11) |
| `125c645e16` | #220 修复 opened 外化双重配对：按候选自身元素真实准入配对（路④） | 126 | `held_tests_1.rs`(70), `compose.rs`(36), `step.rs`(20) |
| `0f8ffc3736` | #199 二类反向「仅残余才纠错」：CoreResidualCorrection 落理由轴 + 断言③生产实证；断言①② | 103 | `compose.rs`(60), `compose_tests_1.rs`(43) |
| `2f5979d952` | #209 修复 fold：一类候选关全部同级本仓腿（S7 级别内全平实装）+ 断言①②终态硬门 | 69 | `compose_tests_1.rs`(62), `compose.rs`(7) |
| `95a217c8f4` | docs(coverage): #247 窄复审两处文本订正——:2341 注释按调用点分述 + C3 限定落点如实标注 | 16 | `held.rs`(11), `step.rs`(3), `leg.rs`(2) |
| `b7dc9b544d` | feat(persistent): #271 registry 首见来源标记 + #233 旧注释清理 | 13 | `ancok.rs`(8), `held_tests_2.rs`(5) |
| `df40d2ef6f` | #146 T2 合并后修复：补齐 VoiceState/VoiceStepInput 新增字段（#149/#150 语义 | 13 | `sizing_tests_1.rs`(13) |
| `93abf6496a` | merge kimi-nest-mainline-20260717——runner.rs seam 骨架安家 ∧ 教义线 | 10 | `compose_tests_1.rs`(4), `test_support.rs`(4), `sizing_tests_1.rs`(2) |
| `dacf5aa61a` | refactor(theta_v0): #282 删除 S6 开空腿账面形态全套（#280 裁定），保触发链与 PanD | 9 | `sizing_tests_2.rs`(9) |
| `6332f0a84e` | #196 阶段A：shadow 双链比对（零行为变更） | 6 | `element.rs`(6) |
| `a83b5e0364` | docs(strategy): #283 评审尾巴——谱系注笔误修正（原 ShortDiff，非原 ReverseOpe | 1 | `role.rs`(1) |

> 其余 52 个更早的 commit（共 5338 行）
> 是 `7e6bf7c7ea` 及之前就存在的内容，同样逐块落点，见下表。

## 三、逐块明细

`src` = 原 coverage.rs 行区间；`dest` = 新树文件；`主要归因` = 该块行数占比最大的 commit。

| # | 类型 | 条目 | src 行区间 | 行数 | dest | 主要归因 |
|---:|---|---|---|---:|---|---|
| 1 | prod | `type:OrderDecision` | 63-64 | 2 | `mod.rs` | `d596f5430b` feat(#80): 协议状态机 + π_Θ 双轨输出（组合R落地① |
| 2 | prod | `type:PiThetaDecision` | 65-75 | 11 | `mod.rs` | `a8b9d1b1c7` feat(theta): deepresearch-impl 三件联 |
| 3 | prod | `struct:AncokProbe` | 76-132 | 57 | `ancok.rs` | `a8b9d1b1c7` feat(theta): deepresearch-impl 三件联 |
| 4 | prod | `macro:thread_local` | 133-151 | 19 | `ancok.rs` | `a8b9d1b1c7` feat(theta): deepresearch-impl 三件联 |
| 5 | prod | `fn:ancok_probe_bump` | 152-160 | 9 | `ancok.rs` | `a8b9d1b1c7` feat(theta): deepresearch-impl 三件联 |
| 6 | prod | `fn:ancok_probe_reset` | 161-165 | 5 | `ancok.rs` | `a8b9d1b1c7` feat(theta): deepresearch-impl 三件联 |
| 7 | prod | `fn:ancok_probe_snapshot` | 166-170 | 5 | `ancok.rs` | `a8b9d1b1c7` feat(theta): deepresearch-impl 三件联 |
| 8 | prod | `macro:thread_local` | 171-177 | 7 | `compose.rs` | `0f8ffc3736` #199 二类反向「仅残余才纠错」：CoreResidualCorr |
| 9 | prod | `fn:t1_target_zero_probe_bump` | 178-185 | 8 | `compose.rs` | `0f8ffc3736` #199 二类反向「仅残余才纠错」：CoreResidualCorr |
| 10 | prod | `fn:t1_target_zero_probe_reset` | 186-191 | 6 | `compose.rs` | `0f8ffc3736` #199 二类反向「仅残余才纠错」：CoreResidualCorr |
| 11 | prod | `fn:t1_target_zero_probe_count` | 192-196 | 5 | `compose.rs` | `0f8ffc3736` #199 二类反向「仅残余才纠错」：CoreResidualCorr |
| 12 | prod | `fn:t1_target_residual_probe_bump` | 197-204 | 8 | `compose.rs` | `0f8ffc3736` #199 二类反向「仅残余才纠错」：CoreResidualCorr |
| 13 | prod | `fn:t1_target_residual_probe_count` | 205-214 | 10 | `compose.rs` | `0f8ffc3736` #199 二类反向「仅残余才纠错」：CoreResidualCorr |
| 14 | prod | `struct:CoverageElement` | 215-253 | 39 | `element.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 15 | prod | `struct:ElementView` | 254-273 | 20 | `element.rs` | `b2b7ceeab7` perf(coverage): 热点② ElementView ov |
| 16 | prod | `impl:impl ElementView` | 274-368 | 95 | `element.rs` | `b2b7ceeab7` perf(coverage): 热点② ElementView ov |
| 17 | prod | `impl:impl std::ops::Index` | 369-379 | 11 | `element.rs` | `b2b7ceeab7` perf(coverage): 热点② ElementView ov |
| 18 | prod | `fn:extract_elements` | 380-409 | 30 | `element.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 19 | prod | `fn:extract_carrier_forest` | 410-470 | 61 | `element.rs` | `37d282abb0` feat(theta_v0): 方案D视图分离[D]——K_i操作c |
| 20 | prod | `fn:dual_view_consistency` | 471-526 | 56 | `element.rs` | `a87540bdb6` feat(theta): A12 双视图生产切换——host^op  |
| 21 | prod | `fn:push_element_tree` | 527-556 | 30 | `element.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 22 | prod | `fn:rmove_side` | 557-581 | 25 | `element.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 23 | prod | `fn:attach_bsp_to_tree` | 582-660 | 79 | `element.rs` | `2d06fab9e1` feat(theta_v0): 塔导出桥(i)导出+(ii)喂入—— |
| 24 | prod | `fn:build_tree_endpoint_index` | 661-676 | 16 | `element.rs` | `4a8ca137aa` perf(strategy): H5/H6/H7/H8跨函数Hash |
| 25 | prod | `fn:attach_bsp_to_tree_indexed` | 677-693 | 17 | `element.rs` | `4a8ca137aa` perf(strategy): H5/H6/H7/H8跨函数Hash |
| 26 | prod | `fn:attach_bsp_carrier_indexed` | 694-728 | 35 | `element.rs` | `b29d3d4b19` feat(theta_v0): acceptance[1]bit-e |
| 27 | prod | `fn:attach_bsp_parent_carrier_indexed` | 729-749 | 21 | `element.rs` | `e6132632cd` feat(pi_bsp): Lift谓词链Context^δ_j+F |
| 28 | prod | `fn:from_classification_levels` | 750-802 | 53 | `element.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 29 | prod | `fn:starting_set` | 803-812 | 10 | `element.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 30 | prod | `fn:ending_set` | 813-822 | 10 | `element.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 31 | prod | `fn:ancestors` | 823-841 | 19 | `ancok.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 32 | prod | `fn:ancestors_by_id_lookup` | 842-865 | 24 | `ancok.rs` | `b14fe2a290` feat(theta_v0): Q4确定性ElementId层+St |
| 33 | prod | `fn:raw_active_set` | 866-887 | 22 | `ancok.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 34 | prod | `fn:ancestor_close` | 888-908 | 21 | `ancok.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 35 | prod | `fn:active_set_step` | 909-924 | 16 | `ancok.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 36 | prod | `enum:Dir` | 925-935 | 11 | `role.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 37 | prod | `enum:Horizontal` | 936-952 | 17 | `role.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 38 | prod | `enum:Vertical` | 953-982 | 30 | `role.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 39 | prod | `enum:GradeRel` | 983-1003 | 21 | `role.rs` | `ec45f9b22b` refactor(theta): SameReverse 权威重构  |
| 40 | prod | `struct:OperationRole` | 1004-1038 | 35 | `role.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 41 | prod | `fn:direction_of` | 1039-1051 | 13 | `role.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 42 | prod | `fn:dir_sign` | 1052-1059 | 8 | `role.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 43 | prod | `fn:parent_sign` | 1060-1072 | 13 | `role.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 44 | prod | `fn:horizontal_relation` | 1073-1112 | 40 | `role.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 45 | prod | `fn:build_prev_sibling_index` | 1113-1127 | 15 | `role.rs` | `4a8ca137aa` perf(strategy): H5/H6/H7/H8跨函数Hash |
| 46 | prod | `fn:operation_role_indexed` | 1128-1180 | 53 | `role.rs` | `4a8ca137aa` perf(strategy): H5/H6/H7/H8跨函数Hash |
| 47 | prod | `fn:operation_role_indexed_split` | 1181-1241 | 61 | `role.rs` | `61622c2187` perf(theta_v0): 工位4c interp _cache |
| 48 | prod | `fn:operation_role_two_segment` | 1242-1306 | 65 | `role.rs` | `5ace35459a` perf(strategy): 工位4d 热点①② base派生索引 |
| 49 | prod | `fn:classify_vertical` | 1307-1324 | 18 | `role.rs` | `ec45f9b22b` refactor(theta): SameReverse 权威重构  |
| 50 | prod | `fn:classify_grade` | 1325-1336 | 12 | `role.rs` | `ec45f9b22b` refactor(theta): SameReverse 权威重构  |
| 51 | prod | `fn:vertical_relation` | 1337-1348 | 12 | `role.rs` | `ec45f9b22b` refactor(theta): SameReverse 权威重构  |
| 52 | prod | `fn:grade_relation` | 1349-1365 | 17 | `role.rs` | `ec45f9b22b` refactor(theta): SameReverse 权威重构  |
| 53 | prod | `fn:operation_role` | 1366-1401 | 36 | `role.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 54 | prod | `impl:impl OperationRole` | 1402-1419 | 18 | `role.rs` | `34fed43877` refactor(theta_v0): #283 词汇对齐——Sho |
| 55 | prod | `struct:SepLeg` | 1420-1448 | 29 | `leg.rs` | `47e21c5e7f` feat(theta): M5/M6/M7 三线并行收割——Over |
| 56 | prod | `struct:LegTarget` | 1449-1471 | 23 | `leg.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 57 | prod | `fn:dir_weight` | 1472-1526 | 55 | `leg.rs` | `0d732723bb` feat(theta): q_Θ v0→v1 σ_higher分级权 |
| 58 | prod | `fn:w_grade` | 1527-1544 | 18 | `leg.rs` | `69574ea68b` feat(theta): w_G 因子化实装 GPT 步骤4 第二部 |
| 59 | prod | `fn:theta_dir_slot` | 1545-1560 | 16 | `leg.rs` | `0d732723bb` feat(theta): q_Θ v0→v1 σ_higher分级权 |
| 60 | prod | `fn:leg_target` | 1561-1580 | 20 | `leg.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 61 | prod | `fn:leg_target_two_segment` | 1581-1604 | 24 | `leg.rs` | `4a8ca137aa` perf(strategy): H5/H6/H7/H8跨函数Hash |
| 62 | prod | `fn:element_depth` | 1605-1646 | 42 | `leg.rs` | `0feed5e429` fix(coverage): #247 影子评审三条关票条件——el |
| 63 | prod | `fn:strategy_target_legs` | 1647-1691 | 45 | `leg.rs` | `5ace35459a` perf(strategy): 工位4d 热点①② base派生索引 |
| 64 | prod | `fn:net_target_units` | 1692-1712 | 21 | `leg.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 65 | prod | `fn:gross_target_units` | 1713-1720 | 8 | `leg.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 66 | prod | `fn:apply_gross_cap` | 1721-1870 | 150 | `leg.rs` | `f1c9700332` feat(theta): G7 K_Θ 毛头寸约束——legs 折叠 |
| 67 | prod | `fn:overlay_net_delta` | 1871-1909 | 39 | `leg.rs` | `c2533bf1ba` feat(fugue): f2 P0-2/P0-3——持仓身份边界声 |
| 68 | prod | `fn:held_leg_tree_index` | 1910-1939 | 30 | `held.rs` | `b14fe2a290` feat(theta_v0): Q4确定性ElementId层+St |
| 69 | prod | `fn:build_tree_id_index` | 1940-1951 | 12 | `element.rs` | `4a8ca137aa` perf(strategy): H5/H6/H7/H8跨函数Hash |
| 70 | prod | `fn:held_leg_tree_index_indexed` | 1952-1987 | 36 | `held.rs` | `09e4307239` fix(strategy): #269 翻向守卫事件化——事件谓词（ |
| 71 | prod | `enum:HeldLegMatch` | 1988-2010 | 23 | `held.rs` | `09e4307239` fix(strategy): #269 翻向守卫事件化——事件谓词（ |
| 72 | prod | `fn:element_as_leg` | 2011-2035 | 25 | `held.rs` | `b14fe2a290` feat(theta_v0): Q4确定性ElementId层+St |
| 73 | prod | `fn:close_indices` | 2036-2168 | 133 | `held.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 74 | prod | `fn:restore_ancestor_chain_from_registry` | 2169-2297 | 129 | `held.rs` | `6bb47fba7b` feat(strategy): persistent overlay |
| 75 | prod | `fn:held_stale_reregister_idx` | 2298-2381 | 84 | `held.rs` | `0feed5e429` fix(coverage): #247 影子评审三条关票条件——el |
| 76 | prod | `fn:coverage_step_from_buckets` | 2382-2400 | 19 | `step.rs` | `47e21c5e7f` feat(theta): M5/M6/M7 三线并行收割——Over |
| 77 | prod | `fn:coverage_step_from_buckets_sep` | 2401-2817 | 417 | `step.rs` | `5f151b3b75` #183 归一：T4 子树清仓镜像函数接线进生产 π loop |
| 78 | prod | `fn:coverage_step_classification` | 2818-2861 | 44 | `step.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 79 | prod | `fn:coverage_step_prebuilt` | 2862-2889 | 28 | `step.rs` | `b29d3d4b19` feat(theta_v0): acceptance[1]bit-e |
| 80 | prod | `struct:PiThetaWeights` | 2890-2905 | 16 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 81 | prod | `impl:impl PiThetaWeights` | 2906-2915 | 10 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 82 | prod | `const:J_SCALE` | 2916-2920 | 5 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 83 | prod | `const:FLAT_EPS` | 2921-2923 | 3 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 84 | prod | `fn:scale_key` | 2924-2931 | 8 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 85 | prod | `fn:lot_round` | 2932-2936 | 5 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 86 | prod | `fn:feasible_net_cap` | 2937-2955 | 19 | `sizing.rs` | `8de292a9b4` fix(formal+rust): 反膨胀修复批次——§18/§19 |
| 87 | prod | `struct:KThetaRiskGate` | 2956-2987 | 32 | `sizing.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 88 | prod | `impl:impl KThetaRiskGate` | 2988-3036 | 49 | `sizing.rs` | `91baf4e36a` theta_v0: 落地 #82 DC-E 盘整背驰角色感知减仓 + |
| 89 | prod | `fn:feasible_candidates` | 3037-3071 | 35 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 90 | prod | `fn:j_theta_key` | 3072-3094 | 23 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 91 | prod | `fn:pi_theta_position` | 3095-3113 | 19 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 92 | prod | `fn:feasible_lex_candidates` | 3114-3141 | 28 | `sizing.rs` | `5db6362093` feat(theta): R5语义插桩修复——3字段暴露(lex_a |
| 93 | prod | `fn:schedule_order` | 3142-3189 | 48 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 94 | prod | `fn:pi_theta_step` | 3190-3264 | 75 | `sizing.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 95 | prod | `fn:pi_theta_step_prebuilt` | 3265-3293 | 29 | `sizing.rs` | `b29d3d4b19` feat(theta_v0): acceptance[1]bit-e |
| 96 | prod | `struct:VoiceVerdict` | 3294-3316 | 23 | `compose.rs` | `1447773411` #201 解释器阶段 B：trace/裁决层统一（显式 Hold，s |
| 97 | prod | `struct:StepTrace` | 3317-3377 | 61 | `compose.rs` | `cd2f326a52` feat(theta): G4 commit-B——coverage |
| 98 | prod | `struct:TwStepCtx` | 3378-3404 | 27 | `compose.rs` | `f9333e21b2` feat(theta): #124 裁定4 TW 进 fold——P |
| 99 | prod | `fn:pi_theta_step_traced` | 3405-3727 | 323 | `compose.rs` | `f9333e21b2` feat(theta): #124 裁定4 TW 进 fold——P |
| 100 | test | `fn:cfg` | 3735-3738 | 4 | `test_support.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 101 | test | `fn:protocol_hold` | 3739-3742 | 4 | `test_support.rs` | `d596f5430b` feat(#80): 协议状态机 + π_Θ 双轨输出（组合R落地① |
| 102 | test | `fn:rc_tower` | 3743-3748 | 6 | `test_support.rs` | `da073f21b0` perf(classifier): 热点① O(n) 塔共享——up |
| 103 | test | `fn:role` | 3749-3755 | 7 | `test_support.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 104 | test | `fn:w_dir_reverse_open_exempt_across_all_presets` | 3756-3784 | 29 | `leg_tests.rs` | `0d732723bb` feat(theta): q_Θ v0→v1 σ_higher分级权 |
| 105 | test | `fn:w_dir_neutral_is_identity_for_all_18_roles` | 3785-3811 | 27 | `leg_tests.rs` | `0d732723bb` feat(theta): q_Θ v0→v1 σ_higher分级权 |
| 106 | test | `fn:vertical_three_class_plus_grade_axis_refinement` | 3812-3841 | 30 | `role_tests.rs` | `ec45f9b22b` refactor(theta): SameReverse 权威重构  |
| 107 | test | `fn:w_dir_sign_neg_slot_unreachable_via_v_axis` | 3842-3857 | 16 | `leg_tests.rs` | `34fed43877` refactor(theta_v0): #283 词汇对齐——Sho |
| 108 | test | `fn:w_grade_consumes_g_axis` | 3858-3878 | 21 | `leg_tests.rs` | `69574ea68b` feat(theta): w_G 因子化实装 GPT 步骤4 第二部 |
| 109 | test | `fn:leg_target_sizing_includes_w_grade_factor` | 3879-3897 | 19 | `leg_tests.rs` | `69574ea68b` feat(theta): w_G 因子化实装 GPT 步骤4 第二部 |
| 110 | test | `fn:w_dir_case3_lookup_and_root_exemption` | 3898-3927 | 30 | `leg_tests.rs` | `0d732723bb` feat(theta): q_Θ v0→v1 σ_higher分级权 |
| 111 | test | `fn:unit` | 3928-3931 | 4 | `test_support.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 112 | test | `fn:ctr` | 3932-3935 | 4 | `test_support.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 113 | test | `fn:nested_l1` | 3936-3944 | 9 | `test_support.rs` | `b14fe2a290` feat(theta_v0): Q4确定性ElementId层+St |
| 114 | test | `fn:eid` | 3945-3949 | 5 | `test_support.rs` | `b14fe2a290` feat(theta_v0): Q4确定性ElementId层+St |
| 115 | test | `fn:aleg` | 3950-3966 | 17 | `test_support.rs` | `b14fe2a290` feat(theta_v0): Q4确定性ElementId层+St |
| 116 | test | `fn:extract_elements_nested_parent_child` | 3967-3985 | 19 | `element_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 117 | test | `fn:element_intervals_from_tower_coords` | 3986-4001 | 16 | `element_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 118 | test | `fn:two_parent_tower` | 4002-4016 | 15 | `test_support.rs` | `b14fe2a290` feat(theta_v0): Q4确定性ElementId层+St |
| 119 | test | `fn:attach_inherits_host_compose_parent_dir` | 4017-4026 | 10 | `element_tests.rs` | `2d06fab9e1` feat(theta_v0): 塔导出桥(i)导出+(ii)喂入—— |
| 120 | test | `fn:attach_right_endpoint_` | 4027-4040 | 14 | `element_tests.rs` | `2d06fab9e1` feat(theta_v0): 塔导出桥(i)导出+(ii)喂入—— |
| 121 | test | `fn:attach_level_context_disambiguates_shared_rho` | 4041-4051 | 11 | `element_tests.rs` | `2d06fab9e1` feat(theta_v0): 塔导出桥(i)导出+(ii)喂入—— |
| 122 | test | `fn:attach_ordinal_identity_not_rmove_struct_equal` | 4052-4061 | 10 | `element_tests.rs` | `2d06fab9e1` feat(theta_v0): 塔导出桥(i)导出+(ii)喂入—— |
| 123 | test | `fn:carrier_forest_all_levels_dedup_unique_id` | 4062-4089 | 28 | `element_tests.rs` | `37d282abb0` feat(theta_v0): 方案D视图分离[D]——K_i操作c |
| 124 | test | `fn:carrier_forest_dedup_triggers_when_l0_nonempty` | 4090-4120 | 31 | `element_tests.rs` | `37d282abb0` feat(theta_v0): 方案D视图分离[D]——K_i操作c |
| 125 | test | `fn:orphan_subtree_tower` | 4121-4139 | 19 | `test_support.rs` | `a87540bdb6` feat(theta): A12 双视图生产切换——host^op  |
| 126 | test | `fn:a12_dual_view_consistency_holds` | 4140-4158 | 19 | `element_tests.rs` | `a87540bdb6` feat(theta): A12 双视图生产切换——host^op  |
| 127 | test | `fn:a12_dual_view_consistency_rejects_forgery` | 4159-4179 | 21 | `element_tests.rs` | `a87540bdb6` feat(theta): A12 双视图生产切换——host^op  |
| 128 | test | `fn:a12_host_op_hits_orphan_frontier_host_struct_misses` | 4180-4204 | 25 | `element_tests.rs` | `a87540bdb6` feat(theta): A12 双视图生产切换——host^op  |
| 129 | test | `fn:attach_host_not_found_is_ambient` | 4205-4211 | 7 | `element_tests.rs` | `2d06fab9e1` feat(theta_v0): 塔导出桥(i)导出+(ii)喂入—— |
| 130 | test | `fn:attach_guard_no_compose_level_is_ambient` | 4212-4224 | 13 | `element_tests.rs` | `2d06fab9e1` feat(theta_v0): 塔导出桥(i)导出+(ii)喂入—— |
| 131 | test | `fn:starting_ending_sets_by_endpoints` | 4225-4238 | 14 | `element_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 132 | test | `fn:ancestor_close_prunes_orphan_child` | 4239-4250 | 12 | `ancok_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 133 | test | `fn:ancestor_close_keeps_closed_family` | 4251-4261 | 11 | `ancok_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 134 | test | `fn:raw_close_then_open` | 4262-4279 | 18 | `ancok_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 135 | test | `fn:role_toplevel_is_first_ambient_not_root` | 4280-4292 | 13 | `role_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 136 | test | `fn:vertical_followparent_vs_reverse_open_by_parent_direction` | 4293-4316 | 24 | `role_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 137 | test | `fn:horizontal_sibling_follow_vs_reverse` | 4317-4337 | 21 | `role_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 138 | test | `fn:leg_target_side_and_depth_weighted_units` | 4338-4361 | 24 | `leg_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 139 | test | `fn:net_target_units_hedges_parent_with_reverse_open` | 4362-4378 | 17 | `leg_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 140 | test | `fn:net_zero_when_hedged_equal` | 4379-4389 | 11 | `leg_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 141 | test | `fn:overlay_net_delta_is_reverse_open_contribution` | 4390-4410 | 21 | `leg_tests.rs` | `c2533bf1ba` feat(fugue): f2 P0-2/P0-3——持仓身份边界声 |
| 142 | test | `fn:gce` | 4411-4424 | 14 | `test_support.rs` | `f1c9700332` feat(theta): G7 K_Θ 毛头寸约束——legs 折叠 |
| 143 | test | `fn:gleg` | 4425-4428 | 4 | `test_support.rs` | `f1c9700332` feat(theta): G7 K_Θ 毛头寸约束——legs 折叠 |
| 144 | test | `fn:gross_cap_single_root_scales_proportionally` | 4429-4450 | 22 | `leg_tests.rs` | `f1c9700332` feat(theta): G7 K_Θ 毛头寸约束——legs 折叠 |
| 145 | test | `fn:gross_cap_constrains_hedged_net_zero` | 4451-4470 | 20 | `leg_tests.rs` | `f1c9700332` feat(theta): G7 K_Θ 毛头寸约束——legs 折叠 |
| 146 | test | `fn:gross_cap_multi_root_waterfilling_nonuniform` | 4471-4496 | 26 | `leg_tests.rs` | `f1c9700332` feat(theta): G7 K_Θ 毛头寸约束——legs 折叠 |
| 147 | test | `fn:gross_cap_waterfilling_zeroes_low_threshold_root` | 4497-4510 | 14 | `leg_tests.rs` | `f1c9700332` feat(theta): G7 K_Θ 毛头寸约束——legs 折叠 |
| 148 | test | `fn:gross_cap_boundary_zero_cap_and_no_trigger` | 4511-4531 | 21 | `leg_tests.rs` | `f1c9700332` feat(theta): G7 K_Θ 毛头寸约束——legs 折叠 |
| 149 | test | `fn:gross_cap_integration_default_bit_exact_enabled_scales` | 4532-4566 | 35 | `leg_tests.rs` | `f1c9700332` feat(theta): G7 K_Θ 毛头寸约束——legs 折叠 |
| 150 | test | `fn:gross_cap_zeroed_open_legs_do_not_enter_active_set` | 4567-4588 | 22 | `leg_tests.rs` | `6a282f5f46` fix(theta): G7 幽灵腿防护——毛约束整体零化的开仓腿不 |
| 151 | test | `fn:from_classification_levels_flat_root_coverage` | 4589-4629 | 41 | `element_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 152 | test | `fn:empty_tower_yields_empty_elements` | 4630-4643 | 14 | `element_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 153 | test | `fn:cand` | 4644-4658 | 15 | `test_support.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 154 | test | `fn:flat_elements` | 4659-4690 | 32 | `test_support.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 155 | test | `fn:view_split` | 4691-4697 | 7 | `test_support.rs` | `b2b7ceeab7` perf(coverage): 热点② ElementView ov |
| 156 | test | `fn:operation_role_split_matches_merged_lcg` | 4698-4740 | 43 | `role_tests.rs` | `0f3f228b42` test(theta_v0): 工位4c bit-exact守卫 o |
| 157 | test | `fn:sell_bsp` | 4741-4753 | 13 | `test_support.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 158 | test | `fn:buy_bsp` | 4754-4766 | 13 | `test_support.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 159 | test | `fn:buckets_open_creates_active_leg_and_target` | 4767-4782 | 16 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 160 | test | `fn:buckets_close_removes_active_leg` | 4783-4797 | 15 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 161 | test | `fn:buckets_target_nets_long_and_short` | 4798-4815 | 18 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 162 | test | `fn:buckets_close_then_open_keeps_survivor` | 4816-4836 | 21 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 163 | test | `fn:buckets_ancok_identity_on_flat_roots` | 4837-4852 | 16 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 164 | test | `fn:buckets_step_immutable_prev_active` | 4853-4867 | 15 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 165 | test | `fn:buckets_record_excluded_from_active_set` | 4868-4881 | 14 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 166 | test | `fn:buckets_empty_yields_empty_and_zero` | 4882-4891 | 10 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 167 | test | `fn:classification_end_to_end_ring5_ring6` | 4892-4917 | 26 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 168 | test | `fn:ring6_active_set_feeds_back_into_interpret` | 4918-4951 | 34 | `step_tests.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 169 | test | `fn:ancok_admits_reverse_open_when_parent_held` | 4952-4988 | 37 | `ancok_tests.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 170 | test | `fn:ancok_admits_reverse_open_under_parent_coord_drift` | 4989-5030 | 42 | `ancok_tests.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 171 | test | `fn:ancok_prunes_reverse_open_when_parent_unheld` | 5031-5063 | 33 | `ancok_tests.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 172 | test | `fn:ancok_admits_ambient_root_without_held_parent` | 5064-5082 | 19 | `ancok_tests.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 173 | test | `fn:ancok_prunes_child_when_parent_closed_same_step` | 5083-5117 | 35 | `ancok_tests.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 174 | test | `fn:engine_bootstrap_container_bsp_admits_depth_child_from_empty` | 5118-5158 | 41 | `ancok_tests.rs` | `b29d3d4b19` feat(theta_v0): acceptance[1]bit-e |
| 175 | test | `fn:engine_bootstrap_does_not_admit_orphan_reverse_open_without_container_bsp` | 5159-5180 | 22 | `ancok_tests.rs` | `b29d3d4b19` feat(theta_v0): acceptance[1]bit-e |
| 176 | test | `fn:cross_bar_held_container_admits_depth_child_next_bar` | 5181-5236 | 56 | `ancok_tests.rs` | `b29d3d4b19` feat(theta_v0): acceptance[1]bit-e |
| 177 | test | `fn:open_candidate_parent_injected_from_registry_admits_depth_child` | 5237-5292 | 56 | `held_tests_1.rs` | `b29d3d4b19` feat(theta_v0): acceptance[1]bit-e |
| 178 | test | `fn:restore_reuses_existing_work_idx_no_duplicate_id` | 5293-5333 | 41 | `held_tests_1.rs` | `b29d3d4b19` feat(theta_v0): acceptance[1]bit-e |
| 179 | test | `fn:restore_rebuilds_parent_link_and_attached_dir` | 5334-5389 | 56 | `held_tests_1.rs` | `bc7b8c26dd` fix(coverage): #247 registry 恢复祖先— |
| 180 | test | `fn:restore_role_rebuild_changes_p_tilde_leg_set_unchanged` | 5390-5481 | 92 | `held_tests_1.rs` | `0feed5e429` fix(coverage): #247 影子评审三条关票条件——el |
| 181 | test | `fn:restore_deep_chain_depth_ge3_weight_zeroed` | 5482-5565 | 84 | `held_tests_1.rs` | `0feed5e429` fix(coverage): #247 影子评审三条关票条件——el |
| 182 | test | `fn:element_depth_cycle_fails_loudly_with_probe` | 5566-5608 | 43 | `held_tests_1.rs` | `0feed5e429` fix(coverage): #247 影子评审三条关票条件——el |
| 183 | test | `fn:restore_broken_chain_element_pruned_never_scored` | 5609-5650 | 42 | `held_tests_1.rs` | `bc7b8c26dd` fix(coverage): #247 registry 恢复祖先— |
| 184 | test | `fn:held_leg_reregister_reuses_restore_pushed_idx_no_duplicate_id` | 5651-5693 | 43 | `held_tests_1.rs` | `0dd83531bf` #216 修复活动集注册：held Stale 重注册复用 rest |
| 185 | test | `fn:open_candidates_same_carrier_id_reverse_pair_annihilates` | 5694-5740 | 47 | `held_tests_1.rs` | `0dd83531bf` #216 修复活动集注册：held Stale 重注册复用 rest |
| 186 | test | `fn:open_candidates_same_carrier_id_same_dir_dedup_first_wins` | 5741-5777 | 37 | `held_tests_1.rs` | `0dd83531bf` #216 修复活动集注册：held Stale 重注册复用 rest |
| 187 | test | `fn:opened_restore_leg_not_externalized_for_same_carrier_candidate_pair` | 5778-5847 | 70 | `held_tests_1.rs` | `125c645e16` #220 修复 opened 外化双重配对：按候选自身元素真实准入配 |
| 188 | test | `fn:held_leg_id_hits_candidate_copy_keeps_held_identity` | 5848-5891 | 44 | `held_tests_1.rs` | `0dd83531bf` #216 修复活动集注册：held Stale 重注册复用 rest |
| 189 | test | `fn:t4_subtree_close_unifies_production_active_set_step` | 5892-5934 | 43 | `held_tests_2.rs` | `5f151b3b75` #183 归一：T4 子树清仓镜像函数接线进生产 π loop |
| 190 | test | `fn:t4_unify_preserves_restore_expansion_for_surviving_legs` | 5935-5966 | 32 | `held_tests_2.rs` | `5f151b3b75` #183 归一：T4 子树清仓镜像函数接线进生产 π loop |
| 191 | test | `fn:open_parent_restore_skips_closed_seed_no_resurrect` | 5967-6031 | 65 | `held_tests_2.rs` | `d7fa0e01a5` #226 修复 open 父注入 restore 复活当 bar 被 |
| 192 | test | `fn:open_reverse_candidate_on_closed_carrier_unaffected` | 6032-6070 | 39 | `held_tests_2.rs` | `d7fa0e01a5` #226 修复 open 父注入 restore 复活当 bar 被 |
| 193 | test | `fn:open_parent_restore_mid_chain_seed_aborts` | 6071-6110 | 40 | `held_tests_2.rs` | `d7fa0e01a5` #226 修复 open 父注入 restore 复活当 bar 被 |
| 194 | test | `fn:parent_direction_flip_terminates_voice_and_liquidates_subtree` | 6111-6177 | 67 | `held_tests_2.rs` | `2459cd9524` #233 实装方向翻转显式化为声部终结事件（父翻向=父终结，蓝图两步 |
| 195 | test | `fn:flip_termination_externalizes_close_track_via_silent_drops` | 6178-6233 | 56 | `held_tests_2.rs` | `2459cd9524` #233 实装方向翻转显式化为声部终结事件（父翻向=父终结，蓝图两步 |
| 196 | test | `fn:flip_same_bar_new_generation_reregisters_via_open_candidate` | 6234-6302 | 69 | `held_tests_2.rs` | `2459cd9524` #233 实装方向翻转显式化为声部终结事件（父翻向=父终结，蓝图两步 |
| 197 | test | `fn:flip_same_bar_reverse_candidate_reregisters_new_generation_same_carrier` | 6303-6357 | 55 | `held_tests_2.rs` | `2459cd9524` #233 实装方向翻转显式化为声部终结事件（父翻向=父终结，蓝图两步 |
| 198 | test | `fn:birth_opposition_without_flip_event_does_not_terminate` | 6358-6405 | 48 | `held_tests_2.rs` | `09e4307239` fix(strategy): #269 翻向守卫事件化——事件谓词（ |
| 199 | test | `fn:carrier_flip_event_terminates_even_without_direction_opposition` | 6406-6450 | 45 | `held_tests_2.rs` | `09e4307239` fix(strategy): #269 翻向守卫事件化——事件谓词（ |
| 200 | test | `fn:open_candidate_parent_not_in_registry_still_pruned` | 6451-6472 | 22 | `held_tests_2.rs` | `b29d3d4b19` feat(theta_v0): acceptance[1]bit-e |
| 201 | test | `fn:stale_non_boundary_root_is_pruned_not_fabricated_root` | 6473-6508 | 36 | `held_tests_2.rs` | `b14fe2a290` feat(theta_v0): Q4确定性ElementId层+St |
| 202 | test | `fn:element_id_deterministic_full_vs_incremental` | 6509-6560 | 52 | `element_tests.rs` | `13826f663b` test(theta): 中枢延伸语义测试诚实重算——10 测试重写 |
| 203 | test | `fn:rcfg` | 6561-6564 | 4 | `test_support.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 204 | test | `fn:pi_theta_weights_reuse_kappa_rho` | 6565-6574 | 10 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 205 | test | `fn:feasible_set_nonempty_contains_zero` | 6575-6586 | 12 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 206 | test | `fn:feasible_candidates_lot_aligned_within_cap` | 6587-6600 | 14 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 207 | test | `fn:pi_theta_position_tracks_ptilde_within_cap` | 6601-6609 | 9 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 208 | test | `fn:pi_theta_position_clamps_over_cap` | 6610-6618 | 9 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 209 | test | `fn:pi_theta_position_lot_rounds_to_nearest` | 6619-6628 | 10 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 210 | test | `fn:schedule_open_from_flat` | 6629-6636 | 8 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 211 | test | `fn:schedule_close_to_flat` | 6637-6643 | 7 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 212 | test | `fn:schedule_add_reduce` | 6644-6652 | 9 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 213 | test | `fn:schedule_hold_wait_no_trade` | 6653-6660 | 8 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 214 | test | `fn:schedule_sign_flip_reverse` | 6661-6667 | 7 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 215 | test | `fn:pi_theta_step_buy_point_entry_gap5` | 6668-6696 | 29 | `sizing_tests_1.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 216 | test | `fn:pi_theta_step_traced_opened_and_bitexact` | 6697-6738 | 42 | `compose_tests_1.rs` | `cd2f326a52` feat(theta): G4 commit-B——coverage |
| 217 | test | `fn:pi_theta_step_traced_reverse_close_attribution` | 6739-6774 | 36 | `compose_tests_1.rs` | `cd2f326a52` feat(theta): G4 commit-B——coverage |
| 218 | test | `fn:sell_classification_at` | 6775-6796 | 22 | `test_support.rs` | `eb5cc9b4a4` #145 T1：typed 出场裁决前移到组合层决策点（StepTr |
| 219 | test | `fn:sell_classification` | 6797-6801 | 5 | `test_support.rs` | `d83bd9422f` #146 T2：按级别消费已确认出场证书 |
| 220 | test | `fn:buy_classification_at` | 6802-6824 | 23 | `test_support.rs` | `69834d17cd` #237 修复断言门按方向拆查（该级该方向——卖批查多侧/买批查空侧 |
| 221 | test | `fn:held_l1_compose_a` | 6825-6838 | 14 | `test_support.rs` | `d83bd9422f` #146 T2：按级别消费已确认出场证书 |
| 222 | test | `fn:pi_theta_step_traced_typed_close_root_and_reduce_core` | 6839-6888 | 50 | `compose_tests_1.rs` | `eb5cc9b4a4` #145 T1：typed 出场裁决前移到组合层决策点（StepTr |
| 223 | test | `fn:t1_target_zero_assertion_fires_in_step` | 6889-6935 | 47 | `compose_tests_1.rs` | `0f8ffc3736` #199 二类反向「仅残余才纠错」：CoreResidualCorr |
| 224 | test | `fn:t1_target_zero_assertion_holds_with_multiple_core_legs` | 6936-6993 | 58 | `compose_tests_1.rs` | `2f5979d952` #209 修复 fold：一类候选关全部同级本仓腿（S7 级别内全平 |
| 225 | test | `fn:t1_target_zero_assertion_sell_batch_checks_long_side_only` | 6994-7100 | 107 | `compose_tests_1.rs` | `69834d17cd` #237 修复断言门按方向拆查（该级该方向——卖批查多侧/买批查空侧 |
| 226 | test | `fn:t1_target_zero_assertion_buy_batch_checks_short_side_only` | 7101-7202 | 102 | `compose_tests_1.rs` | `69834d17cd` #237 修复断言门按方向拆查（该级该方向——卖批查多侧/买批查空侧 |
| 227 | test | `fn:p23_channel_closes_every_reverse_hit_leg_multi_leg` | 7203-7287 | 85 | `sizing_tests_1.rs` | `60a7eeeddc` #202 解释器阶段 C：仅替换 P2/P3（本级证书平仓由 cha |
| 228 | test | `fn:p23_channel_type1_multi_leg_bit_exact_with_fold` | 7288-7336 | 49 | `sizing_tests_1.rs` | `60a7eeeddc` #202 解释器阶段 C：仅替换 P2/P3（本级证书平仓由 cha |
| 229 | test | `fn:t2_cross_level_confirmed_certificate_holds_l1_position_end_to_end` | 7337-7395 | 59 | `sizing_tests_1.rs` | `d83bd9422f` #146 T2：按级别消费已确认出场证书 |
| 230 | test | `fn:t2_same_level_confirmed_certificate_matches_t1_exit_split` | 7396-7430 | 35 | `sizing_tests_1.rs` | `d83bd9422f` #146 T2：按级别消费已确认出场证书 |
| 231 | test | `fn:t2_unconfirmed_same_level_signal_records_without_exit_decision` | 7431-7464 | 34 | `sizing_tests_1.rs` | `d83bd9422f` #146 T2：按级别消费已确认出场证书 |
| 232 | test | `fn:pi_theta_step_traced_typed_close_reverse_open_overrides_trigger` | 7465-7504 | 40 | `compose_tests_1.rs` | `eb5cc9b4a4` #145 T1：typed 出场裁决前移到组合层决策点（StepTr |
| 233 | test | `fn:pi_theta_step_traced_p1_force_flat_risk_exits_all` | 7505-7551 | 47 | `compose_tests_2.rs` | `c4c2a027ad` feat(theta): #124 P1 强平全局分支——force |
| 234 | test | `fn:buy_gamma` | 7552-7567 | 16 | `test_support.rs` | `f9333e21b2` feat(theta): #124 裁定4 TW 进 fold——P |
| 235 | test | `fn:pi_theta_step_traced_p2_close_overlay` | 7568-7620 | 53 | `compose_tests_2.rs` | `f9333e21b2` feat(theta): #124 裁定4 TW 进 fold——P |
| 236 | test | `fn:pi_theta_step_traced_p3_withdraw_consumes_step` | 7621-7673 | 53 | `compose_tests_2.rs` | `f9333e21b2` feat(theta): #124 裁定4 TW 进 fold——P |
| 237 | test | `fn:pi_theta_step_traced_p4_enter_earning` | 7674-7721 | 48 | `compose_tests_2.rs` | `f9333e21b2` feat(theta): #124 裁定4 TW 进 fold——P |
| 238 | test | `fn:a10_c5_eta_correction_gates_p4_enter_earning` | 7722-7774 | 53 | `sizing_tests_1.rs` | `640609071d` 收口提交（claude 438f5dc7 承接 kimi 030be |
| 239 | test | `fn:pi_theta_step_traced_verdicts_normal_path_explicit_hold` | 7775-7801 | 27 | `compose_tests_2.rs` | `1447773411` #201 解释器阶段 B：trace/裁决层统一（显式 Hold，s |
| 240 | test | `fn:pi_theta_step_traced_verdicts_normal_path_typed_close` | 7802-7833 | 32 | `compose_tests_2.rs` | `1447773411` #201 解释器阶段 B：trace/裁决层统一（显式 Hold，s |
| 241 | test | `fn:pi_theta_step_traced_verdicts_p1_force_flat_all_risk_exit` | 7834-7857 | 24 | `compose_tests_2.rs` | `1447773411` #201 解释器阶段 B：trace/裁决层统一（显式 Hold，s |
| 242 | test | `fn:pi_theta_step_traced_verdicts_p2_overlay_typed_and_hold` | 7858-7902 | 45 | `compose_tests_2.rs` | `1447773411` #201 解释器阶段 B：trace/裁决层统一（显式 Hold，s |
| 243 | test | `fn:pi_theta_step_traced_verdicts_p3_consumes_step_holds` | 7903-7947 | 45 | `compose_tests_2.rs` | `1447773411` #201 解释器阶段 B：trace/裁决层统一（显式 Hold，s |
| 244 | test | `fn:pi_theta_step_traced_p1_masks_tw_predicates` | 7948-7985 | 38 | `compose_tests_2.rs` | `f9333e21b2` feat(theta): #124 裁定4 TW 进 fold——P |
| 245 | test | `fn:pi_theta_step_traced_tw_ctx_inert_bitexact` | 7986-8023 | 38 | `compose_tests_2.rs` | `f9333e21b2` feat(theta): #124 裁定4 TW 进 fold——P |
| 246 | test | `fn:pi_theta_step_deterministic_unique_order` | 8024-8047 | 24 | `sizing_tests_2.rs` | `89c23de3ac` feat(theta_v0): 严格全互斥定义策略七链rust实装— |
| 247 | test | `fn:protocol_event_does_not_reorder_p1_p10_order_track_bitexact` | 8048-8154 | 107 | `sizing_tests_2.rs` | `d596f5430b` feat(#80): 协议状态机 + π_Θ 双轨输出（组合R落地① |
| 248 | test | `fn:center_oscillation_default_inactive_order_track_bitexact` | 8155-8212 | 58 | `sizing_tests_2.rs` | `d90e3d6ee2` theta_v0: 落地 #81 中枢盘整背驰独立通道（Center |
| 249 | test | `fn:order_and_protocol_product_total` | 8213-8236 | 24 | `sizing_tests_2.rs` | `d596f5430b` feat(#80): 协议状态机 + π_Θ 双轨输出（组合R落地① |
| 250 | test | `fn:zero_qty_protocol_event_remains_observable` | 8237-8276 | 40 | `sizing_tests_2.rs` | `d596f5430b` feat(#80): 协议状态机 + π_Θ 双轨输出（组合R落地① |
| 251 | test | `fn:pi_theta_position_force_flat_gate_clamps_to_zero` | 8277-8289 | 13 | `sizing_tests_2.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 252 | test | `fn:pi_theta_position_no_increase_cap_forbids_growth` | 8290-8305 | 16 | `sizing_tests_2.rs` | `1896139eac` feat(margin): #113 真保证金模型实装——规则表 d |
| 253 | test | `fn:pi_theta_position_stop_long_gate_forbids_net_long` | 8306-8317 | 12 | `sizing_tests_2.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 254 | test | `fn:pi_theta_step_reverse_open_from_parent_container_not_position` | 8318-8368 | 51 | `sizing_tests_2.rs` | `b36aa3975e` feat(theta_v0): 增量塔MACD接入+接入链身份稳定+ |
| 255 | test | `fn:bit_exact_cached_indices_vs_fallback` | 8369-8435 | 67 | `sizing_tests_2.rs` | `5ace35459a` perf(strategy): 工位4d 热点①② base派生索引 |

## 四、未逐块落点的 22 行（全部为结构性，非逻辑）

多重集逐行核验（原文件 8099 非空行）后，仅下列 22 行在新树中无对应：

- `mod tests {` + 其配对 `}`（2 行）——单文件时代的测试包裹层，已被 11 个
  `#[cfg(test)] #[path = "..."] mod tests_N;` 挂载点取代。
- 13 行文件头 `use`——原样保留在 `coverage/mod.rs`（因 `coverage/mod.rs` 与旧
  `coverage.rs` 模块路径相同，这些 `use` **不需要**加深一层，故与「块内代码统一 +1 跳」
  的归一化规则对不上，属核验脚本的假阳性；已逐条比对确认 13 行全部在 `mod.rs` 内）。
- 7 行测试模块内 `use`——被各测试文件自带的 `crate::theta_v0::...` 绝对路径头取代，
  导入的符号集合等价。

**结论：零逻辑行丢失。**

## 五、可见性三档审计（影子评审要求，「只缩不扩」纪律）

拆分把单文件切成模块，**文件内引用变成跨模块引用**——这是放宽可见性的唯一正当理由，
且只应放宽到「刚好够用」的那一档。首版一律升 `pub(crate)` 是偷懒；本轮逐项重审，
判据取**真实消费者位置**（注释与字符串字面量已剔除）：

| 消费者位置 | 应取档位 | 理由 |
|---|---|---|
| `coverage/` 目录之外 | `pub(crate)` | 跨目录，别无选择 |
| 仅 `coverage/` 内的**兄弟文件** | `pub(super)` | 出不了目录，crate 级过宽 |
| 仅自己那个文件（含其 `#[path]` 测试子模块） | private | **子模块本就能看见父模块私有项**，测试不构成放宽理由 |

### 审计结果

| 档位 | 条目数 | 条目 |
|---|---:|---|
| `pub(crate)` | 8 | `ElementView`, `StepTrace`, `TwStepCtx`, `VoiceVerdict`, `build_tree_id_index`, `coverage_step_prebuilt`, `operation_role_indexed_split`, `pi_theta_step_traced` |
| `pub(super)` | 17 | `HeldLegMatch`, `ancestors_by_id_lookup`, `ancok_probe_bump`, `apply_gross_cap`, `close_indices`, `coverage_step_from_buckets`, `coverage_step_from_buckets_sep`, `dir_sign`, `element_as_leg`, `element_depth`, `feasible_lex_candidates`, `held_leg_tree_index_indexed`, `held_stale_reregister_idx`, `operation_role_two_segment`, `pi_theta_step_prebuilt`, `restore_ancestor_chain_from_registry`, `strategy_target_legs` |
| `private` | 19 | `FLAT_EPS`, `J_SCALE`, `ancestor_close`, `classify_grade`, `classify_vertical`, `direction_of`, `feasible_candidates`, `feasible_net_cap`, `held_leg_tree_index`, `j_theta_key`, `leg_target_two_segment`, `lot_round`, `operation_role_indexed`, `parent_sign`, `push_element_tree`, `raw_active_set`, `rmove_side`, `scale_key`, `theta_dir_slot` |

### 保留 `pub(crate)` 的 8 项——逐项真实调用点

| 条目 | 目录外消费者 |
|---|---|
| `ElementView` | `fill.rs`×1, `incremental.rs`×2, `interp.rs`×1 |
| `StepTrace` | `shadow.rs`×7 |
| `TwStepCtx` | `fill.rs`×1 |
| `VoiceVerdict` | `shadow.rs`×4 |
| `build_tree_id_index` | `incremental.rs`×1, `interp.rs`×1 |
| `coverage_step_prebuilt` | `incremental.rs`×1 |
| `operation_role_indexed_split` | `interp.rs`×1 |
| `pi_theta_step_traced` | `fill.rs`×1 |

### 相对 main 的净变动

以 main 的 `coverage.rs` 为基准（该文件内 74 项可见性未动）：

**放宽 10 项，全部是 private → `pub(super)`，零 `pub(crate)` 放宽**——
这 10 项都被 `coverage/` 内的兄弟文件消费，拆分前它们在同一个文件里，
拆分后必须跨模块，`pub(super)` 是能工作的最窄档：

```
ancok_probe_bump  ancestors_by_id_lookup  dir_sign  element_depth
held_leg_tree_index_indexed  HeldLegMatch  element_as_leg  close_indices
restore_ancestor_chain_from_registry  held_stale_reregister_idx
```

**收窄 8 项**（main 上是 `pub(crate)`，实测无目录外消费者）：

```
operation_role_indexed          pub(crate) -> private
operation_role_two_segment      pub(crate) -> pub(super)
strategy_target_legs            pub(crate) -> pub(super)
apply_gross_cap                 pub(crate) -> pub(super)
coverage_step_from_buckets      pub(crate) -> pub(super)
coverage_step_from_buckets_sep  pub(crate) -> pub(super)
feasible_lex_candidates         pub(crate) -> pub(super)
pi_theta_step_prebuilt          pub(crate) -> pub(super)
```

最终全量分档：`pub` 48 / `pub(crate)` 8 / `pub(super)` 17 / private 19。
`pub` 48 项 = #359 保证的对外导出面，逐项未动。

> 首版曾把 26 项 private 一律升 `pub(crate)`（其中 16 项其实只在自己文件里用）。
> 本轮全部回收：26 项中 16 项复原为 private、10 项降到 `pub(super)`，**无一保留 `pub(crate)`**。
