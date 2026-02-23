# 上层5条定义审计报告

**审计工位**: audit-upper（v151-swarm）
**审计日期**: 2026-02-23
**审计方法**: 定义文件全文 → Grep定位实现关键约束 → 代码段精读 → 交叉验证

---

## 1. 走势类型（zoushi）

**定义文件**: `.chanlun/definitions/zoushi.md` v1.6（已结算）
**实现文件**: `src/newchan/a_move_v1.py`

### 提取的约束条件

| # | 约束 | 来源 |
|---|------|------|
| Z1 | 盘整 = 恰好1个中枢 | 第17课 |
| Z2 | 趋势 = ≥2个依次同向中枢 | 第17课 |
| Z3 | 上涨：后DD > 前GG（中心定理二，GG/DD非ZG/ZD）| 第20课，v1.1修复 |
| Z4 | 下跌：后GG < 前DD（中心定理二）| 第20课 |
| Z5 | 任何走势类型至少3段次级别走势（走势分解定理二）| 第17课 |
| Z6 | 级别=递归层级（禁止时间周期硬编码）| 第12课，006号 |
| Z7 | 走势类型是分类层（不参与构造）| 010号 |
| Z8 | 最后一个Move强制settled=False | 定义文件§v1核心算法 |

### 审计结果

| # | 约束 | 状态 | 代码位置 | 说明 |
|---|------|------|---------|------|
| Z1 | 盘整=1中枢 | ✅ 对齐 | `a_move_v1.py:132-133` | `zs_count==1 → kind="consolidation"` |
| Z2 | 趋势=≥2同向中枢 | ✅ 对齐 | `a_move_v1.py:129-130` | `zs_count>=2 → kind="trend"` |
| Z3 | 上涨用DD/GG | ✅ 对齐 | `a_move_v1.py:67-69` | `_is_ascending`: `c2.dd > c1.gg` |
| Z4 | 下跌用GG/DD | ✅ 对齐 | `a_move_v1.py:72-74` | `_is_descending`: `c2.gg < c1.dd` |
| Z5 | ≥3段次级别走势 | ⚠️ 未显式检查 | — | 中枢本身由3段重叠构成，隐含满足；但单中枢盘整的seg_start到seg_end是否≥3段无显式断言 |
| Z6 | 无时间周期硬编码 | ✅ 对齐 | 全文Grep | 无"日线"/"30分"/"5分钟"/timeframe/tf_level |
| Z7 | 分类层不参与构造 | ✅ 对齐 | — | Move是从Zhongshu构造的分类结果，不反向参与Zhongshu构造（level 1）；level 2+中Move作为递归组件参与更高级别中枢构造，但这是递归定义的正当用途，非010号违规 |
| Z8 | 最后Move settled=False | ✅ 对齐 | `a_move_v1.py:172-173` | `result[-1] = replace(result[-1], settled=False)` |

### 谱系违规检查

- **006号**（级别=递归层级）：✅ 无违规。`a_move_v1.py` 无任何时间周期硬编码。
- **010号**（构造层与分类层分离）：✅ 无违规。Move是分类层产物，不反向参与同级构造。

### 缺口

| 缺口 | 严重性 | 说明 |
|------|--------|------|
| Z5隐含满足无显式断言 | LOW | 走势分解定理二要求≥3段次级别走势。当前依赖中枢构造（3段重叠）隐含满足，但无 `assert len(segments_in_move) >= 3` 类的防御性检查。如果上游bug导致退化中枢（<3段），此处不会拦截 |

### 结论

走势类型定义与实现**高度对齐**。v1.1修复（GG/DD替代ZG/ZD）已正确落地。无谱系违规。唯一缺口是Z5的隐含满足无显式断言，严重性LOW。

---

## 2. 趋势/盘整（qushi）

**定义文件**: `.chanlun/definitions/qushi.md` v1.0（已结算）
**实现文件**: `src/newchan/a_move_v1.py`（共享zoushi实现）

### 提取的约束条件

| # | 约束 | 来源 |
|---|------|------|
| Q1 | 盘整方向=无（第31课L390）| 定义文件 |
| Q2 | 趋势方向=有（上涨/下跌）| 第17课 |
| Q3 | 趋势中枢间不重叠 | 第18课L28 |
| Q4 | 声明-实现落差：007/009号要求Trend/Consolidation子类型，当前仍为字符串标签 | 099号谱系 |
| Q5 | 趋势背驰必然终结趋势；盘整背驰不一定终结 | 第24课 |

### 审计结果

| # | 约束 | 状态 | 代码位置 | 说明 |
|---|------|------|---------|------|
| Q1 | 盘整方向=无 | ⚠️ 语义偏差 | `a_move_v1.py:134` | 盘整的direction取`first_zs.break_direction`（非"无"），定义文件说"盘整无方向"（第31课），但代码赋予方向值用于前端展示。zoushi.md§v1数据结构中记录"盘整：取第一段次级别走势的方向"，与qushi.md"盘整无方向"存在定义层面的张力 |
| Q2 | 趋势方向=有 | ✅ 对齐 | `a_move_v1.py:131` | `direction = direction`（从贪心分组的up/down传递）|
| Q3 | 趋势中枢间不重叠 | ✅ 对齐 | `a_move_v1.py:98-103` | `_greedy_group`中，非ascending且非descending的相邻中枢被截断为不同group，隐含保证同group内中枢不重叠 |
| Q4 | 子类型重构缺口 | ℹ️ 已知技术债 | — | 099号谱系已记录，qushi.md已声明"kind字符串足以支撑当前操作"。非审计缺口，是已知的声明-实现落差 |
| Q5 | 趋势/盘整背驰差异 | → beichi审计 | — | 归入beichi条目审计 |

### 谱系违规检查

- 无新增违规。

### 缺口

| 缺口 | 严重性 | 说明 |
|------|--------|------|
| Q1盘整方向语义张力 | MEDIUM | qushi.md声明"盘整无方向"（第31课L390），zoushi.md§v1记录"盘整=break_direction"，代码实现follow zoushi.md。两份定义文件对同一属性的描述不一致。建议在qushi.md中明确"操作层面盘整无方向偏好，但数据模型中direction字段用于前端展示" |

### 结论

趋势/盘整定义与实现基本对齐。主要缺口是Q1盘整方向的两份定义文件之间的语义张力（MEDIUM）。099号子类型技术债已在谱系中记录。

---

## 3. 背驰（beichi）

**定义文件**: `.chanlun/definitions/beichi.md` v1.1（已结算）
**实现文件**: `src/newchan/a_divergence_v1.py`（v1主线），`src/newchan/a_divergence.py`（v0保留）

### 提取的约束条件

| # | 约束 | 来源 |
|---|------|------|
| B1 | 趋势背驰前提：≥2同向中枢 | 第24课 |
| B2 | A/B/C三段力度比较：C段力度 < A段力度 | 第24课 |
| B3 | 盘整背驰：同向离开段力度比较 | 第24课 |
| B4 | MACD面积辅助（上涨看红柱，下跌看绿柱）| 第24课 |
| B5 | T4前提：B段MACD黄白线回拉0轴 | 第25课 |
| B6 | 三维度OR判定（面积/DIF峰值/HIST峰值，任一满足即背驰）| 第25课，#2已结算 |
| B7 | 无MACD时fallback：价格振幅×持续时间 | 定义文件 |
| B8 | 盘整背驰离开段=[ZD,ZG]（非[DD,GG]）| 第33课，#4已结算 |
| B9 | 力度比较不使用浮点阈值/百分比 | 068号（偏序集）|
| B10 | 级别=递归层级 | 006号 |

### 审计结果

| # | 约束 | 状态 | 代码位置 | 说明 |
|---|------|------|---------|------|
| B1 | ≥2同向中枢 | ✅ 对齐 | `a_divergence_v1.py:388` | `move.kind != "trend" or move.zs_count < 2 → return None` |
| B2 | C < A力度 | ✅ 对齐 | `a_divergence_v1.py:342-350` | `_compare_and_build`中`force_c < force_a`（通过三维度OR） |
| B3 | 盘整背驰 | ✅ 对齐 | `a_divergence_v1.py:460-480` | 同向离开段力度比较，`kind=="consolidation"` |
| B4 | MACD面积方向 | ✅ 对齐 | `a_divergence_v1.py:73-76` | `up → abs(area_pos)`，`down → abs(area_neg)` |
| B5 | T4 0轴穿越 | ✅ 对齐 | `a_divergence_v1.py:87-123` | `_b_segment_crosses_zero`：严格正+严格负同时存在，无阈值 |
| B6 | 三维度OR | ✅ 对齐 | `a_divergence_v1.py:243-255` | `_check_three_dim_divergence`：`t2 or t6 or t7` |
| B7 | fallback力度 | ✅ 对齐 | `a_divergence_v1.py:78-82` | `(high - low) * duration` |
| B8 | 离开段=[ZD,ZG] | ✅ 对齐 | `a_divergence_v1.py:422-424` | `seg.high > zs.zg or seg.low < zs.zd`（使用zg/zd非gg/dd）|
| B9 | 无浮点阈值 | ✅ 对齐 | 全文Grep | 无threshold/percent/ratio常量；力度比较为纯 `<` 运算，0轴穿越为 `> 0` / `< 0`，无任何人为阈值 |
| B10 | 无时间周期硬编码 | ✅ 对齐 | 全文Grep | 无"日线"/"30分"等字符串 |

### 谱系违规检查

- **068号**（偏序集，无浮点阈值）：✅ 无违规。力度比较全部使用纯 `<` 关系运算，T4使用严格正/负判定，无百分比或epsilon阈值。
- **006号**（级别=递归层级）：✅ 无违规。`level_id: int` 参数贯穿始终，无时间周期参数。

### v0管线（a_divergence.py）附带审计

v0管线保留为参考实现。关键差异：
- v0仅使用T2（面积）单维度判定（`a_divergence.py:192`：`force_c < force_a`），未集成T6/T7
- v0的Divergence数据类已扩展`dif_peak_a/c`、`hist_peak_a/c`字段（`a_divergence.py:80-83`，默认0.0），但这些字段在v0检测逻辑中未使用
- v0无T4前提检查
- v0力度计算逻辑与v1一致（MACD面积 + 振幅fallback）

v0与定义的偏差属于已知历史状态，v1已完成修复。

### 缺口

| 缺口 | 严重性 | 说明 |
|------|--------|------|
| 无 | — | 所有定义约束均已在v1管线中正确实现 |

### 结论

背驰定义与v1实现**完全对齐**。6/6未结算问题的结算结果均已正确落地到代码中。T4（0轴穿越）、T6/T7（DIF/HIST峰值）、三维度OR判定——全部无人为阈值，符合068号偏序集要求。无谱系违规。

---

## 4. 级别递归（level_recursion）

**定义文件**: `.chanlun/definitions/level_recursion.md` v1.0（已结算）
**实现文件**: `src/newchan/core/recursion/recursive_level_engine.py`、`src/newchan/core/recursion/recursive_stack.py`

### 提取的约束条件

| # | 约束 | 来源 |
|---|------|------|
| L1 | 级别=递归层级（level_id: ℕ）| 第12课 |
| L2 | Move[0]=Segment（归纳基底） | 第17课 |
| L3 | Center[k]=三个连续Move[k-1]重叠 | 第17课 |
| L4 | 只有settled Move参与递归构造 | #2已结算（Option B） |
| L5 | 递归终止：Move[k] < 3时停止 | 第17课 |
| L6 | max_levels可配（安全阀） | #3已结算 |
| L7 | 无时间周期参数 | 006号 |
| L8 | 口径A（递归级别）是唯一正式路径 | 编排者决断 |

### 审计结果

| # | 约束 | 状态 | 代码位置 | 说明 |
|---|------|------|---------|------|
| L1 | level_id: int | ✅ 对齐 | `recursive_level_engine.py:52-53` | `self._level_id = level_id`，纯int |
| L2 | Move[0]=Segment | ✅ 对齐 | `a_level_protocol.py` | `SegmentAsComponent`适配器将Segment适配为MoveProtocol |
| L3 | 三段重叠 | ✅ 对齐 | `a_zhongshu_level.py` | `zhongshu_from_components()`接收MoveProtocol列表 |
| L4 | settled过滤 | ✅ 对齐 | `recursive_level_engine.py:89` | `settled_moves = [m for m in move_snap.moves if m.settled]` |
| L5 | moves<3终止 | ✅ 对齐 | `recursive_stack.py:69` | while循环条件`current_level < self._max_levels`，内部通过settled moves不足3个→无法形成中枢→空快照→终止推进 |
| L6 | max_levels可配 | ✅ 对齐 | `recursive_stack.py:33` | `__init__(self, max_levels: int = 6)` |
| L7 | 无时间周期参数 | ✅ 对齐 | 全文Grep | 整个`core/recursion/`目录无timeframe/tf_level/日线/30分等 |
| L8 | 口径A唯一 | ✅ 对齐 | — | RecursiveOrchestrator使用RecursiveStack（口径A），TFOrchestrator保留为调试工具 |

### 谱系违规检查

- **006号**（级别=递归层级）：✅ 无违规。整个递归模块使用纯`level_id: int`，无时间周期硬编码。

### 缺口

| 缺口 | 严重性 | 说明 |
|------|--------|------|
| 无 | — | 所有定义约束均已正确实现 |

### 测试覆盖

- `test_recursive_level_engine.py`: 21个测试
- `test_recursive_stack.py`: 16个测试
- `test_recursive_orchestrator.py`: 9个测试
- `test_p9_cross_validation.py`: 7个测试（编排器vs手动管线一致性）
- `test_real_data_recursive_e2e.py`: 12个测试

总计：65个递归相关测试。

### 结论

级别递归定义与实现**完全对齐**。递归构造链（Segment→Center[1]→Move[1]→Center[2]→Move[2]→...）完整可追溯。settled过滤、自然终止、max_levels安全阀均已正确实现。无谱系违规。

---

## 5. 买卖点（maimai）

**定义文件**: `.chanlun/definitions/maimai.md` v1.0（已结算）
**实现文件**: `src/newchan/a_buysellpoint_v1.py`

### 提取的约束条件

| # | 约束 | 来源 |
|---|------|------|
| M1 | 第一类买点=下跌趋势背驰点（≥2中枢） | 第17课/第24课 |
| M2 | 第一类买点仅接受趋势背驰（排除盘整背驰）| 第21课L40-43，#4已结算 |
| M3 | 第二类买点=1B后首个回调结束点 | 第17课/第21课 |
| M4 | 第三类买点=离开中枢后回试不跌破ZG（非GG） | 第20课，#5已结算 |
| M5 | 第三类卖点=离开中枢后回抽不升破ZD（非DD） | 第20课 |
| M6 | confirmed=Move.settled | #2已结算 |
| M7 | 2B+3B可重合 | 第21课 |
| M8 | 无概率/胜率概念 | 067号 |
| M9 | 级别=递归层级 | 006号 |

### 审计结果

| # | 约束 | 状态 | 代码位置 | 说明 |
|---|------|------|---------|------|
| M1 | 1B=趋势背驰 | ✅ 对齐 | `a_buysellpoint_v1.py:104` | `if div.kind != "trend": continue` |
| M2 | 排除盘整背驰 | ✅ 对齐 | `a_buysellpoint_v1.py:88,104` | `_find_assoc_trend_move`要求`m.kind=="trend"`；`div.kind != "trend"` skip |
| M3 | 2B=1B后回调 | ✅ 对齐 | `a_buysellpoint_v1.py:178-209` | buy: 1B→找up段→找down段（回调）；sell: 1B→找down段→找up段（反弹）|
| M4 | 3B回试不跌破ZG | ✅ 对齐 | `a_buysellpoint_v1.py:261` | `pullback_seg.low > zs.zg`（使用zg非gg）|
| M5 | 3S回抽不升破ZD | ✅ 对齐 | `a_buysellpoint_v1.py:263` | `pullback_seg.high < zs.zd`（使用zd非dd）|
| M6 | confirmed=Move.settled | ✅ 对齐 | `a_buysellpoint_v1.py:136,173,232` | Type1: `div.confirmed`；Type2/3: `assoc_move.settled if assoc_move else False` |
| M7 | 2B+3B重合 | ✅ 对齐 | `a_buysellpoint_v1.py:271-297` | `_detect_overlap`函数检测同seg_idx同side的2B和3B |
| M8 | 无概率概念 | ✅ 对齐 | 全文Grep | 无probability/概率/胜率/percent/ratio |
| M9 | 无时间周期硬编码 | ✅ 对齐 | 全文Grep | 无"日线"/"30分"等 |

### 谱系违规检查

- **067号**（合法/非法替代概率）：✅ 无违规。买卖点判定逻辑全部基于结构条件（背驰、中枢突破），无胜率/概率概念。
- **006号**（级别=递归层级）：✅ 无违规。`level_id: int` 参数贯穿始终。
- **010号**（构造层与分类层分离）：✅ 无违规。买卖点是分类层的最终产物，不参与构造。

### 缺口

| 缺口 | 严重性 | 说明 |
|------|--------|------|
| 第三类买卖点"第一次"约束 | LOW | 定义文件M4/M5要求"必须是第一次离开后的回试"。代码在`_detect_type3`中遍历所有settled中枢（`for zs in zhongshus: if not zs.settled or not zs.break_direction: continue`），对每个中枢只找第一个pullback（`_find_next_seg_by_direction`从break_seg+1开始找第一个），隐含满足"第一次"约束。但若同一中枢有多次break/re-entry事件（中枢延伸后再次break），Zhongshu的break_seg/break_direction是否只记录第一次需要在zhongshu层面确认 |

### 测试覆盖

- `test_buysellpoint_v1.py`: 17个测试
- `test_buysellpoint_events.py`: 17个测试

总计：34个买卖点相关测试。

### 结论

买卖点定义与实现**高度对齐**。5/5未结算问题的结算结果均已正确落地。关键约束——Type1仅接受趋势背驰、Type3使用ZG/ZD非GG/DD、confirmed=Move.settled——全部验证通过。无谱系违规。

---

## 总结

### 审计统计

| 定义 | 约束数 | 对齐 | 偏差 | 缺口 |
|------|--------|------|------|------|
| 走势类型 zoushi | 8 | 7 | 0 | 1 (LOW) |
| 趋势/盘整 qushi | 5 | 3 | 1 | 1 (MEDIUM) |
| 背驰 beichi | 10 | 10 | 0 | 0 |
| 级别递归 level_recursion | 8 | 8 | 0 | 0 |
| 买卖点 maimai | 9 | 9 | 0 | 1 (LOW) |
| **合计** | **40** | **37** | **1** | **3** |

### 谱系违规

| 谱系编号 | 描述 | 检查结果 |
|----------|------|---------|
| 068号 | 偏序集（无浮点阈值） | ✅ 全部通过：背驰力度比较使用纯 `<` 运算，T4使用strict `>0`/`<0`，无百分比阈值 |
| 006号 | 级别=递归层级 | ✅ 全部通过：5个实现文件均无时间周期硬编码 |
| 067号 | 合法/非法替代概率 | ✅ 全部通过：买卖点无胜率/概率概念 |
| 010号 | 构造层与分类层分离 | ✅ 全部通过：走势类型和买卖点均为分类层产物 |

### 发现的缺口汇总

| # | 定义 | 缺口 | 严重性 | 建议 |
|---|------|------|--------|------|
| 1 | zoushi | Z5走势分解定理二（≥3段）无显式断言 | LOW | 可在Move构造时添加防御性断言 |
| 2 | qushi | Q1盘整方向：qushi.md说"无"，zoushi.md和代码说"有（break_direction）" | MEDIUM | 统一两份定义文件的描述，明确数据模型direction字段的语义 |
| 3 | maimai | 第三类买卖点"第一次"约束隐含依赖Zhongshu.break_seg的语义 | LOW | 确认Zhongshu在延伸后再次break时break_seg是否更新 |

**总体评估**：上层5条定义与实现的一致性良好。40条约束中37条完全对齐，1条语义偏差（MEDIUM），3条缺口中2条LOW、1条MEDIUM。无谱系违规。
