# TradingView MCP 缠论指标探查——新缠论搭建正确性的验证基线

**产出日期**：2026-04-24
**主体**：v70-swarm/tv-mcp-chanlun-probe（扁平退化子蜂群）
**扁平化理由**：Phase 1→2→3→4 存在严格数据依赖（MCP 能力查明才能拉脚本，脚本读完才能对照，对照完才能协议化），拆分为并行 agent 会复制读取同一批文件，得不到有效并行度。

---

## 0. 诚实开头（必读）：MCP 实时状态

**tv_health_check / pine_list_scripts / chart_get_state 等 MCP 工具已装并暴露在 ToolSearch 注册表**（可证——见 `/Users/silencehan/.claude/skills/tradingview-chanlun/SKILL.md` 及其 references 索引；工具前缀 `mcp__405c29ce-5a8f-4996-b408-da143dc39b65__`），但在 **2026-04-24 auto 模式执行窗口内**，`claude-opus-4-7[1m]` 平台分类器对所有 MCP 工具调用返回：

```
claude-opus-4-7[1m] is temporarily unavailable, so auto mode cannot determine
the safety of mcp__...__tv_health_check right now.
```

**这不是 MCP 不可用，也不是工具不存在**——是平台分类器在 auto 模式下暂时不对该 tool 出判。

**按任务约束我不伪造 MCP 结果**。本报告的 Phase 1/2 基于**用户亲自编写的 skill 文件**（`/Users/silencehan/.claude/skills/tradingview-chanlun/SKILL.md`）——该文件即是 TV 缠论指标经验基线的**静态快照+接口规范**。Phase 3/4 基于**对本仓库代码的只读对照**。

**L2 真实对照（跑 TV 数据 → 跑本仓库 → 逐点比）需要在下一次平台分类器恢复时执行**，本报告末尾给出可执行协议。

---

## 1. 结论

### 1.1 TradingView MCP 能力清单（来自 skill 文件 + ToolSearch 注册表）

MCP `405c29ce-5a8f-4996-b408-da143dc39b65` 提供的能力分五类：

| 类别 | 工具示例 | 用途 |
|-----|---------|------|
| 连接/健康 | `tv_health_check`, `tv_discover`, `tv_launch`, `tv_ui_state` | 确认 CDP 连接，枚举可用 API 路径，启动 TV Desktop |
| 图表控制 | `chart_get_state`, `chart_set_symbol`, `chart_set_timeframe`, `chart_set_type`, `chart_set_visible_range`, `chart_scroll_to_date` | 切标的/切周期/切图表类型 |
| 指标/Pine | `pine_list_scripts`, `pine_get_source`, `pine_open`, `pine_save`, `pine_compile`, `pine_smart_compile`, `pine_analyze`, `pine_check`, `chart_manage_indicator`, `indicator_set_inputs`, `indicator_toggle_visibility` | 列出/读/改 Pine 源码，加载指标，改参数 |
| 数据读取 | `data_get_ohlcv`, `data_get_pine_labels`, `data_get_pine_boxes`, `data_get_pine_lines`, `data_get_pine_tables`, `data_get_study_values`, `data_get_indicator`, `data_get_equity`, `data_get_strategy_results`, `data_get_trades`, `quote_get` | 读 K线、读 Pine 绘制元素（标签/盒/线/表）、读报价 |
| UI/绘图/其他 | `draw_shape`, `draw_list`, `draw_clear`, `capture_screenshot`, `replay_start/step/stop/status`, `layout_list/switch`, `pane_*`, `tab_*`, `ui_*` | 在图上画形状、回放、切布局 |

**用户的缠论指标在 TV 中名为**：`"Chan Theory - CHANLUN | CZSC"`（skill 文件 L35 明示）。

### 1.2 用户缠论指标的输出接口（经验基线——来自 skill 文件）

| 输出对象 | MCP 读取方式 | 语义 | 颜色/格式规则 |
|---------|------------|------|-------------|
| **Box（中枢）** | `data_get_pine_boxes(verbose=true)` | 三段重叠形成的中枢区间 [ZD, ZG] | `bgColor=872064120`→橙色=线段级别（操作级）；`bgColor=856729599`→紫色=趋势级别；蓝色=笔级别 |
| **Label（买卖点+动能）** | `data_get_pine_labels(max_labels=100)` | 买卖点分类 + MACD 面积 | `" 1买 -4"` / `" 1卖(趋势) 116"` / `" 1卖(盘整) 0"` / `" 2卖预期 53"` / `" 3买 -44"` / 纯动能 `"  106"` |
| **Line（笔/线段）** | `data_get_pine_lines()` | 分型构造的笔、笔构造的线段 | — |
| **Table（状态表）** | `data_get_pine_tables()` | 指标的状态面板 | — |

### 1.3 用户已结算的 TV 指标设置（来自 skill §"TV指标设置"）

| 设置项 | 用户选择 | 原文依据 |
|-------|---------|---------|
| 笔模式 | **宽笔（新笔）** | 第81课答疑："那无所谓，只要是独立的就可以" |
| 特征序列包含 | **优化包含** | 第67课非包含处理 + 第71课同序列滚动合并 |
| 特征序列延续 | **严格延续** | 第67课有缺口必查第二特征序列 + 第78课硬约束 |

**关键结论**：TV 指标已锁在**新笔+优化包含+严格延续**组合。本仓库必须能复现此组合的输出。

### 1.4 差异对照表（TV 指标 vs 本仓库 src/newchan）

| 概念 | TV 指标输出 | 本仓库实装 | 一致性判定 | 认识论等级 |
|-----|------------|-----------|-----------|-----------|
| **K 线包含** | 内建（Pine 黑盒） | `a_inclusion.py :: _merge_loop` 带 `reset_dir_on_fractal` 参数（默认 False；金油比等大振幅标的用 True） | **结构一致**——TV 侧算法未开源，但语义与 skill §"缠论原文依据" 对齐 | L0（代数一致） |
| **分型** | 在 Label/Line 端点隐式表达 | `a_fractal.py :: Fractal` + `a_inclusion.py :: _is_fractal_pattern` | **结构一致** | L0 |
| **笔** | 线段模式由指标设置参数决定（宽笔/严笔/新笔） | `a_stroke.py :: strokes_from_fractals(mode="new"/"wide"/"strict")`——三模式齐备，TV 用户设 "宽笔（新笔）" | **接口对齐**：把 mode 固定为 `"new"` 即对应 TV 设置 | L0 |
| **线段** | 在 Box 的起止 + Line 中隐式 | `a_segment_v1.py :: segments_from_strokes_v1`（特征序列法，与 TV "优化包含+严格延续" 对应）；`a_segment_v0.py` 是参考实现（L78 硬约束被 v1 继承） | **接口对齐**（v1 匹配，v0 偏差 4.72x——谱系003已结算） | L0 |
| **中枢** | Box (`bgColor`区分 3 级) | `a_center_v0.py :: Center`（含 `zg/zd/gg/dd/g/d/zg_dynamic/zd_dynamic`）+ `a_zhongshu_v1.py` | **结构一致**——ZG=min(g1,g2), ZD=max(d1,d2) 公式一致；但 TV 用 bgColor 编码级别，本仓库用 `level_id`。**级别映射需要显式配置** | L0（代数）；级别语义 L2 待验证 |
| **买卖点** | Label `"1买/1卖/2买/2卖/2买预期/3买"` 文本 | `a_buysellpoint_v1.py :: BuySellPoint(kind="type1/type2/type3", side="buy/sell")` + `divergence_key` + `overlaps_with` | **结构一致**；TV 特有 `"预期"` 标志（未确认）对应本仓库 `confirmed=False`；TV `(趋势)/(盘整)` 修饰对应本仓库 `Divergence.kind="trend"/"consolidation"` | L0；"预期"语义 L2 待验证 |
| **背驰** | Label 数字（MACD 面积） + `(趋势)/(盘整)` 标签 | `a_divergence.py :: Divergence(force_a, force_c, kind, direction, dif_peak_a/c, hist_peak_a/c)` | **超集**——本仓库除 MACD 面积还做 DIF 峰值（T6）+ HIST 峰值（T7）三维力度（239号谱系：方向性力度）。skill §"注意事项" 明示："TV指标的MACD面积数字只作参考，严格背驰判定以NewChanlun引擎为准（方向性力度，非振幅力度）" | **本仓库严格性高于 TV**——有意为之 |
| **区间套** | 未直接画出，需要在多周期间对照 | `a_nested_divergence.py`（跨级别背驰搜索） | **接口对齐**（需用户切周期后对照） | L2 待验证 |

---

## 2. 定义依据

本报告引用的定义源：

- **用户经验基线**：`/Users/silencehan/.claude/skills/tradingview-chanlun/SKILL.md`（全文）——TV 指标的颜色-级别映射、Label 格式规则、已结算设置选择均由此提供
- **TV MCP 工具清单**：ToolSearch 注册表中前缀 `mcp__405c29ce-5a8f-4996-b408-da143dc39b65__` 的全部 75 个工具
- **本仓库缠论代码**：
  - `src/newchan/a_inclusion.py` L30-100（包含关系合并）
  - `src/newchan/a_stroke.py` L223-267（笔构造主函数 `strokes_from_fractals`，mode 参数三档）
  - `src/newchan/a_segment_v1.py`（特征序列线段，对应 TV "优化包含+严格延续"）
  - `src/newchan/a_center_v0.py` L42-117（Center 结构：ZG/ZD/GG/DD/G/D/zg_dynamic/zd_dynamic/development/level_id/terminated/termination_side）
  - `src/newchan/a_divergence.py` L36-82（Divergence 结构：force_a/c + dif/hist peak a/c）
  - `src/newchan/a_buysellpoint_v1.py` L37-65（BuySellPoint 三类 + side + overlaps_with）
- **缠论原文**：`缠论知识库.md` §2-§11 + `docs/chanlun/text/blog/INDEX.md`（特别是第62课/第67课/第71课/第77课/第78课/第81课）
- **谱系**（已结算）：`001-degenerate-segment.md`（v0/v1 线段概念分离）、`083-clean-terminate-d-strategy.md`（笔定义）、`194-bi-fidelity-audit-10key.md`（笔保真度）、`237-t6t7-tristate-diagnosis.md`（三维力度诊断）、`239-directional-force.md`（方向性力度）

---

## 3. 边界条件（结论会翻转的条件）

1. **Pine 源码与 skill 描述不一致**：若后续 `pine_get_source` 拿到的源码显示 TV 指标内部用的不是"ZG=min(g1,g2), ZD=max(d1,d2)"的原文公式（例如用的是"所有构筑段重叠"的变体口径，§6.3 变体），则 `a_center_v0.py::_zseg_interval` 的公式对齐失败，需要引入 TV 变体口径——对照表中枢行从"结构一致"翻转。

2. **bgColor 级别映射不稳定**：若 TV 用户在不同 layout/参数组合下 `bgColor=872064120` 对应的级别语义不同（如在某些设置下橙色代表笔级别而非线段级别），则本仓库 `level_id` 直接映射失败。

3. **MACD 参数不匹配**：若 TV 指标用的 MACD 参数不是 (12,26,9)，则本仓库 `a_macd.py` 计算的面积数字与 Label 数字比值不为 1——背驰**相对排序**可能仍一致（定性对齐），**绝对值**不对齐（定量失配）。

4. **"预期"语义与 confirmed 映射歧义**：TV `"2卖预期"` 可能包含两类：笔尚未完成的预判 + 笔已完成但更高级别未确认。本仓库 `confirmed=False` 只覆盖第一类。**此边界需要在 L2 验证中查看至少 5 个 `"预期"` label 的时序对照**。

5. **新笔"3根K线"是否考虑包含关系**：`缠论知识库.md` §4.3 明示此口径未定，谱系001待验证。若 TV 指标选了"考虑包含后 3 根"，本仓库 `a_stroke.py::_check_gap` 的 `merged_gap >= 2 and raw_gap >= 3` 双约束需要验证是否与 TV 一致——否则宽笔新笔边界处笔数不等。

6. **特征序列"优化包含"vs"同序列滚动合并"细节**：第71课原文未完全公开源码层细节，TV 指标的具体实现可能与本仓库 `a_segment_v1.py` 不完全一致，导致边界处线段端点偏移 1-2 笔。

---

## 4. 下游推论

若本报告的对照结论成立（且 L2 验证未否证）：

1. **本仓库级别映射需要显式配置层**：新增 `.chanlun/definitions/level-mapping.md`（目前不存在），定义 `bgColor(TV) ↔ level_id(本仓库)` 的注册表。建议格式：
   ```
   { "872064120": {"level_name": "操作级别", "level_id_default": 1},
     "856729599": {"level_name": "趋势级别",  "level_id_default": 2},
     "blue":      {"level_name": "笔级别",    "level_id_default": 0} }
   ```
   当前 `a_center_v0.py::Center.level_id` 由递归引擎填，但填什么数没有与 TV 语义对齐——这是断口。

2. **买卖点"预期"字段需要显式化**：本仓库 `BuySellPoint` 目前用 `confirmed: bool` 二元，但 TV 的"预期"本身是一类语义事件（尚未形成但结构已就绪）。建议在 `a_buysellpoint_v1.py` 增加 `expected: bool` 字段或把 `confirmed` 扩为三态（`expected/unconfirmed/confirmed`）——但这是**概念演化**，必须走谱系流程（不要直接改）。

3. **力度度量的双轨共存**：本仓库的三维力度（MACD 面积 + DIF 峰值 + HIST 峰值）与 TV 的单维 MACD 面积**不应收敛到同一数字**——它们是两套严格性口径。验证协议应**分别对照**：本仓库 `force_c/force_a` 的**排序**是否与 TV Label 数字比值的**排序**一致（定性），而非绝对值相等。这是 skill 文件已明示的立场（§"注意事项"）。

4. **线段 v0 应从对照集移出**：谱系003已将 v0 降级为参考实现。本仓库对照 TV 应**只用 v1**（`a_segment_v1.py`）。任何 v0 与 TV 的数字差异都不构成反例，反之 v1 与 TV 的差异才是有效信号。

5. **区间套未在 TV 直接画出**——需要在 MCP 侧做"双周期 session"：同一标的 D 周期 + 60 周期分别读 Label，在时间轴上取交（日线 C 段时间窗内的 60 分钟背驰点），再与本仓库 `a_nested_divergence.py` 的输出对照。这是 L2 协议的核心步骤。

---

## 5. 谱系引用

- **001号 degenerate-segment**（已结算）：线段概念分离（v0 三笔重叠 vs v1 特征序列法）。v1/v0 差异 4.72x。本报告确认对照**只用 v1**。
- **002号 source-incompleteness**（已结算）：编纂版相对博文有遗漏（古怪线段、新笔、第78课硬约束），本仓库已补录。TV 指标的"新笔+严格延续"设置与此谱系一致。
- **083号 clean-terminate-d-strategy**（已结算）：笔定义清洗终结策略。
- **194号 bi-fidelity-audit-10key**（已结算）：笔保真度 10 项 key 审计——对照 TV 的单标的 Label 端点序列应使用此审计的口径。
- **237号 t6t7-tristate-diagnosis**（已结算）：三维力度 D2 弱方向吸收根因诊断。解释了为什么本仓库背驰用 force + dif_peak + hist_peak 三维，而 TV 只用 MACD 面积一维。
- **238号 multi-tf-architecture**（已结算）：多 TF 输入架构。TV 的"多周期区间套"能力由本仓库 `topology/multi_tf_adapter.py` 对应。
- **239号 directional-force**（已结算）：方向性力度。skill 文件"注意事项"直接引用了此谱系的结论。

**不存在的谱系**（可能的断口）：
- **"TV 级别映射"谱系**——本仓库未记录 bgColor 与 level_id 的正式对照（见 §4.1 推论）。
- **"买卖点预期语义"谱系**——本仓库 `confirmed` 二元与 TV"预期"三元不对齐（见 §3.4 边界）。

---

## 6. 影响声明

**本产出改动了什么**：仅写入 `.chanlun/pdf-review/tradingview-mcp-probe.md`（本文件）。

**本产出影响了哪些模块/定义**：
- **零代码改动**——符合任务约束"不要修改任何本仓库代码"
- 指向 `a_center_v0.py::Center.level_id` 的级别语义断口（§4.1）
- 指向 `a_buysellpoint_v1.py::BuySellPoint.confirmed` 的三态需求断口（§4.2）
- 指向 `a_stroke.py::_check_gap` 的"3根K线是否考虑包含"边界（§3.5）

**本报告不新增任何定义**——审计只标注现状与差距。

---

## 7. 可执行验证协议（不跑，只记录管线）

### 7.1 最小验证用例

| 维度 | 选择 | 理由 |
|-----|------|------|
| 标的 | **CL1!（WTI原油期货近月）** 或 用户操盘的 Brent | memory `user_trading_direction.md` 载：用户押注金油比下降/油涨，线段一买——需要 Brent/WTI 的日线作最小用例 |
| 周期 | 日线 (`"D"`) + 60 分钟 (`"60"`)（双周期做区间套对照） | skill §"多周期区间套"流程 |
| 时间窗 | 最近 2 年（2024-04 至 2026-04） | 覆盖 2025-02-20 regime 断点（user memory 载） |
| 对照项 | Box（中枢）端点 + Label（买卖点+动能）文本 + Line（笔/线段）端点 | skill §"TV MCP 核心工具映射" 的 5 个 `data_get_*` |

### 7.2 管线（伪代码，需 MCP 恢复后执行）

```python
# Phase A — 从 TV 侧拉数据
tv_health_check()
chart_set_symbol(symbol="NYMEX:CL1!")
chart_set_timeframe(timeframe="D")
# 等待数据加载
tv_boxes = data_get_pine_boxes(verbose=True)
tv_labels = data_get_pine_labels(max_labels=500)
tv_lines = data_get_pine_lines()
tv_ohlcv = data_get_ohlcv(summary=True)  # 日线 K 线

# Phase B — 本仓库对同一 OHLCV 跑
from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged
from newchan.a_stroke import strokes_from_fractals
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_center_v0 import centers_from_segments_v0
from newchan.a_divergence import detect_divergences
from newchan.a_buysellpoint_v1 import buysellpoints_from_level

df_raw = tv_ohlcv_to_dataframe(tv_ohlcv)
df_merged, merged_to_raw = merge_inclusion(df_raw)
fxs = fractals_from_merged(df_merged)
strokes = strokes_from_fractals(df_merged, fxs, mode="new", merged_to_raw=merged_to_raw)
segments = segments_from_strokes_v1(strokes)  # v1 对应 TV"优化包含+严格延续"
centers = centers_from_segments_v0(segments)
# ...背驰 + 买卖点

# Phase C — 逐项对照
compare_boxes(tv_boxes, centers)          # ZD/ZG 端点对齐？bgColor ↔ level_id 映射稳定？
compare_labels(tv_labels, bsps)            # "1买"↔type1-buy？"预期"↔confirmed=False？动能数字相关性？
compare_lines(tv_lines, strokes, segments) # 端点索引对齐？
```

### 7.3 预期产出表

| 对照项 | 通过条件 | 否证条件 |
|-------|---------|---------|
| Box 数量 | 本仓库 centers 数 = TV boxes 中 orange+purple 总数（笔级别不对照） | 偏差 > 10% |
| Box 端点 | 每个 center.low/high ≈ 对应 box.y_bottom/y_top（浮点容差 1 tick） | 端点偏差 > 2 ticks |
| Label 买卖点 | 本仓库 BuySellPoint 与 TV Label"1买/1卖/2买/..."一一对应 | 超过 10% BSP 在 TV 中无对应 label，或反之 |
| 动能数字排序 | 本仓库 `force_c / force_a` 的排序与 TV Label 数字比值的排序相关系数 > 0.8 | 相关系数 < 0.6（方向性力度确实与 MACD 面积不等价的证据） |
| 笔端点 | strokes[i].i0 对应的 bar_time = TV line 起点 bar_time | 端点偏差 > 1 根 K 线 |

**任一否证条件出现**：触发谱系写入流程，不修代码。

### 7.4 L2 验证等级标注（强制，231号规则）

- 本报告本身是 **L0**（结构对照，纯定义/代数层）
- §7.2 管线执行后是 **L2**（真实数据单标的假设检验）
- 多标的（至少 3 个：Brent + 金油比 + AAPL）执行后是 **L3**

**L0→L2 的信息增量才是正的**。仅有本报告不足以声明"本仓库与 TV 对齐"，必须 L2 产出否定/确认性证据。

---

## 8. 行动建议（按优先级排序，供 team-lead 评估）

1. **[P0] 等分类器恢复后立即执行 §7 协议的 Phase A**（单纯拉 TV 数据、本仓库跑一次、存 JSON），此步骤零代码改动。
2. **[P1] 新增 `.chanlun/definitions/level-mapping.md`**（§4.1 推论），把 bgColor ↔ level_id 显式化为定义文件。
3. **[P2] 对照结果若显示"预期"语义断口真实存在，开谱系讨论 `BuySellPoint.expected` 字段**（§4.2 推论）——走谱系流程，不直接改。
4. **[P3] 把 v0 从所有 TV 对照脚本中移除**（§4.4 推论），只用 v1。

---

## 9. 执行日志（本次审计）

| 时间 | 事件 |
|-----|------|
| t0 | 收到 team-lead 指令，ToolSearch 加载 TV MCP 工具 schemas |
| t0+1 | 首次 `tv_health_check` → auto 模式分类器不可用 |
| t0+2..t0+5 | 4 次重试 `tv_health_check`、`pine_list_scripts` → 持续分类器限制 |
| t0+6 | 转入诚实记录路径——基于 `/Users/silencehan/.claude/skills/tradingview-chanlun/SKILL.md` 完成 Phase 1/2 |
| t0+7..t0+12 | 读本仓库 `a_inclusion.py`、`a_stroke.py`、`a_center_v0.py`、`a_divergence.py`、`a_buysellpoint_v1.py` |
| t0+13 | 读 `缠论知识库.md` 全文，对齐原文定义编号 |
| t0+14 | 本报告写入 |

**重要**：执行过程中**未修改任何代码**，**未绕过 MCP 不可用**（每次重试都诚实记录错误），**未伪造 MCP 输出**。
