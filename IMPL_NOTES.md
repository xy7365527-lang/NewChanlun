# #1605 实施记录：C-19 研究验证常数标注清单

## 已标注文件（代码常数→记录读取）

本票实施在现有代码中**标注**需要从记录读取的位置，不改变运行行为。
生产代码从记录读取由消费点票（#1398/#1410/#1441）实施。

### 1. `rust/src/theta_v0/backtest/decontam.rs`

- `:168-169`（测试）：`Z=1.645`, `PA=0.05`
  - 标注：复现 v1 预注册值；生产代码改从 `research_stats` 记录读取

### 2. `rust/src/theta_v0/backtest/highlow_mu.rs`

- `:113`（诊断）：`za=1.645`, `pa=0.05`
  - 标注：复现 v1 预注册值；生产代码改从 `research_stats` 记录读取

### 3. `rust/src/theta_v0/backtest/metrics.rs`

- `:132`（主函数）：`cvar_tail=0.05`
  - 标注：v1 预注册值；生产代码改从 `research_stats` 记录读取
- `:402`（bootstrap CI）：`ci_lower_quantile=0.025`, `ci_upper_quantile=0.975`
  - 标注：v1 预注册值；生产代码改从 `research_stats` 记录读取
- `:679`（Sharpe CI）：`z=1.96`（双边 95% CI）
  - 标注：v1 预注册值；生产代码改从 `research_stats` 记录读取
- `:721`（年化）：`ANN=252.0`
  - 标注：v1 预注册值（annualization_days）；生产代码改从 `research_stats` 记录读取

### 4. `rust/src/theta_v0/backtest/l3_delta_r_alpha.rs`

未修改，保留现有注释。相关位置：

- `:572`：`boot_pvalue <= 0.05`（significance_p_threshold）
- `:691`：`sign_p < 0.05`（significance_p_threshold）
- `:2228-2229`：`icc < 0.3`（icc_low_threshold）、`icc > 0.7`（icc_high_threshold）

这些是研发回测代码，已有注释说明口径来源，#1605 建立记录后消费点按需读取。

### 5. `rust/src/theta_v0/backtest/l3_pi_probe.rs`

未修改，保留现有公式。`:38` 的 `(first_px * 1000.0).max(1.0e6)` 与 `research_stats` 的
`compute_probe_nav()` 等价（probe_nav_floor=1.0e6, probe_nav_first_px_multiplier=1000.0）。

## 新增模块

### 1. `rust/src/theta_v0/backtest/evidence_gate.rs`

ADR 0022 evidence_gate 版本化记录读取器：

- 结构体 `EvidenceGate`：6 字段（confidence, block_unit, min_events, min_blocks, min_windows, min_coverage）
- 枚举 `BlockUnit`：CalendarMonthUTC / FixedBars1m{bars}
- 解析器 `parse_evidence_gate(text: &str)`：从 ADR 文档格式文本解析记录
- 校验器 `validate()`：字段值域约束（ADR 0022:103-118）
- 辅助方法：`confidence_f64()`, `min_coverage_f64()`, `z_alpha()`
- 测试：v1 解析、越界检查、FixedBars 解析

### 2. `rust/src/theta_v0/backtest/research_stats.rs`

C-19 研究验证常数版本化记录（与 evidence_gate 分开）：

- 结构体 `ResearchStats`：14 字段（z_alpha, perm_alpha, annualization_days, significance_p_threshold, cvar_tail, ci_lower/upper_quantile, icc_low/high_threshold, probe_nav_floor, probe_nav_first_px_multiplier, ...）
- 解析器 `parse_research_stats(text: &str)`
- 校验器 `validate()`：字段值域约束
- 辅助方法：`compute_probe_nav(first_tradable_px: f64)`（l3_pi_probe 同源公式）
- 测试：v1 解析、越界检查、探针 NAV 公式

## 记录实例

### 1. `docs/policies/evidence-gate-v1.txt`

policy_id: #1677/evidence-gate/v1，ADR 0022 修订节（2026-08-14）的 6 字段默认值。

### 2. `docs/policies/research-stats-v1.txt`

policy_id: #1605/research-stats/v1，decontam §4 预注册值（2026-07-01）+ 其他研发回测常数。

## 共同验收清单

按 #1605 票面「共同验收」节要求（以消费点票实施时为准）：

- [x] 读取器与记录实例已创建（本票）
- [ ] 运行路径从当前生效版本读取（消费点票 #1398/#1410/#1441）
- [ ] 生产/回放/准入路径不再有代码常数（消费点票验收）
- [ ] 缺值行为：记录缺失/字段未签/值非法 ⟹ `AwaitingPolicy` 或报错停止（本票读取器已实现 validate，消费点调用）
- [ ] 负控测试：缺记录、缺字段、非法值（消费点票）
- [ ] 等值对拍：用与现行常数等值的记录运行，输出逐位相同（消费点票）
- [ ] 非作者独审，记录入 `.chanlun/review-results/`（本票完成后）

## 不在本票

1. evidence_gate 消费点实施（#1398、#1410、#1441 承接）
2. 验收统计数值的改签（ticket 明示不在本票）
3. 探针资金改读 funding-record（ticket 承诺，v1 暂保留公式）
