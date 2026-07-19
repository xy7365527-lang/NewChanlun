# p115：谓词口径实装 R1+R2+R3（趋势全离开段 / pan 力度或关系 / pan A′ 锚扩展）

日期：2026-07-17 ｜ 性质：**生产实装 + 全量重放验证** ｜ worktree：`/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`）
实装依据：代理裁定已批（裁定文书由并行工位撰写）；证据链 = `p112-trend-predicate-caseaudit-20260717.md`（R1 规格）、
`p113-pan-caseaudit-20260717.md`（R2/R3 规格）、`doc-trend-divergence-predicate-20260717.md`、
`doc-pan-divergence-doctrine-20260717.md`（教义判定书）、`p114-relaxation-ceiling-20260717.md`（产量上限对照）。

## 0. 结论先行

- **三项教义修复已落生产码**（4 个 lib 文件 + 15 个 bin 调用点的 dif 管线；塔/笔/线段/包含路径零触碰）：
  - **R1**（趋势 seg_c 取段 = 全离开段 + 全合取确认时点）：`level_view.rs:813` `provide_divergence_pairs`
    c_end 由「首个同向段终点」扩为「c_start 起、end ≤ as_of 的末个同向段终点」（037:22 口径，
    p112 T3_ext 窗口）；确认谓词由「混合柱面积单通道」升级为**全合取**（T4 回拉0轴 ∧ T3 三买 ∧
    T2 破极值 ∧ T5 力度或关系），确认时点 t* = 首个全成立时点（`level_view.rs:548` `trend_confirm_time`，
    禁前视）；confirmed 事件的 `interval_b`/`turn_source` 收束到 t*。
  - **R2**（pan 力度或关系）：`segments_diverge_or`（`divergence.rs:334`）= 同色柱面积 C<A（060:44）
    ∨ 黄白线峰 C<A（026:521）∨ 同向柱峰 C<A（025:38），027:32「只要其中一个符合就可以」；
    消费点 = `judge_pan_div`（`signal.rs:746`）与 nest Cons 分支（`level_view.rs:782`）。
  - **R3**（pan A 锚扩至中枢前同向段）：`locate_pan_div_structure_front_anchor`（`signal.rs:676`），
    A′ = `end_index ≤ c.start_index` 的最近同向段（061:28 中枢两头比较，回中枢要件由中枢本身满足）；
    窄锚 locate∧Extreme 任一失败即回退 A′ 重判 Extreme（044:234 维持）。
- **cargo test --lib 全绿**：`1647 passed; 0 failed; 127 ignored` = 基线 1638 + 新增 9，**零既有测试变红**。
- **p92 全量重放对照**（btc_1m_full.json 4,613,599 bar，P92_CKPT=250000）：见 §5（实装后
  `/tmp/p92_ckpt_dump_v2.txt` vs 基线 `/tmp/p92_ckpt_dump.baseline.txt` A=41/B=25）。

## 1. 基线记录

| 项 | 值 |
|---|---|
| HEAD | `fe03fd5c28ff43f182b499f212848f6b9d57878d`（分支 kimi-nest-mainline-20260717） |
| 基线测试 | `cargo test --lib` → `1638 passed; 0 failed; 127 ignored`（存 `/tmp/p115_baseline_test.txt`） |
| 基线 ckpt dump | `/tmp/p92_ckpt_dump.txt` → 已复制 `/tmp/p92_ckpt_dump.baseline.txt`（818 行，末检查点 as_of=4,500,000：A=41 / B=25） |
| 工作树预存改动 | `rust/Cargo.toml` 与 3 个 bin 在实装前已为 modified（本工位未触碰） |

## 2. 逐项改动清单（文件:行号 + 教义依据）

### R1：趋势 seg_c 取段 = 全离开段 + 全合取确认时点

证据：p112 实锤 #105 单腿窗口构造性缺陷（`level_view.rs` seg_c 恒 win_legs==1 @154/154，037:18 三买
无处容身；全离开段口径下 T1∧T2∧T3_ext∧T4∧T5 对 154/154 成立）。裁定实装点：

1. `rust/src/theta_v0/classifier/level_view.rs:855`（`provide_divergence_pairs`）：c_end 改取
   c_start 起、`end_index ≤ as_of` 的**末个同向段**终点。单段离开时与 #105 逐位一致（退化兼容，
   测试 `provide_divergence_pairs_seg_c_spans_full_departure` 双口径锁定）。教义：037:22
   （c 含三买 ⟹ c 至少两个次级别中枢，单腿窗口在结构上不可能含三买）。
2. `rust/src/theta_v0/classifier/level_view.rs:548`（新函数 `trend_confirm_time`）：全合取扫描，
   首个全成立时点 t*——
   - T4 回拉 0 轴（025:761/024:24，p112 主口径 cross_dif）：B 中枢 span（close 下标）内 DIF 变号
     或触 0，`dif_crosses_zero`（`divergence.rs:354`）；
   - T3 三买（037:18/051:314）：`trend_third_class_in_c`（`signal.rs:467`，judge_third 几何去
     anchor 门——anchor 门是 Q7-#1 工程资格非教义，p112 §2-T3 同口径），全离开段内首个
     「同向离开破核心 + 紧邻反向回试不重回 [ZD,ZG]」对；
   - T2 破极值（037:20/061:28）：c 包络破 b 包络，窗口 [c_start, t] 渐进扩展（包络只扩 ⟹ 假→真单调）；
   - T5 力度或关系（027:32）：`[c_start, t]` vs b 段，同色面积 ∨ 黄白线峰 ∨ 同向柱峰
     （proxy 只增 ⟹ 真→假单调，转假即终假——033:26 无衰减即无背驰的市场事实边界）。
   - **禁前视**：全部结构/力度窗口以 `as_of` 为界；t3 时点力度窗口无 bar ⟹ None（诚实判负，
     同旧 `map None ⟹ false` 口径）。
3. 消费侧收束（`level_view.rs:697-713` nest Trend 分支）：confirmed ⟹ `interval_b=(c_start, t*)`、
   `turn_source=t*`（确认时点即事件坐标——在 t* 之前全合取不成立，坐标不得早于确认时点）；
   未确认 ⟹ 保持全离开段结构坐标。`assemble_level_view` 的 `TerminalDivergence` 完成判定
   （`level_view.rs:1032-1082`）改用同一 `trend_confirm_time`（单一谓词来源，消除「D2 两套确认」）。
4. Cand 的 Extreme 预滤（`level_view.rs:573-585`）保持在 seg_c 全离开段包络上（宽预滤，
   精确 T2 在扫描内随 t 判定）。

### R2：pan 力度或关系（面积∨黄白线∨柱高）

证据：p113 Part A——面积单通道必要门 30.4% 聋度（144/474 事件原文下仍具信号），027:32『只要
其中一个符合』或关系。实装点：

1. 新原语（`rust/src/theta_v0/classifier/divergence.rs`）：
   - `same_color_area`（:291，060:44/057:40/024:40 同色柱面积：Long=绿柱 |hist<0| 和，
     Short=红柱 hist>0 和）——混合柱 Σ|hist| 把反向柱计入是 060:44 之外的人为加严
     （p113 §2.2 turn=739707 标本）；
   - `same_dir_hist_peak`（:310，025:16/025:38 柱子伸长的高度）；
   - `segments_diverge_or`（:334）：三通道任一严格 `<` 即成立（027:32/026:521/025:38）。
2. 消费点①：`judge_pan_div`（`signal.rs:746`）Weak 由 `AbcDivergence::diverges`（混合面积单通道）
   改 `segments_diverge_or`；调用点同步加 `dif` 参数（`signal.rs:1287`、`recursive_tower.rs:2084,2130`）。
3. 消费点②：nest Cons 分支 `divergence_confirmed`（`level_view.rs:776-783`）同改或关系。
4. **对称边界**：各 proxy 全无衰减 ⟹ 判负（033:26 市场事实，非放宽）——测试
   `pan_div_or_relation_all_channels_fail_rejects` 锁定。

### R3：pan A 锚扩至中枢前同向段

证据：p113 Part B——1,238 条 Cons-leave 的第一击杀机制是 A 锚窄化（99.6% 块结构层可救回；
中枢震荡域排除 = 0）。实装点：

1. `rust/src/theta_v0/classifier/signal.rs:676`（`locate_pan_div_structure_front_anchor`）：
   A′ = `end_index ≤ c.start_index` 的最近同向段（061:28 形态 A′→中枢→C、080:168、049:36-38
   「最标准」≠唯一、080:364「不是光比较最近这一段的」）；回中枢要件由中枢本身满足；
   C 仍取当前离开 episode（λ_C 与窄锚同一 helper）。
2. 调用顺序（裁定「窄锚优先」）：`judge_pan_div` = 窄锚 `or_else` A′（`signal.rs:765`）；
   nest Cons 分支 = 窄锚 locate∧Extreme，任一失败回退 A′ 重判 Extreme（`level_view.rs:744-755`）
   ——044:234 正名条件维持（p113：扩 A 后 C 仍未越极值者 3 链维持判负）。

### dif 管线（R1/R2 的数据依赖）

`provide_nest_candidate_events`（`level_view.rs:652`）与 `LevelViewMaterial`
（`level_view.rs:888`）各加 `dif: &[f64]` 字段/参数（T4 与黄白线分量所需，与 hist 同源同坐标系）。
bin 调用点全量同步（15 个文件）：p92（`cache.macd_dif()` 同源）、p83/p84/p86/p93/p95/p102/p104/
p107/p108/p109/p111/p112/p113/p76（`compute_macd(...).dif` 或 cache 同源）。**零行为差异的旧口径
探针内部逻辑未改**（仅管线适配；探针均为一次性测量仪器，报告已归档）。

## 3. 新增/更新测试清单（9 新增 = 4+2+3；2 既有夹具更新）

| 测试 | 位置 | 内容 |
|---|---|---|
| `same_color_area_counts_only_move_color_bars` | divergence.rs | 060:44 同色口径；混合柱对照 |
| `same_dir_hist_peak_takes_deepest_move_color_bar` | divergence.rs | 025:38 柱高 |
| `segments_diverge_or_any_single_channel_suffices` | divergence.rs | 027:32 单通道充分 + 混合/同色分离（p113 标本型）+ 全无衰减判负 |
| `dif_crosses_zero_touch_or_sign_flip` | divergence.rs | 025:761 回拉 0 轴；越界 false |
| `trend_third_class_in_c_geometry` | signal.rs | 037:18 立即/延迟三买、as_of 截断禁前视、单腿恒假（p112 机械显形） |
| `pan_div_front_anchor_rescue_and_or_relation_weak` | signal.rs | R3 A′ 救回 + R2 或关系（混合判负、同色救回） |
| `pan_div_or_relation_all_channels_fail_rejects` | signal.rs | 033:26 市场事实边界（不放宽） |
| `provide_divergence_pairs_seg_c_spans_full_departure` | level_view.rs | R1 全离开段取段 + as_of 截断退化 #105 逐位一致 |
| `auto_pairing_pending_when_third_buy_not_established` | level_view.rs | 037:18 缺三买 ⟹ 全合取不成立 ⟹ Pending（负例） |

既有夹具更新 2 处（语义更新的必然结果，非越界）：
- `level_view.rs` `extended_windows()` 夹具补回试腿（腿13，Down 低点 170 > w2.zg=165 不重回核心，
  与腿12 构成三买）——旧 as_of=129 的测试按 end ≤ as_of 过滤本腿，断言值不变；
- `auto_pairing_completes_only_after_real_macd_divergence` 升级全合取夹具（as_of=139 覆盖 t3、
  dif 在 w2 span 变号、同色面积/柱峰 C≪A）——测试意图（真背驰才 Completed）不变，判据口径
  从面积单通道升为教义全合取。

## 4. cargo test 证据

```
$ cd /tmp/kimi-nest-mainline/rust && cargo test --lib 2>&1 | tail -3
test result: ok. 1647 passed; 0 failed; 127 ignored; 0 measured; 0 filtered out; finished in 6.08s
```

- 基线 1638 + 新增 9 = 1647，**零既有测试变红**（含 07a resume/full bit-exact debug_assert、
  bit-exact 塔/笔/线段/包含全部微分测试）。
- `cargo check --bins` 零错误（15 个 bin 的 dif 管线全部编译通过）。

## 5. p92 全量重放对照

- 命令：`cd rust && P92_DUMP=/tmp/p92_ckpt_dump_v2.txt P92_CKPT=250000 cargo run --release \
  --bin p92_nest_replay_postruling -- ../analysis/data_cache/btc_1m_full.json`
- 对照：基线 `/tmp/p92_ckpt_dump.baseline.txt`（末检查点 as_of=4,500,000：A=41 / B=25）。

### 5.0 250k 冒烟对照（P92_MAX_BARS=250000，基线副本 `/tmp/p115_base_rust` 同跑）

| 行 | 基线（无 R1/R2/R3） | 实装后 | 读法 |
|---|---:|---:|---|
| candidates | 198（trend 39 / pan 159） | **913**（trend 83 / pan 830） | R3 A′ 救回大量 Cons 候选（pan 5.2×）；R1 全离开段 Extreme 预滤放宽（trend 2.1×） |
| divergence_confirmed | 83（trend 34 / pan 49） | **465**（trend 47 / pan 418） | R2 或关系确认 pan 8.5×（聋度救回实证）；trend 在全合取（更严）下仍 34→47（T5-OR 补位） |
| terminal_confirmed | 3 | 4 | — |
| P92_CERT | A=7（t1/p4/m2） B=4（t1/p3/m0） | A=8（t0/p6/m2） B=9（t0/p7/m2） | 250k 早期窗产量温和上行；B_zero=false 维持 |
| P92_BIT_EXACT | old_path=0 tower=0 moves/centers/bsp/pan=0 **lifecycle_cp_ownership=1** | **逐行相同**（含 lifecycle=1） | **lifecycle_diff=1 为基线既有**（L0 ownership，全量/增量 frontier 工件，与本次改动无关——基线副本逐字复现） |

- 基线副本构造：`rsync` 源码副本 + 4 个 lib 文件与 6 个已跟踪 bin 回退 `git show HEAD:` 版本 +
  p92 bin 手工回退 dif 管线（恢复后 `grep dif` 零残留）；数据绝对路径直传。
- 冒烟 P92_PROVIDER：complete=true、unresolved_targets=0、future_violations=0（两侧一致）。

### 5.1 全量重放（4,613,599 bar）——后台运行中

- 任务：后台 `bash-dbitynwz`（`P92_DUMP=/tmp/p92_ckpt_dump_v2.txt P92_CKPT=250000`，release 已编译）。
  进度快照：terminal pass 完成（~12 min），prefix pass 推进至 1.5M/4.6M（pending 18,297→12,492）。
- 产物落点（完成后可直接读）：
  - dump：`/tmp/p92_ckpt_dump_v2.txt`（逐检查点 CKPT/CKPT_STATS，与基线同格式同节奏）；
  - 日志：`/tmp/p115_p92_replay_v2.log`（P92_YIELD / P92_CERT / P92_BIT_EXACT / P92_PROVIDER / P92_MISSED）。
- 对照脚本：末检查点（as_of=4,500,000）`grep CKPT_STATS` 取 A/B 计数 vs 基线 A=41/B=25；
  `grep P92_BIT_EXACT` 验塔/笔/线段/中枢 diff=0（lifecycle_cp_ownership 若再出现 1，按 §5.0
  基线复现判定为既有 frontier 工件）；`diff <(grep '^CKPT caliber=A' 基线) <(同上 v2)` 逐行对账。
- 250k 冒烟已验证全链路与对照方法（§5.0）：bit-exact 五维 diff=0、lifecycle=1 基线既有、
  provider complete、no_forward=true。

## 6. 边界与未解决问题

- **改动面纪律**：仅上述三处判据口径 + dif 管线；塔/笔/线段/包含/inclusion（bit-exact 路径）
  零触碰（`git diff --stat` 仅 4 个 classifier 文件 + bin）；未改 `rust/Cargo.toml`；零 git mutation；
  主仓 `/Users/silencehan/Projects/NewChanlun` 零写入。
- **R1 的未决分项（诚实声明）**：
  1. T5 或关系的面积分量取**同色柱**（060:44）——p112 的 154/154 翻转测量用的是混合柱
     （segments_diverge 复算），同色口径下趋势域的逐案通过率为**新测量**（本重放首次产出）；
  2. 70 延迟案的确认时点 t* 迟于旧 turn（平均 ~45k bar），其 interval_b=(c_start, t*) 显著变宽，
     对父级包含（身份门）的影响由重放实测——p114 的 +622 是旧坐标系上限，非本实装的产量承诺；
  3. T4 主口径取 cross_dif（p112 主口径；最严 ratio10 在 154 中仅 3 案失败——该 3 案在本口径下
     仍判正，属裁定内的口径产物）。
- **R2/R3 的未决分项**：A′ 取「中枢前最近同向段」单段区间（p113 最保守口径；多段进入 episode
  的更宽口径未实装）；narrow 成功但 Weak 全假时不回退 A′ 重判力度（p113：力度层=市场事实，
  033:26）；pan 不加 T4 回拉项（027:32 原文两项口径的子集选择，与 p113 主判定四 proxy 并集的
  前三项一致——幅度 proxy 未纳入，task 规格如此）。
- **既有 66 张证书的存续**：R1/R2 是判据升级（更严在合取项、更宽在或关系），旧口径下确认的
  事件在新口径下可能失证（如混合面积过但三通道全假、或三买不存在）——产量对照以 §5 重放为准，
  不为保产量叠加旧口径（禁补丁）。
- 声明与能力一致：本报告只声明「三处判据口径已按裁定实装 + 单测全绿 + 重放对照」，不声明任何
  择时 alpha；L2/L3（谓词的市场有效性）不在本工位。
