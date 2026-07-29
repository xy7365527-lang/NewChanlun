# issue #342 — NT 引擎 >40k 陡增区 profiling：根因定位（推翻 #333 记账成本假说）

- 工位：隔离 worktree `/tmp/kimi-nest-p342`（detach HEAD=f2fd686aa1 起；用毕已清理）。
- 票：#342（#333 逃逸挂账，NT 全窗量化前提）。
- 前置事实：探针源码随 #333 隔离 worktree 销毁，本票按 #333 报告 §0 描述重建等价探针
  `nt_scaling_probe`（直调与 `theta_backtest.rs:144-150` 逐字相同参数 + `entry_delay_bars=0`，
  `--features backtest_bin` 编译；`slice_bar_range(0, N)` 截窗）。

## 0. 结论先行

**根因不在 Nautilus 引擎本体，也不是持仓/订单簿记账成本（#333 假说已否证）。**
根因 = 本仓库自己的 Nautilus 策略适配层 `theta_v0::nautilus::strategy::ThetaCore::recognize_current`
（`rust/src/theta_v0/nautilus/strategy.rs:242-247`）在**每根 bar** 上对**全部累积历史**重新跑一遍
`parser::parse_layer` + `classifier::classify_with_tower`（批量、非增量），而不是复用本仓库已有、
已 bit-exact 验证的 `IncrementalClassifier`（`rust/src/theta_v0/backtest/incremental.rs`，CLI 生产
路径 `run_theta_v0_pi_inner` 正在用）。单 bar 代价 O(当前长度)，n bar 回测总代价 O(n²)。

## 1. 计数器隔离：先排除"记账成本"假说

`nt_scaling_probe`（`slice_bar_range` 截窗，OKLO，bypass_logging=true 排除日志 I/O 混杂）：

| n_bars | wall_secs | total_orders | total_positions |
|---|---|---|---|
| 2,000 | 0.265 | 2,485 | 233 |
| 5,000 | 4.808 | 11,696 | 819 |
| 10,000 | 5.723 | **11,696**（不变） | **819**（不变） |
| 20,000 | 11.775 | **11,696** | **819** |
| 40,000 | 41.259 | **11,696** | **819** |

**订单/持仓数在 5,000 bar 后完全不再增长**（策略在这之后不再产生新决策——OKLO 该窗后续数据未触发
新缠论结构），但 wall time 从 5k→40k 仍从 4.8s 涨到 41.3s（×8.6）。若成本正比于持仓/订单累积
（#333 假说），5k 之后 total_orders/positions 平台期内成本应趋于常数——**观测与该假说矛盾**，
可排除"持仓/订单簿记账"作为根因。

数值与 #333 报告的旧曲线（含日志，2k=0.33/5k=5/10k=6/20k=12/40k=50）逐点吻合（差值即
bypass_logging 省下的日志格式化开销，量级一致），确认探针重建等价、复现口径正确。

## 2. 采样 profiling：热点分布在 theta_v0 全链路，非某个"记账"函数

`/usr/bin/sample`（macOS 内置采样 profiler）对 40k 窗口跑中进程采 6 秒，1ms 间隔，按栈顶函数聚合
（`newchan_rust::theta_v0::*` 命中排序前 12，去除内核 idle 帧）：

```
26  classifier::descend::RMove::hi
24  classifier::classify_impl
22  classifier::descend::RMove::lo
13  strategy::interp::coverage_elements_and_gamma_with_tower_cached_gen
13  classifier::signal::extract_signals_with_hist_anchored
12  parser::feature_seq::second_seq_has_fractal
12  nautilus::strategy::ThetaCore::plan_for_bar
12  classifier::cached_segment_area
11  strategy::coverage::push_element_tree
11  classifier::signal::judge_segment
10  classifier::recursive_tower::full_trend_qualification_evidence
 9  strategy::recognize_nested
```

热点**分散在整条 parse→classify→recognize 管线**（RMove 分型识别、classify_impl 分类主循环、
线段/中枢特征序列扫描、递归塔资格判定……），不集中在任何"持仓/订单簿"相关函数——这与"记账成本"
假说预测的热点分布（应集中在 portfolio/position/order 相关代码）不符，但与"每 bar 全量重跑
parse+classify"的预测完全吻合：栈顶命中就是这条管线本身，因为它确实在被反复整条重跑。

## 3. 代码级证据：批量重跑 vs 生产 CLI 的增量链

`rust/src/theta_v0/nautilus/strategy.rs:242-247`：

```rust
fn recognize_current(&self) -> Vec<VoiceDecision> {
    let l0 = parser::parse_layer(&self.bars, &self.config);          // 全量重解析，O(len)
    let (classification, tower) = classifier::classify_with_tower(&l0, &self.config); // 全量重分类
    ...
}
```

`plan_for_bar`（同文件 125-126 行）每根新 bar 调一次 `recognize_current`，`self.bars` 逐 bar
`push`（累积、不截断）。`parser::parse_layer(bars: &[Bar], ..)` 签名（`parser/mod.rs:143`）
本身不带增量状态——每次调用都是从头扫描传入的整个切片。第 i 根 bar 的单次调用代价 ∝ i，
n 根 bar 总代价 = Σᵢ₌₁ⁿ i ∝ n²。

对照生产 CLI 路径 `rust/src/theta_v0/backtest/runner.rs:504-514`（`run_theta_v0_pi_inner`）：

```rust
let mut classifier_incr = super::incremental::IncrementalClassifier::new(bars, &config);
...
|i| { let (cls, tower) = classifier_incr.classify_at_with_l0(i); ... }
```

`IncrementalClassifier`（`incremental.rs:59-95`）内部走真增量链——`ParseLayerIncr::append`
（O(1)/bar 摊还）+ `classify_with_tower_incremental`（`TowerCache` 跨 bar 复用）——runner.rs:498
注释明示："**bit-exact 不变**：增量链 == 全量 `classify_with_tower(parse_layer(..=i))`"，并有专门
测试族 `incremental::bit_exact_*` 锁定该等价性（本次 `cargo test --release --lib` 中
`theta_v0::backtest::incremental::tests::*`、`theta_v0::classifier::tests::incremental_tower_scaling_dominates_full_synthetic`
等均 PASS，见 §5）。

**结论**：CLI 生产路径早已用增量链解决了同一个"因果前缀重分类"需求（`runner.rs:481-483` 注释：
"fill loop 每 bar i 经 `IncrementalClassifier::classify_at(i)` 得因果塔+因果分类——只用 ≤i 数据"），
但 Nautilus 策略适配层（`ThetaStrategy`/`ThetaCore`，goal acceptance[5] ③ 的新增代码）绕开了这条
已验证的增量基础设施，重新手写了一份 O(n²) 的朴素批量版本。这不是 Nautilus 引擎的限制，是本仓库
自己两条平行实现之一忘了接现成的优化件。

## 4. 为何未在同一票内直接修

`IncrementalClassifier<'a>` 的签名是 `new(bars: &'a [Bar], config: &'a ThetaConfig)`——它假设
调用方持有一个**生命周期内不再变化的完整切片**（批量回测场景：`dataset.bars` 一次性加载，长度
固定，`classify_at(i)` 只读前缀 `[0..=i]`，`ParseLayerIncr::append` 复用的是"切片不重分配"这个前提）。

Nautilus 流式场景（`ThetaCore::on_bar`）里 `self.bars: Vec<Bar>` 逐 bar `push`，`Vec` 增长可能
触发重新分配，任何指向旧内存的 `&'a [Bar]` 借用都会失效——`IncrementalClassifier` 当前的 API
形状无法直接嵌入一个"边接收边增长"的宿主结构体，需要新增一个**自持缓冲区、暴露
`append_bar()`（而非要求预先给整段切片）的增量分类器变体**，并对照现有 `incremental::bit_exact_*`
测试族补一套"流式增量 vs 全量批量"的等价性测试，才能安全替换 `recognize_current`。

no-patch-mentality 明令禁止"先做一半"——引入新 API 形状、跑通 bit-exact 等价性验证、覆盖
`ThetaStrategy` 的行为不回归，这是一次完整的 TDD 周期，不是本票"profiling + 定位根因"范围内能
安全收口的改动。故本票**不动生产代码**，如实交付根因定位 + 规避口径，修复动作另开票
（跟踪：NT 流式适配器接入 IncrementalClassifier 的 append 变体）。

## 5. 规避口径：窗口预算表（照实，未变，机制已明）

沿用 #333 §1 曲线（本票 §1 用 bypass_logging 复测确认无日志混杂）：

| bars | 2,000 | 5,000 | 10,000 | 20,000 | 40,000 | 60,000 | 100,000 | 全窗(343,282) |
|---|---|---|---|---|---|---|---|---|
| wall（含日志） | 0.33s | 5s | 6s | 12s | 50s | 143s | >420s(杀) | >42min(杀) |

400s 预算内可行窗口 = **60k**（两品种 OKLO/BTC 均在案，见 #333 §2）。60k→100k 的外推（O(n²)
机制下 (100/60)²≈2.8×，143s×2.8≈400s，与实测"100k 在 420s 超时线附近被杀"量级吻合，反向印证
本票 O(n²) 机制成立）。

## 6. cargo test --release --lib 基线

`1867 passed; 1 failed; 132 ignored`——唯一失败 `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`，
与 #110 在案失败一致，本票未新增/未修改任何生产代码（只加了一个探针 bin `nt_scaling_probe.rs` +
一条 `[[bin]]` Cargo.toml 项，均在隔离 worktree 内，未合入），基线不破。

## 7. 结果包六要素

1. **结论**：#342 陡增根因 = `ThetaCore::recognize_current`（`nautilus/strategy.rs:242-247`）
   每 bar 批量重跑 `parse_layer+classify_with_tower`，O(n²)；非 Nautilus 引擎、非持仓/订单簿记账。
   本票不改生产代码（no-patch 完整性要求，见 §4），修复另开票。
2. **定义依据**：`incremental.rs:59-95` `IncrementalClassifier` + `runner.rs:481-500` 注释（生产
   CLI 路径的增量链定义与 bit-exact 等价性声明）；`nautilus/strategy.rs:242-247` 的批量调用对照。
3. **边界条件**：若未来 `IncrementalClassifier` 获得 owned-buffer/`append_bar` 变体并通过
   bit-exact 等价性测试，本结论中"修复需另开票"的部分失效——届时 NT 全窗量化不再受 60k 窗口预算
   约束。若后续 profiling 在其它数据集/品种上发现热点分布不同（如 BTC 因结构更密集在低 n 即触发
   O(n²) 主导），窗口预算表数字需重测（机制结论不变）。
4. **下游推论**：#333 未决问题（60k 零差异 vs CLI 全窗非零）**仍未解决**——本票只解释了"为什么
   NT 段跑不到差异区"，不改变"差异区在哪"这一问题；60k 窗口预算是本票给出的当前上限，NT 全窗
   量化仍需等增量化修复票落地。
5. **谱系引用**：延续 #333（wayfinder #333 相切=重合口径量化）的 follow-up 1（"NT 引擎陡增区
   profiling"），本票是其闭环；未发现需要 `/escalate` 的定义冲突（纯性能根因定位，无概念矛盾）。
6. **影响声明**：本票未改动任何生产文件（`theta_v0/nautilus/strategy.rs`、`incremental.rs`、
   `runner.rs` 均只读）；新增文件仅本报告 + 隔离 worktree 内已随 worktree 清理的探针源码
   （`rust/src/bin/nt_scaling_probe.rs` + `Cargo.toml` 一条 `[[bin]]`，未合入主线）。
