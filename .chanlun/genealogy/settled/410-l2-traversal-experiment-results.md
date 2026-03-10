---
id: '410'
number: 410
title: "L2穿越引擎实验——β₁轨迹分析+fold/negate判据验证（227K事件）"
type: experiment-result
status: 已结算
date: 2026-03-10
source: v215-session
depends_on: ['390', '391', '392', '393', '395']
topo_effect: ""
tensions_with: []
---

# 410号：L2穿越引擎实验——β₁轨迹分析+fold/negate判据验证（227K事件）

## 来源标注

- **认识论等级**: L2（真实数据验证，单标的/多图——但多图混合无session隔离）
- **数据**: traversal-events.jsonl, 227,712 events, 7 distinct graphs identified by β₁ band
- **分析脚本输出**: `tmp/beta1_trajectory_analysis.json`, `tmp/fold_negate_criteria_analysis.json`

## 核心发现

### 390号声明验证

| 声明 | 判定 | 证据摘要 |
|------|------|----------|
| 390-1 (β₁有界增长) | **否定** | small图趋于稳态振荡(非减速增长): β₁在129-257范围内反复growth/contraction/plateau，净变化仅+71；medium图震荡加剧: β₁范围1935-5539，净变化-464，单窗口振幅达±2611 |
| 390-2 (settlement保护递增) | **分裂** | medium图确认(fold_block_rate: 95%→100%→97.6%)；small图否定(fold_block_rate: 97%→94.4%→45.6%，晚期反而下降) |
| 390-3 (效率递减) | **确认** | 两图一致：small图discovery_rate 0.87%→0.02%→0.19%；medium图8.04%→null→1.73%。发现率系统性下降 |

**关键数据**：

- small图（138,404事件）：β₁起始129，终值200，范围[129,257]，15个macro_phase中growth/contraction/plateau交替出现——典型的稳态振荡，不是"有界增长"
- medium图（81,298事件）：β₁起始5262，终值4798，范围[1935,5539]，13个macro_phase，包含振幅达2611的剧烈收缩——震荡加剧，也不是"有界增长"

### 391-393号声明验证

| 声明 | 判定 | 证据摘要 |
|------|------|----------|
| 393-1 (f=0是fold充分条件) | **部分确认** | f=0选择性100%排除negate/sublate（negate_blocked中f=0: 0个，sublate中f=0: 0个），但fold成功率仅8.3%（141/1703 f=0事件是fold成功，其余1562是fold_blocked）。vs 393号声称的40%成功率 |
| 393-5 (测量时机原则) | **支持** | f=0比例差异(6.45% vs 393号预期40%)恰好说明操作改变图后f值变化——测量时机影响观测值 |
| 391-2 (g_value辨别力) | **不可测** | g_value未记录在事件日志中。392号独立分析报告d=7.03效应量 |
| 392-1 (jaccard fold预测器) | **不可测** | jaccard未记录在事件日志中。393号报告44x效应(0.622 vs 0.014) |
| negate操作 | **100% blocked** | 171,338次negate尝试全部被blocked。settlement锁区过大（已知问题） |

**f=0 分布详情**：
- fold成功且f=0: 141 (6.45% of fold)
- fold_blocked且f=0: 1,562 (3.03% of fold_blocked)
- negate_blocked且f=0: 0
- sublate且f=0: 0

### 额外发现

1. **f-value随时间急剧增长**: 成功fold的f值从早期μ=43→晚期μ=1531，增长35倍
2. **β₁增长主要由sublate驱动**: sublate每次稳定+1（delta_beta1恒等于1），fold多数降低β₁（μ=-2.64, 中位数-1.0）
3. **成功fold的f值显著高于fold_blocked**: fold均值578.7 vs fold_blocked均值115.1，Cohen's d=1.135（大效应量），Mann-Whitney z=-17.93, p<0.001
4. **negate_blocked的f值大量为sentinel -99**: 171,338个negate_blocked中仅24,500个有真实f值（排除sentinel后μ=68.5）
5. **多图交织**: 7个不同规模的图混合在同一事件文件中，无session隔离标记

## 方法论限制

1. **events.jsonl混合多图多walker无session隔离**: 必须通过β₁ band反推图归属，存在band重叠时的归属歧义
2. **390号原始实验的图（β₁起始2325, 2524顶点）与当前数据不匹配**: 底层图已重构。small图起始β₁=129（远小于2325），medium图起始β₁=5262（远大于2325）
3. **严格验证需要单图隔离session重跑**: 当前分析是对已有数据的事后分析，不是受控实验
4. **sentinel值-99污染统计**: sublate和negate_blocked的f_value全部或大量为-99，需要在统计前过滤

## 边界条件

- 如果在390号原始图（或等价规模图）上单session重跑，结果可能不同——390号声明可能在特定图规模/结构下成立
- 本分析的"否定"是对当前数据的判断，不是对390号方法论的否定
- f=0的fold成功率8.3%低于393号的40%，可能因为当前图结构不同，或因为测量时机不同（393-5所述）
- 如果增加g_value和jaccard的事件日志记录，391-2和392-1可能被确认或否定

## 下游推论

1. **events.jsonl 需要 session 隔离标记**: 多图混合严重影响分析——每个 walker session 应有唯一 ID，每个事件记录应包含 graph_id 和 session_id
2. **g_value/jaccard 应记录在事件日志**: 当前只记录 f_value，缺少 391-2/392-1 验证所需数据。encounter_step() 应在事件中附加 g_value 和 jaccard_similarity
3. **negate 100% blocked 需要诊断**: settlement 锁区过大导致穿越引擎丧失否定能力——这是架构级问题，不是参数调优问题
4. **β₁动态模式重新表征**: 390号的"有界增长"应修正为"图依赖的动态模式"——small图稳态振荡，medium图大幅震荡，不存在统一的增长形态
5. **f-value与fold成功的强关联**: Cohen's d=1.135表明f值确实是fold可行性的强信号，但方向与直觉相反——f值更高时fold更可能成功（而非f=0时）

## 影响声明

- **390-1 部分否定**: β₁增长不是"有界"而是"稳态振荡"（small图）或"加剧震荡"（medium图），取决于图结构。390号的增长模型需要图依赖的参数化
- **390-2 分裂**: settlement保护的单调递增性质不是普遍的——small图晚期fold_block_rate从97%降到45.6%，与390号预期相反
- **390-3 确认**: 效率递减是跨图规模的稳健结论
- **393-1 部分确认**: f=0对negate/sublate的排除性100%可靠，但对fold的预测性远低于预期
- **新增方法论要求**: 事件日志session隔离 + g_value/jaccard记录 + negate blocked诊断

## 谱系关联

- **depends_on**: 390（β₁增长声明）, 391（f/g判据声明）, 392（jaccard声明）, 393（fold充分条件声明）, 395（穿越引擎实装）
- 本记录是 390-395 系列的**首次系统性 L2 验证**
- 否定性结果（390-1否定、390-2分裂）的价值高于确认性结果——它们缩小了声明的有效域边界（见 formalization-validity-domain 规则）
