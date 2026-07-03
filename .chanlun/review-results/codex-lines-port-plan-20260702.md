# Downloads 两条 codex 线的算法增量盘点 + 移植方案（s1 输入）

- 工位: ws-codexport（task #4，goal g-full-mutex-impl）
- 日期: 2026-07-02
- 源: 主仓 worktree 直接 git 对比（两线均为 `.git/worktrees/` 下的主仓 worktree，ref 可直接解析）
- 环境: 纯只读，零 git 写。所有 diff/存在性均 `git cat-file`/`git ls-tree`/`git diff --stat` 核过。
- 交付: 拿什么 / 放哪 / 与 s1 逐维对照表（`full-mutex-b1-design-20260702.md`）的维度映射 / 风险

---

## 0. 一句话结论（对任务假设的关键翻转）

编排者指示「算法在别的线优化了，你可以拿去用」。**核对结论：两条线都没有可移植的 rust 算法增量**——因为**三条 codex 线（complete-classification / strict-formal / codex-line-20260625）的 alpha 回测引擎文件 `econ_positive.rs` 计数全为 0**，而 s1 的真缺口整个落在 `rust/src/theta_v0/backtest/econ_positive.rs`（alpha 桶键丢弃 role/σ_p）。**一条不含该文件的分支无法优化该文件**。

唯一真增量 = complete-classification 线的 **72 个 `formal/Origin/Round*.lean` 形式桥**（HEAD 有 0 个），其中 Round130-137 直接**形式化**了 s1 关心的分类载体/关系分类器/决策充分性。但它们是 **L0 Lean 证明**（全部自我限定「不主张 Rust bit-extraction、不主张当前引擎满足假设」），是**附加知识、不是 rust 算法**——对 s1 的 P0 rust 接入零帮助，对 s1 §2.3 的 **H 轴进 z 裁定（task #1）是强形式输入**。

移植建议一句话：**rust 侧拿零；formal 侧 Round130-137 对 task #1「引用而非合并」**（裁定可直接读分支引用，物理合并成本高收益低）。

---

## 1. git 拓扑与增量总账

| 维度 | complete-classification-origin | strict-formal |
|------|-------------------------------|---------------|
| 分支 HEAD | `8b83454ba9` | `1b63379b0f` |
| merge-base vs 当前 HEAD | `ad87acc657`（= #84 引擎移植 merge-base） | `ea32a6b54b` |
| 分支独有提交（HEAD..branch） | **216** | **1** |
| 当前 HEAD 独有（branch..HEAD） | **426** | **427** |
| 含 `econ_positive.rs`？ | ❌ 无 | ❌ 无 |
| 含 `mu_estimator.rs`/`selector.rs`？ | ❌ 无 | ❌ 无 |
| 含 `coverage.rs`？ | ❌（分支删了，见下） | ❌ |
| `formal/Origin/Round*.lean` 数 | **72**（HEAD=0） | 0（这条线走 formal/Strict） |

**方向学关键**：`git diff HEAD branch` 中 rust 侧几乎全是**减号**（`coverage.rs 3666 -----`、`interp.rs 1741 -----`、`ledger.rs 946 -----`）——减号=**HEAD 有、分支无**。即两条分支的 rust 是**分叉前的旧引擎**，HEAD 才是引擎权威线（#84 之后 426 提交把 alpha 引擎建到 `backtest/`）。分支 rust 的 `++` 部分（`parser/segment.rs`、`risk.rs`、`theta_origin_l2_real_promoted_oklo.rs 2146++` 测试）是旧分叉的自有演化，**不是相对 HEAD 的优化**。

> 排错记录：HEAD 的 alpha 引擎在 `rust/src/theta_v0/backtest/`（`econ_positive.rs`/`mu_estimator.rs`/`selector.rs`），不在 `strategy/`。s1 设计稿用裸名 `econ_positive.rs` 指的就是 `backtest/` 这份；HEAD 确实持有，且是三条 codex 线唯一持有者（含 `main`）。

---

## 2. 逐块分类：算法/引擎（可用）vs 证据/文档（可选）

### 2.1 complete-classification-origin（216 提交）

| 块 | 内容 | 分类 | 可用性 |
|----|------|------|--------|
| `formal/Origin/Round1-129.lean`（约 64 个） | Lean 形式桥链（源公理→分类→策略族→TΘ 生命周期） | 证据/形式 | Round130-137 的**依赖前驱**，物理合并须整链带上 |
| `formal/Origin/Round130-137.lean`（8 个） | 分类载体/关系分类器/决策充分性形式化（详见 §3） | **形式（真增量）** | 对 task #1 **引用**；物理合并可选 |
| `formal/Origin/*.lean`（VoiceCover/TotalWealth/ThetaInstantiation 等，被分支删） | HEAD 已有的旧 Origin 定理，分支删除 | — | HEAD 已有，不涉及 |
| rust 侧全部 | 分叉前旧引擎（无 alpha 引擎） | 引擎（旧） | **零可用**——HEAD 是超集 |
| `analysis/_db_sec_pull.py`/`_fetch_es_1s_1y.py` | 数据脚本微调（-18 行） | 纯技术 | 忽略 |

### 2.2 strict-formal（1 提交 `1b63379b0f` "add strict formal engine package"）

| 块 | 内容 | 分类 | 可用性 |
|----|------|------|--------|
| `formal/Strict/HybridAssembly.lean`(+658)/`HybridStep.lean`(+267) | 严格引擎 Lean | 形式 | **HEAD 已超集**（HEAD 版比分支再大 457 行净增，`diff` 显示 `HybridAssembly 457 +----`=HEAD 独有） |
| `docs/formal/result-package/`（README/MANIFEST/VERIFICATION/evidence/…） | 证据包（验证清单 + legacy 审计报告 + test-results） | 证据/文档 | **可选**。HEAD 无此目录 |
| `docs/formal/result-package/original_inputs/pasted-text.txt`（1620 行） | **完全分类原文源文本**（Round 桥所引 source rows 的出处） | 证据/**溯源** | **可选但溯源价值高**（HEAD `pasted-text` 计数=0） |
| OCR 截图 `*完全分类-fpscreenshot*.txt` | 原文图片 OCR | 证据 | 可选 |
| `analysis/_attrib_segment_divergence.py`(+292)/`_xcheck_secondkind_v1.py`(+72)/`segment_refsem_cert.py`(+80) | 段背驰归因 / 二类交叉检验 / 段参照语义证书脚本 | **分析工具（可能可用）** | 待核是否 HEAD 已有等价（见 §5 风险） |

**strict-formal 总评**：engine/formal 侧 HEAD 全超集，rust 侧相对 HEAD 是 `1056 +/47587 -`（几乎纯删=HEAD 独有）。这条线唯一非超集内容 = 证据包 + 3 个 analysis 脚本。**无算法增量**。

---

## 3. Round130-137 逐桥 → s1 逐维对照表的维度映射（任务重点）

依赖链（线性）：`Round130→131→132(+FullDefinitionStrategy)→133→134→135→136→137`。全部标注 `/teach` L0，自我限定「非 Rust bit-extraction、非当前引擎满足假设、非 all-futures quotient」。

| Round | 形式化内容 | 对应 s1 维度 / 缺口 | 对 s1 的作用 |
|-------|-----------|---------------------|--------------|
| **130** CenterRelationCompleteness | 中枢延伸/终结 + 双中枢续接/升级关系（源自 theorem 007） | s1「γ 区间套 N^δ」+ 中枢关系 | 背景形式化，非 P0 |
| **131** ChanLevelStateCarrier | 状态载体 `S=(τ,r,b,u)`：趋势类 τ + R6 中枢相对位 r + **BSP 位 Bool^6（非互斥和）** b + 未完成尾账 u | s1「状态 z」+「I_γ 64 类非坍缩」 | **形式确认 s1 §1 的「BSP bits 是 Bool^6 不压扁」**（对齐 `MuClass.i_class` class_index 6-bit） |
| **132** GlobalClassificationCarrier | 全局分类载体 `C_Θ(x)=(m_Θ 离散模式, r_Θ 充分风险统计)` + 静态唯一类打包 | s1「细状态 Z vs 粗投影 Y」 | 形式框架：分类=离散模式×风险统计包 |
| **133** DecisionSufficiency | **同类→同决策**（若 feasibleSet/intent/objective/policy 都过分类因子分解，则等类产等响应）；前向方向 | s1 §13 细分类优势定理的形式核 | **形式支撑「回测须按分类分桶」**——但明确「不证当前 rust 引擎已满足因子分解假设」（=s1 真缺口的形式镜像） |
| **134** CurrentOrderProjection | **「current Rust policy reads both CompleteClass and position」**；订单键=类×position 细化，粗类仍定 intent | s1 §2.2 推荐 (a)「以 MuClass 为桶键」 | **直接形式背书 s1 的 MuClass(…,position) 桶键设计**（class refined by position） |
| **135** CurrentDecisionProjection | 扩到完整决策束：intent + 有限候选 + objective ledger + risk 投影 + 调度订单 | s1 σ_p/role 接入后的下游决策束 | 形式确认决策束按分类因子分解 |
| **136** RootlessSelfSimilarity | 去根化自相似载体：无特殊根/前沿边界/统一递归单元/前沿升级涌现 + 资本尺度边界 | s1「ℓ_max 去根化前沿」 | 对齐 memory `regime=级别截断伪影` / 去根化前沿 |
| **137** RootlessRelationClassifier | **精确有限关系分类器：纤维恰为 H/V/δ 轴等价类**；字段 `mutuallyExclusive`/`uniqueClass`/`shortDiffIff`/`ambientNoParent`/`exactFiberIffAxisEquiv` | s1 §2.3 **H 轴（Horizontal）是否进 z（选择类，task #1）** | **task #1 的强形式输入**（见 §4） |

---

## 4. 对 task #1（H 轴进 z 裁定）的实质输入

s1 §2.3 把「H 轴进 z」列为**选择类**上浮 codex，理由是「H 对 μ 贡献待验（L2 spec 未覆盖）」。Round130-137 对这个裁定给出**形式骨架**，但**不改变 s1 的裁定性质**：

- **支持「补 H 进 z」侧**：Round137 证明完全互斥关系分类器的**纤维恰为 H/V/δ 三轴**（`exactFiberIffAxisEquiv`+`mutuallyExclusive`+`uniqueClass`）——即原文的 R(g) 完全分类**内在含 H**，去掉 H 就不是那个精确分类器。Round133 证明**决策须过分类因子分解**（同类同决策）。两者合起来=「若决策依赖 H，则 z 必须携带 H」的形式前件。
- **不settle 真缺口**：Round133/134/137 **全部自我限定**「不证当前 rust 引擎满足因子分解假设」「非 Rust bit-extraction」。即它们**规定了目标（z 应含完整 R(g) 含 H），但把「H 是否实际改变 μ/决策」的 L2 经验问题留空**——这**正是 s1 标记的裁定 crux**。
- **对 s1 的净效果**：Round130-137 **强化 s1 tradeoff 的「完备性」侧**，但**不越过 s1**——H 对 μ 的 L2 贡献仍需经验验证，形式桥不能替代。**建议交给 task #1 的 codex 审计时，把 Round137/Round133/Round134 作为「完全分类含 H 的形式依据」一并提供**，让裁定在「形式完备 vs L2 样本经济」之间做的是有形式锚点的价值判断，而非无依据的偏好。
- **附带收获**：Round134 是 s1 §2.2 推荐 (a)「以 MuClass(…,position) 为 canonical 桶键」的**直接形式背书**（「订单键=类×position 细化，粗类定 intent」）——task #1 审 P0 接入设计时可引用，增强「用 MuClass 作桶键=正确」的论据。

---

## 5. 移植方案：拿什么 / 放哪 / 风险

### 5.1 rust 引擎/算法
**拿：零。** 两条线均无 alpha 引擎（`econ_positive.rs` 计数=0），rust 侧相对 HEAD 是分叉前旧引擎。s1 的 P0 接入（role/σ_p 进 `backtest/econ_positive.rs` 桶键）**只能在 HEAD/main 上实装**，与 codex 线无关。**不移植任何 rust。**

### 5.2 formal/Origin Round130-137（真增量）
**推荐：对 task #1「引用而非合并」（cite-not-merge）。**

- **引用路径（推荐，零成本零风险）**：task #1 的 codex 审计直接读 `git show codex/complete-classification-origin-20260626:formal/Origin/Round13{0..7}.lean` 作形式依据。裁定不需要 Lean 文件驻留 HEAD。
- **物理合并（可选，仅当编排者要形式链驻留）**：放 `formal/Origin/`。**但**：Round130-137 依赖链回溯到 `FullDefinitionStrategy` + Round1-129 整条（72 文件）+ HEAD 从未有的 Origin base。HEAD 当前 Round 计数=0 → 移植尾部必须带整棵 Origin Round 子树 + `lakefile.toml` 改动。**这是大规模 formal 嫁接，独立 formal-doc PR，与 rust/alpha 完全隔离（不改任何 alpha 数字）。**

### 5.3 溯源证据（可选）
若物理合并任一 Round 桥，其 source 出处 `docs/formal/result-package/original_inputs/pasted-text.txt`（strict-formal 线，1620 行完全分类原文）应一并带入 `docs/formal/`（HEAD 无）——保证 Round 桥的 `sourceBacked` 字段可回溯。纯溯源价值，非算法。

### 5.4 风险

| 风险 | 说明 | 缓解 |
|------|------|------|
| **Lean 工具链/lake build** | 物理合并 Round 链须 72 文件 + FullDefinitionStrategy + lakefile 改；HEAD Origin base 与分支分叉，可能不 build | 走「引用而非合并」→ 规避；若必须合并，独立 PR + `lake build` 护航，与 rust CI 隔离 |
| **rust 误移植** | 误把分支 rust `++` 当优化移植，会**回退** HEAD 的 alpha 引擎 | 硬结论：rust 拿零；分支 rust 无 `econ_positive`，任何 rust 移植=降级 |
| **形式桥被误当实装** | Round133/137 自我限定「非当前引擎满足假设」，若当成「H 已实装」会声明膨胀（090/231） | task #1 引用时须带其自我限定语；形式桥=目标规定，非实装证据 |
| **analysis 脚本重复** | strict-formal 的 3 个脚本可能 HEAD 已有等价 | 若需，实装工位先 `git diff` 核 HEAD 对应文件，无则可选纳入（纯技术，非本方案范围） |

---

## 6. 结果包（六要素）

1. **结论**：两条 codex 线**无可移植 rust 算法增量**——三线 `econ_positive.rs` 计数=0，s1 缺口全在 HEAD 独有的 `backtest/econ_positive.rs`。唯一真增量=complete-classification 线 72 个 `Round*.lean` 形式桥，其中 Round130-137 形式化了状态载体（131）/全局分类载体（132）/决策充分性（133）/订单·决策投影（134-135）/去根自相似（136）/精确关系分类器 H·V·δ（137）。它们是 L0 Lean 附加知识，对 s1 的 P0 rust 接入零帮助，对 task #1 的 H 轴裁定是**强形式输入**（Round137 证 R(g) 精确含 H + Round133 证同类同决策），但不 settle s1 标记的 L2 经验 crux。移植建议：**rust 拿零；Round130-137 对 task #1 引用而非合并**。
2. **定义依据**：s1 逐维表 §1（真缺口=alpha 桶键丢 role/σ_p）、s1 §2.2 推荐 (a)（MuClass 桶键）、s1 §2.3（H 轴选择类）；分支侧 `Round131`（S=(τ,r,b,u)）/`Round133`（decision sufficiency）/`Round134`（class×position 订单键）/`Round137`（fibers=H/V/δ exact）的定理陈述与自我限定 docstring；git 存在性 `econ_positive count=0`（三线）vs HEAD/main=1。
3. **边界条件（结论翻转）**：(a) 若编排者要求形式链物理驻留 HEAD ⟹ 从「引用」升为「合并整棵 Origin Round 子树」，风险从零升为 Lean build（§5.4）；(b) 若 task #1 codex 裁定 H 对 μ 有 L2 贡献 ⟹ Round137 从「参考」升为「必须实装 H 进 MuClass」的形式依据；(c) 若核出 strict-formal 的 3 个 analysis 脚本 HEAD 无等价且 s2 检验管线需要 ⟹ 可选纳入（纯技术，交 s2/task #2 判）。
4. **下游推论**：(a) task #1 审 P0 接入时应附 Round137/133/134 作「完全分类含 H + MuClass 桶键正确」的形式锚点；(b) 任何「从 codex 线拿算法」的后续指令须先核该线含不含目标文件——本盘点证明「别的线优化了算法」在 rust 层不成立（那些线无 alpha 引擎），优化实际发生在 HEAD 自身的 426 提交里；(c) 若未来做 formal 链合并，须带 `pasted-text.txt` 源保溯源。
5. **谱系引用**：230/231（有效域≠定义域——Round133「不证当前引擎满足假设」是同一模式的形式镜像）；090（声明膨胀禁止——形式桥不得当实装）；s1 设计稿 §2.3（H 轴选择类，本盘点提供形式输入不越裁定）；memory `regime=级别截断伪影`（Round136 去根化前沿对齐）、`T引擎BTC权威基线`/`生产路径=nt`（HEAD 是引擎权威线，codex 线是旧分叉，佐证 rust 拿零）。**建议 genealogist 新结晶**：「『别的线优化了算法』须核目标文件存在性——分叉线可能整个缺失被优化对象，此时增量在形式层不在算法层」。
6. **影响声明**：纯只读，零 git 写，零代码改动。产出=本方案。**不触及**任何 rust/alpha 文件、不合并任何分支、不改 settled 定理。**建议下游动作**：task #1 codex 审计引用 Round130-137（cite-not-merge）；rust 侧 s1 P0 接入照 s1 设计稿在 HEAD `backtest/econ_positive.rs` 独立实装（与 codex 线无关）；formal 物理合并仅在编排者明示时作独立 PR。
