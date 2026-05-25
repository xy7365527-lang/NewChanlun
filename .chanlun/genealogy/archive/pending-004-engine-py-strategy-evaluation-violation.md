---
id: pending-004-engine-py-strategy-evaluation-violation
timestamp: 2026-04-27
status: 已结算
settlement: 修正（严格扬弃，已执行 + 111测试验证GREEN）
settled_by: '295'
settled_date: 2026-05-24
settled_classification: 定理
settlement_scope: "删 BacktestResult 的 win_rate/profit_loss_ratio/max_drawdown_pct/win_count/loss_count + CostSummary.cost_to_gross_profit_ratio + 全部消费方(batch_backtest.py + 5测试文件)"
type: domain
negation_source: homogeneous
negation_form: expansion
topo_effect: "revise:src/newchan/backtest/engine.py:local（原标 split，扬弃后为 revise）"
---

## 结算（2026-05-24，定理类——严格扬弃已执行 + 验证）

**决断四分法：定理**（五重权威一致否定 win_rate 等绩效指标——SKILL§2.2/220号/295号第四层/蓝图第一原则/缠师；无价值判断）。按系统"定理→自动结算/执行，不提问"直接执行（不上浮）。**结晶：修正（严格扬弃）。**

**消费图（执行前完整测绘，含 2 处初遗漏）：**
- `engine.py`：BacktestResult 5 属性 + CostSummary.cost_to_gross_profit_ratio（定义处）
- `scripts/batch_backtest.py`：`backtest_result_to_row`(L80) CSV 列 + 主流程 print(L135)
- 测试：`test_pipeline_backtest.py`、`test_batch_backtest.py`、`test_pipeline_backtest_e2e.py`、`test_cost_integration.py`、`test_backtest/test_backtest_engine.py`（后者初次 grep 漏扫，全套测试运行后捕获）
- **排除误匹配**：`tightness_selection_l2.py` 的 win_rate 是局部假设检验变量（Wilcoxon，350号推论4），非 BacktestResult 消费——非违规。

**执行的扬弃：**
1. engine.py 删 win_count/loss_count/win_rate/profit_loss_ratio/max_drawdown_pct 属性；CostSummary 删 cost_to_gross_profit_ratio 字段 + gross_profit 计算。保留 trade_count/trades/total_bars + 纯成本（total_slippage/commission/cost）= L1 管线事实。
2. batch_backtest.py：row 函数 + print 降为 trade_count + total_cost（batch 报告从"跨标的策略表现排名"降为 L1 成本/交易计数报告）。
3. 5 测试文件：删策略评价断言，保留代码正确性断言；删 test_max_drawdown + test_profit_loss_ratio 整方法；test_win_rate_and_ratio 改名 test_two_trades_generated。

**验证（verification-before-completion）**：`pytest tests/test_backtest/ + 4 文件 = 111 passed`；grep 无残留真消费；import engine/gateway/batch_backtest 干净。**非补丁**（无 `# TODO: deprecate`，符合 no-patch-mentality）。

**下游推论**：batch 回测工具不再产策略排名——若将来需要跨标的比较，只能比 L1 管线度量（降成本速度/收敛排序/机会成本，267号推论1），不可比胜率/盈亏比。子包重命名（pipeline_replay）W-7 建议为可选，本次未做（名实问题，非违规，留作低优先 cleanup）。

**影响声明**：改 `engine.py`、`scripts/batch_backtest.py` + 5 测试文件。本节点由生成态转已结算（修正/扬弃）。

---

# backtest/engine.py 违规属性——win_rate/profit_loss_ratio/max_drawdown

## 矛盾

W-7 自标：`src/newchan/backtest/engine.py` 第 184/190/205 行：
- `win_rate`（胜率）
- `profit_loss_ratio`（盈亏比）
- `max_drawdown_pct`（最大回撤）

这是**测策略好不好**的指标，违反多重权威：
- SKILL.md § 2.2 判断标准："测策略好不好 → 禁止；测代码对不对 → 允许"
- 220号：风控不是独立度量
- 295号下游推论2：回测验证前三层（地图/语法/方法），不验证第四层（决断质量）
- 蓝图 v3 第一原则：不回测策略

## 否定了什么

否定的是"已通过测试覆盖率检查 = 工程合规"的假设。W-7 的 backtest 子包 9 个文件其中 8 个合规（types/k4_config/scanner/state_machine/analysis/orchestrator/full_pipeline 都是 L1 管线度量），但 `engine.py:BacktestResult` 把交易记录用胜率/盈亏比聚合 = 隐式声明"这个策略是好策略" = 295号第四层评价的误用。

## 推导链

- 缠师原文（220）→ 风控不是独立度量
- 267号下游推论1 → 回测应验证三项（降成本速度、收敛排序效果、机会成本）——L1 管线度量
- 269号 → 编排者裁定"理论穷尽下一步是跑数据"——回测合法
- 295号 → 第四层（决断）= objet petit a 不可形式化
- SKILL.md § 2.2 → 显式分类规则
- 五重权威源（缠师/谱系/编排者/SKILL/蓝图）一致——违规事实清晰

## 谱系链接

- 220号、267号、295号、338号、349号（W-7 主链）
- 016号（规则没有代码强制就不会执行）
- 090号（严格性语法规则，no-patch-mentality）
- 137号（声明层无效，需结构层强制）

## 影响声明

- 影响：`src/newchan/backtest/engine.py`（行 175-219 删除）+ `src/newchan/trading/pipeline_backtest.py`（消费传播）+ 子包重命名（建议 `pipeline_replay` 或 `algorithm_validation`）
- 处理方向（W-7 已给出严格扬弃方案）：
  1. 删除 BacktestResult 的违规属性，保留 Trade 单笔事实记录
  2. cost_summary 保留，移除 cost_to_gross_profit_ratio
  3. 子包重命名（可选但推荐——名实一致）
  4. 同步处理 pipeline_backtest.py
- 不允许补丁式（在 BacktestResult 上加 `# TODO: deprecate`）——no-patch-mentality

## 338 修正4 工程化遗漏（次级矛盾）

W-7 同时识别：338 修正4 要求"次级别买点价格 >= 前次卖出价格 → 不执行加回"，`cost_reduction_fsm.py:_close_short_diff` 未实现该规则。这是 268a 异质质询接受后的工程化遗漏（016号实例）。

## 异质审计降级

本 session 全程 gemini-challenger 不可用。W-7 自标"上游可考虑配置 genai SDK 后对 backtest 裁定段落做 Round-1 异质审讯（重点攻击 engine.py 重构方案）"。
