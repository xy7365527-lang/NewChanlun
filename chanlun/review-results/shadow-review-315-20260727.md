# 影子评审报告：#315 held 腿占位统一 fixup（commit `2ca040d9fa`）

- **评审票**：#347（实装票 #315）　**评审者**：独立影子评审（新上下文 Opus，未参与 #315 实装）
- **对象**：`2ca040d9fa`，`rust/src/theta_v0/strategy/coverage.rs` 单文件 +125/−19
- **worktree**：`/tmp/kimi-nest-mainline`，HEAD=`7d8b45be70`（#315 已是祖先），`git status` 干净
- **日期**：2026-07-27

## 0. 结论

**无 HIGH，不建议 reopen #315。** 时序孔真正堵死：fixup 移到两个物化循环之后、AncOK 判定
之前，三张查表为本 bar 终态。浮出 **1 MED**（probe 声明超出实装）、**3 LOW**。

## 1. 复跑

| 项 | 结果 |
|---|---|
| `cargo test --release --lib held_leg` | **5 passed / 0 failed** |
| `cargo test --release --lib` | **1879 passed / 1 failed / 133 ignored**，唯一失败 = `classifier::signal::tests::extract_signals_bit_exact_digest_guard`（在案，未碰未归因） |
| diff 面 | coverage.rs 单文件，无越界 |

## 2. Spec 轴

| # | 票面核对点 | 判定 | 行号证据 |
|---|---|---|---|
| S1 | 收集占位 idx，AncOK 判定**前**统一 fixup | **PASS** | `2266` 声明 `placeholders`；`2335`/`2365` 两臂只 push idx；`2451-2453` fixup 循环；`2458` `ancestor_close_by_id`。`2453→2458` 之间无任何 `work.push`/`raw.push` ⟹ 三表确为终态 |
| S2 | 真正堵死父晚物化的时序孔 | **PASS** | fixup 位于 open 候选父链恢复循环（`2437` restore）之后；`raw` 含全部本轮物化元素 ⟹ 三级解析（`2203-2207`）不再受迭代序约束 |
| S3 | `parent_id` 取值与立即式等价 | **PASS** | 立即式传 `leg.op_parent`；统一式读 `work[idx].parent_id`（`2452`），而两臂 push 时 `parent_id: leg.op_parent`（`2328`/`2358`）⟹ 逐值等价，无语义漂移 |
| S4 | 「占位存活性不受影响」论证 | **PASS** | `ancestor_close_by_id`（`853-856`）判据 = `raw_ids: HashSet<ElementId>` + `parent_id` 链，**从不读** `parent: Option<usize>` ⟹ fixup 时序与存活性正交；新测试 `5283-5287` 先断言存活再断言角色，坐实非静态推断 |
| S5 | 角色输入边界仅 V/depth/units | **PASS** | 受 `parent`/`attached_dir` 影响的消费面 = `element_depth`（`1590`）、`parent_sign`→V、`q_units`；新测试三项全断（`5289-5296`） |
| S6 | RED 证据真实性 | **PASS（静态核对，未执行回退）** | RED 值写死在 doc（`Ambient`/600/`p̃=0`，`5243-5247`）与 GREEN 断言（`ShortDiff`/300/`p̃=+300`，`5289-5296`）逐项互斥 ⟹ 回退必红。本评审只读，未实际 no-op 复现 |
| S7 | probe 只读声明 | **PASS** | `2216` 仅 `ancok_probe_bump`，`None` 分支不写 `work`/`raw`（`2208-2219`）⟹ 控制流不变 |
| S8 | 「恒」字四处订正无漏网 | **PASS** | 订正处：helper doc `2185-2193`、两调用点 `2331-2332`/`2360-2362`（改为指针，无全称）、测试 docstring `4995-4998`。全文剩余唯一「恒被 AncOK 剪除」在 `2136-2137`，属 **#247 restore 语境**（registry 丢失 ⟹ 永不物化），#284 已判其成立，非漏网 |
| S9 | 新测试红→绿；既有 held_leg 4/4 不破 | **PASS** | 5/5，见 §1 |

## 3. Standards 轴

| # | 标准 | 判定 | 证据 |
|---|---|---|---|
| T1 | 与 #247（`b57da4bde4`）形状逐字对齐 | **PASS** | 解析三级 + `let p_eps = work[pidx].eps` + `set_parent_attached`（`2203-2211`）与 #247 fixup（`2142-2152`）逐字同构；`for &idx in &placeholders` 与 `for &idx in &restored` 同形 |
| T2 | 090 声明与实际一致 | **FAIL ×1（MED-1）** | probe doc（`100-105`、`2193`、`2450`）声明「生产窗口可观测，可与 AncOK 剪除计数交叉核对」，但唯一 probe 报表 `ancok_l2_ceiling_exposure_real_btc`（`runner.rs:4946-4991`）既未打印该字段、也无对应的「占位被剪除」计数 ⟹ 交叉核对不可执行。其余声明逐条与实装相符 |
| T3 | 测试只测外部行为 | **PASS** | `held_leg_placeholder_parent_materializes_in_later_iteration`（`5250`）只调 `coverage_step_from_buckets_sep` 公共出口，断言 `(next_active, p_tilde, sep_legs)`，无一处读私有字段 |
| T4 | 占位收集向量确定序 | **PASS** | `Vec<usize>`（`2266`），push 序 = `prev_active` 迭代序（确定），非 `HashSet` ⟹ fixup 序确定 |
| T5 | 自指保护 | **LOW-1** | #284 T5 的保护（`rebuild` 在 `raw.push` **之前**）被本 commit 移除：fixup 时 `raw` 含占位自身，`raw.iter().find(...)`（`2207`）未排除 `r == idx`。若 `parent_id == self.id`（或两占位互为父），`element_depth`（`1590-1596`）的 `while let` 无环保护 ⟹ 挂起（非 panic）。可达性需 registry 自环，实践极低；#247 fixup 同形，非本 commit 独有 |
| T6 | doc 四处近似重复（判断题） | **LOW-2** | 时序孔成因在 helper doc（`2176-2183`）、两调用点（`2331`/`2360`）、fixup 尾部注释（`2443-2450`）表述 4 次，尾部 8 行与 helper doc 近逐句同义。**可接受**（各处有本地读者、无相互矛盾），建议尾部收为指针 |
| T7 | no-patch：非防御兜底 | **PASS** | `None` 臂只计数不造值（`2214-2217`），无 `unwrap_or`/无 fallback 伪父 |
| T8 | 同类位点闭合完整性 | **LOW-3（超本票范围）** | 本票把占位 fixup 提到 step 函数尾部，但 #247 的 restore fixup 仍在 `restore_ancestor_chain_from_registry` **内部**（`2142`）⟹ 若某恢复元素的父由**更晚一次 restore 调用**物化，同类时序孔在 #247 路径未闭合。未观测，建议 follow-up |
| T9 | 文件规模 | **NOTE** | `coverage.rs` >5300 行，超 `coding-style.md` 800 上限；非本 commit 引入（+125），#59 收尾宜挂拆分票 |

## 4. 分级

| 级 | 编号 | 摘要 | 建议 |
|---|---|---|---|
| MED | MED-1 | `placeholder_parent_unresolved` 声明「生产窗口可观测 + 可交叉核对」，但未接入唯一 probe 报表（`runner.rs:4946-4991`），且无对应剪除计数 | 补 1 行 `eprintln!` + 封闭性 assert，或订正 doc 措辞（090）。不构成回归，不 reopen |
| LOW | LOW-1 | fixup 移到 raw 终态后，`find` 未排除自身 idx，环形 `parent_id` 会使 `element_depth` 挂起 | 加 `r != idx` 过滤（#247 fixup 同步） |
| LOW | LOW-2 | 时序孔成因 doc 四处近似重复 | 尾部注释收为指针 |
| LOW | LOW-3 | #247 restore 路径的同类时序孔未闭合（超本票范围） | 挂 follow-up 票 |
| NOTE | — | `coverage.rs` >5300 行 | #59 收尾拆分 |

## 5. 结果包六要素

1. **结论**：#315 通过，无 HIGH，不 reopen；1 MED + 3 LOW + 1 NOTE。复跑 1879/1 + held_leg 5/5。
2. **定义依据**：#315 票体四条（统一 fixup / 措辞订正 / 1 测试 / 1 probe）+ #347 核对点；
   `shadow-review-267-20260726.md` §4 MED-1；#247 前例 `b57da4bde4`；090 `no-patch-mentality.md`；
   `coding-style.md`。
3. **边界条件**：结论翻转条件——(a) 若 `2453→2458` 之间新增任何 `raw.push`，S1/S2 失效；
   (b) 若 `ancestor_close_by_id` 改用 `parent` 索引判据，S4 的存活性论证失效；
   (c) 若 registry 允许自环 `op_parent`，LOW-1 升为 HIGH（挂起 = 生产不可用）。
4. **下游推论**：held 腿占位在同 bar 内父晚物化场景的 V/depth/units 由本 commit 起改变
   ⟹ 命中该场景的窗口 p̃ 会变（本 commit 未附生产对拍，因场景两窗口未观测——诚实）；
   `#247` restore 路径仍留 LOW-3 同类孔。
5. **谱系引用**：090（声明与实际一致，MED-1 依据）；#284 MED-1（本票起点）；
   #247 缺口二（形状来源）；#301 探针族（probe 惯用形状）。
6. **影响声明**：本评审只读，未改任何源码；唯一新增文件 = 本报告。
