# 影子评审 #346 — #345 Nautilus 增量分类（commit 13b91e33a5）

- 评审者：独立新上下文（claude opus，非实装 lineage）；worktree `/tmp/kimi-nest-mainline`，工作区干净（无 wipe）。
- 结论：**无 HIGH，不 reopen**；3×MED + 2×LOW。

## Spec 轴（对照 #346 票体 + nt-engine-scaling-profile-20260726.md）

| 核对点 | 判定 | 证据 |
|---|---|---|
| owned config 破解生命周期不兼容 | PASS | `classifier/streaming.rs:38-49` 零生命周期参数，`append_bar` 吃 owned `Bar`；`ThetaCore.classifier` 可安全嵌入增长中的宿主（`strategy.rs:78`） |
| 「同值拷贝非双源」论证 | **FAIL → MED-3** | `strategy.rs:56` `pub config: ThetaConfig` 可被外部改写，classifier 内拷贝无同步；注释（`strategy.rs:76-77`）称「二者永不分叉」无任何强制 |
| append_incr_layer 抽取完整性（有无第三份） | PASS | `parser/mod.rs:222-294` 唯一步进实现；`ParseLayerIncr::append` 委托之；生产路径 `parse_layer(` 每-bar 残余仅剩测试对照（`strategy.rs:341`），`l3_pi_probe` 为窗口级非每-bar |
| 生产入口唯一性 / 锁步失配 | **FAIL → MED-2** | `strategy.rs:57` `pub bars` 允许绕道；`recognize_current` 首行 `debug_assert_eq!(self.bars.last(), Some(&new_bar))`（`strategy.rs:258-262`）在 `plan_for_bar:132` push 之后**恒真**，无法捕获跳 bar/外部 push。实证：同文件测试 `strategy.rs:433` 手工 `core.bars.push(stop_bar)` 后 `:453` 调 `plan_for_bar`，classifier 从未见 stop_bar（内部 1 根 vs `self.bars` 2 根），断言仍通过、测试仍绿 = 失配被吞 |
| bit-exact 三证可复现 | 部分 FAIL → MED-1 | 见下 |
| 100k 0.47s 口径 | FAIL → LOW-1 | 全仓 grep 无测量载体（探针/bench 未入库）；commit message 未声明组件级还是引擎级。`recognize_nested`（`strategy/mod.rs:704`）成本随累积 `classification.levels`/gamma 增长，`plan_for_bar` 全路径并非纯 O(1)/bar；用组件级 0.47s 对照引擎级 420s 得「≥890×」不可比 |

### MED-1：`owned_bit_exact_synthetic` 合成数据退化

`incremental.rs:154` — `let cycle = ((i as f64) / 50.0).sin() as i64 * 30;`
`as` 优先级高于 `*`，`sin() ∈ [-1,1]` 截断为 0 → **cycle 恒为 0**（独立复算：非零 0/2000）。序列退化为严格单调 `close = 1000 + 2i`，无分型/笔/线段/中枢，2000 bar「逐 bar 全字段 bit-exact」实际只验证了空结构（测试 0.01s 亦佐证）。
对照 `owned_matches_borrowed_variant_synthetic:200`（括号正确，非零 1476/1500）与集成测试 `strategy.rs:333`（正确）——两者有效。
后果：唯一 always-run 的「owned vs legacy 全量」组件级证据空转；vs-legacy 覆盖退化为 80 bar 集成测试 + 传递链（owned==borrowed ∧ borrowed==legacy）。commit message「owned_bit_exact_synthetic(2000) 绿」属声明膨胀（090）。
修法：括号订正为 `(((i as f64)/50.0).sin() * 30.0) as i64`，重跑确认仍绿。

## Standards 轴

| 项 | 判定 | 证据 |
|---|---|---|
| streaming.rs 无条件编译判定 | PASS | `cargo check --release --lib`（无 features、非 test）通过；`nautilus` 无门控（`theta_v0/mod.rs:104`）而 `backtest` 有（`:95`），故类型不能落在 backtest——模块头论证正确。`incremental.rs:129-133` 的 `pub use` 仅供门控内测试引用，无新增 warning |
| debug_assert 锁步护栏 | FAIL | 见 MED-2（恒真谓词，与所声明契约无关；同 #344「门控断言反空转」同类） |
| 克隆-窥视集成测试质量 | PASS | `strategy.rs:327-350` 只经 `plan_for_bar` 驱动，纯函数论证成立（`Rc::make_mut` CoW 语义下克隆重放等价）；`classifier_before.clone()` 二次克隆冗余但无害 |
| incr_* Data Clumps | LOW-2 | `append_incr_layer` 6 参数、4 个 `incr_*` 字段在 `ParseLayerIncr` 与 `OwnedIncrementalClassifier` 两处重复声明。严格形状 = 抽 `IncrParseState` 结构体由两宿主各自持有。判断题，接受未修，但登记为结构债 |

## 复跑

- `cargo test --release --lib`：**1879 passed / 1 failed / 133 ignored**，唯一失败 `extract_signals_bit_exact_digest_guard`（#115 线，未修未归因）。
- `owned_` 定向：2 passed / 1 ignored（`owned_bit_exact_per_bar_real_symbols` 默认 ignore，本轮未跑真实数据）。
- `--features backtest_bin owned_`：2 passed / 1 ignored，门控两侧一致。
- `recognize_current_integration_bit_exact_via_plan_for_bar`：passed。
- `cargo check --release --lib`（无 features）：通过（34 条既有 warning，无新增）。

## 分级

- MED-1 合成数据退化致 bit-exact 三证之一空转（`incremental.rs:154`）
- MED-2 锁步 debug_assert 恒真 + `pub bars` 绕道未封（`strategy.rs:57,258`）
- MED-3 `pub config` 双源无强制，注释声明「永不分叉」（`strategy.rs:56,76`）
- LOW-1 100k 0.47s 无载体 / 口径未声明 / ≥890× 跨口径比值
- LOW-2 incr_* Data Clumps 结构债

无 HIGH：bit-exact 契约仍由「owned==borrowed（1476/1500 有结构）+ borrowed==legacy 既有测试 + 80 bar 集成直证」覆盖；失配路径当前无生产调用方（`nautilus` 仍为骨架，未接 `nautilus_*`）。
