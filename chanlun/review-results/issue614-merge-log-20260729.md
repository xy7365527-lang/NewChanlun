# #614 双线并轨合并执行日志（方向 (a)：kimi 线并入 main）

- 日期：2026-07-29
- 工位：`/private/tmp/wt-614`，分支 `merge-614`（off main tip `efcd6b20bf`）
- 执行车：opus（实施执行车，高难档）
- 票据：#614；方向裁定 = 编排者 2026-07-28「(a) kimi 线并入 main，main 保持名义现役」
- 交付 commit：合并 `79a2715070`，收口 `2080d2bcad`
- 纪律：主仓 `/Users/silencehan/Projects/NewChanlun` 与 `/private/tmp/kimi-nest-mainline` 全程只读，
  零写入；未 push、未 update-ref、未改动 `main` / `kimi-nest-mainline-20260717` 任何引用；
  编译/测试全部 `CARGO_TARGET_DIR` 隔离。

---

## §0 前置：清场与端点（⚠与任务书快照有偏离）

前一辆执行车在 merge 中途被叫停，现场残留 `MERGE_HEAD`。`git merge --abort` 首次失败
（`Entry '…/ancok_tests.rs' not uptodate`）——工作树有前车手改但未与 index 同步的文件。
`git reset --hard` 被 git 护栏拦截，改用非破坏路径：先 `git checkout -- <非 unmerged 脏文件>`
把工作树对齐 index，再 `git merge --abort`（rc=0）。清场后 `HEAD == efcd6b20bf`，
`merge-614` 自有 commit 数 = 0，唯一残留是前车手写的 `.resolve_614.py`（已备份至
`/tmp/wt614-prevcar-resolve.py` 后删除）。**未继承任何半成品解法。**

### 端点复算

| 项 | 任务书给定 | 实测（开工时） | 处置 |
|---|---|---|---|
| main tip | `efcd6b20bf` | `efcd6b20bf` | 一致 |
| kimi tip | `1cab40f9d8` | **`797c9ad35c`** | ⚠ 见下 |
| merge-base | `7e6bf7c7ea` | `7e6bf7c7ea` | 一致 |
| 重叠面 | 70 文件 | 70 文件 | 一致 |

⚠ **移动靶**：`kimi-nest-mainline-20260717` 在开票后又推进了 10 个提交（#624 retrace_ledger
S4 对拍 + 旧模块删除、#636 研究报告入库），tip 从 `1cab40f9d8` 走到 `797c9ad35c`。
实测 `1cab40f9d8` 是 `797c9ad35c` 的**严格祖先**（反向不成立）。

**处置 = 按当前 tip `797c9ad35c` 合并**，理由：(i) 严格更完整，不丢弃任何一侧；
(ii) #614 正文自己写明「两线均在增长中——本票数字为开票时点快照，实装时须复算」；
(iii) 任务书要求的 `1cab40f9d8` 祖先验证由此自动满足（已实测，见 §5）。

### 两线规模（merge-base 之后）

- main 领先 **232** 提交 / **1183** 文件
- kimi 领先 **217** 提交 / **360** 文件
- merge-base `7e6bf7c7ea`（2026-07-25 03:40）：main 曾于 `93abf6496a`（2026-07-25 05:25）
  合过 kimi 线到此点；**此后两线再次分叉四天**，这是本次冲突面的成因。

---

## §1 冲突全景（49 文件）

`git merge --no-ff --no-commit 797c9ad35c` 产生 49 个冲突文件。分三组：

| 组 | 文件数 | 性质 |
|---|---|---|
| A 文书/纪律面 | 5 | 迁移性删除 vs 增补 |
| B CI / Python 面 | 9 | 同一问题的两种修法 + 注释措辞 |
| C+D rust 生产面 | 35 | **两次独立大重构 + 互不相交的功能开发** |

### 本次合并的根本困难（先讲清楚，再看逐文件）

C+D 组不是普通冲突。实测三条硬事实：

1. **两线各自独立实现了同一份拆分蓝图**。
   - `strategy/coverage.rs`（merge-base 5760 行）：kimi 线 #359 拆成 7 模块 + 门面；
     main 线 #398 commit message 原话「按 #359 蓝图**在 main 8436 行版本上重做**」。
   - `classifier/mod.rs`：kimi 线 #576 拆成 ~14 个子模块（mod.rs 3901 → 149 行）；
     main 线保持内联主体并另加 `center_lifecycle` / `interval_necessity`。
   - 两侧模块**边界不同**（main 的 coverage 多出 `compose.rs` 三件，kimi 没有）。
2. **两侧在同一模块的后续修复互不相交**（票号面实测，coverage/ 内出现次数）：

   | 票 | main 侧 | kimi 侧 | | 票 | main 侧 | kimi 侧 |
   |---|---|---|---|---|---|---|
   | #315 | 0 | 12 | | #572 | 6 | 0 |
   | #350 | 0 | 30 | | #594 | 5 | 0 |
   | #446 | 0 | 13 | | #625 | 2 | 0 |
   | #512 | 0 | 2 | | #398 | (拆分) | 0 |

3. **main 做过全局标识符改名**：`ShortDiff → ReverseOpen`（#281 裁定 / #283 实装，
   commit message 明载「行为零改动」）。coverage/ 内 `ReverseOpen` main 18 处 / kimi 0 处；
   `ShortDiff` main 14 处（均为历史注释）/ kimi 200 处。

**结论**：C+D 组不存在机械并集路径。任何「按文件挑一侧」的混搭都会产生编译不通的树，
而真正的语义并集 = 把一侧的重构在另一侧重做一遍，那是独立票的工程量，不是合并解。
本次按**「main 为主干 + kimi 独有能力逐项并入」**处置，所有未随入项在 §3 逐条登记。

---

## §2 冲突逐文件留痕

### A 组：文书 / 纪律面（5）

| 文件 | main 侧改了什么 | kimi 侧改了什么 | 解法 | 理由 |
|---|---|---|---|---|
| `CLAUDE.md` | #517 纪律文本单源化：正文 89 行全删，只留「基因组已迁 AGENTS.md」指针 | 净增 21 行「形式化验证节拍（fixture 漂移 gate）」（#263/#245） | **并集** | main 的删除是针对 merge-base 旧内容的政策迁移；kimi 的 21 行是**新增能力**且其配套件（`scripts/check_fixture_drift.py` + ci.yml `fixture-drift` job）随本次合并入树。实测 main 的 AGENTS.md 不含该节（grep=0），删掉即真丢失 |
| `.claude/rules/common/coding-style.md` | #517 整删（同批删 23 个 `.claude/rules/**`，共 -1121 行） | +6 行 #421 S-H1 裁定注记（NestLifecycleBook 不可变例外） | **取 main 删除**，⚠LOW-11 | 删除是全域政策（23 文件同批），不是针对本文件。实测该裁定实质另存于 kimi 侧 `chanlun/review-results/shadow-429-*.md` 等 9 份文档，随合并入树，裁定本身不丢 |
| `chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md` | 建档（+70） | 建档（+74） | **取 kimi** | 逐行 diff：kimi 版 = main 版 + §3.4 域界澄清 + 两个签字位已落（main 侧仍是 `[ ] 空位`）。**严格超集**，零丢弃 |
| `chanlun/review-results/gap2-gamma-candidate-dump-design-20260719.md` | #504 归档出仓整删（同批 849 文件） | +2 行 #563/#600 勘误 | **取 main 删除**，⚠LOW-10 | 与 849 文件同批政策一致；单独复活 2 份反而制造不一致。kimi 该勘误的代码侧同款注释随 fill.rs 钩子移植入树（见 ⚠MED-5）。恢复指针：`797c9ad35c` 下 blob `d290516b34` |
| `chanlun/review-results/v3-lifecycle-rebuild-impl-20260724.md` | 同上（#504 整删） | +48/-4 三段订正注记（#421/#527/#559） | **取 main 删除**，⚠LOW-10 | 同上。订正实质指向 `issue421-acceptance-selfcheck-20260727.md` / `issue527-panlive-l1-provider-20260728.md`，二者随合并入树。恢复指针：blob `c3fcd1b546` |

### B 组：CI / Python 面（9）

| 文件 | main 侧 | kimi 侧 | 解法 | 理由 |
|---|---|---|---|---|
| `.github/workflows/ci.yml` | 新增 `rust-check` job（#438 闭合 required-features 盲区） | 新增 `fixture-drift` job（#265 Lean→fixture 闭环） | **并集：两个 job 全留** | 两个 job 相互独立，只因插在同一位置才冲突。落地后 YAML 解析校验通过：jobs = `test`(10 steps) / `rust-check`(4) / `fixture-drift`(5)；触发面 `main-rewritten` 两侧一致未动 |
| `src/newchan/a_divergence_topo.py` | #488：恒等 diagram 短路返 0.0，其余仍走 `persim.wasserstein` | #324：整个换成自实装 `wasserstein_1`（cdist 代价矩阵），彻底不走 persim | **取 kimi** | 同一问题（persim 的 sklearn 平方展开在零距离留 ~1e-7 平台相关残差）的两种修法，不可并集。kimi 版覆盖面严格更大（对**所有**输入平台无关，非仅恒等情形），且经编排者三裁。实测 main 侧在该文件的改动**全部落在冲突区内**（无旁路 hunk），故取 kimi 不丢 main 其他工作。**并已核**：main 的 `test_identical_diagrams_bypass_solver_residual`（monkeypatch persim 返 1e-8 后断言结果 == 0.0）在 kimi 实现下仍成立（根本不调 persim） |
| `src/newchan/a_topology.py` | 同上（`_wasserstein_distance` 内加短路） | 删除 `_wasserstein_distance`，模块头改指 `a_wasserstein.wasserstein_1` | **取 kimi** | 同上。已核 `_wasserstein_distance` 在合并树无任何消费方（grep 仅命中其自身定义）；`src/newchan/a_wasserstein.py` 随合并入树 |
| `tests/test_concept_registry.py` | #488：内联加 `_relations_is_lfs_pointer` 守卫 | #324：改用 `real_relations` fixture + `@pytest.mark.integration` | **取 kimi** | kimi 的 `tests/conftest.py::real_relations_usable` 守卫（存在 + 首行 JSON 可解析）是 main 那条（仅 LFS pointer 检测）的**超集**，且该 conftest 已干净并入。已核 `_relations_is_lfs_pointer` 在合并树仍有 3 个消费方，无悬空 |
| `analysis/k4_1min_rust_lib.py`<br>`analysis/residual_chanlun_flow_velocity.py`<br>`tests/test_rust_recursive_equivalence.py`<br>`tests/test_rust_segment_equivalence.py`<br>`src/newchan/a_segment_v1.py` | #246 口径变更注释（较简版） | 同一注释，信息更全（补 #277 裁路①、#288 落码、BZ 全年 strict 28 段/optimized 159 段实测数） | **取 kimi** | 纯注释措辞，kimi 侧严格更全。`a_segment_v1.py` 已核 main 侧改动全在冲突区内 |

### C 组：`strategy/coverage/` 18 文件（add/add）

**解法：全部取 main 侧（#398 拆分）。** 这是本次最大的单点取舍，理由链：

1. 方向 (a) = 并入 main，main 是名义现役线；
2. main 的 coverage/ 多出 `compose.rs` / `compose_tests_1.rs` / `compose_tests_2.rs`
   （#398「trace→compose 正名」），这三件作为 main-only 新增**已无冲突地在树上**，
   取 kimi 侧会让它们成为引用不存在符号的悬空件；
3. main 侧其他无冲突新增模块 `account.rs` / `channel.rs` / `shadow.rs` 硬依赖 main 版
   coverage 的导出（`Vertical` / `StepTrace` / `VoiceVerdict` / `CoverageElement` /
   `Dir` / `GradeRel` / `Horizontal` / `OperationRole`），而 kimi 的 #359 收口**删掉了
   22 个导出**（门面导出集：main 48 / kimi 10）；
4. main 侧 232 提交 / 1183 文件 vs kimi 侧 217 / 360——取 main 侧丢弃面更小。

未随入项 → ⚠HIGH-1。

同组关联处置 `strategy/mod.rs`：**并集**。保留 main 的 `pub mod account;`，同时保留 kimi 的
五个 LEE 模块声明（`level_ledger` / `level_attrib` / `level_order` / `level_clock` /
`level_risk`），使 kimi 的 LEE M1–M4 层随合并**保持可编译**。可行性已实测：`level_ledger`
的唯一硬依赖 `use super::coverage::{SepLeg, Vertical}` 在 main 版 coverage 中均已导出；
其余四个模块对 coverage 只有 doc 链接、无代码依赖。文档行的 `ShortDiff` 同步改为
`ReverseOpen` 以对齐 main 词汇。

### D 组：rust 生产核心（16）

| 文件 | main 侧 | kimi 侧 | 解法 | 理由 |
|---|---|---|---|---|
| `theta_v0/parser/segment.rs` | 重构为复用 `Interval::overlaps`（两两相交，一维 Helly） | 保留内联 `max_lo <= min_hi` + 增 `overlap_probe_bump` 相切计数（#346/#347） | **逐 hunk 取 kimi** | 两者按一维 Helly **逻辑等价**（main 自己的 doc 就是这么论证的）；kimi 版额外保住 probe 仪器。**未用 `--theirs`**——main 在该文件非冲突区新增的测试 `three_stroke_overlap_tangent_counts_as_overlap` 原样保留（已核在树） |
| `theta_v0/parser/feature_seq.rs` | 同型（缺口谓词注释） | 同一谓词 + `gap_probe_bump` 计数 | **三 hunk 全取 kimi** | 谓词字面完全相同（`b_l > a_h` / `a_l > b_h`），差别只有 probe。同上逐 hunk 处置 |
| `segment.rs`（legacy，仓根 rust/src） | 内联 3 个相切测试 | 外置 `segment_tangency_tests.rs`（#317 回归锁） | **取 kimi** | 实测 kimi 外置模块含 6 个测试，**完整覆盖 main 3 个用例**并多出 3 个（pairwise tangent / 两个 strict-cases-unchanged）。两侧谓词改动（`lo <= hi`、`b_l > a_h`）本就相同已自动合并，落地后已核仍在 |
| `rust/src/lib.rs`<br>`rust/src/orchestrator.rs` | #246 口径 doc（较简） | 同一 doc，信息更全 | **取 kimi** | 已逐一核实 main 侧改动全部落在冲突区内，无旁路 hunk |
| `bin/theta_overlay.rs` | — | 尾行加 #305 评审 LOW-1 口径注 | **取 kimi** | 纯注释增补 |
| `bin/pi_bsp_timing.rs` | #412 测试外置（`mod cli_args_tests`，避 800 行硬顶） | 内联 `mod tests`（#65 量纲③ qty 免疫测试） | **并集** | 两个测试模块主题完全不同（CLI 解析 vs χ 量纲③），模块名不冲突。已核 `chi_dimension_three_return` 在合并树可解析（第 60 行导入） |
| `theta_v0/nautilus/strategy.rs` | 仅改 doc（+8/-6） | #345 把每 bar 全量 `classify_with_tower` 改为 `OwnedIncrementalClassifier::append_bar`（O(n²)→摊还 O(1)，+115/-13） | **代码取 kimi（自动合并结果），doc 并集** | main 几乎没动这个文件，故代码区 kimi 的改写干净应用。doc 合成 = main 的关⑤接线锚 + kimi 的 #345 口径；`ShortDiff` → `ReverseOpen` 对齐 main 词汇。**该决定连带要求保留 kimi 的 `classifier::streaming`**，见下 |
| `theta_v0/classifier/mod.rs` | 内联主体 3901 行 + 自有 +226/-40（#486 结构锚 / #487 historical_bound 三类证书 / #321 边界声明作废 / #607 D2 fixture / `level_idx: u32` 签名） | 拆成门面 149 行（+43/-3609） | **取 main 主体**，并**补声明 kimi 独有三模块** | 决定性约束：已自动合并的 `classifier/signal.rs` 是 main 的大版本（`extract_first_third_resume` 多 `level_idx: u32` 参数），kimi 的 `pipeline`/`incremental` 用旧签名 ⟹ 取 kimi 门面必编译不过。补声明的三个是 kimi**新增能力**且有树上消费方：`ledger_kernel`（`nest_lifecycle.rs:101` 消费）、`streaming`（`nautilus/strategy.rs:36` 消费）、`retrace_ledger`（#624 裁定 A 下 `first_retrace_replay` 的迁入去处）。同时**删除** `pub mod first_retrace_replay;` 悬空声明——该文件已由 kimi `8b8905def2`（#624 裁定 A「5 删」）删除，功能迁入 `retrace_ledger/` |
| `theta_v0/classifier/signal.rs`（红线：教义判据） | GOLDEN `0xe371_3897_d9bf_978c`（#455 删 `BspPoint.level_origin` 之后） | GOLDEN `0xe6a2_63e3_43e4_3845` + #610 bisect 归因取证 | **取 main 值 + 并入 kimi 归因** | 判据是**合并树的真实结构**：已核 `bsp.rs` 无 `level_origin` 字段（main #455 生效）⟹ main 值是对的那个。main 自己的历史值表就把 kimi 的现行值记作「该字段删除前，#455 之前最后一版」，两侧口径自洽。kimi 的 #610 归因（bisect 定位翻转引入提交 `bbbd8f89fa`）是真取证，已逐字并入注释链并加⚠说明本树取 #455 后口径。**GOLDEN 未重算、未自定义 Debug 掩盖**——摘要闸测试实测绿 |
| `theta_v0/backtest/{admission,fill,opsem_dump,runner,wverify_run}.rs` | 主体（fill +3102/-1057、runner +2936/-2128、wverify_run +1430/-13、opsem_dump +1091/-6） | 各自的拆分/新增（fill +1788/-229、runner +1217/-74 等） | **全部取 main** | 与 classifier 同因（main 的 signal.rs 主导 + main 侧绝对工作量更大 + main 无冲突新增模块依赖其 API）。冲突 hunk 形态实测印证「主体 vs 门面」：`wverify_run` h3 = main 1613 行 vs kimi 56 行、h4 = main 224 行内联测试 vs kimi 2 行外置；`fill` h3 = 2219 vs 349、h14 = 1 vs 755；`runner` h12 = 0 vs 970。未随入项 → ⚠HIGH-3 |
| `rust/tests/theta_v0_lean_parity.rs`（红线：证书真值路径） | 整删（`aa75566a67` #181，原因原文「验证对象整体退役」——同批删 `closed_loop/sell.rs` 432 行 + 卸载 `pub mod sell`） | +128 行 §7 相切机器见证（#248→#296 Decidable→#312 主缝锁→#319 端点机器耦合） | **并集：卖侧段随对象退役，§7 逐字保留** | §1–§6 依赖 `closed_loop::sell`，该模块在合并树已不存在（已核目录无 `sell.rs`、`mod.rs` 无 `pub mod sell`）⟹ 保留必编译不过。§7 只依赖 `parser::segment::Interval`，与卖侧无关。**关键核查**：`Interval::gap`/`overlaps` ↔ Lean `decide(HasGap)`/`decide(Overlaps)` 的断言**只存在于 §7**——crate 内 `parser::gap_overlap_fixture` 的三个测试只查 fixture 自身性质（端点合法/真值互补/形态匹配），**不测 `Interval`**。丢 §7 = 丢真实验证覆盖。§7 及其 `GapOverlapSection`/`GapOverlapCase`/`intervals()` 与 kimi tip **逐字相同，零语义改写**；被删的只有引用已不存在符号的卖侧段 → ⚠MED-7 |

---

## §3 ⚠ 待人工复核清单（12 条）

### ⚠HIGH-1 — `strategy/coverage/` 取 main 侧，kimi 侧 coverage 域工作未随入

**未随入的 kimi 提交**（均在 merge-base 之后，实测非 main 祖先）：

| commit | 票 | 内容 |
|---|---|---|
| `b57da4bde4` | #247 | registry 恢复祖先角色输入重建（kimi 侧独立实现；main 另有自己的 #247 三提交） |
| `8bc7c67e98` | #267 | held 腿占位元素角色输入重建 |
| `ecb8923c19` | #266 | 头表第三来源归属 + element_depth 链域声明 |
| `2ca040d9fa` | #315 | held 腿占位修补改统一 fixup 形状（堵时序孔） |
| `7d8b45be70` | #310 | **LEE M4 级别 sizing/risk**：`w_ℓ` 资金权 + 级别风险帽 |
| `c3cd34bcea` | #346/#347 | 合成数据波形/真结构护栏/config 私有化/probe 交叉核对/环形守卫 |
| `19aea33a26` | #351 | M4 级别帽四 MED 补课（Σw 校验接线/帽后二次裁剪等） |
| `760c3520c5` | #350 | restore fixup 时序孔修复（#247 同类第三位点） |
| `ea027130f7` | #358 | 评审浮出小修批 |
| `28773fbc99` | #359 | 收口 H-1/M-1：删 22 死重导出 + `build_tree_id_index` 归位破环 |
| `f838540eff` | #446 | **活动集同 ElementId 双计修复**（open/restore 按 ID 判重） |
| `8d8895c652` | #512 | 修复 #511 打回 MED×5（release 防线升 fail-loud + golden v2） |

其中 **#446 双计修复 / #350 时序孔 / #315 占位修补是生产正确性修复**，不是纯重构。
`clamp_levels_to_weighted_cap` / `level_cap`（#310 M4 帽的实际裁剪点）只存在于 kimi 版
`sizing.rs`，本合并树无此函数——kimi 的 `level_risk.rs` 模块头对它的引用现为悬空 doc 链接
（rustdoc 警告级，不影响编译/测试）。

**建议**：另开票，把上述 12 个提交的语义 delta 移植到 main 的 `coverage/` 拆分上。

### ⚠HIGH-2 — `classifier/mod.rs` 取 main 内联主体，kimi #576 拆分未随入

以下 kimi 文件**留在树上但未被任何 `mod` 声明**（Rust 不编译未声明模块，故无害，但是死重）：

```
classifier/pipeline.rs          classifier/cand_delta.rs      classifier/tower_cache.rs
classifier/incremental.rs       classifier/sublevel.rs        classifier/stage_profile.rs
classifier/cp_replay_diagnostics.rs   classifier/oracle_probe.rs   classifier/incremental_profile.rs
classifier/tests/{cache_and_units,classify_basics,incremental_tower,…}.rs
```

它们与 main 内联主体是**同一份代码的两种组织**，同时声明会重复定义
（`classify` / `classify_with_tower` / `Classification` / `LevelState` / `TowerCache` /
`classify_with_tower_incremental`）。**建议**：另开票裁定「main 内联 vs kimi 拆分」二选一，
胜方吸收另一方 delta，败方文件出仓。

### ⚠HIGH-3 — `backtest/` 五文件取 main 侧，kimi 侧未随入项

- **LEE M1–M4 在 `fill.rs` 的接线**：kimi 版 fill.rs 有 ~400 行 `level_ledger`/`level_order`/
  `level_clock`/`level_attrib` 逐 bar 归因接线（h3 = 349 行 + h14 = 755 行）。本合并树里
  这五个 `level_*` 模块**已保留可编译**（见 §2 C 组 `strategy/mod.rs` 并集处置），但**未接线**
  ——即能力在、生产路径不走它。
- **#419 逐科目 fee quote**：见 ⚠MED-6。
- **`wverify_run` 拆分**：`wverify_run/{report,m8,tests,issue71_chi_gamma}.rs` 留树未声明（死文件）。
- 其余 kimi 侧 hunk：#571 四类 carrier-only 关闭事件统一 drain、#289 LOW-1 窗口终点单源化、
  runner.rs h12 的 970 行端到端做空腿测试等。

### ⚠HIGH-4 — `TowerCache` 三访问器 + `freeze_boundary` 手工重放（**已修，需复核**）

合并提交 `79a2715070` 落地后 `cargo check --all-targets` 出现 12 错，全在
`src/bin/p123_fast_replay.rs`（kimi 线 #421/#527/#601 PanLive 探针；实测 main 线对该文件
**一行未动**，kimi +2605/-395，故自动合并取 kimi 版）。它调用 kimi 拆分模块
`classifier/tower_cache.rs` 的 `level_scan_cursor` / `level_scan_units` / `freeze_boundary`。

**两条基线的 `cargo check --all-targets` 均为 0 错**（实测，见 §4）⟹ 这是合并引入的回归，
且 main CI 的 `rust-check` job 跑的正是该命令 ⟹ 不修即 CI 红。

收口提交 `2080d2bcad` 的处置：
- `LevelCache` 增 `last_freeze_boundary: usize`（`#[derive(Default)]`，构造点无需改）；
- 在 `classify_with_tower_incremental` 的 `decompose_resume` **之后**无条件登记该水位——
  位点与公式按 kimi `incremental.rs:597` 同位重放，实参 `(prefix_count, dirty_e)` 与下方
  `extract_first_third_resume` 单一同源；`signal::freeze_boundary_src` 本身是 kimi 线随本次
  合并带入的既有单源公式，**未改一字**；
- `TowerCache` 增三个访问器，分别读 `levels[].scan_cursor` /
  `l0_units_cache`+`levels[].projected_units` / `levels[].last_freeze_boundary`。

**行为面论证**（非机器证明，故列⚠）：新字段是**只写缓存**，生产分类/交易/订单/风控路径
零读点；唯一读点是新访问器，唯一消费方是诊断 bin `p123_fast_replay`。
**验证**：`cargo test --lib` 补齐前后逐项相同（2441/0/138），`--all-targets` 由 12 错转 0 错。

### ⚠MED-5 — #563/#600 γ dump 生产钩子手工移植进 main `fill.rs`

`backtest/gamma_dump.rs` 是 kimi 独有模块（#563），其生产钩子在 kimi 的 `fill.rs` 里；
取 main 版 fill.rs 后钩子丢失，而模块与其 4 个测试留在树上 ⟹ 4 红
（`gamma_candidates.jsonl` 从未被写出）。

移植内容（三处，按 kimi 注释**钉死的不变量**定位，非猜位）：
1. `GammaDump::from_env()` 紧随 `OpsemDump::from_env()`（同 kimi 相对位置）；
2. `gamma_chi_admitted` 紧随 χ 过滤产出 `step_gamma_trade`、**在 nest gate 收窄同一 Vec 之前**
   ——kimi 原注释：「下游 Nest/Xzd 会继续收窄同一 Vec，故须在其消费前取 χ 真值」。main 的
   `fill.rs` 结构同形（χ 过滤 → `step_gamma_trade` → nest gate 同名遮蔽），位点唯一确定；
3. `write_step` 钩子紧随 `pi_theta_step_traced_with_risk_seeds` 产出 `step_trace`
   ——kimi 原注释：「step_gamma／生产 χ 成员／step_trace 三者首次同时在手、且在 PanDiv
   最终选址消费前」。

env 门控：未设 `OPSEM_GAMMA_DUMP_DIR` ⟹ `from_env()` 返 `None` ⟹ 两处分支不进入，
零分配零写入（#563 L5 订正措辞：「零额外指令」应读作「零额外**分配/写入**」，该订正
逐字随移植带入注释）。**验证**：移植后 4 红转绿，全库 2441/0/138。
**⚠ 待复核**：这是跨两套 fill.rs 的手工重放，未经影子评审。

### ⚠MED-6 — `overlay_state.rs` 合并后回退至 main 版（#419 未随入）

该文件自动合并成功，结果带 kimi 的 #419 签名（`apply_open`/`apply_close`/
`settle_forced_virtual` 的 `fee_rate: f64` → `fees: &FeeQuoter`），但唯一消费方是 main 版
`fill.rs`（实测 main 的 fill.rs **完全没有 `FeeQuoter`**，grep=0），三处调用点类型不匹配。

**处置**：`git checkout efcd6b20bf -- rust/src/theta_v0/strategy/overlay_state.rs`。
理由：这是**资金/费用路径**，在 fill 循环里现造 `FeeQuoter` 属于凭空写钱路代码，红线
「拿不准语义就⚠，不许猜」。实测 main 侧在该文件的改动只有 4 行（就是 ShortDiff→ReverseOpen
改名），故回退代价 = 丢 kimi 的 #419 逐科目 fee quote 重构（+74/-24）。
附带修正：`side_sign` 提为 `pub(crate)`（恢复 kimi 版可见性，kimi 的 `level_ledger.rs` 消费它）。

### ⚠MED-7 — `rust/tests/theta_v0_lean_parity.rs`（证书真值路径）

见 §2 D 组末行。要点复述：§7 与 kimi tip **逐字相同**，未作任何语义改写；被删的仅是引用
已不存在符号（`closed_loop::sell`）的 §1–§6。该文件属 #449 禁令域相邻面，故列⚠。

### ⚠LOW-8 — `l3_*` 三处 `fee_rate` 解包移除

`l3_fullwindow.rs:148` / `l3_pi_falsify.rs:165` / `l3_delta_r_alpha.rs:891` 原为
`res.fee_rate.expect(SCALAR_COST_RATE_UNDEFINED)`（kimi 的 `Option<f64>` + fail-loud 口径，
#512 系）。main 的 `RunResult.fee_rate` 是 `f64`（runner.rs:144），故改为直取。
**代价**：kimi 的「成本率未定义即 fail-loud」纪律在这三处退化。

### ⚠LOW-9 — `Vertical::ShortDiff` → `ReverseOpen` / `ExitType::CloseShortDiff` → `CloseReverseOpen`

对 kimi 侧仍用旧名的**已编译**文件做改名对齐（`backtest/open_ledger.rs` 7+3 处、
`strategy/level_ledger.rs` 5 处、`bin/theta_overlay.rs` 1 处）。依据 = main 的枚举定义注释
原文「原 `ShortDiff`，#281 更名（#283 实装）」+ 该 commit message「行为零改动」。
未改动的：`coverage/role.rs:53` 是历史 doc；`wverify_run/{report,m8,tests}.rs` 是未声明死文件。

### ⚠LOW-10 — 两份 review-results 随 main #504 归档删除（见 §2 A 组，含 blob 恢复指针）

### ⚠LOW-11 — `.claude/rules/common/coding-style.md` 随 main #517 下线（见 §2 A 组）

### ⚠LOW-12 — 未声明死文件留树

见 ⚠HIGH-2 / ⚠HIGH-3 的清单（classifier 9 个 + `classifier/tests/` + `wverify_run/` 4 个）。
不影响编译与测试，但是并线债，建议随 ⚠HIGH-2/3 的裁定一并清理。

### 红线合规声明

- `formal/**/*.lean`：本次**零冲突、零改动**（49 个冲突文件中无一在 `formal/`）。
- #449 禁令域 / v3 硬禁令域：**零触碰**。
- 证书真值路径：仅 `theta_v0_lean_parity.rs` 一处，处置见 ⚠MED-7（逐字保留，无语义改写）。
- 教义判据（`classifier/signal.rs` GOLDEN、`nest_lifecycle.rs`）：`signal.rs` GOLDEN 取
  **与合并树真实结构一致**的那个值并保留双侧归因；`nest_lifecycle.rs` 无冲突（自动合并取
  kimi 版），仅通过补声明 `ledger_kernel` 使其可编译，**文件本身零改动**。

---

## §4 三方测试对照（照实登记）

命令：`cargo test --lib`，三方各自独立 `CARGO_TARGET_DIR`（无缓存串扰）。

| 树 | worktree / target | passed | failed | ignored | EXIT |
|---|---|---:|---:|---:|---|
| 基线 main `efcd6b20bf` | `/private/tmp/wt-614-base-main` / `/tmp/wt614-target-basemain` | 2137 | **0** | 136 | 0 |
| 基线 kimi `797c9ad35c` | `/private/tmp/wt-614-base-kimi` / `/tmp/wt614-target-basekimi` | 2199 | **0** | 138 | 0 |
| **合并树 `2080d2bcad`** | `/private/tmp/wt-614` / `/tmp/wt614-target` | **2441** | **0** | 138 | **0** |

### 失败集指纹比对

三方失败集**均为空集**：

```
基线 main   : {}
基线 kimi   : {}
合并树      : {}
```

⟹ **零新增红**。合并树用例数 2441 > max(2137, 2199)，是两侧测试集的并（扣除未随入模块
所带的用例）。

### 额外验证：`cargo check --all-targets`（超出任务书要求，因发现回归而登记）

| 树 | errors | EXIT |
|---|---:|---|
| 基线 main | 0 | 0 |
| 基线 kimi | 0 | 0 |
| 合并树（合并提交 `79a2715070` 时）| **12** | 101 |
| 合并树（收口提交 `2080d2bcad` 后）| **0** | 0 |

12 错全部集中在 `src/bin/p123_fast_replay.rs` 一个 bin，归因与处置见 ⚠HIGH-4。
**若只跑任务书要求的 `cargo test --lib`，这条回归不会暴露**（该命令不编译 bin），
而 main CI 的 `rust-check` job 会红——故本项虽超范围仍执行并登记。

### 中间态记录（090 照实）

合并提交前的中间态曾出现 4 红，全部同因（`gamma_candidates.jsonl` 未写出）：

```
theta_v0::backtest::gamma_dump::tests::gamma_dump_chi_consistency
theta_v0::backtest::gamma_dump::tests::gamma_dump_env_gated_bit_exact
theta_v0::backtest::gamma_dump::tests::gamma_dump_schema_on_synthetic
theta_v0::backtest::gamma_dump::tests::gamma_dump_thread_local_override_is_parallel_safe
```

归因 = kimi 的 γ dump 生产钩子随 main 版 fill.rs 丢失；处置见 ⚠MED-5，修复后转绿。

---

## §5 双向祖先验证

```
$ git merge-base --is-ancestor efcd6b20bf merge-614 ; echo $?
0        # main tip  → IS ancestor ✓

$ git merge-base --is-ancestor 1cab40f9d8 merge-614 ; echo $?
0        # 任务书给定的 kimi tip → IS ancestor ✓

$ git merge-base --is-ancestor 797c9ad35c merge-614 ; echo $?
0        # 实际合并的 kimi tip → IS ancestor ✓

$ git log -1 --format='%H parents=%P' 79a2715070
79a2715070d6ed59dbf6491a7c5ded0eaaa0cca4 parents=efcd6b20bf955f05d5f34e935d36f0d88741a657 797c9ad35c239baf253b57e738303684af69f974
```

⟹ #614 验收条 2「合并后单线同时包含两线旧 tip」**成立**，且对任务书快照 tip 与实际
分支 tip **双双成立**。

---

## §6 未决问题（落线前还需什么）

1. **⚠HIGH-1/2/3 的三次裁定**（编排者）：coverage / classifier / backtest 三个模块各自
   「main 内联 vs kimi 拆分」二选一，胜方吸收败方 delta。这是并线债的主体，
   建议各开一票，**不建议在 #614 内解决**——每一项都是独立的重构移植工程。
2. **⚠HIGH-1 的 12 个 kimi 提交移植**，其中 #446/#350/#315 是生产正确性修复，优先级最高。
3. **⚠MED-5 / ⚠HIGH-4 两处手工重放的影子评审**（γ dump 钩子、TowerCache 访问器）。
   两者都有「零生产读点 / env 门控 no-op」的行为面论证，但未经独立评审。
4. **⚠MED-6 的 #419 fee quoter 归属裁定**（资金路径，需人裁）。
5. **CI 触发链**（#614 验收条 5）：`ci.yml` 触发面仍是 `main-rewritten`（两侧一致，本次未动）。
   方向 (a) 无结构改动，按票据评论「main-rewritten 镜像随下次常规推送随动」——
   **本车未验证该镜像的实际状态**，如实登记为未完成项。
6. **AGENTS.md 工作线记忆更新**（#614 验收条 6）：本车未改 AGENTS.md。落线（`update-ref`
   到 main）之后才应更新为「合并后事实」，否则记忆先于事实。
7. **落线动作本身**：本车按纪律**未 push、未 update-ref**。`merge-614` 分支与
   `/private/tmp/wt-614` 工位均按要求保留，等编排者裁定后再落。
8. **死文件清理**（⚠LOW-12）：随 1 的裁定一并处理。

---

## §7 工位与产物

- 保留：`/private/tmp/wt-614`（分支 `merge-614`，tip `2080d2bcad`）
- 已清理：`/private/tmp/wt-614-base-main`、`/private/tmp/wt-614-base-kimi`（`git worktree remove`）
- 日志留档：`/tmp/wt614-{merged,basemain,basekimi}-test.log`、`/tmp/wt614-{check-all,basemain-check,basekimi-check}.log`
- 主仓与 `/private/tmp/kimi-nest-mainline`：全程只读，零写入
