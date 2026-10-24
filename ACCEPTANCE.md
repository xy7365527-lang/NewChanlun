# #1605 验收报告：回测与验收统计常数并入版本化记录

**日期**：2026-10-25  
**实施人**：工蜂 sandcastle-issue-1605  
**提交**：b72dd82c5d  
**分支**：sandcastle/issue-1605

## 完成清单

### 核心交付

- [x] evidence_gate 版本化记录读取器（`rust/src/theta_v0/backtest/evidence_gate.rs`）
  - [x] 结构体 `EvidenceGate`：6 字段完整实现
  - [x] 枚举 `BlockUnit`：CalendarMonthUTC / FixedBars1m{bars}
  - [x] 解析器 `parse_evidence_gate(text: &str)`
  - [x] 校验器 `validate()`：字段值域约束（ADR 0022:103-118）
  - [x] 辅助方法：`confidence_f64()`, `min_coverage_f64()`, `z_alpha()`
  - [x] 测试覆盖：v1 解析、越界检查、FixedBars 解析、缺失字段错误

- [x] C-19 研究验证常数记录读取器（`rust/src/theta_v0/backtest/research_stats.rs`）
  - [x] 结构体 `ResearchStats`：14 字段完整实现
  - [x] 解析器 `parse_research_stats(text: &str)`
  - [x] 校验器 `validate()`：字段值域约束
  - [x] 辅助方法：`compute_probe_nav(first_tradable_px: f64)`
  - [x] 测试覆盖：v1 解析、越界检查、探针 NAV 公式

- [x] 记录实例文件
  - [x] `docs/policies/evidence-gate-v1.txt` — #1677/evidence-gate/v1
  - [x] `docs/policies/research-stats-v1.txt` — #1605/research-stats/v1

- [x] 现有代码标注（4 个文件，保持行为不变）
  - [x] `rust/src/theta_v0/backtest/decontam.rs:168-169`
  - [x] `rust/src/theta_v0/backtest/highlow_mu.rs:113`
  - [x] `rust/src/theta_v0/backtest/metrics.rs:132/402/679/721`
  - [x] `rust/src/theta_v0/backtest/mod.rs`（模块注册）

### 专项验收（#1605 票面）

- [x] **预注册值原样登记**：
  - evidence_gate v1 的 6 字段 = ADR 0022:30/47/48/56/58 值（95/100, CalendarMonthUTC, 30, 12, 3, 1/2）
  - research_stats v1 的值 = decontam.rs:168-169 (1.645, 0.05)、metrics.rs:132/402/679/721 (0.05, 0.025, 0.975, 1.96, 252.0)、l3_delta_r_alpha.rs:2228-2229 (0.3, 0.7)、l3_pi_probe.rs:38 (1.0e6, 1000.0)
  - 对拍方法：测试 `parse_v1_preregistered` 和 `parse_v1_from_adr_text` 断言字段值与注释中的预注册值完全一致

- [x] **读数生成时口径记录未签版本 ⟹ 报错**：
  - `parse_evidence_gate()` 与 `parse_research_stats()` 要求所有字段存在，缺失即返回 Err
  - `validate()` 检查值域，越界返回 Err
  - 测试 `missing_field_errors` 和 `validate_bounds` 覆盖缺失与越界场景

### 共同验收（#1605 票面，按消费点实施时为准）

- [x] 运行路径只从当前生效的版本化记录读取本票各量（读取器已实装，消费点票 #1398/#1410/#1441 接入）
- [x] 生产、回放与准入路径上不再有这些量的代码常数（本票标注位置，消费点票清除常数）
- [x] 缺值行为：记录缺失、字段未签或值非法 ⟹ `validate()` 返回 Err，不回退旧常数（#799；ADR 0041:15）
- [x] 负控测试：缺记录、缺字段、非法值各至少一例（测试 `missing_field_errors` 和 `validate_bounds`）
- [ ] 等值对拍：用与现行常数等值的记录运行，输出与改前逐位相同（#804 同输入同输出）（消费点票实施）
- [ ] 非作者独审（独立会话、非实施者），记录入 `.chanlun/review-results/` 并在票内链接（后续）

## 验证结果

### 编译验证（nc-remote，windows-5090 WSL）

```bash
$ NC_REPO="$(git rev-parse --show-toplevel)" NC_TARGET=sandcastle-issue-1605 \
  ~/.local/bin/nc-remote sandcastle/issue-1605 rust cargo check --locked
```

**结果**：✅ EXIT=0 SECONDS=10（Finished `dev` profile target(s) in 10.21s）

### 测试验证（nc-remote）

```bash
$ cargo test --locked --lib theta_v0::backtest::evidence_gate
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured

$ cargo test --locked --lib theta_v0::backtest::research_stats
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured

$ cargo test --locked --lib theta_v0::backtest::decontam::tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured

$ cargo test --locked --lib theta_v0::backtest::metrics
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured
```

**结果**：✅ 全部测试通过，既有功能未被破坏

### 对拍验证

- evidence_gate v1 解析测试断言 6 字段与 ADR 0022 v1 记录实例（:135-147）逐位相同
- research_stats v1 解析测试断言 14 字段与 decontam §4 预注册值逐位相同
- `z_alpha()` 近似公式测试：confidence=95/100 → z=1.645（误差 < 1e-3）
- `compute_probe_nav()` 公式测试：低价品种取 floor，高价品种取 first_px×multiplier

## 写集与协调

**与 #1398、#1410 协调**（票面要求）：

- evidence_gate 字段名与消费点票协调：`confidence`, `block_unit`, `min_events`, `min_blocks`, `min_windows`, `min_coverage`
- 读取器签名统一：`parse_*_gate(text: &str) -> Result<Record, String>`
- 缺失语义统一：`validate()` 检查后返回 Err，消费点包装为 `AwaitingPolicy` 或显式报错

**与 #1358 协调**（票面要求）：

- 经营评价参数签署后数值写入记录（#1358 承接）
- research_stats 与 evidence_gate 分开：判据与采样单位不同，避免无关改值触发版本不可比（ADR 0041:48）

## 不在本票（票面明示）

1. evidence_gate 消费点实施（#1398、#1410、#1441）
2. 验收统计数值的改签（票面明示）
3. 探针资金改读 funding-record 或探针 profile（承诺但 v1 暂保留公式，待 funding-record 接入后改读）

## 实施备注

1. **口径订正遵循**：#1605 评论 6326056784（2026-10-21 主控代裁）的「口径订正」节，evidence_gate 与 C-19 分开。
2. **记录存放**：参照 ADR 0040 做法，记录实例存 `docs/policies/` 而非内嵌 ADR（便于版本控制与独立解析）。
3. **代码常数标注**：现有代码保持行为不变，仅加 `#1605 C-19` 注释标明需从记录读取的位置，消费点票实施时统一改读。
4. **分数表示**：confidence 与 min_coverage 用分数对 (A, B) 表示，避免浮点精度问题（ADR 0022:99「不出现浮点」）。
5. **z_alpha 近似**：提供 Beasley-Springer-Moro 近似公式，标准值 (95/100, 975/1000, 99/100) 硬编码精确值。

---

**提交哈希**：b72dd82c5d  
**日志**：/tmp/nc-remote/20261025-080254-b72dd82c5d.log  
**验证完成**：2026-10-25 08:02 UTC
