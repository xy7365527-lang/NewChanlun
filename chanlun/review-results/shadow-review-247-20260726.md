# 影子评审：#247 registry 恢复祖先角色输入重建（issue #266）

- **评审对象**：commit `b57da4bde4`（3 文件：`rust/src/theta_v0/strategy/coverage.rs` +263/−11、`formal/Origin/AncestorClosure.lean` 纯注释、`formal/Origin/ActiveSet.lean` 纯注释）
- **评审者**：Opus 5 新上下文（禁自评：实装由另一 session 完成，资格成立）
- **worktree**：`/tmp/kimi-nest-mainline`（HEAD=`2670b90ba3`，b57da4bde4 是其祖先）
- **日期**：2026-07-26
- **权限**：只读 + 本报告落盘；未改任何被评审文件
- **行号口径**：除注明外，均为 `git show b57da4bde4:rust/src/theta_v0/strategy/coverage.rs` 的行号

## 结论

**无 HIGH，不需 reopen #247。** 缺口二（角色输入重建）实装正确、语义边界如实兑现；缺口一（声明补第三来源）主体到位但留下**三条 MED 级声明不一致**——其中两条恰属本票要修的同一缺陷类（声明与实装不符）。

---

## 一、Spec 轴（票 #247 票体 + `mutex-domain-loadbearing-20260725.md` §二）

| # | 验收项 | 判定 | 行号证据 |
|---|--------|------|---------|
| S1 | 缺口二：恢复元素用已知 `parent_id` 重建 `parent` 索引 + `attached_dir` | **PASS** | 修补段 `2350-2374`；`restored` 收集 `2303`/`2347`；写入器 `ElementView::set_parent_attached` `487-498`。解析三级 `id_idx(base) → overlay_seen(overlay) → raw 扫`（`2361-2364`），与循环内 `already_in_raw`/复用分支同口径（`2073-2088`） |
| S2 | `attached_dir` 口径 = 父容器 eps（`push_element_tree` 同口径） | **PASS** | 修补段取 `work[pidx].eps`（`2371`）；`push_element_tree` 压子元素传 `Some(eps)`（`471`/`477`）。口径一致 |
| S3 | 「父不在时明确该情形语义，与防御分支区分」 | **PASS** | 断链保持 None/None（`2364` 解析失败 ⟹ 不写）+ 注释 `2365-2369` 显式区分；∂ 由 `if let Some(pid)` 短路（`2360`） |
| S4 | 断链元素恒被 AncOK 剪除（不伪造的正当性） | **PASS（独立复核）** | `ancestors_by_id_lookup`（`785-792`）**先 `chain.push(pid)` 再 lookup** ⟹ 不可解析的 `pid` 也进祖先链 ⟹ `raw_ids` 不含 ⟹ `ancestor_close_by_id`（`874-881`）剪除。且对**下游多级链传递成立**（子的链含 C 与 P_lost 两跳） |
| S5 | ∂（`parent_id=None`）保持 None/None ⟹ V=Ambient | **PASS** | `2360`；测试 `restore_boundary_germ_keeps_parent_none`（`4896-4914`） |
| S6 | 缺口一：`RegistryRestore` 写进转移声明/契约锚 + Lean 侧对应表述 | **PARTIAL（MED-1）** | rust 12 处锚（`18/720/887/1593/1853/1980/2044/2432/2491/2809/4054` 等）；Lean `ActiveSet.lean:302-309`、`AncestorClosure.lean:217-227` 纯注释。**但**文件头对照表 `18` 把三来源公式挂到 `active_set_step`，而同一 commit 的 `884-887` 明写 `active_set_step` 是**无第三来源**的 M16 理想式原语 ⟹ commit 内部自相矛盾 |
| S7 | 「恢复元素不再恒定 V=Ambient/G=SameLevel；**防御分支回归「不应到达」**」 | **PARTIAL（MED-2）** | restore 路径 PASS（测试 `4780-4811`/`4831-4857` 断言 ShortDiff/SubLevel/depth=1/units=300）。**但同文件 held 腿占位三处 `2255-2256`/`2281-2282`/`2299-2300` 仍写死 `parent:None, attached_dir:None`**，恒命中同一条「父越界/None 防御归 ℓ_g」分支（`1276-1279`）⟹ 该验收条字面未满足。commit 照实声明并挂 #267；#267 已在 HEAD 落地（`held_leg_*` 4 测试绿） |
| S8 | 「若行为改变影响下单：给出改动前后 `p̃`/腿集合对拍证据」 | **PARTIAL（LOW-1）** | 仅单测层对拍（`restore_rebuilds_parent_attached_dir_full_chain`：p̃ 0→+300）。生产窗口无差值证据。票面「未决前提」允许先出 trace 票，commit 照实标注；工作区未提交的 #301 探针正在补 |
| S9 | 既有 `coverage.rs:4578` 场景测试 + 全量不退化 | **PASS** | 复跑 `cargo test --release --lib`：**1834 passed / 1 failed**，唯一失败 = `extract_signals_bit_exact_digest_guard`（#110 在案，未修未归因）。定向：`coverage::tests::restore` 5/5 绿、`held_leg` 4/4 绿 |

---

## 二、Standards 轴

| # | 项 | 判定 | 证据 |
|---|----|------|------|
| T1 | 090「声明与实际一致」 | **FAIL ×3（MED）** | MED-1（S6）、MED-2（S7）、**MED-3**：`element_depth` doc（`1582`）声明「parent 索引指向 base 段 carrier（`< candidate_start ≤ base.len`），链全在 base」——本 commit 使该不变量失效（恢复元素的 parent 可指向 **overlay** 段，测试 `4787` 断言 `work[0].parent == Some(1)`=overlay），doc 未同步。HEAD 仍为旧文（`coverage.rs:1817`） |
| T2 | 090「不留半成品/不加 workaround」 | **PASS** | 无 try/catch 吞异常、无 TODO 遗留、无特例硬编码；断链走 AncOK 正规剪枝而非兜底分支 |
| T3 | 语义口径 `attached_dir = 父 eps` | **PASS** | 见 S2 |
| T4 | 断链不伪造 / ∂ 保持 | **PASS** | 见 S3/S4/S5 |
| T5 | doc 锚语义（RegistryRestore=物化机制，非新数学来源）与实装一致 | **PASS** | Lean 两处均为注释块内（`/-- -/`、`/-! -/`），无证明语句改动；理想式 `AncOK[(A_t\D_t)∪B_t]` 未动。`python3 scripts/check_fixture_drift.py` = **exit 0，全部无漂移** |
| T6 | 生产路径真消费修补结果（非仅测试） | **PASS** | 调用序 restore(`2271`/`2357`) → `ancestor_close_by_id`(`2365`) → `strategy_target_legs`(`2368`)；`strategy_target_legs` 的 `overlay_sibling` 从 `elements.overlay` 现建（`1616-1619`）⟹ 读到修补后字段 |
| T7 | 性能/bit-exact 论证不被破坏 | **PASS** | 修补是循环后 O(restored) 一趟，`raw` 扫兜底有界；`operation_role_two_segment` 的双段等价论证（`1611-1612`）在新 key `(Some(pidx), ℓ)` 下仍成立（base idx < base_len ≤ overlay idx，升序分段查等价） |
| T8 | 不可变性（`coding-style.md` CRITICAL） | **LOW-2** | `set_parent_attached` 就地 mutate。与本文件 `ElementView` 既有 push/mutate 惯用法一致，不计缺陷 |
| T9 | 测试覆盖 | **LOW-3** | 未覆盖：(a) 断链**多级下游链**的传递剪除（现测试仅单元素）；(b) 跨 restore 调用经 `overlay_seen` 解析父的路径；(c) `debug_assert!`（`491`）在 `--release` 验证 profile 下不生效 |

---

## 三、问题分级

| 级 | 编号 | 问题 | 建议处置 |
|----|------|------|---------|
| — | 无 HIGH | — | 不 reopen #247 |
| MED | MED-1 | 文件头对照表 `coverage.rs:18` 把 `∪RegistryRestore` 三来源公式挂给 `active_set_step`，与 `884-887` 自述「理想式原语，无第三来源」直接矛盾——正是本票缺口一要消除的缺陷类 | 表格第 2 行改指 `coverage_step_from_buckets`，或拆两行（理想式原语 / 生产三来源）。挂新 issue 或并入 #59 收尾 |
| MED | MED-2 | 验收条「防御分支回归不应到达」在 b57da4bde4 时点字面未满足（held 腿占位三处仍命中同一防御分支） | 已由 #267 在 HEAD 闭合，无需新动作；建议在 #247 评论补一句「该验收条由 #247+#267 联合满足」 |
| MED | MED-3 | `element_depth` doc（`1582`，HEAD `1817`）的「parent 链全在 base」不变量被本 commit 打破，doc 未同步 | 一行 doc 订正；HEAD 仍待修 |
| LOW | LOW-1 | 无生产窗口 `p̃`/腿集合对拍 | #301 探针在跟进，照实声明已达 no-声明膨胀要求 |
| LOW | LOW-2 | 就地 mutate 与 `coding-style` 不可变原则的局部偏离 | 不建议改（与本文件既有设计一致） |
| LOW | LOW-3 | 断链传递性 / 跨调用父解析 未测 | 各补 1 条测试即可 |

---

## 四、结果包六要素

1. **结论**：见上。无 HIGH；三条 MED 均为声明层，不影响 `p̃`/订单正确性。
2. **定义依据**：`push_element_tree`（`456-479`）确立 `attached_dir=父eps` 口径；`classify_vertical`（`1294-1301`）+ `classify_grade` 消费 `attached_dir`/`parent`；`ancestors_by_id_lookup`（`780-793`）+ `ancestor_close_by_id`（`847-882`）确立 AncOK 按 `parent_id` 判、不可解析祖先入链的剪除语义；anc.pdf §11（物化非新数学来源）。
3. **边界条件（结论何时翻转）**：(a) 若 `ancestors_by_id_lookup` 改为跳过不可解析 `pid`，则 S4「断链恒剪除」失效，断链元素会带 None/None 进角色计算 ⟹ 本评审的 PASS 翻为 HIGH；(b) 若 `element_depth` 或任何消费者引入「parent 必在 base 段」的运行时假设（如 `elements.base[p]` 直接索引），MED-3 立即升为 HIGH；(c) 若 registry 中出现 `parent_id == 自身 id` 的元素，`element_depth` 无环守卫 ⟹ 死循环（现有 ID 由 `(level, ordinal)` 生成，父级别严格更高 ⟹ 不可达）。
4. **下游推论**：恢复元素的角色从恒定 `(Ambient, SameLevel, depth=0)` 变为真角色 ⟹ 走 `dir_weight` 的 ShortDiff 豁免/禁用分支与 `w_depth[depth>0]`；H 轴 key 由 `(None,ℓ)` 变 `(Some(pidx),ℓ)` ⟹ `role.h` 也可能翻（不进 `dir_weight`，不改 units，但改 `SepLeg.role` 诊断面）。所有依赖旧口径的 restore 场景统计需重算。
5. **谱系引用**：090（严格性=语法规则，声明膨胀禁止）；`formalization-validity-domain`（本 commit 的对拍等级 = L1 管线正确性，非 L2 真实数据——commit 已照实标注）；#244 裁定 `mutex-domain-loadbearing-20260725.md` §二。
6. **影响声明**：本评审只读，未改任何代码/文档；唯一写入 = 本报告。跑了 `cargo test --release --lib`（工作区含别 session 未提交的 #301 `#[cfg(test)]` 探针与 runner.rs 改动，纯旁路不改控制流）与 `scripts/check_fixture_drift.py`（写临时目录，未碰 `rust/tests/fixtures/`）。`git status` 中 ~370 个 ` D` 条目属别 session，未触碰。
