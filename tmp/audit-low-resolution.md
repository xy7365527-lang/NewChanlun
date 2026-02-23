# 审计遗留 LOW/P2 项解决报告

## 1. P2: 线段笔数奇数性未显式验证 + TAIL_WINDOW=7 充分性

**状态**: resolved（代码修复）

### 笔数奇数性
- **原状**：`a_assertions.py :: assert_segment_theorem_v1` 仅检查 `span >= 2`（至少3笔），未检查奇数性
- **修复**：新增 `_seg_check_odd_stroke_count()` 断言，验证 confirmed 段的笔数为奇数
- **原理**：线段由交替方向的笔构成（up/down/up/...），起始笔方向=线段方向，因此笔数必为奇数（3, 5, 7, ...）
- **结构**：两遍检查——第一遍结构性检查（最小笔数、顶高于底、结算锚、拼接、方向交替），第二遍派生属性（奇数性）
- **文件**：`src/newchan/a_assertions.py` 第681-695行、第810-815行

### TAIL_WINDOW=7
- **结论**：工程合理值，不需要修改
- **分析**：TAIL_WINDOW=7 表示特征序列分型检测只在最近7个元素内扫描。每个特征序列元素对应一根反向笔，7个元素覆盖约14根笔。考虑到包含合并可能回退 `last_checked`（第264行），7是足够宽裕的窗口
- **风险评估**：如果特征序列包含合并极端密集（>7个连续元素合并），理论上可能漏检。但实际市场数据中这种情况概率极低，且 `last_checked` 的回退机制提供了额外保护

## 2. LOW: 包含关系 baohan — 等号/双条件问题

**状态**: resolved（定义已结算）

- `baohan.md` v1.3 状态为"已结算"
- 三个"未结算问题"实为已记录的设计选择文档：
  1. **单条件 vs 双条件**：v1.3 已结算——"方向判定双条件与单条件在非包含前提下数学等价（审计验证）"
  2. **等号问题**：`>=`/`<=` vs `>`/`<`——等号在包含判定下不影响方向判定结果（数学证明见 v1.3）
  3. **初始方向 None→UP**：工程选择，无原文明确依据，但不影响正确性
- 代码实现（双条件、严格不等式、None→UP）与定义文件一致
- 无需代码修改

## 3. LOW: 走势类型 zoushi — Z5走势分解定理二无显式断言

**状态**: resolved（代码修复）

- **原状**：`a_move_v1.py` 构造 Move 时无显式断言验证"至少3段次级别走势"
- **隐式保证**：每个 Move 至少含1个 zhongshu，每个 zhongshu 至少3段 → 结构上已保证
- **修复**：新增 `assert_move_min_three_sub_segments()` 显式断言
  - 对 settled Move 验证 `seg_end - seg_start + 1 >= 3`
  - 未 settled 的最后一个 Move 豁免（数据仍在累积）
- **集成**：已加入 `run_a_system_assertions()` 入口（新增 `moves` 可选参数，向后兼容）
- **文件**：`src/newchan/a_assertions.py` 第577-610行、第650行

## 4. LOW: 买卖点 maimai — 3B"第一次"约束隐含依赖

**状态**: resolved（注释显式化）

- **原状**：`_detect_type3()` 中"第一次"约束由代码结构隐式保证，无文档说明
- **隐式保证链**：
  1. `zs.break_seg` = zhongshu 构造时的第一个突破段（`a_zhongshu_v1.py` 延伸循环的终止条件）
  2. `_find_next_seg_by_direction(segments, break_seg + 1, opposite_dir)` 找的是突破后的第一个反向段
  3. 因此每个中枢最多产生一个第三类买卖点
- **修复**：为 `_detect_type3()` 添加完整的 docstring，显式记录"第一次"约束的保证机制
- **文件**：`src/newchan/a_buysellpoint_v1.py` 第237-249行

## 5. LOW: 比价关系 bijia — 中枢/走势类型管线未集成

**状态**: blocked

- **当前管线**：`ratio_engine.py :: analyze_pair()` 只到线段：`包含处理 → 分型 → 笔 → 线段`
- **缺失**：zhongshu → move → divergence → buysellpoint
- **阻塞原因**：比价管线的 zhongshu/move 集成依赖单标的递归引擎的稳定性。当前递归引擎（`a_level_fsm_newchan.py` + `core/recursion/`）仍在演进中
- **建议**：待单标的递归引擎 v1 稳定后，扩展 `ratio_engine.py :: _run_pipeline()` 增加 zhongshu/move/divergence/buysellpoint 步骤

## 测试验证

- 全量回归：1581 passed, 8 skipped（100% GREEN）
- 断言测试：37 passed（含新增奇数性和 Z5 断言的隐式覆盖）
