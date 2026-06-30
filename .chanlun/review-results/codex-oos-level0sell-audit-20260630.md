# Codex 异质审计：level0卖 OOS 阳性结果（防口径伪阳性）

**模式**：review（read-only，不改代码）
**日期**：2026-06-30
**对象**：`acc_level0sell_oos` 测试 + `econpositive-oos-level0sell-20260630.md` 阳性报告（663 推论 OOS 验证）
**调用**：`codex exec --skip-git-repo-check --sandbox read-only -c 'mcp_servers={}' -c 'model_reasoning_effort="high"'`（model=gpt-5.5）
**完整交互**：prompt=`/tmp/oos_audit_prompt.txt`，response=`/tmp/oos_audit_out2.txt`

> 注：首次以 `service_tier="fast"` + reasoning xhigh 后台运行被 SIGTERM 杀（exit 144，只回显 prompt 无响应）；改前台 `model_reasoning_effort="high"` + timeout 280 成功（exit 0）。

---

## 被审查的阳性主张

663 推论 OOS 报阳性：从**全样本** per-class 表挑出最强正类 level0卖（level=0,δ=−1，full-sample actual_pnl=+3.47e4），做 train/holdout(0.6) 切分验证：
- holdout level0卖 actual_pnl=+7.87e3>0 → 判「OOS 初步稳健」
- 2 子窗（一涨一跌）卖均优于买 → 判「方向不对称结构性」
- 标题主张：「缠论买卖点真 alpha 首个 OOS 稳健证据」

## Codex 五问逐答

| Q | 判定 | 核心理由 |
|---|------|---------|
| Q1 时间泄漏 | **SOUND** | train/holdout 半开无重叠，classifier 每切片 fresh 重建、因果只见本切片，退出配对不跨切片。cold-start/路径依赖（切窗改变级别涌现）会扭曲早期信号但**不是泄漏** |
| Q2 选择偏差 | **BIAS-FATAL** | level0卖从**含 holdout 的全样本**挑出，再在 holdout 上"验证"——holdout 进入了假设发现环节，逻辑上不再是干净 OOS。低基数缓解 winner's-curse **量级**但不缓解**逻辑污染** |
| Q3 方向不对称 cherry-pick | **INSUFFICIENT** | 2 个 150K 子窗远不足以称"结构性"；n/2 边界任意，两窗几乎无时间稳定性证据。"卖>买"是**弱主张**（买巨亏 −5.7e4/−5.1e4，卖仅轻微正 +1.4e4/+1.3e4）——强主张应是"卖跨多独立窗稳定正" |
| Q4 holdout 涨段论证 | **INSUFFICIENT** | n=128 modest，35% 胜率+正总和**恰好符合"少数大赢家主导"**。需：逐笔 PnL 分布、中位数/截尾均值、top-k 贡献、bootstrap CI、剔除最大 1/3/5 赢家后是否仍正 |
| Q5 单切分脆弱 | **INSUFFICIENT** | 单 60/40 切分**不是 OOS 稳健**。因信号类路径依赖+级别涌现随切片变，改 train_frac→0.5/0.7 可能改变**计数与类身份**（不只是聚合 PnL）。需 train-only 选类 + 多锚定/滚动 walk-forward |

**Codex 最终判定**：标题**不可信（as stated）**，需更多验证。单一最严重风险=**Q2 全样本（含 holdout）挑 level0卖的致命选择偏差**。"可能是真实线索，但不是干净 OOS 证据。"

---

## 工位简化质询（判定 codex 否定是否成立）

1. **Q2 选择偏差真实存在？** 成立。`econ_positive.rs:591-597` 测试 doc + 报告 line 4 明写"对 **in-sample 最强正类** level0卖（in-sample actual_pnl=+3.47e4）做 train/holdout 切分"——挑类用的是全样本 per-class 表，全样本 = 截断窗全部 300K = **含 holdout 区间**。codex 未误读。正确做法（train-only 挑类 → 锁定 holdout 验）未执行。

2. **Q2 是否已被报告自身边界覆盖？** 否。报告"边界条件"（line 42）只提 holdout 涨跌方向 + train_frac 脆弱，**未提选择偏差这一逻辑污染**。报告 line 28 甚至直接声称"+3.47e4 不是纯挑赢家产物"——但用挑赢家所用的同一数据验证不能反驳挑赢家。缺口未覆盖。

3. **Q2 严重性判定合理？** 成立。codex 的 magnitude vs logical contamination 区分严格正确：低基数 (level,δ)（2 个 δ × 少数 level，非大超参网格）确实让 winner's-curse 量级小，但**逻辑上 holdout 已进入假设发现**，不再是干净 OOS——这个区分比"低基数所以没事"更严格，符合 formalization-validity-domain（有效域 ≠ 定义域）。判定 BIAS-FATAL 对"干净 OOS"主张成立。

4. **codex 误读上下文？** 否。Q1 判 SOUND 与工位已知（IncrementalClassifier 每切片 fresh、因果）一致；Q4 的 35% 胜率+正总和=少数大赢家诊断是独立有效观察（报告确实只给聚合 Σ，无逐笔分布）。

**工位附加确认**（codex 未直接点出但同源）：报告 line 43 下游推论"level0卖可作信号层 entry 候选"在 Q2 BIAS-FATAL 下**不成立**——该候选基于被污染的 OOS。

---

## 结果包（简化版）

**结论**：阳性主张**伪阳性（方法论层面，非数值造假）**——codex 判定标题"缠论 alpha 首个 OOS 稳健证据"不可信，单一致命风险=Q2 选择偏差（从含 holdout 的全样本挑 level0卖再在 holdout 验证 = holdout 进入假设发现环节，不是干净 OOS）。工位三点简化质询全部坐实 codex 否定。次级问题（Q3/Q4/Q5 全 INSUFFICIENT）：2 子窗不足证"结构性"、"卖>买"是弱主张（买巨亏而非卖赚钱）、35% 胜率+正总和疑似少数大赢家、单切分非稳健。仅 Q1 时间泄漏 SOUND（切片+因果 classifier 无泄漏）。

**边界条件（结论翻转条件）**：
- 若**改为 train-only 挑类**（在 train=[0,180000) 上选最强正类）→ 锁定 holdout 评估，且 level0卖仍是 train 期赢家且 holdout 仍正 → Q2 污染消除，主张可升为干净 OOS（但 Q3/Q4/Q5 仍需补）。
- 若逐笔 PnL 分布显示剔除最大 1/3/5 赢家后 holdout 卖仍正 → Q4 "少数大赢家"被排除。
- 若多锚定/滚动 walk-forward（≥5 折）level0卖持续正 → Q5 单切分脆弱被排除，"结构性"才成立。

**有效域边界**：当前阳性结果有效域 = **L1↔L2 之间的污染态**——名义上 L2（真实数据单标的），但因选择偏差用了 holdout 信息挑类，实际信息增量低于声称的 OOS（接近"全样本拟合的事后切分"）。在干净 train-only 选类重做前，不能声称 OOS 稳健，更不能升基座（formalization-validity-domain：声明有效域=定义域需等大验证）。

**影响声明**：不改任何代码（read-only 审计）。
- 涉及模块：`rust/src/theta_v0/backtest/econ_positive.rs`（`acc_level0sell_oos` 测试选类逻辑 = 全样本挑类）。
- 涉及报告：`.chanlun/review-results/econpositive-oos-level0sell-20260630.md`（"OOS 初步稳健"+"不是纯挑赢家产物"措辞被 Q2 否定）。
- 需补强（行动类，待 Lead 决策）：(1) 改 train-only 挑类 → 锁定 holdout 验（消 Q2 致命偏差，最高优先）；(2) 逐笔 PnL 分布 + 剔最大赢家敏感性（Q4）；(3) 多折 walk-forward（Q3/Q5）。(1) 是 OOS 主张成立的必要条件，不补则该阳性结果不可向下游传递为 alpha 证据。
