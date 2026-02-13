# 缠论术语 ↔ 代码符号对照表（NewChanlun / newchan）

> 目的：把“缠论定义（Spec/公理）→ 代码实现（A 系统）→ 可视化/接口（B 系统）→ 单元测试（回归锚）”串起来。
> 这份表本身不创造正确性，但能显著降低**术语/定义/实现之间的语义漂移**。

## 规范来源（Single Source of Truth）

- **可实现规格**：`docs/chan_spec.md`
- **形式化公理/不变量**：`docs/formal_axioms.md`
- **知识库（便于 AI 引用）**：`缠论知识库.md`

> 建议工作纪律：改“定义/口径”时，**必须同步改 Spec + 断言/测试**；否则就会发生“代码口径变了但文档还在讲旧话”的漂移。

---

## A 系统对象图（Object Graph）快速索引

- **Bar（原始K线）**：`src/newchan/types.py::Bar`（概念类型），工程上主要以 `pd.DataFrame` 传递
- **MergedBar（包含处理后K线）**：`src/newchan/a_inclusion.py::merge_inclusion` 输出 `df_merged + merged_to_raw`
- **Fractal（分型）**：`src/newchan/a_fractal.py::Fractal / fractals_from_merged`
- **Stroke（笔）**：`src/newchan/a_stroke.py::Stroke / strokes_from_fractals`
- **Segment（线段）**：
  - v0：`src/newchan/a_segment_v0.py::Segment / segments_from_strokes_v0`
  - v1（特征序列法）：`src/newchan/a_segment_v1.py::segments_from_strokes_v1`
- **Center（中枢）**：`src/newchan/a_center_v0.py::Center / centers_from_segments_v0`
- **TrendTypeInstance（走势类型实例）**：`src/newchan/a_trendtype_v0.py::TrendTypeInstance / trend_instances_from_centers`
- **递归级别（Move[k] / Center[k]）**：`src/newchan/a_recursive_engine.py::build_recursive_levels`
- **L*（三锚裁决输出）**：`src/newchan/a_level_fsm_newchan.py::select_lstar_newchan`

---

## 术语 → 规格章节 → 实现入口 → 测试锚点

| 术语/概念 | 规格/公理 | 主要实现（A 系统） | 关键测试（回归锚） | 断言（不变量） |
|---|---|---|---|---|
| K线包含处理（Inclusion） | `chan_spec.md` §2 | `a_inclusion.merge_inclusion` | `tests/test_inclusion.py` | `a_assertions.assert_inclusion_no_residual` |
| 分型（Fractal） | `chan_spec.md` §3 | `a_fractal.fractals_from_merged` | `tests/test_fractal.py` | `a_assertions.assert_fractal_definition` |
| 笔（Stroke） | `chan_spec.md` §4 | `a_stroke.strokes_from_fractals` | `tests/test_stroke.py` | `a_assertions.assert_stroke_alternation_and_gap` |
| 线段 v0（三笔交集重叠） | `chan_spec.md` §5.1 / §5.4(v0) | `a_segment_v0.segments_from_strokes_v0` | `tests/test_segment_v0.py` | `a_assertions.assert_segment_min_three_strokes_overlap` |
| 线段 v1（特征序列法） | `chan_spec.md` §5.4(v1) | `a_segment_v1.segments_from_strokes_v1` | `tests/test_segment_v1.py` | `a_assertions.assert_segment_theorem_v1` |
| 中枢（Center） | `chan_spec.md` §7；`formal_axioms.md` §3.2 | `a_center_v0.centers_from_segments_v0` | `tests/test_center_v0.py` | `a_assertions.assert_center_definition` |
| 走势类型实例（Trend/Consolidation） | `chan_spec.md` §8；`formal_axioms.md` §3.1 | `a_trendtype_v0.trend_instances_from_centers` | `tests/test_trendtype.py` |（以测试为主） |
| 归纳递归（Move/Center/TTI） | `chan_spec.md` §6；`formal_axioms.md` §3.1 | `a_recursive_engine.build_recursive_levels` | `tests/test_recursive_engine.py` |（可对 levels 做一致性断言） |
| L*（三锚裁决） | `chan_spec.md` §9；`formal_axioms.md` §4 | `a_level_fsm_newchan.select_lstar_newchan` | `tests/test_level_fsm_newchan.py` | `a_assertions.assert_single_lstar` |
| A→B overlay（前端消费 schema） |（工程契约） | `ab_bridge_newchan.build_overlay_newchan` | `tests/test_ab_bridge_overlay.py` |（可在 debug 模式下跑全链路断言） |
| B 系统渲染语义（中心框/非叙事连线） | `formal_axioms.md` I7 | `frontend/src/primitives/ChanTheoryPrimitive.ts` |（前端 e2e/视觉回归为主） |（前端侧约束） |

---

## 全链路入口（从行情到 overlay）

核心入口函数：`src/newchan/ab_bridge_newchan.py::build_overlay_newchan`

调用链（高层）：

1. `merge_inclusion(df_raw)` → `df_merged, merged_to_raw`
2. `fractals_from_merged(df_merged)` → `fractals`
3. `strokes_from_fractals(df_merged, fractals)` → `strokes`
4. `segments_from_strokes_v1(strokes)`（默认）→ `segments`
5. `build_recursive_levels(segments)` → `levels`（多级别 centers/trends）
6. `select_lstar_newchan(level_views, last_price)` → `L*`
7. 组装为 `newchan_overlay_v2` JSON

---

## “防语义漂移”的推荐使用方式（工程化）

- **写清定义**：先改 `docs/chan_spec.md` / `docs/formal_axioms.md`（口径变更要可追溯）
- **在对象层固化语义**：让 `Stroke/Segment/Center/TrendTypeInstance` 的字段成为“唯一真相”（桥接层只映射，不重算）
- **把不变量写进断言/测试**：优先写“不会随实现细节变化”的约束（例如：交替、覆盖、索引边界、ZG>ZD、confirmed 规则等）
- **每次改完都跑**：`pytest`

当你用 Cursor/Claude Code（带 Serena）时，最常见的高效提问方式：

- “请从 `build_overlay_newchan` 追到 `centers_from_segments_v0`，解释中枢 ZD/ZG 的取值依据，并指出对应 Spec 段落。”
- “我想把‘严笔/宽笔’参数化，列出受影响的函数与测试，并给出最小改动方案。”
- “我要改线段 v1 的触发条件（特征序列分型/缺口），请先做影响面分析（哪些模块/哪些测试会失败）再动手。”
