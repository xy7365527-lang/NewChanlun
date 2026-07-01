# σ_higher 上级方向态 dump（666 号，ChatGPT alpha 分离路线图 Step4 前置）

**认识论等级：L2**（BTC 单标的真实数据逐信号分解，231 号；可产否定性结果）。

## 结论

在 `econ_positive::SignalDecomp` 上新增 `sigma_higher: i8` 字段（入场确认时的上级方向态），
在 `decompose_capturable_spread` 的信号收集路径实装其派生，并在 `l2_btc_capturable_spread_diagnosis`
测试里新增逐信号台账 CSV dump（15 列含 sigma_higher）+ σ_higher 分布统计。全历史 BTC（461万 bar）
跑出 12626 信号台账。

**σ_higher 分布**（δ = 交易方向，σ_higher = 上级走势方向态）：

| 类别 | 判据 | 计数 | 占比 |
|---|---|---|---|
| 顺上级 | δ == σ_higher | 8411 | 66.6% |
| 逆上级 | δ == −σ_higher | 4212 | 33.4% |
| 无上级 | σ_higher == 0 | 3 | 0.0% |

σ_higher 原始值分布：{+1: 6337, −1: 6286, 0: 3}；δ 分布：{+1: 6567, −1: 6059}。
交叉核对：8411 + 4212 + 3 = 12626 ✓。

per-level 顺/逆比在各级别稳定约 2:1（L0: 7530/3617、L1: 627/432、L2: 196/128、L3: 41/28、
L4: 15/6、L5: 2/1），σ_higher=0 仅出现在 L0 早期 bar（上级 L1 走势尚未形成）与 L5 稀疏点。

## 定义依据

σ_higher 语义 = 缠师递归级别系统的**上级走势方向**（`Origin.RecursiveLevelSystem` + 走势裁决
`Origin.TrendCompleteClassification`，classifier/mod.rs:15-16）。缠论买卖点是反转交易（664 号）：
底背驰买点 δ=+1 出现在下跌段末端，顶背驰卖点 δ=−1 出现在上涨段末端。σ_higher 检验的是该反转
交易方向 δ 与其**所在级别的上级走势方向**是否一致——路线图假设「顺上级 δ=σ_higher 的买卖点」
才有正 α（把奇偶交替伪结构 perm_p=0.69 替换为方向条件）。

**派生实装（可达性约束，不是补丁）**：上级层走势是 `RMove::Compose`（descend.rs:53），**无 direction
字段**；`LevelState.moves: Vec<MoveKind>` 的 `MoveKind::Trend` 也不分 Up/Down。走势裁决
`MoveOutcome::Trend(Direction)`（level.rs:37）内部有方向，但在信号收集作用域不可直接读。故
`sigma_higher_at()` 取上级层 `tower_i[level+1]` 末 `LeveledMove` 的 `start_index/end_index`
（L0 原始 K 序端点，覆盖该走势全跨度）close 净差符号：+1 净涨 / −1 净跌 / 0 持平。这与
Trend(Up) ⟺ 走势端点净涨语义等价，是该作用域的严格可达解（非改符号的同义反复补丁）。

## 边界条件

结论翻转条件：
1. **端点价法 ≠ 走势裁决方向**：若某上级走势的端点净差符号与 `MoveOutcome::Trend(Direction)`
   的裁决方向不一致（如强震荡走势端点持平但裁决为 Trend），则该信号的 σ_higher 被误标。当前
   σ_higher=0 仅 3 例（<0.03%），说明端点持平极罕见，但端点法与裁决法在**震荡段**可能分歧——
   若下游 α 分析要求精确到走势裁决方向，需把 `MoveOutcome::Trend(Direction)` 透传到收集作用域
   （当前架构不支持，需改 classify 输出携带方向）。
2. **level+1 越界**：最高涌现级别的信号无上级层 → σ_higher=0（本窗仅 L5 顶 1 例，可忽略）。

## 下游推论

- 台账 `/tmp/btc_663_ledger_sigma.csv`（12626 行 + 表头，15 列）是路线图 Step4（顺/逆 α 分离）
  的输入。下游可按 sigma_higher 列分组做 μ̂=Σactual_pnl/n 与反事实 permutation 检验（本工位**不做**）。
- 顺上级 66.6% : 逆上级 33.4% 的 2:1 全局比 + 各级别稳定 2:1，提示 δ 与 σ_higher **非独立**
  （买卖点系统性偏向顺上级方向）——这对下游「是否只有顺上级有 α」的检验是先决观测，但**尚未**
  验证顺上级子集的 α 是否为正（那是 Step4）。

## 谱系引用

- 664 号：反转交易腿对象修复（δ = 交易方向 ≠ 触发段笔方向 ε）——σ_higher 建立在 δ 语义之上。
- 666 号：σ_higher 上级方向态实验（本工位）。
- perm_p=0.69 否证：缠论买卖点奇偶交替（买L0+L1−…）= beta 漂移伪影（[[project_oddeven_mu_identity]]）
  ——σ_higher 是 ChatGPT 给出的「第一优先级」翻案条件。
- 231 号 / formalization-validity-domain：本产出标 L2（BTC 单标的），非跨标的 L3。

## 影响声明

改动文件（worktree /private/tmp/sigma-wt）：
- `rust/src/theta_v0/backtest/econ_positive.rs`：
  - `SignalDecomp` 新增 `sigma_higher: i8` 字段 + 诚实注释（删除原「classify_move 派生」声明膨胀注释）。
  - 新增 `sigma_higher_at()` helper（端点价法）。
  - signals 元组加 sigma_higher 分量（收集时算），带到 decomps 构造。
  - `l2_btc_capturable_spread_diagnosis` 测试：新增 σ_higher 分布统计（写入 md 报告）+ 逐信号
    15 列 CSV dump（→ /tmp/btc_663_ledger_sigma.csv，env ECON_LEDGER_CSV 可覆盖路径）。
  - OOS 合成 helper `synth` 补 sigma_higher: 0。
- **只新增旁路观测，不改配对逻辑**（next_long/next_short/退出配对不动）→ bit-exact 未破。

**验证**：
- `cargo test --lib` = 1304 passed / 0 failed / 87 ignored（σ_higher 旁路未破既有测试）。
- 全历史真封 `l2_btc_capturable_spread_diagnosis`（ECON_L2_MAX_BARS=5000000）= 1 passed（811.27s，
  含分解恒等式逐信号真封①-④全过）。
- CSV 12626 行 + 表头；sigma_higher 列 {+1:6337, −1:6286, 0:3} 与报告分布交叉核对一致。

**全局归因（本窗 bars=4613599，2017-08-17→2026-05-31，与 σ_higher 无关，供参照）**：
ΣAb_rev=2.16e6、Σcaptured(adverse-only)=−6.58e5、actual_pnl_proxy=3.21e5（真实成交口径正）、
全级别累积净值终值=3.21e5。σ_higher 为纯旁路观测，未改这些量。

**数据依赖注记**：worktree 无 analysis/data_cache（LFS 未同步，622 号），已 symlink 到主仓库
`analysis/data_cache/btc_1m_full.json`（未拷贝 329MB）。
