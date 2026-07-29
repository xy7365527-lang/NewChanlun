# 影子评审：票 #248 相切=重合口径落码（commit `491f638a8c`）—— 评审票 #276

- 日期：2026-07-26
- 评审人：Opus 5 主控（新上下文，禁自评规则满足——实装由 Kimi session 完成）
- 评审对象：worktree `/tmp/kimi-nest-mainline` 分支 `kimi-nest-mainline-20260717` commit `491f638a8c`（7 文件 +652/−13）
- 评审性质：只读评审 + 独立复跑；未修改任何被评审文件（本报告为唯一写入）
- **时点声明**：按 commit 当时形态评。当前 HEAD 已含 #296（`Overlaps` 补 `Decidable` instance、导出器直化 `decide (Overlaps)`），涉及 `decide (¬ HasGap)` 绕法的判定按当时形态给出，并单列现态实证。

---

## 0. 独立复跑证据（全部本 session 亲跑）

| 复跑项 | 命令 | 结果 |
|---|---|---|
| lib 全量 | `cargo test --release --lib` | **1834 passed / 1 failed / 133 ignored**；唯一失败 `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#110 在案），digest `left=16618955402698307653 / right=10432481772907336594` 与报告 §2 记录**逐位相同** → 未修未归因 |
| parity | `cargo test --release --test theta_v0_lean_parity` | **11 passed / 0 failed**（含 `lean_gap_overlap_tangent_bit_exact`） |
| fixture 机器导出 | `cd formal && lake env lean Origin/ParityFixtureExport.lean` → `cmp` 盘上 fixture | exit=0，**BYTE-IDENTICAL**（现态导出器已直化 `decide (Overlaps)`，再生仍与盘上逐字节一致） |
| 量化探针 OKLO | `cargo run --release --bin tangency_probe -- analysis/data_cache/oklo_1m_databento.json ...` | `gap_evals=292 tangent_up=8 tangent_down=13 overlap_calls=4 overlap_tangent=0`；`segments=236` |
| 量化探针 BTC | 同上，`btc_1m_full.json` | `gap_evals=307 tangent_up=2 tangent_down=1 overlap_calls=1 overlap_tangent=0`；`segments=245` |

**四数字全部复现**：OKLO 21/292（7.2%）、BTC 3/307（1.0%）、三笔起点两品种均 0 次、段数 236/245 —— 与报告 §3/§4 逐数字一致。

> 注：报告 §2 记的 lib 基线是 `1821/1`，本次复跑为 `1834/1`。差值来自 worktree HEAD 已推进（#295/#296 等新增测试），非本 commit 引入；唯一失败测试与 digest 值不变。

---

## 1. Spec 轴（票 #248 票体 + 三评论 / #249 SPEC / 裁定书 §4·§5·§6）

| # | 要求（出处） | 判定 | 行号证据 |
|---|---|---|---|
| S1 | 缺口谓词改严格 `>`（裁定书 §4 表行1） | **PASS** | `feature_seq.rs:132`（`b_l > a_h`）、`:145`（`a_l > b_h`） |
| S2 | 三笔重合改含等号 `<=`（裁定书 §4 表行2） | **PASS** | `segment.rs:141`（`max_lo <= min_hi`） |
| S3 | 两处**同批**改（裁定书 §6；分批制造 `gap_iff_not_overlap` 断裂） | **PASS** | 同一 commit `491f638a8c`，无中间态 |
| S4 | Lean 侧零改动（SPEC「Lean 侧：零改动」/裁定书 §4） | **PASS** | 本 commit `formal/` 唯一改动 = `ParityFixtureExport.lean` +32 行，全部为 `def gapOverlapJson` / `def feOf` / `fixtureJson` 字面量；**无 theorem / instance / 证明义务改动**；`SegmentFeatureSeq.lean` 未被触碰 |
| S5 | SPEC 增补 (a)：复用 `Interval::overlaps`，或保留独立实现但注释显式指明同口径 | **PASS**（走豁免路径） | `segment.rs:127-128` 有「与既有原语 `Interval::overlaps`（本文件 :90-93，闭区间 `≤`）**同口径**；保留独立实现仅为避免构造三个 `Interval` 的开销，非另起口径」 |
| S6 | SPEC 增补 (b)：fixture 含相切两形态 + 严格分离 + 严格重叠，期望值机器导出禁手填 | **PASS**（用例齐、机器产） | `ParityFixtureExport.lean:181-189` 5 用例；`theta_v0_parity.json` `gap_overlap` 段；逐字节再生一致（§0） |
| S7 | 主缝：`theta_v0_lean_parity` 断言**两个谓词**的期望值来自 Lean 导出（SPEC「Testing Decisions / 主缝」，且明言次缝不替代主缝） | **FAIL** | 见 **HIGH-1** |
| S8 | 次缝：模块内补相切单测 | **PASS** | `feature_seq.rs:490-505`（2 个）、`segment.rs:799-818`（2 个） |
| S9 | 连带作废声明（裁定书 §5：两处 Python bit-exact 对齐声明作废） | **PASS** | `feature_seq.rs:100`（原「bit-exact 对齐 Python `_is_fractal_and_gap`」已删）+ `:111-112` 作废声明；`segment.rs:120-126` 作废 + supersede 登记 |
| S10 | #84 点3 supersede 登记指向裁定书 | **PASS** | `segment.rs:117-119`、`:121-125` |
| S11 | 影响量化：频次 / 段划分 / 下游（裁定书 §6） | **PASS** | 报告 §3/§4/§5；四数字本 session 复现 |
| S12 | §5 收窄裁定如实注明 | **PASS** | 报告 §5 开头 ★ 段完整引述编排者 2026-07-25 收窄裁定，并声明原全窗半成品 dump 作废未使用 |
| S13 | 成交/净值维度缺项照实 | **PASS** | 报告 §6.3 给出编译失败的具体符号与行号（`fill.rs:328`/`runner.rs:72,79` 引 `#[cfg(test)]` 项），归 ESCALATE-2 不修不触碰 |
| S14 | 回填 map #59 | **PASS** | map #59 正文已有 #248 条目（含 5 用例、1821/1、ESCALATE 四项） |

---

## 2. Standards 轴（090 严格性 / 机器耦合纪律 / 可复现性）

| # | 标准 | 判定 | 证据 |
|---|---|---|---|
| T1 | fixture 机器导出、禁手填 | **PASS** | 导出器全部字段为 `decide` 求值；独立再生 `cmp` 逐字节一致 |
| T2 | `decide (¬ HasGap)` 绕法是否如注释所证 | **PASS（附 LOW-1）** | 见下节 |
| T3 | 两谓词同批 | **PASS** | S3 |
| T4 | 量化数字可复现 | **PASS** | §0 五项全复现 |
| T5 | 090 诚实性（不声明代码不具备的能力） | **PASS（报告层）/ 边缘（commit message 层）** | 报告 §9 明写断言对象是「rust `Interval::overlaps`/`gap`（theta_v0::parser::segment 既有原语）」——如实；但 commit message 与 map 回填只写「fixture 机器见证落地 / parity 11/11」，未点明该断言不覆盖本次改动的两个谓词。归入 HIGH-1 一并处理 |
| T6 | ESCALATE 照实上报不绕过 | **PASS** | 报告 §8 四项均具名到行号；旧版 `rust/src/segment.rs` 活代码缺口主动上报（后由 #277/#288 承接落地） |
| T7 | 未量化项不粉饰 | **PASS** | §5.3 PROVIDER views 312→310、一条 TERM 行序前移 2 位均照实登记并给出「一致解释，非独立证明」的归因限定 |
| T8 | 认识论等级标注（`formalization-validity-domain.md`） | **PASS** | parity §7 注释沿用 L0/L1 标注，明写「绿 = 实装忠实于形式化，**不**构成盈利或实盘有效声明」 |
| T9 | 探针对生产路径的侵入 | **PASS（附 LOW-2）** | 见下节 |

---

## 3. 问题清单

### HIGH-1｜主缝（Lean parity）零覆盖本次改动的两个谓词 —— 建议 reopen #248

**事实**（可逐条核）：

1. 本次改的两个谓词是**私有** `fn`：`feature_seq.rs:113 fn is_fractal_and_gap(`、`segment.rs:130 fn three_stroke_overlap(`（均无 `pub`）。集成测试 `rust/tests/theta_v0_lean_parity.rs` 在 crate 外，**无法调用**。
2. `theta_v0_lean_parity.rs` 全文对这两个名字的出现只有一处，且在注释里（`:437`）。测试体（`:459-491`）断言的是 `a.gap(&b)` 与 `a.overlaps(&b)`，即 `segment.rs:90-97` 的 `Interval::gap` / `Interval::overlaps`。
3. `Interval::overlaps` **本次未改动**，且按 #249 SPEC 自述「早已是闭区间（`≤`，与 Lean `Overlaps` 一致）」——即主缝新增的 5 个断言，锁的是一个改动前就已合规的第三份实现。

**失效场景**（可直接推演）：把 `feature_seq.rs:132` 的 `b_l > a_h` 改回 `>=`，或把 `segment.rs:141` 的 `<=` 改回 `<`，`cargo test --test theta_v0_lean_parity` 仍然 **11/11 全绿**。SPEC 主缝的立项目的——「口径日后若在任一侧漂移，测试会红」——对本次统一的口径未达成。

**后果**：两个生产谓词的相切口径，当前唯一保障是模块内 4 个**手填期望值**的单测（`assert!(!g)` / `assert!(three_stroke_overlap(..))`）。SPEC 明确定其为「次缝，**不替代**本条主缝」，且手填正是 SPEC 要消除的转录漂移来源（631 机器耦合模式）。

**补法建议**（任一即可）：
- 将两谓词提为 `pub(crate)`，在 `rust/src/theta_v0/parser/` 内新增 crate 内 parity 测试，`include_str!` 同一 fixture 断言其相切结果；或
- 在 `feature_seq` / `segment` 的 `mod tests` 内读同一 fixture（`include_str!("../../../../tests/fixtures/theta_v0_parity.json")`）替换手填期望值，使次缝也接上机器耦合链。

### MED-1｜三笔形态在 Lean 侧无见证，「同构」是断言而非已证命题

导出器注释（`ParityFixtureExport.lean:174-176`）与报告 §9 以「Lean 无三笔重合谓词；三笔 `max(lows)==min(highs)` 与两区间 `a.high==b.low` **同构**」为由不导出三笔用例。该同构在数学上确实成立（三区间交非空 ⟺ `max(lows) ≤ min(highs)`，Helly 一维），但：Lean 侧无三笔谓词、无该同构的形式化证明，因此 `three_stroke_overlap` 的相切行为**没有任何机器见证**，只有一个手填单测。属主缝的第二个缺角。

实效风险低（两品种真实数据三笔相切命中均 0 次，报告 §3 已实测），故定 MED 而非 HIGH。建议：若 Lean 侧补 `OverlapsTriple` 及其与 `Overlaps` 的关系定理，可一并闭合；否则在 SPEC 层显式登记该缺角为已知边界。

### LOW-1｜`decide (¬ HasGap a b)` 绕法：结论正确，见证间接，注释省略一步依据

按 commit 当时形态核实（`git show 491f638a8c:formal/Origin/SegmentFeatureSeq.lean`）：`HasGap` 有 `Decidable` 实例（:105），`Overlaps` **无**。`Decidable (¬ P)` 由 `Decidable P` 自动导出，故 `decide (¬ HasGap a b)` 可求值——技术路径成立，非伪造。

真值等价成立，但需一步注释未写的依据：`gap_iff_not_overlap` 给的是 `HasGap ↔ ¬Overlaps`；要得 `Overlaps ↔ ¬HasGap` 还需 `¬¬Overlaps → Overlaps`。该步依赖 `Overlaps`（`Int` 上 `≤` 的合取）的可判定性 / 经典逻辑，成立但注释里的「严格互推，非绕道近似」把它略过了。

更实质的一点：该绕法只求值 `HasGap`，**不触及 `Overlaps` 的定义体**。若 `Overlaps` 定义漂移（例如改成严格 `<`），fixture 值不变、parity 不红——只有 `lake build`（`gap_iff_not_overlap` 证明失败）能捕获，而 `scripts/check_fixture_drift.py` 会跑 lake build，故有兜底。

**现态实证（本 session 亲跑）**：#296 已直化为 `decide (Overlaps a b)`，重跑导出器与盘上 fixture **逐字节一致** ⟹ 当时绕法给出的 5 组真值与直求值完全相同，无污染。本项已由 #296 消解，登记为历史项，不需回票。

### LOW-2｜探针使两个生产谓词带副作用

`three_stroke_overlap`（`segment.rs:135-140`）与 `is_fractal_and_gap`（`feature_seq.rs:125-131`、`:138-144`）在返回前 `thread_local` `RefCell::borrow_mut()` 计数，谓词不再是纯函数——与 `.claude/rules/common/coding-style.md`「无隐藏副作用 / immutability」有张力。报告 §7 已给留存理由（`ancok_probe` 同模式、复议签字需可重跑）与可逆性说明，属**已声明取舍**，不构成违规。

两点建议（非阻塞）：
1. `thread_local` 计数**不跨线程聚合**。当前 `parse_layer` 单线程，计数完整；若日后线段划分并行化，量化会静默漏计。建议在 `GapTangentProbe` doc 上补一行注明该有效域。
2. 若编排者不再需要长期复测，随裁定书两签字位落定时摘除（改动面 = 两文件探针段 + 1 个 bin，可逆）。

---

## 4. 结果包六要素

1. **结论**：两轴 14+9 项中 **22 项 PASS，1 项 FAIL（S7 主缝覆盖）**。口径本身（相切=重合⟹无缺口）在两处谓词上落码正确、同批、与裁定书 §4 表逐字一致；Lean 证明项零改动；fixture 机器导出链成立且独立复现；量化四数字全部复现。唯一实质缺口是**保障机制**而非**口径正确性**。
2. **定义依据**：裁定书 §1「相切算有重合区间」+ §4 落码表（两谓词目标形态）+ §6 实装约束（同批 + 影响量化）；#249 SPEC「Testing Decisions / 主缝」（期望值来自 Lean 机器导出，次缝不替代主缝）；Lean `SegmentFeatureSeq.lean:102`（`HasGap` 严格 `<`）、`:111`（`Overlaps` `≤`）、`:118`（`gap_iff_not_overlap`）。
3. **边界条件**（结论翻转条件）：
   - HIGH-1 若被判定为「SPEC 主缝的合规实现方式之一」（即认为断言同构原语 `Interval::gap/overlaps` 已足以锁口径），则 S7 转 PASS，本评审全绿。此判定权属编排者——但需同时接受「改回旧口径 parity 不红」这一事实。
   - 若取得第 67/71/77/78 课原图并发现端点相切反例（裁定书 §3.1 已登记的重议触发条件），整个口径与本 commit 的正确性判定一并翻转。
   - 量化结论「段划分零变化」的有效域 = OKLO/BTC 各前 2000 笔两窗口（L2 单数据域观察），换窗口/换品种可能翻转；报告 §5 已按收窄裁定标明。
4. **下游推论**：口径已统一 ⟹ `gap_iff_not_overlap` 在 theta_v0 实装侧成立；本窗口段划分与下游证书/买卖点零变化 ⟹ 建立在旧口径上的本窗口历史结论**不因本次改动过期**（报告 §6.1 已逐条登记）。HIGH-1 若不补，本次统一的口径在 CI 层缺乏防漂移保护，未来任何回改都是静默的。
5. **谱系引用**：`.claude/rules/no-patch-mentality.md`（090 严格性 / 声明膨胀）——报告层诚实合格，commit message 层的表述边缘已在 T5 登记；`.claude/rules/formalization-validity-domain.md`（认识论等级）——parity §7 的 L0/L1 标注与量化报告的 L2 窗口限定均合规；#84 点3 → #246 supersede 谱系已在两处注释与裁定书 §2 完整登记。
6. **影响声明**：本评审为只读，未改动任何被评审文件。写入仅本报告 `chanlun/review-results/shadow-review-248-20260726.md`。复跑产生的临时文件（`/tmp/fixture_regen_276.json`、`/tmp/shadow276_{oklo,btc}.jsonl`）在仓库外。

---

## 5. 处置建议

| 项 | 级别 | 建议 |
|---|---|---|
| HIGH-1 主缝零覆盖两谓词 | HIGH | **reopen #248**（或另立子票挂 map #59）补主缝；口径本身无需回退 |
| MED-1 三笔形态无 Lean 见证 | MED | 与 HIGH-1 同批处理，或在 SPEC 层显式登记为已知缺角 |
| LOW-1 `decide (¬ HasGap)` 绕法 | LOW | **已由 #296 消解**，仅作历史登记，无需动作 |
| LOW-2 探针副作用 | LOW | 补一行 `thread_local` 有效域注释；摘除时机由编排者定 |

裁定书两签字位（「实装后复核」「编排者复议」）本评审不代签——本报告可作为「实装后复核」的证据材料。
