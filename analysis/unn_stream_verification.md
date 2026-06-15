# unn 引擎流式化验证报告（535号边界B）

> 任务：把 unn 引擎从批量 tape 消费改为逐 bar 流式接口，PyO3 导出给 NautilusTrader 实时驱动。
> 编排者裁决（2026-06-14）：push_bar 收 **BarSig**（边界B），非原始 OHLC。

## 结论

unn 引擎已流式化。三层逐 bar 步进，每层与各自批量路径**共享同一段循环体代码**：

| 层 | 批量 | 流式 | 共享单元 |
|----|------|------|---------|
| 信号层 | `compute_organic_signals`（for 循环） | `StreamingSignalReader.process_bar` | `process_bar`（薄封装 = for 循环调它） |
| 引擎层 | `run_unified_necessity`（for 循环） | `UnnStreamCore::step` + `finish` | `step`（批量入口 = for 循环调它） |
| PyO3 | `run_positional_rust(mode="unn")` | `UnnStream.push_bar` / `finish` | `positional_result_to_dict` |

NautilusTrader BacktestEngine 逐 bar 回放 → `on_bar`(process_bar → push_bar) → `on_stop`(finish)。

## 定义依据

- **unn 引擎定义域 = SignalTape（BarSig 序列）**，非 OHLC（`positional.rs` 入口
  `run_unified_necessity(tape, floor_ladder)`）。缠论引擎（`RecursiveOrchestrator`）是
  unn 的**上游独立模块**，早有流式接口 `process_bar`——故 push_bar 收 BarSig 是 unn
  的自然流式边界（编排者裁决）。
- **bit-exact 是构造性保证，非对齐努力**（no-patch.md）：批量与流式调同一个 `step`/
  `process_bar`，差异仅在外部驱动方式。bit-exact 由两个因果性质推出：
  - 信号层 **append-only 前缀冻结**：`i_signals.append` 永不回填，所有 seen-set 单调，
    epoch/cache-diff 门控只读"自上次以来的新增"⇒ BarSignalI[i] 在第 i bar 后永久冻结。
  - 引擎层 **纯因果无前视**：`step` 只读 `tape.bars[i]` + 因果累积状态
    （voices/free/book/depth_ref/nest/located/flip_ptr）。
- **8 条必然性 prove（N1-N8）在流式 step 内每 bar 检查**（`unified_necessity.rs`
  prove_n1..n8 + prove_chain），violation = panic = 验收标准（编排者裁决）。

## 边界条件（结论翻转条件）

1. **若信号层引入回填**（某 BarSignalI 被未来 bar 修改）⇒ 流式前缀冻结破坏 ⇒ bit-exact
   失败。当前架构保证不回填（差分守卫 + settled 单调，organic_signals.py docstring）。
2. **若 unn 循环体引入前视**（读 `tape.bars[j]`, j>i）⇒ 流式无法提供 ⇒ 失败。当前
   `step` 零前视（已审计）。
3. **eod 清算（finish）不属任何 bar**：finish 的 eod 关根 + cascade 子树（exit_bar=末根
   bar），不在 push_bar 吐单中——若把它们计入逐 bar 信号即范畴错误。

## 验证结果（认识论等级标注，formalization-validity-domain.md）

| 验证 | 等级 | 结果 |
|------|------|------|
| Rust 单测（unn N1-N8）| L1（管线）| 9/9 PASS（批量路径走 UnnStreamCore 零漂移）|
| Rust 全量单测 | L1 | 363/363 PASS（dict 提取零回归）|
| `verify_unn_stream` CL 200k | L1（跨 marshal）| 110 键 + equity + final_nav 全 == |
| `verify_unn_stream` **CL 全量 5.5M** | **L2**（真实数据）| ✅ 110 键全等，无 panic |
| `verify_unn_stream` **ES 全量 5.59M** | **L2** | ✅ 110 键全等，无 panic |
| `backtest_unn_stream` NT 流式 CL 200k | L1（端到端）| bit-exact PASS（8 analyze 键）|

**L2 含义**：CL+ES 共 11.1M bars 真实数据，流式逐 bar 驱动产出与批量逐位相同，
8 个 prove 函数零 panic ⇒ N1-N8 在流式驱动下同样成立。L1→L2 信息增量：流式接口
正确性从"合成单测"扩到"真实全量数据"有效域。

## 下游推论

- NautilusTrader 实时驱动 unn 引擎已具备（每 bar push_bar 返回交易信号 + snapshot 查询）。
- `UnnStream` 是 unn 唯一新增 PyO3 流式接口；批量 `run_positional_rust(mode="unn")` 路径
  与流式共享 `step`/`finish` ⇒ 二者永久同步（改 step 同时改两条路径，无分叉风险）。
- production `ChanlunStrategy` **未改**：它走独立 `ChanlunBridge` 信号路径（阶段1），与
  organic_signals/unn 是两套信号架构。接入需 ChanlunBridge 产出 unn 兼容磁带——独立任务。

## 谱系引用

- 540号（压缩→展开时序）：`step` 内 `Pending.since_bar` + `compress≤confirm≤bar` 时序
  断言，流式逐 bar 同样检查（N6）。
- 边界B 裁决：push_bar 收 BarSig 而非 OHLC——避免把"信号层 Rust 化"（独立大任务）
  混入"unn 流式化"（范畴混淆，formalization-validity-domain）。
- 不确定是否有"流式 vs 批量"专属谱系；本验证未触及定义分离，是引擎接口范畴的工程
  重构（共享循环体）+ 边界裁决。

## 影响声明

- **改**：`rust/src/trading/unified_necessity.rs`（提取 `UnnStreamCore` struct + step/
  finish/snapshot/accessors；`run_unified_necessity` 重写为共享 step 的批量封装）；
  `rust/src/lib.rs`（导出 `UnnStream` pyclass；提取 `positional_result_to_dict` 复用）；
  `analysis/organic_signals.py`（提取 `StreamingSignalReader`，`compute_organic_signals`
  改薄封装）。
- **新增**：`analysis/verify_unn_stream.py`（Rust 流式≡批量 bit-exact 验证）；
  `trading_system/backtest_unn_stream.py`（NT 真流式回测 + bit-exact 对账）。
- **未改**：缠论引擎核心（RecursiveOrchestrator）；unn 会计/必然性逻辑（step 逐字搬运）；
  其他 mode 路径；production ChanlunStrategy。
- **零漂移证明**：批量 unn 结果在重构前后逐位不变（363 Rust 单测 + CL/ES 全量 verify）。
