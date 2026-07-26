# #323 实施报告：中枢单点核心成立谓词从严——影子评审披露返工

- 日期：2026-07-26
- 票：#323（parent #321 裁定、#59）
- 原实施 commit：`07fe27e00e`「fix(center): #323 中枢单点核心成立谓词从严（zd>zg → zd>=zg）」——**已落地**，
  谓词改动本身（`center.rs` 两处 `if zd > zg` → `if zd >= zg`）**独立影子评审判定正确、保留不动**。
- 本次性质：**披露返工**。独立影子评审判该实施**不过**，两条 Critical **都是披露不足**（非逻辑错误、非谓词
  错误）。本次只补披露文字，**不改任何谓词逻辑、`if` 条件、测试断言、测试函数名**。
- 上游文书：GitHub issue #321（裁定）、#323（本票）、#327/#328（本次返工浮出、编排者已立、OPEN）；
  体例参考 `.chanlun/review-results/center-tangency-impl-20260726.md`（#314 实施报告）；爆炸机制同类参照
  `.chanlun/review-results/center-tangency-blast-radius-20260726.md`。
- 纪律：只改本票范围内 5 个源码/文档文件 + 新建本报告；既有脏文件（`.claude/rules/*`、`CLAUDE.md`、
  `AGENTS.md`、`rust/src/bin/p107_level_calib.rs`、`tmp/`、`.agents/` 等）未动未 add；未 amend 已有 commit、
  未发任何 GitHub 评论。

---

## 0. 结论摘要

| 项 | 结果 |
|---|---|
| 原实施 commit | `07fe27e00e`（已落地）——谓词改动本身独立影子评审判定**正确、保留** |
| 本次改动性质 | 纯披露返工（doc comment + markdown 文档 + 新建报告），**不改任何逻辑代码** |
| 谓词改动（沿用，非本次改） | `center.rs` `center_from_segments`/`center_from_window` 两处 `if zd > zg` → `if zd >= zg`（#321 裁定，单点 `zd==zg` 不成立）|
| **Critical-1 披露**：真实爆炸半径 | 级联重排 **L0 26 / L1 18 / L2 8 / L3 3 / L4 2**（与实施方自旗一致）——偏离验收条件「变更仅限 ZD==ZG 两类点位、其余轨迹逐位一致」的暗示；机制 = 单点中枢剔除 → 扫描游标平移 → 中枢序列级联重排，与 #314 爆炸半径 §3.3 同类 |
| **Critical-2 披露**：四把锁措辞 | 锁窗（BTC OOS 前 16000 bar）内 `zd==zg` 命中 **0 次**——「零翻动」是**零覆盖的结果**，**不构成安全证据**（原 commit message「读数与既有基线完全一致，零翻动」的措辞未标注覆盖为零，本报告订正） |
| 新立票（编排者已立，OPEN，本票不做） | **#327**（真覆盖见证锁：落在中枢变动块的 ZD==ZG 判据命中断言）、**#328**（`third_spans_core`（637）等价式严格版重述：补「每段 `lo<hi`」前提的新论证）|
| 慢锁 | 未跑（沿用原实施的未跑事实），本票同样未跑，清单见 §6 |
| `cargo check --lib` | 通过（0 error，仅预存在 warning，见 §5）|
| `cargo test --lib` center 子集（24 条） | 全部 `ok`（见 §5）|

---

## 1. 改动文件清单（本次返工）

```
rust/src/theta_v0/classifier/center.rs           doc comment 订正（5 处，任务2+任务4）
rust/src/theta_v0/classifier/mod.rs              detect_centers_complete/geometric doc 补⚠边界声明作废（2 处，任务3）
rust/src/theta_v0/types.rs                        措辞订正（1 处，任务4）
docs/reference-theta-v0.md                        口径同步 + 措辞订正（2 处，任务3）
docs/canonical-coverage-rust-impl.md              措辞订正（2 处，任务4）
.chanlun/review-results/center-strict-single-point-impl-20260726.md   新建（本报告，任务1）
```

**无任何 `.rs` 文件外的逻辑改动、无测试断言改动、无测试函数改名。** 验证见 §5。

---

## 2. (a) 真实爆炸半径披露（评审 Critical-1）

### 2.1 数字

从严后中枢级联重排计数（与实施方自旗一致，本次转录为正式披露，未重新独立实测）：

| 级别 | 重排数 |
|---|---|
| L0 | 26 |
| L1 | 18 |
| L2 | 8 |
| L3 | 3 |
| L4 | 2 |

### 2.2 与验收条件的偏离

#323 票面验收条件写的是：

> 靶向验证（不重放）：OKLO 0 次/BZ2024 2 次单点案例的行为对照（**变更仅限这两类点位，其余轨迹逐位一致**）

原 commit message 的表述「新旧口径产出的中枢序列 LCS 显示差异集合高度局限（OKLO 差 4/561、BTC 差
26/9260），**首个分歧点之前逐位 bit-exact**」容易被读成「变更是局部替换、只动了 ZD==ZG 那几个点位」。

**这偏离了验收条件的暗示。** 「首个分歧点之前逐位 bit-exact」只说明**分歧点之前**一致，**不**说明
**分歧点之后**是局部替换——上表 L0 26/L1 18/L2 8/L3 3/L4 2 是**级联重排**，不是仅仅替换 ZD==ZG 那几个
点位本身。本报告显式订正这个暗示：实际影响面是「26/9260」这类差异计数所暗示的局部占比，**而非**变更
影响的结构范围——一次单点中枢被剔除会改变其后所有中枢的扫描起点，从而重排后续整条序列。

### 2.3 机制（与 #314 爆炸半径同类）

**单点中枢被剔除 → 中枢扫描游标平移 → 其后中枢序列整体重排。**

`recursive_tower::detect_centers_windowed_resume`（`center_from_segments`/`center_from_window` 的单一
扫描来源，见 `mod.rs` `detect_centers_with`）用 seed + 延伸吸收的窗口扫描：seed 成立后游标停在该中枢
终点继续扫描，seed 不成立则游标只前进一段。#321 从严后，原本在某个三段组合上成立的单点核心
（`zd==zg`）现在判不成立 ⟹ 游标前进的量变了（只挪一段，而非跳到原单点中枢的终点）⟹ 该处**及其后**
所有中枢候选的三段窗口起点全部平移 ⟹ 后续整条中枢序列**级联重排**，直到某处再次「碰巧」重新对齐。

这与 `.chanlun/review-results/center-tangency-blast-radius-20260726.md` §3.3「翻转点 (a)：v0 相切段从
『离开』变『延伸』」描述的机制**同类**——都是「单点/边界判据翻转 → 扫描游标推进量变 → 后续候选序列整体
平移」，区别只在于 #314 是相切延伸判据、本票是核心非空判据。

### 2.4 有效域声明（照实）

本节数字（L0 26/L1 18/L2 8/L3 3/L4 2）转录自实施方自旗读数，**本次返工未独立重跑靶向验证复现**——
不在「照实」原则的可信度上高于原实施方声明的水平。真正落在**变动块**（而非全局差异计数）上的见证锁
留给 **#327**（见 §3）。

---

## 3. (b) 四把锁措辞诚实化（评审 Critical-2）

### 3.1 四把锁

```
btc_type2_open_short_channel_witness              （#200 BTC 见证，#[ignore] 需 BTC 数据）
btc_prune_leg_exit_type_matches_account_identity   （G4 typed ledger BTC 见证，#[ignore] 需 BTC 数据）
btc_type2_residual_correction_witness              （#199 BTC 见证，#[ignore] 需 BTC 数据）
typed_ledger_btc_smoke                             （G4 typed ledger BTC 冒烟，#[ignore] 需 BTC 数据）
```

四把均是 `src/theta_v0/backtest/runner.rs`（前三把）/ `l3_delta_r_alpha.rs`（第四把）里 `#[ignore]` 标注
的 BTC 数据见证锁，默认 `cargo test --lib` 不跑，需 `--ignored` 显式跑。

### 3.2 覆盖事实

锁窗（**BTC OOS 前 16000 bar**）内，从严判据 `zd==zg`（本票改动的判据分支）**命中 0 次**。

### 3.3 措辞订正

原 commit message：「四把锁（…）逐把复跑，读数与既有基线完全一致，零翻动。」

**订正**：这句话本身没有说假话（确实零翻动），但**未标注覆盖为零**，容易被读成「四把锁验证了改动安全」。
本报告显式订正为：

> **零翻动是零覆盖的结果，不构成安全证据。** 四把锁在其数据窗口内从未触发本票改动的判据分支（`zd==zg`
> 命中 0 次），因此它们「零翻动」这一事实**不能**用来支持「本票改动不影响生产链」的结论——它们根本没
> 测到这条改动路径。四把锁继续保持绿色是**必要但不充分**的信号；充分的信号需要一把**真正命中变动块**
> 的见证锁。

### 3.4 真覆盖见证锁

需要落在**变动块**（如 2022-02 段——即 §2.1 级联重排计数所在窗口）的见证锁，断言该判据命中次数 >0
且新口径轨迹符合预期。该锁**另立票 #327**（"真覆盖见证锁：落在中枢变动块的 ZD==ZG 判据命中断言（#323
评审浮出，锁零覆盖补课）"，编排者已立，OPEN，本票不做）。

---

## 4. (c) 慢锁未跑标注

### 4.1 本票范围内（center.rs / center_lifecycle.rs / mod.rs）

`cargo test --lib` 默认跑的非 `#[ignore]` 用例本次全绿（§5 已验证 24/24 `ok`，含 #321/#323 新增/翻转的
两条判据用例）。**本票没有新增 `#[ignore]` 用例**，因此本票范围内没有"本票自身引入但未跑"的慢锁。

### 4.2 四把 BTC 数据见证锁（已跑，见 §3）

不计入"未跑"——原实施已用 `--ignored` 显式复跑过，读数见 commit message，本报告 §3 已订正其措辞。

### 4.3 crate 级 `#[ignore]` 全量清单（未跑，事实在案）

原实施与本次返工均**未跑**这份清单外的其余 `#[ignore]` 用例。`grep -rn "#\[ignore" --include=*.rs
src tests` 实测（2026-07-26，本次返工时点）：**196 条**，分布：

```
src/theta_v0/backtest/runner.rs             31
src/theta_v0/backtest/wverify_run.rs        24
src/theta_v0/backtest/incremental.rs        24
src/theta_v0/backtest/econ_positive.rs      17
src/recursive_t/rec_stream.rs               16
src/theta_v0/backtest/l3_delta_r_alpha.rs   15
src/theta_v0/backtest/l3_pi_probe.rs        12
src/theta_v0/parser/profile.rs               7
src/recursive_t/t_engine_run.rs              6
src/theta_v0/classifier/mod.rs               5
src/theta_v0/backtest/l3_fullwindow.rs       4
src/recursive_t/backtest_run.rs              4
src/c_segment_verify.rs                      4
tests/econ_oddeven_diagnosis.rs              3
src/theta_v0/strategy/interp.rs              3
src/theta_v0/classifier/signal.rs            3
tests/theta_v0_07b_gating.rs                 2
src/trading/trade_behavior.rs                2
src/theta_v0/backtest/l3_pi_falsify.rs       2
src/theta_v0/backtest/highlow_mu.rs          2
tests/theta_v0_perf_profile.rs               1
src/zhongshu.rs                              1
src/trading/sublevel_confirmation_ablation.rs 1
src/trading/consolidation_ablation.rs        1
src/theta_v0/strategy/coverage.rs            1
src/theta_v0/parser/mod.rs                   1
src/theta_v0/backtest/segment_gn.rs          1
src/theta_v0/backtest/pooling_icc.rs         1
src/theta_v0/backtest/capture_oos.rs         1
src/segment_layers.rs                        1
```

绝大多数标注为「需 BTC/OKLO/CL 数据缓存」「O(n²) --release 专用」（DATA BLOCKER 纪律不伪造）。**未逐条
做与本票改动的相关性核验**——这是一份完整清单（grep 实测，非编造），但不是一份风险分级表。`src/theta_v0/
classifier/mod.rs` 的 5 条 `#[ignore]` 与本票改动同模块，相关性最高，**留意但本票未跑**；其余大多是回测/
经济假设检验链，读码判断相关性低但**未实测确认**。

**清单待补的部分**：哪些具体测试名称会触及本票改动的判据分支（而非泛泛的模块归属），未逐条核验，事实
在案，不编造跑过的读数。

---

## 5. 验证

### 5.1 `cargo check --lib`

```
cd rust && cargo check --lib
```

**通过，0 error**。仅有预存在的 dead-code 相关 warning（44 条，`RevOpenLog`/`RevCloseLog` 等字段未读，
与本次 doc-only 改动无关，改动前后同样存在）。`rust/src/bin/p107_level_calib.rs` 是**预存在脏文件**，
`cargo check --lib` 不含 `bin/`，未触及，与本次返工无关。

### 5.2 `cargo test --lib` center 子集

```
cd rust && cargo test --lib theta_v0::classifier::center
```

**24 passed; 0 failed; 0 ignored**（含 `complete_center_rejects_boundary_zd_eq_zg`、
`geometric_window_rejects_boundary_zd_eq_zg`、`born_sliding_window_after_empty_core` 等 #321/#323 判据
用例，逐条 `ok`）。本次返工只改 doc comment，**未重跑全仓 `cargo test --lib`**（原实施 commit message
已声明 1901→1902，本次未复验全量计数，仅复验直接相关子集）。

---

## 6. 声明订正与措辞订正记录（任务2/3/4 逐条）

| # | 文件:行（改后） | 旧 | 新 | 理由 |
|---|---|---|---|---|
| 1 | `center.rs:30`（模块头 637号等价式） | `[方向交替 ∧ 全三段核心非空 \`ZD_B≤ZG_B\`]` | `[方向交替 ∧ 全三段核心非空 \`ZD_B<ZG_B\`]`（严格，#321 裁定） | 该段描述"当前实装成立口径"（B 口径的定义式），须与从严后实装一致；改后在段尾新增⚠注：本恒等式的证明基于旧弱口径（靠 `d₃≤g₃` 段不变量永真补齐），严格版重述需补「每段 `lo<hi`」前提的新论证，已另立票 #328 |
| 2 | `center.rs:141/144`（`third_spans_core` doc） | `ZD_B≤ZG_B`（核心非空）⟺…；`third_spans_core(a,b,c) ⟺ ZD_B≤ZG_B` | **保留 `≤` 不改**，新增⚠注（:146-150） | 这两处是**引用 637号原等价式（弱版）**本身的数学表述（codex 异质核验成果），不是"当前实装成立口径"的直接描述；未随 #321 重做严格版证明前不得静默改成 `<`（那会断言一个未证明的命题），故保留 `≤` 并加注「该式为弱口径版本，严格版见 #328」 |
| 3 | `center.rs:163`（`center_from_segments` 支2 doc） | `compute_zd(a,b,c) ≤ compute_zg(a,b,c)` | `compute_zd(a,b,c) < compute_zg(a,b,c)`（严格，#321 裁定） | 这是 `center_from_segments`（支2）**当前实装**的直接描述，与实装 `if zd >= zg { return None }`（须 `zd<zg` 才成立）矛盾，必须改 `<` |
| 4 | `mod.rs`（`detect_centers_complete` doc，紧邻契约锚段） | 无边界声明 | 新增「⚠边界声明作废」段，体例照抄 `center.rs` `center_from_segments` doc | 契约锚 `Origin.CenterComplete.CenterConfirmedComplete` 在 Lean 侧核心非空支仍是 `ZD≤ZG`（弱），与本函数调用的 `center_from_segments`（严格）端点分支不一致，需登记 |
| 5 | `mod.rs`（`detect_centers_geometric` doc，紧邻契约锚段） | 无边界声明 | 新增「⚠边界声明作废」段，体例照抄 `center.rs` `center_from_window` doc | 契约锚 `Origin.centerHolds` 在 Lean 侧是 `ZD≤ZG`（弱），与本函数调用的 `center_from_window`（严格）端点分支不一致，需登记 |
| 6 | `docs/reference-theta-v0.md:23`（中枢边界行） | `ZD≤ZG` 成立即中枢成立；旧 ⚠作废注（"新票 #321，矛盾未裁"）| `ZD<ZG`（严格）成立，单点不成立；⚠作废注更新为"#321 已裁决从严，Python/Rust 均严格，**Lean centerHolds 仍弱**，该落点对齐声明作废、Lean 侧跟进留对方线" | 原作废注写于 #321 裁决**之前**，描述的是"矛盾未决"；#321 已于 2026-07-26 裁决从严，需更新为"已裁决"版本，且明确 Lean 侧现状（仍弱，未跟进）而非笼统"三方矛盾" |
| 7 | `docs/reference-theta-v0.md:33`（中枢确认行） | "前三完成次级别走势**重叠**即确认"（未定语） | "**严格重叠**即确认（单点相切**不**算重叠）"，标注 #321 裁定 | "重叠"含糊，未区分严格/含端点；按 #321 从严口径明确为严格重叠 |
| 8 | `rust/src/theta_v0/types.rs:113` | "单点中枢之问被缠师回避在案" | "单点中枢之问，缠师答的是级别谬误——只说明该例子只构成 1 分钟中枢的延续，未答单点 `[ZD,ZG]` 边界本身是否成立" | 任务4措辞如实化；原文核对见 §7 |
| 9 | `center.rs:176`（`center_from_segments` doc 依据段） | "单点中枢之问被缠师回避在案" | "单点中枢之问，缠师答的是级别谬误（只说明该例子只构成 1 分钟中枢的延续，未答单点边界本身是否成立）" | 同上 |
| 10 | `center.rs:383`（`complete_center_rejects_boundary_zd_eq_zg` 测试 doc，**只改注释**） | "之问被缠师回避" | "之问，缠师答的是级别谬误（未答单点之问，只说明该例子只构成 1 分钟中枢的延续）" | 同上；测试断言与函数名未动（硬约束1） |
| 11 | `docs/canonical-coverage-rust-impl.md:99` | "22 课 Q&A `022-第22课.md:514` 被缠师回避" | "22 课 Q&A `022-第22课.md:514` 缠师答的是级别谬误，未答单点之问" | 同上 |
| 12 | `docs/canonical-coverage-rust-impl.md:122` | "被缠师回避在案" | "缠师答的是级别谬误（未答单点之问）在案" | 同上 |

**#327/#328 立票登记**（不在上表，独立记录）：
- **#327**「真覆盖见证锁：落在中枢变动块的 ZD==ZG 判据命中断言（#323 评审浮出，锁零覆盖补课）」——编排者已立，OPEN，本票不做。
- **#328**「third_spans_core（637）等价式严格版重述：补「每段 lo<hi」前提的新论证（#323 评审浮出）」——编排者已立，OPEN，本票不做。

---

## 7. 原文核对（任务4依据）

**先 Read 原文** `docs/chanlun/text/blog/022-第22课.md:512-522`（Q&A 原文，一字未改）：

> [匿名] 之乎者 2007-01-12 16:39:58
>
> 缠MM：关于中枢还是有些不明白。
>
> 盘整中的中枢好明白，有点象N字形。但如果是次级别上涨+盘整+上涨是否就构成本级别上涨中枢？举个极端的
> 例子，比如1分钟级别上：连续3分钟构成的ｋ线分别是5-5.1元、5.1元、5.1-5.2元，那么是否意味着就构成5
> 分钟级别的中枢，该中枢[ZD,ZG]仅是5.1元这个点位？
>
> ===
>
> 不对，首先，没有什么上涨中枢的概念。上涨有两个中枢，你说什么是上涨中枢？当然，可以说是回调形成的
> 中枢，也可以说回升形成的中枢，但这都只是为了形象，不是精确的概念。
>
> 你说的例子，只形成一个1分钟中枢的延续。

**核对结论：属实。** 提问明确问的是"该中枢 [ZD,ZG] 仅是 5.1 元这个点位（单点）是否构成 5 分钟级别的
中枢"——这正是本票单点核心 `ZD==ZG` 的问题。缠师的回答分两层：(1) 否定"上涨中枢"这一命名概念不精确
（术语层面，与单点问题无关）；(2) "你说的例子，只形成一个1分钟中枢的延续"——这句话回答的是**级别**
问题（该结构仍停留在 1 分钟级别，不构成 5 分钟级别新中枢），**没有**直接回答"单点 `[ZD,ZG]` 本身是否
构成一个（任意级别的）合法中枢"这个边界口径问题。

故"缠师答的是级别谬误（未答单点之问）"这一描述**与原文相符**，本次全部据此改写，未发现原文与描述不
符的情况，不存在需要"停下来照实报告"的偏差。

---

## 8. 复算索引

```bash
cd /Users/silencehan/Projects/NewChanlun/rust

# doc-only 改动编译验证
cargo check --lib                                    # 0 error（预存在 warning 不变）

# center 判据用例（含 #321/#323 判据用例）
cargo test --lib theta_v0::classifier::center         # 24 passed / 0 failed / 0 ignored

# 四把 BTC 见证锁（需 --ignored + BTC 数据缓存，本次未重跑，沿用原实施读数）
# cargo test --lib --ignored btc_type2_open_short_channel_witness
# cargo test --lib --ignored btc_prune_leg_exit_type_matches_account_identity
# cargo test --lib --ignored btc_type2_residual_correction_witness
# cargo test --lib --ignored typed_ledger_btc_smoke

# crate 级 #[ignore] 清单（§4.3 数字来源）
grep -rn "#\[ignore" --include=*.rs src tests | wc -l   # 196
```

**代码锚（改后）**：`rust/src/theta_v0/classifier/center.rs:30,39-45,141-150,163,176,383`；
`rust/src/theta_v0/classifier/mod.rs:306-311,325-330`；`rust/src/theta_v0/types.rs:113`；
`docs/reference-theta-v0.md:23,33`；`docs/canonical-coverage-rust-impl.md:99,122`。

**上游文书**：GitHub issue #321（裁定，2026-07-26）、#323（本票）、#327（真覆盖见证锁，OPEN）、
#328（637号等价式严格版重述，OPEN）；`.chanlun/review-results/center-tangency-impl-20260726.md`
（#314 体例先例）；`.chanlun/review-results/center-tangency-blast-radius-20260726.md`（爆炸机制同类
参照）；原文 `docs/chanlun/text/blog/022-第22课.md:512-522`（一字未改，仅本报告 §7 引用核对）。
