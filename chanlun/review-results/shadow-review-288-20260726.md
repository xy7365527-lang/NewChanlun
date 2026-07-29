# 影子评审：#288 legacy+Python 相切=重合同批切口径（commit 77040317f2）

- **评审票**：#302；**实装票**：#288（裁定出处 #277 裁路①，全域口径来源 #246）
- **评审对象**：`77040317f2`（7 文件 +93/−7），worktree `/tmp/kimi-nest-mainline`
- **评审者**：独立影子评审（新上下文 Opus，禁自评资格成立；未参与实装）
- **Spec 轴依据**：#288 票体 + Resolution、#277 裁定评论（2026-07-26）、主仓报告
  `.chanlun/review-results/legacy-tangency-blast-radius-20260726.md`（只读）
- **Standards 轴依据**：090（严格性/声明膨胀禁止）、no-patch-mentality、
  formalization-validity-domain（认识论等级）、TDD

## 结论

**两轴主体 PASS，1 项 HIGH 需回补后再关票。** 谓词改法、Python 同批、8 处声明作废
逐处吻合票面；翻转清单经本评审**独立机器枚举 bit-exact 复现**；cargo/pytest 基线独立
复跑成立。HIGH 项不在"改错了"，在"改对了但新口径没有回归锁，且旧口径措辞在单测里存活"。

---

## 一、Spec 轴（逐项）

| # | 票面要求 | 判定 | 行号证据 |
|---|---|---|---|
| 1 | `segment.rs` `three_stroke_overlap`：`lo < hi` → `<=` | **PASS** | `rust/src/segment.rs:131` `lo <= hi` |
| 1 | `segment.rs` `is_fractal_and_gap`：两臂 `>=` → `>` | **PASS** | `rust/src/segment.rs:378` `b_l > a_h`、`:382` `a_l > b_h` |
| 2 | Python 参考三处（`:51` / `:262` / `:265`） | **PASS** | `src/newchan/a_segment_v1.py:59` `<=`、`:277` `b_l > a_h`、`:280` `a_l > b_h`（票面 `analysis/` 路径为笔误，实际 `src/newchan/`，Resolution 已照实登记） |
| 3 | 声明作废 8 处同批登记 | **PASS** | `segment.rs:7-14`（模块头）、`lib.rs:8-12` + `:155-159`、`orchestrator.rs:12-17` + `:776-780`、`recursive_t/ffi.rs:55-59`、`tests/test_rust_segment_equivalence.py:6-15`、`tests/test_rust_recursive_equivalence.py:8-14` —— 与票面枚举（`segment.rs:1-5`、`lib.rs:4-6`+`:146-147`、`orchestrator.rs:2,767`、`ffi.rs:52`、两 parity docstring）一一对应 |
| 4 | golden 重锚 + 翻转清单照实登记 | **PASS** | `test_segment_bitexact_real` 是 rust↔Python **parity** 用例（`tests/test_rust_segment_equivalence.py:233-238`）、非冻结 fixture ⟹ 两侧同切即回绿，无 fixture 需改写；翻转清单登记于 commit message 与 Resolution |

### Acceptance 复跑（本评审独立执行）

| 锚 | 声明 | 独立复跑 | 判定 |
|---|---|---|---|
| `cargo test --release --lib` | 1834/1，唯一失败 #110 在案 | **1834 passed / 1 failed**，失败=`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（`signal.rs:3382`） | **PASS**——该失败属 #110 既线，`chanlun/review-results/spec-owner-attribution-fix-20260724.md:11` 记「勿修勿归因」 |
| 两 parity 文件（含 slow） | 10 passed | **10 passed, 1 deselected, 45.35s**（含 `test_segment_bitexact_real[strict]/[optimized]` BZ 全年真跑） | **PASS** |
| 翻转清单（机器枚举） | BZ 全年 strict 28 / optimized 159，全 `second→none`，非 gap_type 分歧 0 | 独立旧/新口径换码枚举：512563 bar → **43123 笔 → 4542 段**；`strict 28` / `optimized 159`，方向全 `('second','none')`，**非 gap_type 分歧 0/4542** | **PASS（逐位吻合）** |
| 段端点零变化 | 实测成立 | 同上（端点六元 + s0/s1/i0/i1/dir/high/low/confirmed/kind/trigger_k/fractal_abc 全等） | **PASS** |
| 段落伞（非 slow） | 不退化 | `test_segment_v1_settlement / gap_classification / weird_cases / settlement_anchor_events / identity_skip` **76 passed** | **PASS** |
| 工作面零残留 | 零残留 | `git status --porcelain` 对 7 文件输出为空 | **PASS** |

> 环境注记：主仓 venv 是 uv cpython-3.12.13，而 `cargo build` 产物链 homebrew
> `python@3.12` framework ⟹ 直接 symlink `.so` 触发 `Fatal Python error: PyInterpreterState_Get`。
> 本评审改用 worktree `maturin build --release` 产物 wheel 解包到 `/tmp/pyreview`（sys.path shadow，
> **未触碰主仓共享 venv**），`PYTHONPATH=/tmp/pyreview:<worktree>/src` 锁定 worktree 双侧源。

---

## 二、Standards 轴（逐项）

| 项 | 判定 | 证据 |
|---|---|---|
| 090 声明不膨胀（新口径侧） | **PASS** | 8 处均带「**实测**段端点零变化」限定，未把 L2 实测升格为全域定理；`segment.rs:11-13` 明确「不再 bit-exact 对齐 2026-07-26 前旧口径的历史输出/基线」——废的是旧基线、不是等价声明本身，措辞照实不过度 |
| 090 声明不膨胀（旧口径侧） | **FAIL → HIGH-1** | `tests/test_segment_v1_settlement.py:330` 仍声明「边界相等 → 无重叠（**严格 <**）」 |
| Lean 对齐声明属实 | **PASS** | `formal/Origin/SegmentFeatureSeq.lean:102` `HasGap := a.high < b.low ∨ b.high < a.low`（严格 `<`）、`:111` `Overlaps := a.low ≤ b.high ∧ b.low ≤ a.high`（闭区间）、`:118` `gap_iff_not_overlap`。三笔「公共交集 `max_lo ≤ min_hi`」与 Lean 二元 `Overlaps` 的桥接靠 1D Helly（区间两两相交 ⟺ 公共交集非空）——`segment.rs:122-125` 已把这层桥接写明，非跳步 |
| 翻转清单机器枚举（禁手编） | **PASS** | 已独立复现（见上表）；与主仓报告 §4 line 14 及 main 线 `61c958d204` 三方一致 |
| 未全量复跑 recursive 大 slow 的替代证据链 | **PASS（有界且已挂票）** | 替代链：BZ 全年 wide 模式最终段列表零翻转（报告 line 95：146 次相切、0 翻转）+ confirmed 段 `break_evidence` 发射后不可变 + SegCheckpoint resume≡full 测试 + OKLO 447K 差分背书；报告 line 153 自认「未逐 bar 重放实测」；**#307 已把该大 slow 挂进重型窗口**（#307 评论：「顺带补跑…若窗口成本实测失控，照实上报不硬跑（090）」）⟹ 非静默截断 |
| no-patch-mentality / no-workaround | **PASS** | 无兼容分支、无 feature flag、无旧口径 fallback；两侧原子同切 |
| 认识论等级标注 | **PASS** | 两 parity 文件 docstring 保留 L1/L2 标注，本次变更未动其等级声明 |
| 残余口径副本清零 | **FAIL → MED-1 / LOW-1** | 见下 |

---

## 三、问题分级

### HIGH-1｜新口径无相切回归锁，且旧口径规则在单测里存活

- `rust/src/segment.rs` 改后**零新增单测**——全模块无 `tangent` 用例（grep `fn .*tangent` 于
  `rust/src/segment.rs` 无命中）。对照 #248 同款模板确有：`theta_v0/parser/segment.rs:800`
  `three_stroke_overlap_tangent_counts`、`parser/feature_seq.rs:491/:500` `*_tangent_no_gap`。
- Python 侧 `tests/test_segment_v1_settlement.py:329-336` `test_overlap_boundary_equal`：
  用例名声称覆盖边界相等，但数据 `max_lo=15 > min_hi=10` 是**严格分离**、不是相切；`:330`
  docstring 与 `:335` 内联注释把判据表述为旧口径「严格 `<`」。改后断言仍绿 ⟹ 该用例既没覆盖
  真相切、又留下一条与新口径相反的规则声明（090）。
- `tests/test_segment_weird_cases.py:6` C 组标题「相切 vs 重叠」同为注释残留（主仓报告 line 77
  已认「实际构造全为严格不等，无精确相切用例」）。
- **失败场景**：`.cache/BZ_1min_2024_raw.parquet` 是 gitignored 软链，CI/新克隆环境缺失 ⟹
  `tests/test_rust_segment_equivalence.py:210-211` `pytest.skip` ⟹ 两个 `test_segment_bitexact_real`
  静默跳过。此时若有人把 `segment.rs:131` 改回 `lo < hi`（或照 `test_segment_v1_settlement.py:331`
  的注释"修正"回旧口径），**全套非 slow（cargo 1834 + 段落伞 76 + parity 8）仍全绿**，
  #246 要消灭的路径依赖分歧无声回到生产 PyO3 路径。
- **对照**：#277 决议记录 main 线同裁 commit `61c958d204` 的验收锚含「相切测试先红后绿
  （**rust 3 + Python 3，构造真相切**）」。本 commit 同票同裁但无此锚 ⟹ 两线不等价。
- **建议**：回补 rust 3 + Python 3 真相切单测（照 #248 模板），并订正
  `test_segment_v1_settlement.py:329-331` 措辞（或改数据使其真为相切）。**回票 reopen 或另下补锁票。**

### MED-1｜第三份旧口径谓词副本存活于 analysis harness，且其自身声明已不成立

- `analysis/_attrib_segment_divergence.py:146-150` `three_stroke_overlap` 仍 `max_lo < min_hi`（`:150`，旧口径），
  而**同一文件** `:142-144` `iv_gap` 已是新口径（`overlaps = e1[0] <= e2[1] and e2[0] <= e1[1]`）
  ⟹ 文件内部两谓词现在不同口径（`gap_iff_not_overlap` 在该文件内不成立）。
- 该文件 `:15` 声明「参考语义 = `a_segment_v1.segments_from_strokes_v1`（编排者裁定唯一口径）」——
  Python 参考本 commit 已切，此声明**现已不成立**（090）。
- 该脚本是 OKLO 冻结 fixture 的消费方之一（主仓报告 line 97 列名）。
- **无票承接**：#290 只管中枢侧三处（golden / `a_center_v0.py:126` / `a_level_fsm_newchan.py:91`）；
  `gh issue list --search "attrib_segment" --state all` 零命中。
- **失败场景**：日后复用该 harness 做段归因，其复刻侧仍按旧口径判重叠，结论与生产口径不可比，
  却因文件头声明"= 唯一口径"而被当作参考基线引用。
- **建议**：下票（切新口径 or 标 deprecated/冻结并订正 `:15` 声明）。不阻塞本 commit。

### LOW-1｜`test_segment_gap_classification.py` 注释以旧口径表述缺口判据

`tests/test_segment_gap_classification.py:163`（`跳空 gap: 18 >= 12（b_l >= a_h）`）、
`:177`、`:226`、`:382` 用 `>=` 表述判据。用例数据为严格跳空故仍绿（76 passed 已验），但
判据措辞是被 supersede 的旧口径，未进 8 处清单。建议随 HIGH-1 一并订正。

### LOW-2｜`ffi.rs:52` 处置与 main 线相反（登记备查，非缺陷）

#277 决议对 main 线的处置是「`recursive_t/ffi.rs:52` **豁免不作废**（其 bit-exact 声明属 FFI
自身 API 兼容性）」；本 commit 按 #288 票面第 3 项在此加了注（`ffi.rs:55-60`）。加注内容经核**属实**：
`grep -rn "gap_type" rust/src/recursive_t/` 仅命中 `:58` 该注释自身，接口确实不读 `gap_type`。
两线口径差登记备查，不构成缺陷。

---

## 四、本评审未覆盖 / 未独立复现的项（照实登记，090）

1. `test_bitexact_large_real_streaming`（512K bar 逐 bar 流式）：本评审已按 wide 模式启动独立
   复跑，**报告落盘时仍在跑、结果未取**——不作为本次判定依据。该锚由 #307 承接（见 Standards
   轴对应行），本评审对它的判定基于「有界 + 已挂票」而非「已复跑通过」。
2. Resolution 中「BZ wide 前 30000 bar 逐 bar 六层指纹零分歧」与「OKLO 冻结 fixture 236/236
   端点全等」两项动态数字：**本评审未独立复跑**，引用自实装 session 交付 + 主仓报告
   line 95/97，带此标注。
3. `cargo test --release`（非 `--lib` 的集成测试面）与全仓 pytest 伞：未跑；本评审只跑
   `--lib` + 段落/parity 相关 21 文件面。

---

## 五、结果包六要素

1. **结论**：Spec 轴 4/4 项 + 6 项 Acceptance 全 PASS（含独立复跑与独立机器枚举）；Standards 轴
   7 项 PASS、1 项 FAIL。问题：HIGH×1、MED×1、LOW×2。建议 #288 因 HIGH-1 回票补锁，
   MED-1 另下票。
2. **定义依据**：#246 裁定「相切=重合」全域生效；#277 裁路①（legacy + Python 参考同批切）；
   Lean `formal/Origin/SegmentFeatureSeq.lean:102/:111/:118`（`HasGap` 严格 `<`、`Overlaps` 闭区间、
   二者互斥穷尽）。实装两谓词的算子形态与上述定义逐字吻合。
3. **边界条件**（本评审结论何时翻转）：(a) 若存在**不依赖 BZ parquet** 的相切回归锁（本轮 grep
   未找到）→ HIGH-1 降为 LOW；(b) 若 `analysis/_attrib_segment_divergence.py` 已有退役/冻结
   标记或已被票承接 → MED-1 撤销；(c) 若 `test_segment_bitexact_real` 在 CI 实际不 skip
   （parquet 入库）→ HIGH-1 的失败场景需改写为"锁存在但数据依赖"，等级降 MED。
4. **下游推论**：新口径下 `break_evidence.gap_type` 的 `second→none` 翻转会同步改变
   `SegmentBreakPendingV1` / `SegmentSettleV1` 的 `gap_class` 字段
   （`src/newchan/core/recursion/segment_state.py:62,80`）。该字段落在 **DomainEvent 流**上，
   而 orchestrator.rs 与两 parity 测试的既有声明已把 events 明确划出等价范围
   （`orchestrator.rs:19-24`「下游引擎只读 `snapshot.moves` 不读 `.events`」）⟹ 与
   「下游中枢/走势/BSP 链不读 gap_type」不矛盾，但**事件消费方（若有）会看到标签变化**，
   属已声明的范围外面，登记备查。
5. **谱系引用**：#246（相切=重合全域裁定，supersede Lead #84 点3）、#248（同款改法先例，
   `491f638a8c`）、#277（路①裁定 + main 线 `61c958d204`）、#290（中枢侧残余）、
   #307（大 slow 重型窗口承接）；裁定书
   `chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`（存在已核）。
6. **影响声明**：本评审为**只读**产出，除本报告外未改任何文件（`git status --porcelain` 对
   评审对象 7 文件为空）。评审期间在 `/tmp/pyreview`（仓外）解包了 worktree 自建 wheel 作
   sys.path shadow，未触碰主仓共享 venv、未 `maturin develop`。`rust/target/` 因复跑
   `cargo test --release --lib` 与 `maturin build --release` 产生构建产物（gitignored）。
