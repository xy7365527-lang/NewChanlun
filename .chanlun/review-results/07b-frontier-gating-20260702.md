# 07b extract_second frontier 门控（task #23，A 泳道 resume 家族）

工位 ws-07b / 认识论：门控 bit-exact = L1（管线正确性，确定性等价）；计时 = L1（CPU 度量，231号零信息增量）
/ 代码影响：`rust/src/theta_v0/classifier/mod.rs`（+~90 行）+ 新增 `rust/tests/theta_v0_07b_gating.rs`（集成验收）

## 阶段0（profile-first）：07b 重复计算模式确认

`classify_with_tower_incremental` 每 memo-miss（`bsp_key=(centers.len, upper_moves.len, seg_len)` 变化）
调 `extract_second_for_level(&lc.upper_moves[..], hist, close_src)`——**全塔重扫**所有 U 个 upper_moves
parent，每个 parent 双侧（Long/Short）跑 `extract_second_signals` → `find_second_type_structure` +
`sublevel_diverges`。跨 N bar 累积 O(U²)。

**profile 坐实（CL，release，THETA_PROFILE_STAGES=1，修前）**：

| N bar | 07b_extract_second | 备注 |
|-------|--------------------|------|
| 300000 | 346 ms（旧树基线）/ — | 指数≈2.06（346→4116 旧树） |
| 1000000 | 3990 ms（当前树，与门控 apples-to-apples）| O(n²) 确认 |

诊断（DIAG 探针，1M CL）：memo-miss=3847，make_mut COW=0（原地），confirmed-delta 扫描累计 166704 +
frontier 扫描累计 308966 = **~476K parent-scan（线性）**；L0 层 07b 输出**恒空**（`l0clone_total=0`，
结构上界 `l0_level_emits_no_second_class_window_bound` 在真实 CL 坐实）。⟹ O(n²) **不在 parent 数**，
在**每 parent 的 `segments_diverge` MACD 面积累加**：对照走势 `prev` 的 close 区间随 n 增长，`|hist|`
逐点求和 O(range) 随窗口线性增长（B3 报告 271.5M |hist| ops 同源）。

## 门控设计（与 A3 同族：frontier 证书 + confirmed 前缀缓存）

每个 parent 的 B2 **纯函数于该 parent**（`second_for_parent`：c1=Compose 首中枢 + subs 侧车 + 全局
hist/close_src，不依赖其它 parent）。confirmed 前缀 parent（`upper_moves[..prefix_count]`，`prefix_count`
= 主循环 pop 后 `lc.upper_moves.len()`，anc.pdf §16 跨 bar immutable + hist 前缀 append-only 稳定）的
B2 跨 bar 不变 ⟹ **缓存前缀 B2、跳过其重复背驰扫描**，只对 frontier tail `[prefix_count..]` 每 bar 重算。

- `LevelCache` 增 `cached_second: Vec<BspPoint>` + `cached_second_count: usize`（推进锚）。
- `extract_second_resume`：单调性守卫（`cached_count>prefix_count` → 保守重置）+ 推进新晋 confirmed
  parent 的 B2 入缓存（一生一算）+ 结果 = 缓存前缀 clone + frontier tail 重算。
- cascade_reset（前缀重排）→ 与 `cached_bsp` 同步 clear（前缀失效重扫）。
- **debug_assert 逐调用对拍**：门控输出 == `extract_second_for_level` 全量重扫（前缀 immutable/hist 稳定
  不变式被违反即 panic）。

## 计时对照（pre/post，CL，release，1M CL，当前树 apples-to-apples）

| 阶段 | 修前（ungated） | 修后（gated） | 降幅 |
|------|-----------------|---------------|------|
| 07b_extract_second @1M | 3990 ms | 1026 ms | **74%（3.9×）** |
| 07b_extract_second @300K | —（旧树 346）| 90 ms | — |
| 全窗墙钟 @1M | 19.8 s | 15.5 s | 22% |

门控**摊还线性化了 confirmed 前缀重扫**（476K parent-scan，每 parent 一生一算），消除了 O(U²) 的
「confirmed parent 每 bar 重复背驰扫描」这一源。

## ★关键结论：门控 sound 但 07b 残余 O(n²)（不是 NO-SHIP，是 B3 #4 unblock）

门控后 07b 仍 **90ms@300K→1026ms@1M，指数≈2.03**——残余 O(n²) 不在门控可覆盖域：根因是 **frontier
parent 的 `segments_diverge` MACD 面积累加**，`prev` 对照区间随 n 线性增长 ⟹ 每 parent O(range)。门控
只能跳过 confirmed 前缀 parent 的面积累加（已做），frontier parent 的面积累加**每 bar 必重算**（frontier
可变），其 O(range) 成本随窗口增长 = 残余 O(n²)。

**这正是 B3（task #4）的 area-memo 领域**：B3 NO-SHIP 负结果的**翻转条件**明确写「extract_second 拿到
frontier 门控后 area 上位残余热点」+「正解=冻结历史 area 缓存 `(start,end)→f64` 单调增 + bit-exact
单测（禁前缀和差分，浮点累加序）」。本工位**已兑现 B3 的 frontier 门控翻转条件**，area-memo 是下一
正交靶（两者叠加才能把 07b 完全线性化；本工位不越界接入 area-memo，属 B3 #4 owner 域）。

- 门控 vs 原地增量 cached_bsp 版实测同噪声（1026 vs 1022ms）⟹ 缓存 clone **非**瓶颈（area 累加主导）；
  故未保留原地增量 cached_bsp（ponytail：clone 消除的收益被 area-memo 前置，属 area-memo 落地后的靶）。

## 护航（全绿）

| 套件 | 结果 |
|------|------|
| classifier lib（含 signal digest + 门控 debug_assert 路径）| 232 passed |
| bit_exact battery | 54 passed |
| GOLDEN digest guard（07a，未触 signal.rs ⟹ 不变）| passed |
| extract_signals_bit_exact_vs_orig_per_case | passed |
| a3_oracle ×2 | passed |
| backtest::incremental（bit_exact_per_bar 等，incremental==full 含 B2）| 4 passed |
| backtest::econ_positive（并发域，未破坏）| 31 passed |
| 全 lib 套件 | 1394 passed / 0 failed |
| **07b 门控 bit-exact 验收（新增集成测试，debug，BTC 30K bar 逐 bar debug_assert）** | passed（123514 第二类端点，门控路径重度 exercise）|

信号集逐字不变：07b 是 Type2 提取，`incremental_tower_preserves_b2_second_buy` + 集成验收（30K bar
门控 debug_assert 逐调用对拍 == 全量重扫）双锁 B2/S2 逐字段不变。

**验证路径旁注**：`--lib` test 二进制在本工位执行中一度因并发域 #40 的 `RMove::Compose.subs`→`Rc<Vec>`
改动破坏 cand_predicate.rs/econ_positive.rs 构造子而不编译；本工位新增的 `tests/theta_v0_07b_gating.rs`
是**独立 crate 集成测试**（不编译 lib 的 cfg(test) 模块），故门控验收与计时独立于 #40 WIP 可跑。#40
完成后 `--lib` 恢复编译，全套 1394 绿已复核。

## 结果包六要素

1. **结论**：07b frontier 门控 sound 实装（bit-exact，1394 lib + 集成验收全绿），CL 1M 07b 3990→1026ms
   （74%↓）。门控消除「confirmed 前缀 parent 每 bar 重复背驰扫描」O(U²) 源。**残余 O(n²)（指数 2.03）
   仍在**——根因 frontier parent 的 `segments_diverge` MACD 面积累加 O(range) 随窗口增长，属 B3 #4
   area-memo 领域；本工位已兑现 B3 的 frontier 门控翻转条件，area-memo 为正交下一靶。
2. **定义依据**：B2/S2 = `Origin.RMoveCompose.SecondTypeStructure` + `IsType2`（买卖点定律一 §10.2，
   第14/15课）；门控正确性依据 anc.pdf §16（confirmed 前缀 immutable）+ B2 纯函数于 parent（本工位
   `second_for_parent` 结构证明：只读 parent.rmove/sub_moves + 全局 hist/close_src）。
3. **边界条件**：门控 bit-exact 在「confirmed 前缀 parent 跨 bar immutable（§16）∧ hist 前缀 append-only
   稳定（MacdState::append bit-exact）」下成立——单调性违反（`cached_count>prefix_count`）触发保守全量
   重置（守卫已实装）；cascade_reset 触发前缀缓存 clear。若 §16 immutable 前提被破（frontier 定义变），
   debug_assert 逐调用 panic 捕获。
4. **下游推论**：(a) B3 #4 area-memo 翻转条件已满足（frontier 门控落地），area-memo 落地后 07b 完全
   线性化；(b) 05_compose（#40 后 2.2s@1M）+ 07b（1.0s）+ 07a（1.2s）为剩余三靶，07b 残余属 area-memo；
   (c) 信号集不变 ⟹ 不影响 W-VERIFY(#13) alpha。
5. **谱系引用**：A3（frontier 证书 truncate+extend，`a3-impl-codex-audit-20260702.md`）；B3（07b 定位 +
   area-memo NO-SHIP 翻转条件，`b3-skip-negative-result-20260702.md`）；231号（L0-L3 认识论：门控/计时
   均 L1，非行情有效断言）；090号（严格性——残余 O(n²) 照实入册不声明膨胀）；275号（局部依赖：area-memo
   为 B3 owner 域，本工位不越界接入）。
6. **影响声明**：改 `mod.rs`（LevelCache +2 字段 / cascade clear +2 行 / 新增 `second_for_parent` +
   `extract_second_resume` / miss 路径改调门控）；新增集成测试 `tests/theta_v0_07b_gating.rs`（门控
   bit-exact 验收 + CL 计时 profile）。未碰 signal.rs（extract_second_signals 逻辑不变）、econ_positive.rs/
   strategy/closed_loop（并发域）。改动 Bash/工具落盘后独立读回验证 + 全套测试复核。
