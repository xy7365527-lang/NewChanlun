# 影子评审：#317 / commit `381ae777ae`（2026-07-26）

评审者：独立 claude lineage（新上下文，Opus），只读。资格：#317 实装为另一 lineage。
对象：`381ae777ae`（6 文件）。Spec 轴 = `gh issue view 317`（票体 + 范围增补评论 + Resolution）
+ `chanlun/review-results/shadow-review-288-20260726.md` §三 HIGH-1/MED-1/LOW-1。

---

## 一、Spec 轴

| 项 | 判定 | 证据 |
|---|---|---|
| ① rust 真相切单测 ≥3 | **PASS（超额 4+2）** | `rust/src/segment_tangency_tests.rs:48/65`（`three_stroke_overlap` 含等号：互异三笔 / 两笔相切）、`:99/:118`（`is_fractal_and_gap` 上下两臂严格 `>`）；对照 `:81/:135`。两谓词均私有（`segment.rs:128`、`:367` 无 `pub`）⟹ crate 内单测是唯一可达路径 |
| ② Python 真相切单测 ≥3 | **PASS（4+1）** | `tests/test_segment_v1_settlement.py:350` `TestTangencyCaliber`，`:364/:374/:384/:398` + 对照 `:409` |
| ③ 措辞订正逐处 | **PASS** | `test_overlap_boundary_equal`→`test_overlap_strict_disjoint`（名实对齐：数据 max_lo=15 > min_hi=10 是严格分离）+ `:335` 注释；`test_segment_weird_cases.py:6/:241-245`；`test_segment_gap_classification.py` 四处 `>=`→`>`。独立复扫三文件残余 `>=` 判据措辞 **零命中** |
| ④ MED-1 处置 | **PASS，附 MED-1（见三）** | 行号 `:164` `iv_gap` / `:170` `three_stroke_overlap` / `:288` live import 三处逐一核对**属实**；`grep -rn _attrib_segment_divergence`（全仓、排自身）**零外部消费者**属实 |
| ⑤ 篡改双向（独立复验） | **PASS** | rust `<=`→`<`：`segment::tests` 2 相切红、4 其余绿；`>`→`>=` 两臂：2 缺口相切红、4 绿；复原 md5 `ef43028a…` 22/22 绿。Python 参考三处回旧口径：`TestTangencyCaliber` 4 failed / 24 passed，复原 md5 `3dda7f58…` 回绿 |
| ⑥ 全套非 slow | **PASS（照实）** | `cargo test --release --lib` = **1842 passed / 1 failed / 132 ignored**，唯一失败为 #110 `extract_signals_bit_exact_digest_guard`（`left=0xe6a263e343e43845`，在案）。ignored 132 vs 票面 133 系并行 session 漂移。pytest 三文件 **62 passed**（venv + `PYTHONPATH` shadow，已验 `a_segment_v1` 解析到 worktree） |
| ⑦ 与 main 线 `61c958d204` 等价 | **PASS（真超集）** | main：rust 3（`tangent_counts_as_overlap` / `tangent_up_no_gap` / `tangent_down_no_gap`）+ Python 3（落 `weird_cases`）。本 commit：rust 4 相切 + 2 对照（多出 pairwise 相切形态与向下严格对照）、Python 4+1（落 `settlement`）。锚项「先红后绿」已由 ⑤ 独立复现 |

`.cache/BZ_1min_2024_raw.parquet` 为 gitignored 软链（`.gitignore:37`）、`test_rust_segment_equivalence.py:208-210` 确为 `pytest.skip` ⟹ 模块头「立项事实」属实，HIGH-1 的失败场景成立且已被本 commit 堵死。

## 二、Standards 轴

| 项 | 判定 | 证据 |
|---|---|---|
| `#[path]` 挂载 + 800 行红线 | **PASS（压线）** | `segment.rs` 799 行；挂载点 `:798-799`。见 LOW-2 |
| 测试只测外部行为 | **PASS（设计如此）** | 两谓词私有，集成测试不可达；`#[cfg(test)] mod` 是票面唯一可行手段，非绕过封装 |
| 090 声明=实际 | **PASS 主体** | 模块头显式登记「手填 vs 机器耦合」边界（不冒充 Lean 导出）、L0 等级；`main()` 运行时打印退役声明。例外见 MED-1 |
| no-patch-mentality / no-workaround | **PASS** | 无 feature flag、无旧口径 fallback、无 TODO 尾巴 |
| 认识论等级标注 | **PASS** | rust 模块头与 `TestTangencyCaliber` docstring 均标 L0 并声明「不构成实盘有效声明」 |
| 前件保护 | **PASS（优于 main）** | 每条相切用例先 `assert_eq!(lo, hi)` / `assert!(is_fractal)`，杜绝断言退化为同义反复——main 线 3 条无此保护 |

## 三、问题分级

### MED-1｜MED-1 的处置理由自相矛盾（不 reopen）

`analysis/_attrib_segment_divergence.py` 模块 docstring 同段内并列两句：
- 不切口径的理由：「切口径会使下方实测数字不可复现」；
- 三行后：「参考侧是 live import ⟹ 下方『237 / 221 / 237』是旧口径下的历史记录，**重跑必然漂移**」。

后者成立则前者失效——数字已经不可复现，保留旧口径复刻侧不再保护任何可复现性。票的**范围增补评论**原文是「同票切或显式冻结，**别只登记**」，交付为「退役登记 + 运行时打印」。

**不判 HIGH / 不建议 reopen**：shadow-review-288 §五(b) 自设的撤销条件是「已有退役/冻结标记或已被票承接 → MED-1 撤销」，本 commit 已满足；且原失败场景（被当作参考基线引用）已被「**不得作为任何口径的参考基线引用**」+ `main()` 运行时打印双重阻断，零外部消费者亦经独立复核。残余仅是一句失效的论证。
**建议**：删去该句，或在后续票中把复刻侧两谓词按 #246 切齐（数字重取）。

### LOW-1｜`theta_v0/parser/feature_seq.rs` 四处旧口径判据措辞（#317 范围外，新发现）

`:468`（`b_l >= a_h → 缺口`）、`:471`（`b_l=12 >= a_h=10`）、`:484`（`a_l >= b_h → 缺口`）、`:487` 仍以 `>=` 表述缺口判据，而同文件生产侧 `:133/:147` 已是严格 `>`。与 shadow-review-288 LOW-1（Python `gap_classification` 四处）**同类同数**，但落在 theta_v0 侧，不在 #317 声明工作面内。用例数据为严格跳空故仍绿。**建议另下票同批订正**（可并入 #248/#312 工作线）。

### LOW-2｜单行叠属性写法使 800 行合规依赖「不跑 rustfmt」

`segment.rs:798` 把 `#[cfg(test)]` 与 `#[path = "..."]` 压在一行。`cargo fmt` 会拆为两行 ⟹ `segment.rs` 变为**恰好 800 行**（仍 ≤ 上限，未越线）。本仓无 fmt gate（`.github/workflows/` 无 fmt/clippy job；`cargo fmt --check` 全仓 4767 处漂移，非本票引入），故不构成缺陷，登记备查。

### LOW-3｜`weird_cases.py:6` 声明在合流后会变假

「本组无真相切用例」对本分支属实（C 组用例 `:255-296` 构造确为严格跳空递减 / 明确重叠，已逐条核对）。但 main 线 `61c958d204` 已向**同一文件**加入 3 条相切用例（`weird_cases` +45 行）⟹ 两线合流后该声明失效（090 前瞻风险）。**建议**：合流时把该句改为指路句，或把 Python 锁位置与 main 线对齐。

## 四、本评审未覆盖 / 异常照实登记

1. 未跑：`cargo test --release`（集成面）、段落伞 155、parity 8/8、`test_bitexact_large_real_streaming`（后者 #307 承接）。Resolution 中「OKLO 236/236」「BZ 前 30000 bar 六层指纹」未独立复跑。
2. **不可复现异常**：首次 rust 篡改跑中，`theta_v0::parser::segment::tests` 有 2 条同时转红（`three_stroke_overlap_lean_fixture_bit_exact` / `distinct_triple_tangent`）。二次以同一篡改定向复跑 **3/3 绿**，复原后 22/22 绿；静态核查确认 theta_v0 谓词（`parser/segment.rs:130-140`，`max_lo <= min_hi`）与我改的 legacy 谓词无任何代码路径耦合、无 `#[path]` 共享。判为并行 session（工作区 `rust/src/theta_v0/strategy/coverage.rs` 处于 ` M`）造成的构建态一过性，**非本 commit 缺陷**，但如实登记。
3. 工作区：评审对象 6 文件 `git status --porcelain` 全程为空（篡改后均以 md5 校验复原）；`runner.rs`/`coverage.rs` 与 ~370 个 ` D` 未触碰。

## 五、结果包六要素

1. **结论**：Spec 轴 7/7 PASS（含独立篡改双向复验与 main 线锚等价性核对），Standards 轴 6/6 PASS。问题 MED×1、LOW×3，**无 HIGH，不建议 reopen #317**；#288 可凭本 commit 关闭。
2. **定义依据**：#246 裁定「相切=重合」全域生效；Lean `formal/Origin/SegmentFeatureSeq.lean:102/:111/:118`（`HasGap` 严格 `<`、`Overlaps` 闭区间、`gap_iff_not_overlap`）；实装两谓词（`segment.rs:131` `lo <= hi`、`:378/:382` 严格 `>`）与之逐字吻合，新增用例的期望值即该口径的人工编码（模块头已诚实登记非机器导出）。
3. **边界条件**（本结论何时翻转）：(a) 若 #246 裁定被再 supersede，本轮 8 条相切用例（rust 4 + Python 4）须同批改，否则由 PASS 转 FAIL；(b) 若日后 `_attrib_segment_divergence.py` 出现消费者，MED-1 升 HIGH；(c) 若仓库引入 fmt gate，LOW-2 升 MED（`segment.rs` 触 800 上限，再加一行即越线）。
4. **下游推论**：两谓词现被非数据依赖的 L0 锁覆盖 ⟹ #246 口径不再能在 CI（parquet 缺失、`test_segment_bitexact_real` skip）下静默回退。`break_evidence.gap_type` 的 `second→none` 翻转面不受本 commit 影响（零生产代码改动）。
5. **谱系引用**：#246（相切=重合裁定，supersede Lead #84 点3）、#248（同款模板 `491f638a8c`）、#277（裁路① + main 线 `61c958d204`）、#288（落码，因 HIGH-1 reopen）、#302（评审）、#312（theta_v0 机器耦合缝）、#307（重型窗口）；裁定书 `chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`。
6. **影响声明**：只读产出，除本报告外未改任何文件。评审期间对 `rust/src/segment.rs` 与 `src/newchan/a_segment_v1.py` 做过临时篡改，均已 md5 校验复原（`ef43028a572d2353d1d57bbb3663fbe6` / `3dda7f5879f0a56694480e2d1d872b4e`）。`rust/target/` 因复跑产生构建产物（gitignored）。未触碰共享 venv（`PYTHONPATH` shadow）。
