# 独立影子评审：#358（评审对象 #350，commit `760c3520c5`）

- **评审人**：独立 claude lineage，新上下文，未参与 #350 实装（资格成立）
- **worktree**：`/tmp/kimi-nest-mainline`，`git status` 干净，HEAD=`760c3520c5`
- **对象**：`coverage.rs`（净 +292）、`runner.rs`（+28）、实装方自评 `shadow-review-350-20260727.md`
- **结论**：**无 HIGH、无 MED，4 项 LOW**。不建议 reopen。

## Spec 轴（对照 #358 票体四项核对点）

| 核对点 | 判定 | 证据 |
|---|---|---|
| 孔场景真实性（invalidated 父 registry_lost 固化 + 更晚非 restore push） | **PASS** | `coverage.rs:2450-2467` 的 `Closed\|Invalidated`+`is_boundary_root` 分支确为生产路径（直接 `raw.push`，不经 registry/`overlay_seen`）；`invalidate(&p)` 使 `registry.get(P)` 失败 ⟹ 断链，两条判据（registry 作废 vs leg 状态）独立，构造合法 |
| 票面偏差标注照实 | **PASS（并可加强）** | 票面字面「父由更晚一次 restore 调用物化」实为**不可达**：链内子→父同一 `while` 轮次即处理，唯一断链因是 `registry_lost`，而 registry 状态 bar 内不变 ⟹ 后续任何 restore 调用同样 get 失败。实装方选非 restore push 路径是必然而非将就；报告只写「不完全同构」，未给此不可达论证 |
| #315 同形状执行到位 | **PASS** | 累加器合并 `placeholders`→`pending_parent_fixup`（`coverage.rs:2337-2345`），restore 内立即修补循环整段删除（原 2141-2168），统一 fixup 位于两循环之后、`ancestor_close_by_id` 之前且中间无 `raw.push`（`coverage.rs:2534-2540`）——报告边界条件(a) 成立 |
| 697 ceiling 旧断言推翻论证 | **PASS** | 旧文档 `>0 ⟺ 声部树非严格` 确被反例推翻；新判据 `unresolved − pruned` 语义自洽：unresolved ⟹ 三级解析（含 raw 扫）全失手 ⟹ 父不在 raw ⟹ AncOK 必剪 ⟹ 计入 pruned |
| 无同类第四位点遗留 | **PASS** | 边界根直接 push 元素 `parent_id` 硬写 `None`（真根），不需 fixup；`rebuild_placeholder_parent_attached` 对 `parent_id=None` 返回 `true`（`coverage.rs:2272`），不误计 unresolved |

## Standards 轴

| 核对点 | 判定 | 证据 |
|---|---|---|
| `resolve_pending_parent_fixups` 真消重复 | **PASS** | `coverage.rs:2282-2298` 单一实现；生产（2534）与测试 helper（3648）均委托，无第二份解析循环；`r != idx` 自指守卫（#347 LOW-1）随 `rebuild_placeholder_parent_attached` 继承，无回归 |
| 测试只测外部行为 | **部分 PASS** | 新 #350 测试走生产入口 `coverage_step_from_buckets_sep`（`coverage.rs:5145`）✔；5 个 restore 测试仍白盒（见 LOW-4） |
| 两轮「先反后正」订正链落档完整 | **PASS** | 自评报告 §0/§1/§3 记录了「结论从孔不存在订正为孔存在」及 H1/J1/J2 处置；`coverage.rs:5054-5059` 在旧测试 docstring 内显式标注「本测试不覆盖的方向」并指向新测试，未抹平 |

## 复跑

| 项 | 结果 |
|---|---|
| `cargo test --release --lib` | **1889 passed / 1 failed / 133 ignored**，唯一失败 `extract_signals_bit_exact_digest_guard`（#115 线在案，未碰未归因） |
| `cargo test --release --lib restore_` | **12 passed / 0 failed**（含 2 个新增 #350 测试） |

## 问题分级

- **LOW-1（声明膨胀，090）** `coverage.rs:87`「该差值 #350 修复后**结构性恒为 0**」过强。环形 `parent_id`（`work[idx].parent_id == Some(自身 id)`）下：`rebuild` 的 `r != idx` 守卫 ⟹ 计 unresolved，而 `ancestors_by_id_lookup`（`coverage.rs:832`）环检测使 `chain=[自身 id] ⊆ raw` ⟹ AncOK **保留** ⟹ 差值 >0。此时差值>0 是正确报警（确属「被 admit 却接线不上」），故非功能缺陷，但「恒为 0」措辞若被后续接成生产不变量断言会误伤。建议改「非环形 `parent_id` 数据下恒为 0」。
- **LOW-2（诊断可读性）** `runner.rs:5107` 的 `u64` 减法在 `eprintln!` 中先于其下的 `assert!(pruned <= unresolved)` 执行；release 关 overflow-checks ⟹ 记账不封闭时先打印一个 wrap 巨值再 assert 失败。建议 assert 上移或用 `saturating_sub`。
- **LOW-3（可读性）** 测试 helper `resolve_pending_parent_fixup`（`coverage.rs:3648`）是丢弃返回值的纯委托薄壳，与生产 `resolve_pending_parent_fixups` 仅差一个 `s`。重复逻辑确已消除，但多留一层近同名壳，仅省一个 `let _ =`，建议删除直接调生产函数。
- **LOW-4（测试形状，非本票引入）** 5 个 restore 测试直调私有 `restore_ancestor_chain_from_registry` 后由 helper 自行扮演调用方补跑 fixup ⟹ 若生产调用方漏调统一 fixup，这些测试仍 GREEN。该风险已被新 #350 测试（走生产入口）覆盖，故不阻塞。

## 结果包六要素

1. **结论**：#350 修复形状正确、Spec 五项核对点全 PASS、Standards 抽取真消重复，复跑与实装方声明一致；4 项 LOW 均为文档措辞/诊断可读性/测试形状，无 HIGH，不回票。
2. **定义依据**：#358 票体两轴核对点；#315 统一 fixup 形状；#247 缺口二（角色输入丢失）；090（声明与实际一致）。
3. **边界条件**：结论翻转条件——(a) 若在统一 fixup（`coverage.rs:2534`）与 `ancestor_close_by_id`（2540）之间新增任何 `raw.push`，本次 PASS 判定失效；(b) 若 LOW-1 的「恒为 0」被接成生产 assert，环形数据下将成误判源；(c) 若 `ancestor_close_by_id` 改用 `parent`（idx）而非 `parent_id`（结构）判据，孔的触发条件改变。
4. **下游推论**：`restore_break_registry_lost` 不再是 697 ceiling 判据（已订正为旁路诊断）；`placeholder_parent_unresolved` 语义扩大到 restore 链元素——旧窗口的该 probe 历史读数与新读数**不可直接比较**（本报告新增提示，实装方报告未点明）。
5. **谱系引用**：#247 缺口二（形状来源）→ #315（第二位点）→ #350（第三位点）；#347 LOW-1（自指守卫）/MED-1（交叉核对 probe）；090（声明膨胀）。
6. **影响声明**：本次评审只读，唯一写入 = 本文件。未改 `coverage.rs`/`runner.rs`，未做 git mutation。
