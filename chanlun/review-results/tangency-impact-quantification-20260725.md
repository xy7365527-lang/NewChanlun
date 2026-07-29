# 相切=重合口径（#246 裁定）实装影响量化报告 —— 票 #248

- 日期：2026-07-25
- 裁定书：`chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`（编排者已签，supersede Lead #84 点3）
- 实装 worktree：`/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`，HEAD `b57da4bde4`）；改前对拍副本：`/tmp/rust-tangency-before`（APFS clonefile 副本，仅把两处谓词切回旧口径，探针保留）
- 纪律：全程无 git mutation；未触碰与本票无关的文件（含 backtest_bin 坏痕迹，见 §6.3）

## 1. 改动清单（两处谓词边界 + 注释 supersede 登记 + 测试 + 探针）

### 1.1 `rust/src/theta_v0/parser/feature_seq.rs`

| 位置 | 改前（旧口径 #84 点3） | 改后（#246 裁定） |
|---|---|---|
| `is_fractal_and_gap` 向上段缺口谓词 | `b_l >= a_h`（相切算缺口） | `b_l > a_h`（相切=重合 ⟹ 无缺口） |
| `is_fractal_and_gap` 向下段缺口谓词 | `a_l >= b_h` | `a_l > b_h` |
| doc 注释（:60-63 原「bit-exact 对齐 Python `_is_fractal_and_gap`」） | 对齐声明 | 作废声明改写：Python 参考在缺口谓词边界上不再是权威基线，指向裁定书与 Lean `gap_iff_not_overlap`；分型判定及其余 Python 对齐声明不受影响 |

### 1.2 `rust/src/theta_v0/parser/segment.rs`

| 位置 | 改前 | 改后 |
|---|---|---|
| `three_stroke_overlap` 重合谓词 | `max_lo < min_hi`（相切不算重合） | `max_lo <= min_hi`（相切算重合，对齐 `Origin.SegmentFeatureSeq.Overlaps` 的 `≤`） |
| 注释（:79-87 原记录 #84 点3 口径） | 「边界相切=零测度重合，从严不算」 | 改写为 supersede 登记：#84 点3 已被 #246 裁定替代并指向裁定书；#84 其余条款（方案一 + 五 seam 拆分）不受影响 |

Lean 侧 `HasGap`（严格 `<`）/ `Overlaps`（`≤`）未改——已符合裁定（裁定书 §4）。两处 rust 谓词**同批**改动（裁定书 §6：分批会在中间态制造 `gap_iff_not_overlap` 断裂）。

### 1.3 新增测试（4 个，全部通过）

| 测试 | 覆盖 |
|---|---|
| `feature_seq.rs::is_fractal_and_gap_up_tangent_no_gap` | 向上段 `b_l == a_h` 相切 → 无缺口（新口径） |
| `feature_seq.rs::is_fractal_and_gap_down_tangent_no_gap` | 向下段 `a_l == b_h` 相切 → 无缺口 |
| `segment.rs::three_stroke_overlap_tangent_counts` | `max_lo == min_hi` 相切 → 有重合 |
| `segment.rs::three_stroke_overlap_strict_cases_unchanged` | 严格内含/严格分离新旧口径一致（防过改） |

既有测试无一直接固化相切旧口径（既有用例的边界值均非相等情形），故无「固化旧口径」测试需改期望。

### 1.4 相切探针（留存，理由见 §7）

- `feature_seq.rs`：`GapTangentProbe`（thread_local RefCell）——`gap_evals` / `tangent_up`(`b_l==a_h`) / `tangent_down`(`a_l==b_h`)，埋点在 `is_fractal_and_gap` 本体内。
- `segment.rs`：`OverlapTangentProbe`——`calls` / `tangent`(`max_lo==min_hi`)，埋点在 `three_stroke_overlap` 本体内。
- 探针只计数不改返回值；计数语义与新旧口径无关（只记「边界相等」数据事实）⟹ 改前/改后可比。仿 `strategy/coverage.rs` `ancok_probe` 模式。
- `rust/src/bin/tangency_probe.rs`（新增）：OKLO 真实数据 → `parse_layer` → 前 2000 笔（#84 xcheck 同窗口）→ 打印探针计数 + 段端点 JSONL。

## 2. 测试基线 → 终态

| 项 | 基线（改前 HEAD） | 终态（改后） |
|---|---|---|
| `cargo test --release --lib` | 1817 通过 / 1 失败 / 132 ignored | **1821 通过 / 1 失败 / 132 ignored** |
| 失败测试 | `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（票 #110 在案） | 同一测试、**同一 digest 值**（left=16618955402698307653, right=10432481772907336594，两跑逐位相同） |

**失败判定登记**：唯一失败为票 #110 已知失败，改前改后 digest 逐位相同 ⟹ 与本口径变化无关，勿修勿归因（票面纪律）。**无任何测试因口径改变而失败**——无「固化旧口径 vs 实装错误」的判定个案需要登记。4 个新增边界单测全过。

## 3. 量化①：相切出现频次（OKLO 前 2000 笔，#84 xcheck 同窗口）

探针埋在生产谓词本体内 ⟹ 计数路径 = 生产调用路径（非离线复算，无等价性论证负担）。改前/改后计数相同（计数语义与口径无关）：

| 谓词 | 求值次数 | 相切命中 | 含义 |
|---|---|---|---|
| `is_fractal_and_gap` 向上段（`b_l == a_h`） | `gap_evals` = 292（分型成立的缺口求值总数） | **8** | 旧口径判 SecondKind（有缺口）→ 新口径判 FirstKind（无缺口） |
| `is_fractal_and_gap` 向下段（`a_l == b_h`） | （同上 292 内） | **13** | 同上 |
| `three_stroke_overlap`（`max_lo == min_hi`） | `calls` = 4（仅段起点扫描处调用：首起点 1 次命中 + 窗口尾扫 3 次未中） | **0** | 本窗口该谓词 0 次相切 ⟹ 其口径变化在本窗口无实效应 |

合计 OKLO 窗口：缺口谓词 **21 次**相切方向翻转（8 上 + 13 下）；三笔起点谓词 **0 次**。

**第二品种复测（BTC 前 2000 笔，同一 probe）**：`gap_evals`=307，`tangent_up`=**2**，`tangent_down`=**1**（合计 3 次）；`overlap_calls`=1，`overlap_tangent`=**0**。两品种一致结论：相切在真实数据低频但非零（OKLO 21/292 ≈ 7.2% 求值命中，BTC 3/307 ≈ 1.0%），裁定口径有真实生效面；三笔起点谓词相切在两品种均 0 次。

## 4. 量化②：线段划分变化量（段数 / 边界位移）

### 4.1 管线等价性（probe 加载 vs #84 xcheck 导出管线）

probe 自含加载逐字抄 `p123_fast_replay.rs:1543-1619`（不依赖 `backtest::data`，原因见 §6.3）。等价性：OKLO json 343282 bar OHLC 无 NaN/None（已核验）⟹ `load_symbol` 的 null 分支不触发；untradable 判据/quantize/`date_to_timestamp` 逐字相同 ⟹ 逐 bar 相同。**实证**：probe 改前段 == 主仓冻结 `_xcheck_oklo_strokes.json` 的 `rust_segments`（236/236 逐段一致）⟹ 管线等价成立。

### 4.2 四方对比（OKLO 前 2000 笔）

| 对比 | 结果 |
|---|---|
| probe 改前 vs 主仓冻结 `rust_segments` | 236 = 236，**逐段 identical** |
| probe 改前 vs Python 参考（`a_segment_v1`，旧口径）confirmed | 236 = 236，**逐段 identical**（= `segment_refsem_cert.py` 认证通过复现） |
| probe 改后 vs Python 参考 confirmed | 236 = 236，**逐段 identical** |
| probe 改后 vs probe 改前 | **逐字节 identical**（`cmp`） |

**结论：本窗口内口径变化对线段划分结果零影响**——段数 236 → 236（Δ=0），逐段端点零位移。21 次相切处分支从 SecondKind 翻为 FirstKind，但最终段端点全部收敛相同（相切处随后的第二特征序列确认与直接确认殊途同归）。三笔起点谓词 0 次相切，亦无贡献。

`analysis/segment_refsem_cert.py`（对参考语义认证 harness，主仓只读跑通）：认证结论**通过**（236/236），改前改后均通过——新口径下对旧口径参考的差分认证在本窗口无差。

**第二品种复测（BTC 前 2000 笔）**：改前/改后段端点 JSONL `cmp` **逐字节相同**——段数 245 → 245（Δ=0），零位移。两品种两窗口均为「相切命中非零、划分结果零变化」。

## 5. 量化③：下游对拍（收窄窗口：OKLO 前 2000 笔 + BTC 前 2000 笔）

★口径收窄裁定（2026-07-25，#248 评论「编排者裁定：§5 量化③口径收窄」）：票面原口径「p123 全量 replay，BTC 全窗」实跑成本过高（改前/改后双跑各 77+ 分钟 CPU 未完，单线程无并行度，dump 已落 7197 行对），编排者裁定**收窄为与 §3/§4 同窗口**：OKLO 前 2000 笔 + BTC 前 2000 笔。全窗 replay 若有需要另立票（依赖 backtest_bin 编译修复，§8 项 2）。原全窗半成品 dump（`/tmp/p123_dump_before.jsonl` / `/tmp/p123_dump_after.jsonl`）按裁定**作废**，本节未使用。

### 5.1 窗口桥（笔窗口 → bar 截断，机器导出）

p123 replay 以 bar 为输入，「前 2000 笔」须换算为 bar 前缀。tangency_probe 新增 `TANGENT_WINDOW` 输出行：`bar_cutoff = 第 2000 笔端点 bar + 1`（笔/段 `start_index`/`end_index` 即原始 bar 序号域，types.rs:89-105、canonical.rs:103-108 与 :206 自证断言）。笔构造只经包含/分型，不触本票两谓词（两谓词调用点均在线段划分内：feature_seq.rs:428 / segment.rs:151）⟹ 改前/改后窗口桥相同，对拍用同一截断。

| 品种 | 笔窗口 | bar_cutoff（=`P116_MAX_BARS`） | 全量 bars |
|---|---|---|---|
| OKLO | 前 2000 笔 | 31495 | 343282 |
| BTC | 前 2000 笔 | 32625 | 4613599 |

对拍setup：`p123_fast_replay <json>`，env `P116_MAX_BARS=<bar_cutoff>` + `P116_DUMP=<路径>`（`P116_CKPT`/`P123_SHADOW` 不设，与验收协议同）；改前 = `/tmp/rust-tangency-before`（旧口径，探针保留），改后 = 本 worktree。加载口径 = §4.1 已证等价的同一 `load_bars` 抄本；数据文件为主仓 data_cache 符号链接（只读）。收窄后单遍秒级完成（实测墙钟 <1s/遍）。改前/改后二进制唯一变量 = 两谓词口径（p123 bin 源未改，差全部来自 lib）。

### 5.2 证书计数（P123_CERT + dump CERT 行）——两窗口零变化

| 窗口 | 口径 | caliber_A | A_trend/pan/mixed | caliber_B | B_trend/pan/mixed | dump CERT 行 |
|---|---|---|---|---|---|---|
| OKLO | 改前 | 7 | 1/3/3 | 6 | 1/4/1 | 13 |
| OKLO | 改后 | 7 | 1/3/3 | 6 | 1/4/1 | 13 |
| BTC | 改前 | 13 | 0/13/0 | 9 | 0/9/0 | 22 |
| BTC | 改后 | 13 | 0/13/0 | 9 | 0/9/0 | 22 |

dump CERT 行（judge_at 首证钟向量为主键）两窗口逐字相同（OKLO 全部 13 行原位，BTC 22 行逐字节）。

### 5.3 买卖点/候选计数（P123_YIELD 等门行）——两窗口零变化

| 门行 | OKLO（改前 → 改后） | BTC（改前 → 改后） |
|---|---|---|
| P123_YIELD | candidates=99 trend=9 pan=90 divergence_confirmed=46 trend_div=5 pan_div=41 terminal_confirmed=4 → **全同** | candidates=101 trend=4 pan=97 divergence_confirmed=55 trend_div=2 pan_div=53 terminal_confirmed=7 → **全同** |
| P123_D3 | edges=2 violations=2 rate=1.0 → 同 | edges=3 violations=2 rate=0.667 → 同 |
| P123_BASELINE | old_candidates=2 old_terminal=0 old_certs=0 / new=99/4/6 → 同 | old=0/0/0 / new=101/7/9 → 同 |
| P123_SNAPSHOT / BIT_EXACT / R7 / MISSED | 逐字同（BIT_EXACT 全 0；MISSED=0；provider_complete=true） | 逐字同（MISSED=0） |
| P123_PROVIDER | snapshots=100 同；**views=312 → 310**（唯一门行差） | snapshots=102 views=338 → 同 |

### 5.4 关键事件行 diff（dump 全行比对）

| 窗口 | dump 行数（类型分布） | 逐字节 cmp | 事件集合（排序后 cmp） | 差异明细 |
|---|---|---|---|---|
| OKLO | 93 = 93（13 CERT / 46 DIV / 17 FALLBACK / 14 TERM / 3 TURN） | differ（1 处） | **完全相同** | 唯一差：`TERM level=1 bar=31188 side=Long` 从第 61 行前移至第 59 行（越过 `DIV level=1 end=30097 pan Short` 与 `DIV level=1 end=30339 pan Short` 两行）——同内容、同总数，仅落盘行序位移 2 位 |
| BTC | 116 = 116（22 CERT / 55 DIV / 31 FALLBACK / 8 TERM） | **逐字节相同** | 相同 | 无 |

stderr 稀疏物理统计（不进验收面，照实登记）：triggers 两窗口两口径均同（OKLO 602 / BTC 664）；OKLO reevals 310→308、reuses 80→81、syncs_self 178→174、syncs_lower 310→308；BTC reevals=335 等全部相同。`wm_cross_without_lower=0` 两窗口两口径（(iii)⊂(ii) 构造性断言持续成立）。

### 5.5 结论与归因

**最终下游产出——证书计数与首证钟、买卖点/候选/背驰确认/终端背书计数、D3 边、事件集合——两窗口改前改后零变化**，与 §4「段划分零变化」一致：段几何相同 ⟹ 下游事件集合相同。裁定口径在本两窗口的下游效应为「最终产出不变」，照实登记。

OKLO 窗口存在**评估路径层**微差（照实登记，不进验收面语义）：PROVIDER views -2、reevals/syncs -2/-4、一条 TERM 行落盘行序前移 2 位。归因（一致解释，非独立证明）：21 次相切处分支从 SecondKind 翻为 FirstKind（§3），增量塔中段确认的瞬态路径随之改变（第一特征序列直接确认 vs 等第二特征序列），塔内容快照在个别 trigger 处瞬态不同 ⟹ 稀疏 dirty 判定与 TERM 反查落盘时机微移；最终段端点收敛相同（§4.2 逐字节）⟹ 事件集合与全部计数不变。该现象与 p123_fast_replay 模块头已登记的「跨档残余：DIV 1 行序位移（dump 行无 key 身份/兜底时机差）」同类——dump 行序对物理评估时机敏感，不携带语义差异。OKLO 的 views 差同时实证了「谓词变化确已传入 p123 管线」（若下游不消费该谓词，两遍应逐字节相同）；BTC 窗 3 次相切命中未产生可观测路径差（连物理计数也全同）——相切命中是路径差的必要非充分条件。

## 6. 量化④：bit-exact 基线变更登记 + 未能项

### 6.1 作废/过期标注登记

| 基线 | 处置 |
|---|---|
| 「bit-exact 对齐 Python `_is_fractal_and_gap`」声明（feature_seq.rs） | **作废（仅缺口谓词边界）**，注释已改写指向裁定书；分型判定部分仍有效 |
| 「bit-exact 对齐 Python `_three_stroke_overlap`」声明 + Lead #84 点3 注释（segment.rs） | **supersede 登记**，注释已改写 |
| 主仓 `_xcheck_oklo_strokes.json` 的 `rust_segments` | 旧口径导出产物；**本次实证新口径重算逐段相同（236/236）**——语义未变，标签应注「旧口径导出；#246 后实测无差」（map #59 回填素材） |
| `segment_refsem_cert.py` 参考基线 | Python 参考含旧口径两谓词；裁定书 §5 后其在该边界上不再是权威基线。本窗口实测新口径 rust 与参考仍 236/236——认证 harness 结论在本数据域不受口径变化影响 |
| `extract_signals_bit_exact_digest_guard`（票 #110 在案失败） | 改前改后 digest 逐位相同 ⟹ signal 提取链输出不受本口径变化影响；该失败仍归 #110，与本票无关 |

不受影响（裁定书 §2/§5 明确）：`second_kind.rs:27,34` 的「Lead #84 点3」引用属**方向语义**条款（67 课博文），非 ≤/< 口径，不在 supersede 范围，未动；`_has_any_fractal`/`_apply_inclusion`/`_second_seq_has_fractal` 等其余 Python 对齐声明不受影响，未动。

### 6.2 历史结论口径复核

- 「406 段 vs 237 段」（#84 归因报告）与「237/237 认证通过」为**旧算法/旧口径时代**数字；当前主仓冻结基线已是 236/236（笔序列或管线在 #84 后有演进，归 #243 系列，不属本票）。本票窗口内改前改后均 236/236。

### 6.3 未能量化项（照实）

- **成交与净值（`theta_backtest` / `run_theta_v0_pi`）**：`backtest_bin` feature 在本 worktree **基线上编译即失败**——`backtest/fill.rs:328` 与 `backtest/runner.rs:72,79` 引用 `OPSEM_DUMP_DIR_OVERRIDE` 与 `pi_theta_fill_loop_voice`，而二者在 `fill.rs:366` / `opsem_dump.rs:185` 均为 `#[cfg(test)]` 项（非 test 构建不可见）。属别 session 在制品痕迹（票面已预警），与本票改动无关（本票未触碰 backtest/ 任何文件），按纪律不修复不触碰。⟹ 成交与净值维度**未能量化**；缺口 = backtest_bin 编译修复（建议交还该 session 所有者）。下游覆盖改由 p123 replay 承担（证书/买卖点计数，§5）。

## 7. 探针留存说明

探针（`GapTangentProbe` / `OverlapTangentProbe` + `tangency_probe` binary）**留存**，理由：① 与 `ancok_probe`（697 号 ceiling 度量面）同模式——thread_local +=1 零分配，换任意数据 run 可复测相切频次；② 裁定书 §6 要求影响量化，复测/复议（裁定书签字位「实装后复核」「编排者复议」）需要可重跑的生产路径度量；③ 旧版 `rust/src/segment.rs`（§8 ESCALATE）若后续开票拉齐口径，同模式探针可直接复用模板。若编排者裁定移除，改动面 = 两文件探针段 + bin 文件，可逆。

## 8. ESCALATE 项

1. **旧版 `rust/src/segment.rs`（非 theta_v0）同谓词仍为旧口径**：`:110-116` `three_stroke_overlap`（`lo < hi`）与 `:343-363` `is_fractal_and_gap`（`>=`），且为**活代码**——`segments_from_strokes_v1` 经 `segment_layers.rs:477,701`、`orchestrator.rs:48` 服务 recursive_t/orchestrator 生产链路。裁定书「全域生效」覆盖该落点，但本票实装范围明确限定两文件（票面：「超出两文件处先报告再动」）⟹ 上报编排者裁定是否开票跟进（同口径 + 同模式量化）。
2. **backtest_bin feature 编译失败**（fill.rs:328 / runner.rs:72,79 → `#[cfg(test)]` 项）：阻塞成交/净值维度量化，建议归还该在制品所有者修复后补测。
3. **`Origin.SegmentFeatureSeq.Overlaps` 缺 `Decidable` 实例**（SPEC 增补 (b) 实测发现）：fixture 导出走 `decide (¬ HasGap ..)`（经已证 `gap_iff_not_overlap` :115 与 Overlaps 严格互推），机器见证语义完整；但**补 `instance ... Decidable (Overlaps a b)` 属证明项级改动，按指示未直接加**，请示是否立项补齐（一行 instance，模式同 :105 HasGap 实例）。
4. **裁定书签字位**：「实装后复核」「编排者复议」两空位待签；裁定书 §3.1 未能判定项（67/71/77/78 课外链配图无法核实）照实保留。

**状态补记（2026-07-26 接手 session 独立复核，非新 ESCALATE）**：

- **fixture 落地情况**：SPEC 增补 (b) 已落地（明细 §9）。本 session 独立重跑全链验证：formal 全量 `lake build` 成功（144 jobs，热构建 replay，仅既有 axiom info 报告与 SelfSimilarity 未用变量 linter 警告，均非本票引入）；`lake env lean Origin/ParityFixtureExport.lean` 再生输出与盘上 `rust/tests/fixtures/theta_v0_parity.json` **逐字节一致**（cmp）；`cargo test --release --test theta_v0_lean_parity` 复跑 **11/11**（含 `lean_gap_overlap_tangent_bit_exact`）。
- **Lean 证明项零改动**：项 3 的 `Decidable (Overlaps a b)` 实例未加（仍开放待裁），未新增/修改任何 theorem / instance / def 证明义务。
- **测试终态复核**：全量 `cargo test --release --lib` 复跑 = **1821 通过 / 1 失败 / 132 ignored**，唯一失败 `extract_signals_bit_exact_digest_guard`（#110 在案，digest left=16618955402698307653 / right=10432481772907336594 与 §2 基线逐位相同）——相对 §2 终态**无新增失败**。
- **§5 落地**：量化③ 已按收窄裁定完成（本节 §5），产物：stdout/stderr/dump 改前改后各两份（OKLO：`/tmp/s5_oklo_{before,after}.{stdout,stderr,dump}`；BTC：`/tmp/s5_btc_{before,after}.*`），窗口桥由 tangency_probe `TANGENT_WINDOW` 机器导出。

## 9. SPEC 增补项 (b)：#246 相切 fixture 机器见证（已落地）

- **lake build**：formal 全量 `lake build` 成功（144 jobs，无错误；仅既有 axiom 依赖 info 报告）。
- **导出器扩展**（`formal/Origin/ParityFixtureExport.lean`，无重写、最小扩展）：+`import Origin.SegmentFeatureSeq`；新增 `gapOverlapJson`（单区间对导出，`has_gap`=`decide (HasGap ..)`、`overlaps`=`decide (¬ HasGap ..)`，全字段机器求值）与 `feOf`（用例区间构造，`valid` 由 omega 直推）；`fixtureJson` 末尾新增 `gap_overlap` 段。
- **用例清单（5 例，区间端点=用例输入，期望值全机器产）**：
  | 用例 | 区间对 | 机器导出 has_gap | overlaps |
  |---|---|---|---|
  | `tangent_a_high_eq_b_low` | [5,10]/[10,20]（a.high==b.low 相切） | false | true |
  | `tangent_b_high_eq_a_low` | [10,20]/[5,10]（b.high==a.low 相切） | false | true |
  | `strict_disjoint` | [5,10]/[11,20] | true | false |
  | `strict_disjoint_rev` | [11,20]/[5,10] | true | false |
  | `strict_overlap` | [5,12]/[8,20] | false | true |

  两相切形态机器见证 `has_gap=false ∧ overlaps=true` —— **#246 裁定语义（相切=重合、无缺口）被 Lean `decide` 在 Bool 层直接坐实**；5 例全部满足 `has_gap == !overlaps`（互补）。
- **三笔形态判定不适用**（如实）：Lean 形式化层无三笔重合谓词（`Overlaps` 为两区间版；`HasGap`/`Overlaps` 即裁定书 §3.3 强制性依据的两谓词）；三笔 `max(lows)==min(highs)` 与两区间 `a.high==b.low` 同构，已由 `tangent_a_high_eq_b_low` 覆盖；rust 三笔谓词相切行为由 `segment.rs` 单测 `three_stroke_overlap_tangent_counts` 机器覆盖。
- **regen fixture**（`lake env lean Origin/ParityFixtureExport.lean`）：`rust/tests/fixtures/theta_v0_parity.json` 旧字段**零漂移**（逐键比对 HEAD 版本全同），仅新增 `gap_overlap` 段——机器耦合链成立。
- **rust parity 断言**（`rust/tests/theta_v0_lean_parity.rs` §7，沿用 §0 631 机器耦合模式与认识论标注）：`lean_gap_overlap_tangent_bit_exact`——5 用例逐一断言 rust `Interval::overlaps`/`gap`（theta_v0::parser::segment 既有原语）== fixture 导出值，外加 fixture 自证互补断言；`cargo test --release --test theta_v0_lean_parity` **11/11 全过**。
- **Lean 证明项零改动**：未新增/修改任何 theorem、instance、def 证明义务（导出函数为纯 #eval 序列化代码）。
