# PH 四假说 L2 验证报告 — QQQ 日线

**标的**：NASDAQ:QQQ  **周期**：日线  **时间**：2024-08-27 至 2026-05-29（440 根）
**认识论等级**：L2（单标的单时段，不外推）
**数据源**：analysis/data_cache/QQQ_1d_max.json + qqq_daily_chanlun.json

**TV 缠论标注统计**：买点 63 个 / 卖点 44 个 / 合计 107 个

---

## 假说1：alive 分量递减 = 背驰

**假设**：sublevel tree（on close）的 alive 分量数在背驰区域前趋势性递减。
**信号**：前 5 根 alive_count 的线性斜率 < 0。
**事件**：TV 缠论买点（底部背驰代理）。
**Null baseline precision@5bar**：53.8%

| 指标 | 值 |
|------|-----|
| 信号总数 | 168 |
| 买点事件数 | 63 |
| 信号 precision（5bar 内有买点） | 66.7% |
| null_precision（随机基线） | 53.8% |
| lift = prec / null | 1.2387 |
| 信号 recall（买点前 5bar 有信号） | 73.0% |
| 买点前 slope 均值 | -0.0109 |
| 买点前 slope < 0 比率 | 41.3% |
| 全局 slope < 0 比率 | 38.6% |

**关键比较**：买点前 neg_slope 比率 vs 全局 neg_slope 比率
→ 41.3% vs 38.6%

**结论**：待定（差异较小，需更多数据）（lift=1.0686）

---

## 假说2：superlevel settle persistence > sublevel settle persistence = 趋势反转

**假设**：双向测试——top_settle > bot_settle → 卖压 → 卖点；bot_settle > top_settle → 买压 → 买点。
**实现**：DualMergeTree T_high(-high) + T_low(low)，比较最近新增 settle persistence。
**Null baseline precision@5bar**：41.0%（事件密度 24%，随机信号期望 precision）

| 信号方向 | 信号数 | 事件数 | precision | null_prec | lift | event 命中率 |
|---------|-------|-------|-----------|-----------|------|------------|
| top>bot → 卖点 | 108 | 44 | 47.2% | 41.0% | 1.1531 | 40.9% |
| bot>top → 买点 | 321 | 63 | 67.0% | 53.8% | 1.2445 | 79.4% |

**注意**：lift > 1 表示信号 precision 高于随机基线；lift ≈ 1 = 无区分力；lift < 1 = 反向。

**结论**：初步成立（至少一个方向 lift > 1.2，信号有效区分力）

---

## 假说3：双树 alive 差值变号 = 趋势转折

**假设**：DualMergeTree（T_low/T_high on HL）的 alive 差值变号预示趋势转折。
测试两个差值：
  (a) dom_diff = dom_p_bot - dom_p_top（dominant persistence 差）
  (b) cnt_diff = n_alive_bot - n_alive_top（alive 个数差）
**信号**：相邻两根 bar 的差值符号翻转。
**事件**：TV 缠论买卖点。

### (a) dominant persistence 差变号

| 指标 | 值 |
|------|-----|
| 信号总数（dom 变号次数） | 2 |
| 信号 precision | 50.0% |
| null_precision（随机基线） | 75.2% |
| lift | 0.6651 |
| 信号 recall | 0.9% |
| 买卖点前 dom 变号命中率 | 0.9% |

### (b) alive 个数差变号

| 指标 | 值 |
|------|-----|
| 信号总数（cnt 变号次数） | 4 |
| 信号 precision | 100.0% |
| null_precision（随机基线） | 75.2% |
| lift | 1.3303 |
| 信号 recall | 5.6% |
| 买卖点前 cnt 变号命中率 | 5.6% |

**结论**：待定（信号数量不足：dom=2个 cnt=4个；无法拒绝随机假设）

---

## 假说4：alive 数做连续力度指标

**假设**：alive_count 或 dominant_persistence 与未来 N 日收益存在显著 IC。
**方法**：Pearson IC，|IC| > 0.1 为有意义阈值（量化惯例）。

| 前瞻窗口 | IC(alive_count close) | IC(dom_p close) | IC(dom_p T_low HL) |
|---------|-----|-----|-----|
| 5日 | 0.1587 | 0.0786 | 0.0897 |
| 10日 | 0.1817 | 0.0776 | 0.0957 |
| 20日 | 0.2544 | 0.0982 | 0.1219 |

**max |IC| 综合**：0.2544
→ 初步成立（存在 >0.1 的 IC，alive 数具有一定预测信息含量；实用性待 L3 交叉验证）

---

## 边界条件与翻转声明

以下条件下，上述结论可能翻转：

1. **数据窗口**：本报告仅 440 根 K 线（~1.75 年），结论对 2024-08-27 至 2026-05-29 有效。 不同市场环境（牛/熊市转换）可能导致显著差异。
2. **Close 价限制**：使用 close 价运行 merge tree（而非 high/low 价）， 会低估 persistence（tick 内的振幅不在 close 序列中体现）。
3. **TV 标注对齐**：label_time 0 假设对应 QQQ 第 6408 根 K 线（2024-08-27）。 如果 TV 图表加载了不同历史长度，对齐关系需重新校正。
4. **买卖点代理问题**：用全部买卖点代理背驰事件（H1）， 实际 type2/type3 买卖点不需要背驰，会降低信号纯度。
5. **DualMergeTree 实现**：H2/H3 使用 T_high(-high) + T_low(low)， H1/H4 使用 close 的 sublevel tree。两套树的信号互不干扰但也不完全等价。

## 谱系引用

- 不确定是否有直接的谱系记录针对这四条假说的分离过程，以下为相关记录：
- `a_online_persistence.py` 注释中的 §7.5 升级方向（alive/settled 概念来源）
- persistence_theory.md §7.5（因果 merge tree 的形式化位置）
- `.chanlun/genealogy/` — 未检索到 PH 四假说专属的谱系条目

## 影响声明

本脚本为只读验证，不修改任何模块。
产出：`analysis/ph_hypotheses_results.md`（当前文件）。
对下游系统无直接影响，结论供 L3 交叉验证和信号设计参考。
