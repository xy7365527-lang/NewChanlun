# Opus 4.8 引擎改动审计报告（O(N²)→O(N) 优化全链）

> 审计范围：本 session 的 Rust 引擎增量化改动（bi_engine / orchestrator / segment /
> segment_layers / lib）+ Python 层（m1_i / m1_e / fugue_alpha_diagnosis / fugue_version_i）。
> 审计方法：逐文件读码 + 与 Python 原版（bi_engine.py / a_segment_v1.py /
> core/recursion/*_engine.py / move_state.py / diff/helpers.py / fugue_alpha_diagnosis.py）
> 逐字段比对 + 不变量推演 + 对 `_fast_alive_top2` 做 20,000 例随机性质测试（本次审计内执行，全过）。
> 沙盒无 cargo，in-repo Rust differential tests 未在本次审计中复跑（依赖既有运行记录）。
>
> 认识论等级声明：本审计为静态逻辑审计（L0 推演）+ 一项 L1 性质测试；引用的 bit-exact
> 结论依赖 session 内已跑的真实数据差分脚本（`_verify_segment_incremental.py` /
> `_verify_segment_layers.py` / `_verify_bi_zhongshu_new_signals_equiv.py`，L1 管线等价）。

---

## 总评

整体设计严谨：增量器普遍采用"永久前缀 + 有界易变尾重算"模式，配差分测试与真实数据
端到端验证。逐字段比对未发现移植性 off-by-one 或索引错位。但发现 **2 个中等级别的
理论性不变量缺口**（均只有经验性守卫、无结构性证明，且 in-repo 合成测试在维度上
覆盖不到）和 **1 个 V-I 回测层的真实记账漏洞**，以及若干低风险点。

| 文件 | 判定 | 发现 |
|------|------|------|
| rust/src/bi_engine.rs | ✅ 通过 | F4（低，理论） |
| rust/src/orchestrator.rs | ⚠️ 有保留通过 | F1（中，理论） |
| rust/src/segment.rs | ✅ 通过 | 与 F1 共享判据 |
| rust/src/segment_layers.rs | ✅ 通过 | F5（低） |
| rust/src/lib.rs | ⚠️ 有保留通过 | F2（中，理论）、F6（极低） |
| analysis/m1_i_rust_engine.py | ✅ 通过 | 索引映射逐字段核对无误 |
| analysis/m1_e_rust_engine.py | ✅ 通过 | 消费字段集核实成立 |
| analysis/fugue_alpha_diagnosis.py | ✅ 通过 | top2 贪心 20K 例性质测试全过 |
| analysis/fugue_version_i.py | ⚠️ 有保留通过 | **F3（中，真实记账漏洞）**、F7（极低） |

---

## 发现（按严重度）

### F1【中·理论缺口】线段稳定性判据在边界上把易变笔计入扫描窗口

位置：`orchestrator.rs SegCheckpoint::update`（210 行）/ `segment_engine.py _update_checkpoint`（共享判据，Rust 为忠实移植）。

判据 `trigger_k + 1 + MAX_SECOND_SEQ_SCAN ≤ n_strokes` 中的 `n_strokes` 是**快照全长**，
包含尾部 1–2 根**易变笔**（snapshot 末笔恒 unconfirmed 可整体消失；倒数第二笔在其后继为
pending 衍生临时笔时仍可被 `extend_prev_stroke` 延伸）。后果：

1. **等号成立时**（`trigger_k+50 == n_strokes-1`），`second_seq_has_fractal` 的扫描窗口
   `[b+1, b+51)` 末端恰好落在易变笔上——该笔后续变异可翻转分型存在性 → 改变 gap 触发判定
   → 已标"永久稳定"的段断点理论上可变 → 冻结前缀污染，resume ≠ full。
2. `n_strokes` **并非严格单调**（pending 衍生笔可消失使快照变短），"trigger_k 固定、
   n_strokes 单调增 ⟹ 一旦稳定永远稳定"的注释论证在边界两格（`n_strokes ∈
   {trigger_k+51, trigger_k+52}`）不严格成立。

边界条件：仅当段恰在边界值上转入稳定、且窗口末端易变笔变异恰好翻转第二特征序列分型时触发。
现有守卫：OKLO 447K 逐 bar + DX/GC/CL 真实数据差分均未观测到发散（经验性）；但
`seg_checkpoint_tests` / `incremental_tests` 的合成笔是 **confirmed=true 纯追加**，
按构造无法覆盖"尾笔变异/消失"维度（注释本身诚实承认）。
严格修复形式：判据右端改为只计入冻结笔（如 `trigger_k + 1 + MARGIN ≤ n_strokes − 2`，
或从 BiEngine 暴露 `synced_prefix` 作为冻结边界），并补一个驱动 BiEngine 真实变异维度的
Rust 差分测试。
影响声明：若触发则同时污染 SegCheckpoint、IncrementalSegZhongshu/Div/Bsp 的全部
"永久前缀"（它们的稳定性都从此判据导出）。

### F2【中·理论缺口】sync_inc 的 append-only 契约把可变异笔当成永久笔推入

位置：`lib.rs sync_inc`（1177 行）/ `bi_zhongshu_bsp.rs` push 契约。

`confirmed_count = n−1`（仅排除末笔）后推入 `strokes[inc_pushed..confirmed_count]`。
但快照中**索引 n−2 的 confirmed 笔**在其后继为 pending 衍生临时笔时并非永久：临时笔消失
→ 该笔重新成为可延伸末笔 → fxs 末分型被同类更极端分型替换 → 该笔 p1/high/low/i1 变异。
此后笔数再增长时 `inc_pushed` 已越过该索引，IncrementalBiZhongshuBsp 持有**陈旧版本**，
而全量路径 `current_bi_zhongshu_buysellpoints` 用的是变异后版本 → 理论上可发散。

契约注释"方向反转 ⟹ 前序 confirmed 笔已永久固定"对"后继是真实笔"的情形成立，对
"后继是临时笔"的情形**不成立**——声明强于实际（090 号意义上的轻度声明膨胀）。
现有守卫：`_verify_bi_zhongshu_new_signals_equiv.py` 在 OKLO 333K 全量 + ES/GC 各 500K
上信号级 bit-exact 全过（L1，跨 3 标的 ~1.3M bar）——经验上未触发或未传导到 4 个布尔信号。
Rust 端 differential_tests 同样只覆盖纯追加维度。
严格修复形式：只推入 BiEngine 冻结前缀（`synced_prefix` 内）的笔，或推迟一根
（`confirmed_count = n−2`），并证明/测试该口径下信号时点不漂移。
影响声明：影响 m1_i ladder2（笔中枢 BSP delta 信号）。

### F3【中·真实漏洞】run_version_i `_close` 在 total_shares≤0 时静默丢弃交易

位置：`fugue_version_i.py _close`（813 行）+ 收尾 `if ... pos.total_shares > 0`（909 行）。

```python
if pos is None or entry_price <= 0 or pos.total_shares <= 0:
    state = _FLAT; pos = None; active_levels = []; return
```

挣股数阶段（earning）`open_diff` 真实减仓 `total_shares -= total_shares * frac`。当
**N_sub == 1**（如 floor=2、entry_ladder=3 → active_levels=[2]，level_frac=1.0）时一笔
开放短差即把 `total_shares` 减到 **0**。此时若触发任何出场（type1 卖点 / 止损 A / EOD）：
- `_close` 的前置守卫在**回补短差之前**检查 `total_shares <= 0` → 直接弃仓返回；
- 该笔交易**不产生 CompletedTrade、不进 diag、不进归因统计**，持仓凭空消失；
- 收尾 EOD 分支同样被 `total_shares > 0` 条件跳过。

这不是理论边界：N_sub=1 是 `I_seg2`（floor=2）配置下 entry_ladder=3 的常态组合，只要
该笔交易进入 earning 阶段且短差开放时被出场信号命中即触发。正确形式：守卫只保留
`pos is None`，先回补全部开放短差（回补本身会恢复 `total_shares`），再做估值与记录；
或估值公式纳入开放短差的应收（`cyc.shares * cyc.sell_price / price`）。
影响声明：影响 I 系列全部变体的交易数/复利/归因统计的完备性；E 基线不受影响。
（注：诊断 trace 插桩本身核对无误——`trace=None` 时零数值影响，profit 公式
`(sell−buy)×shares` 与 ShortDiffCycle 一致，was_earning 守恒律 open/close 同律正确。）

### F4【低·理论】bi_engine.rs 删除了 Python 侧 1e-9 容差复用，存在亚容差分歧窗口

Python `_compute_strokes_from_checkpoint` 末尾在 `len 相等 ∧ 末笔 i0/i1 相等 ∧
|Δp1| < 1e-9` 时**复用 prev 引用**（保留旧值）；Rust 新 snapshot 路径每次物化新值。
当 `0 < |Δp1| < 1e-9` 时 Python 输出陈旧 p1、Rust 输出新 p1 → 逐位等价被破坏。
边界条件：真实报价的价格变动 ≥ tick，落入 (0, 1e-9) 实际不可能；纯理论。且 Rust 行为
是"更正确"的一侧。其余 snapshot 原地维护逻辑核对无误：cp 前缀 `[..len−1]` 不可变、
每 bar 至多一次 advance、`is_empty` 路径自愈（清零后下次全量重建仍正确）、
`synced_prefix` 单调性成立（cp_strokes 只 push/extend-last、reset 同步清零）。

### F5【低】IncrementalSegDivergences 对 moves 前缀持久性的依赖以 panic 兜底

`&moves[self.processed_moves..]`：若 moves 列表长度跌破 `processed_moves`（即已 commit
的走势消失），Rust 直接越界 panic。不变量"seg_end < n_seg−256 的走势永不消失"依赖
易变段尾 ≪ 256 的经验事实（与 F1 同根）。panic 是响亮失败而非静默错值，可接受，
但属于未在注释中声明的隐式假设。同类：`IncrementalSegBsp` 的
`hi + 1 - win_lo` / `lo - win_lo` 减法在 `stable_anchor ≤ n_seg − 1` 失效时下溢
panic——同样由"尾部波动 ≪ 256"保护。

### F6【极低】lib.rs sync_inc 的 level_id 首调绑定

`inc_bz` 懒创建绑定首次调用的 level_id，后续不同 level_id 的调用静默复用旧引擎。
实践中仅 `BI_ZHONGSHU_LEVEL_ID` 一个值，无实际影响；严格形式应断言或按 level_id 分槽。

### F7【极低·声明精度】"每级操作 1/N_sub 满仓"在 earning 阶段漂移

`open_diff` 的 `shares = total_shares * frac` 按**当前** total_shares 计算；earning 阶段
total_shares 随短差增减，故单笔量 ≠ 初始满仓的 1/N_sub。非 earning 阶段（股数守恒）精确。
docstring 的"每个操作 1/N_sub 满仓"在 earning 阶段不精确——建议注明。

---

## 核实通过的关键等价性（逐项证据）

1. **短路键逐字段一致**（Rust ↔ Python 引擎）：
   - `SegTailKey` ↔ `_stroke_tail_key`（n + 末两笔 i0/i1/p0/p1）✓
   - `ZsSegKey` ↔ zhongshu_engine 的 `(n, segs[-2].s0, s1, dir)` ✓
   - `MoveZsKey` ↔ move_engine 的 O(1) settled 推导键（A/B 两形含义同构：B 形
     `(n_zs, 0, ns)` 同时覆盖 n_zs==0 与 n_settled==0）✓
   - `BspKey` ↔ buysellpoint_engine 的 `(n_seg, segs[-2].s0, s1, n_zs, n_mv)` ✓
   → epoch 门控（move/bsp/recursive_epoch）的"未变 ⟹ 输出逐位不变"前提相对 Python
   引擎**自身短路行为**严格成立；任何"key 不变但内容应变"的盲区是从 Python 继承而非新引入。

2. **move_settle_delta ≡ diff_moves 的 MoveSettleV1 集**：与 `diff_by_prefix` +
   `_move_equal`（kind/direction/seg_start/zs_end/settled）+ `same_move_identity`
   （seg_start）逐分支比对一致——公共前缀、index 对齐同身份 F→T 升级、全新+settled，
   发射顺序均为 curr 索引序 ✓。`take` 语义（同 bar 二次调用返回空）在 m1_e 单消费点下安全。

3. **segments_from_strokes_v1_into 移植**：与 a_segment_v1.py 逐函数比对一致，含
   71 课 append 预演双调 scan_trigger 的 `last_checked` 副作用序、TAIL_WINDOW=7 起点、
   `skip_trigger`、67 课 gap_closed_by_c、`_finalize_last_segment` 三分支（含"尾部过短延伸
   前段"）、`_ensure_last_unconfirmed`、`_second_seq_has_fractal` 的 dir_state 初始化
   反转怪癖（`"DOWN" if seg_dir=="up"`）均忠实移植 ✓。`resume_start >= n` 分支 ✓。

4. **设计 B（复用 stable_count−1）的等价论证成立**：发射循环保证
   `segments[j].s0 == segments[j-1].trigger_k`、方向交替、fresh FeatureSeqState 与全量
   路径"发射后状态"逐字段一致；reuse=0 时由 `find_overlap_start` 作用于冻结前缀锚定。
   `-1` 偏移确保 finalize/ensure 只触碰重算区（最后稳定段必被重新发射，零发射不可达——
   以 F1 判据成立为前提）。`try_resume` 用 to_bits 比对严于 Python 的 round(p,8)，
   失配仅退化全量，安全方向 ✓。去掉 Python 早退分支：每次刷新 stroke_key 自洽，
   无正确性影响 ✓。

5. **IncrementalSegZhongshu / SegDivergences / SegBsp**：永久化判据
   （`settled ∧ break_seg < comp_stable_len`；`seg_end < n_seg−256`；
   `seg_idx < n_seg−256`）的依赖封闭性论证成立（以 F1 为前提）；
   `stable_t1`/`tail_type1` 按 `stable_anchor` 严格不相交（bar 350200 双重计入修复确认）；
   type3 的 partition_point 谓词对 zs_break 排列（settled 升序前缀 + unsettled 尾）单调 ✓；
   finalize 区间 `[lo, hi)` 推进与 full 拼接不重不漏 ✓；跨界 overlap(2B+3B) 因同
   seg_idx 邻近性落在同侧 ✓。防御性回退（stable_count 回退 → 全量重建）齐备 ✓。

6. **m1_i_rust_engine**：`_level_bsps` 的 MoveTuple/LevelZhongshuTuple/DivergenceTuple
   全部下标映射逐字段核对正确（含 `d[0][5]/d[0][6]` = seg_c_start/end、`z[7]/z[6]` =
   break_direction/break_comp）；`zg_max != 0.0` ↔ Python `or` 的 falsy 语义一致；
   recursive 的 `cache_key` 内容比对 ⟺ Python `move_events or zhongshu_events` 门控；
   max_ladder 在 epoch 门控下演化与逐 bar 版一致（层集合只能随 epoch 变化）✓。
   type2_buy 的"状态首现近似"已在 docstring 诚实标注为报告口径 ✓。

7. **m1_e_rust_engine**：`run_swing_trading` 实际读取字段经 grep 核实恰为
   close/down_move_settled/up_move_settled/entry_div_ok/exit_div_ok/l2_flip_short
   六个——`enable_bsp=False` 跳过的层（bsp/递归）确实零消费；moves 层在 bsp 之前计算，
   关闭 bsp 不改 moves 输出（代码路径确认）✓。macd.update 先于 process_bar 的顺序、
   settle 记录先于 BarSignal 构造的顺序均与原版一致 ✓。

8. **_fast_alive_top2**：本次审计执行 20,000 例随机性质测试（含大量并列 val），
   top-2 前缀 + `len≥2` 语义与 `_fast_alive` 全等（严格 `<` 正确复刻稳定降序并列取栈序）。
   `top2_only=True` 仅用于 bar/bi/l2 proxy 树（消费者只有 detect_settle），
   fugue_alpha_diagnosis 自身的 l1/l0/l2 树保持默认全量路径（median/ratio 消费者安全）✓。

9. **enable_bsp 门控 / epoch 单调性**：u64 不回绕；epoch 自增 ⊇ 内容变化（superset），
   调用层只做无害冗余扫描，不漏信号；`trend_new_signals` 的 seen-set 下沉与
   "未变 bar 扫描必全命中 seen" 的状态-事件二象性论证成立 ✓。

---

## 结果包（简化版，纯技术审计）

- **结论**：9 个文件中 6 个通过、3 个有保留通过。无移植性 off-by-one；发现 1 个应修的
  回测记账漏洞（F3）和 2 个仅有经验守卫的理论不变量缺口（F1/F2，二者同根于
  "快照尾部 1–2 笔可变异/消失"未被稳定性判据与 push 契约计入）。
- **边界条件**：F1/F2 的"通过"判定会在以下条件下翻转——任一真实数据流出现
  (a) 段恰在 `n_strokes = trigger_k+51/52` 边界转稳定且窗口末易变笔变异翻转第二特征序列
  分型，或 (b) n−2 位 confirmed 笔在 pending 衍生后继消失后被延伸且延伸值改变笔中枢
  BSP 信号。现有 ~1.3M–2M bar 真实数据差分未观测到。
- **影响声明**：本审计未改动任何代码；建议的修复（F3 必修；F1/F2 判据收紧 + 补变异维度
  测试）影响 orchestrator 稳定性判据、lib.rs sync_inc、fugue_version_i._close 三处。
