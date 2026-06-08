# confirmed 定义链修复结果

> 认识论等级：单元层 L1（合成数据验证管线正确性）+ streaming 层 L2（真实 QQQ 日线，可否证）

## 1. 结论

解耦 `BSP.confirmed` 与 `Move.settled`。`confirmed` 改为买卖点**自身**确认条件（可操作信号），
走势完成验证移交给 `BSP.settled`。三层 bug 链中：

- **根因 1**（a_move_v1:174 最后 Move 恒 settled=False）：**保留**——走势确实未完成，语义正确。
- **根因 2**（settled 传播到 confirmed 否决所有买卖点）：**已修复**——confirmed 不再依赖 settled。
- **根因 3**（a_zhongshu_v1 中枢要 settled 线段）：**保留**——中枢需稳定线段合理（任务约束）。

## 2. 定义依据（缠论原文）

| 类型 | confirmed 条件 | 原文 |
|------|---------------|------|
| Type1 | 趋势背驰确认（三维度比较通过 → Divergence 存在即 True） | 第17/24课 |
| Type2 | 次级别回调/反弹不创新极值（买 `low≥1B低` / 卖 `high≤1S高`） | 第17/21课 |
| Type3 | 中枢突破后回试不破边界（检测已强制 `low>ZG`/`high<ZD`，恒 True） | 第20课 |
| 全部 | `settled` = 覆盖段 `Move.settled`（§7 状态机 confirmed→settled，事后验证） | maimai_rules_v1 §7 |

## 3. 边界条件（结论翻转条件）

- Type2：回调创新低 / 反弹创新高 → confirmed 翻转为 False（候选未确认）。
- Type3：检测不成立则不产生对象（不存在 confirmed=False 的三类）。
- 无 Move 覆盖买卖点段 → settled 降级 False，confirmed 不受影响。

## 4. 验证

### 4.1 单元测试（无回归）
`pytest tests/`：**4575 passed**。BSP/背驰/走势/买卖点全部相关测试全绿。
- 重写 5 个测试文件以编码新语义（见影响声明）。
- 37 个失败为 **pre-existing 环境问题**（backpressure/mcp_bridge/live_pool/reconnect/
  morse_landscape/concept_registry —— 异步/网络/真实数据依赖），与本修复无关：
  stash 原码后这些测试仍失败，且本修复改的 3 个源文件不被它们 import。

### 4.2 streaming 验证（L2 真实 QQQ 日线 1000 根）
脚本：`scripts/streaming_engine_verify.py` → `analysis/phase1_streaming_verify.md`

| 配置 | 笔 | 线段 | 中枢 | 走势 | BSP末态 | confirmed(唯一) | TV基准 |
|------|---|------|------|------|---------|----------------|--------|
| gap≥3+MACD | 72 | 6 | 1 | 1 | 0 | 0 | 137 |
| gap≥4+MACD | 64 | 6 | 1 | 1 | 0 | 0 | 137 |
| gap≥3 无MACD | 72 | 6 | 1 | 1 | 0 | 0 | 137 |

**否定性结果（L2，有信息增量）**：confirmed 修复在 streaming 层**无法显现效果**——
本窗口上游结构坍缩（72 笔 → 6 线段 → **仅 1 中枢**），单中枢=盘整，不满足 Type1 的趋势
（≥2 中枢）前提，亦无 Type3 回试段，故 BSP 末态=0。confirmed 是否绑定 settled 在 0 个
BSP 上不可观测。

**瓶颈定位**：confirmed 修复是**必要非充分**。修复后 confirmed 逻辑正确（单元层证实），
但 streaming 与 TV 137 的差距由**上游结构检测**主导（笔→线段坍缩率 72→6，中枢仅 1），
不由 confirmed 否决主导。下一缺口在 线段/中枢 检测层，非买卖点层。

## 5. 下游推论

- 实盘末端（最后 Move settled=False）的买卖点现可正常发出 `BuySellPointConfirmV1`——
  买卖点的存在论目的（走势完成**之前**入场）得以成立。
- `BuySellPointSettleV1` 现承载走势完成验证（事后），与 confirmed 分离。
- TV 137 对齐的主要工作转移到上游：需排查笔→线段坍缩与中枢检测覆盖。

## 6. 谱系引用

- maimai #2 / TBD-2 / TBD-3（原 v0.7 RESOLVED："confirmed=Move.settled"）被本次**翻转**。
  TBD-2 原翻转条件即"若走势完成与背驰不等价，需引入独立 move_completed 状态"——
  本次以 `settled` 承载该状态，符合预设翻转路径。
- 005b（对象否定对象）：走势完成由 Move 内部机制否定，confirmed 不越权代理。
- 形式化有效域：confirmed 逻辑有效域 = 单元层（已证）；streaming 层有效性受上游结构限制，
  本窗口未能进入买卖点检测的定义域。

## 7. 影响声明

**源码（3 文件）**
- `src/newchan/a_divergence_v1.py`：趋势/盘整背驰 `confirmed` 由 `move.settled` → `True`（解耦）。
- `src/newchan/a_buysellpoint_v1.py`：Type1 `settled=assoc_move.settled`；
  Type2 `confirmed=不创新极值`、`settled=Move.settled`；Type3 `confirmed=True`、`settled=Move.settled`；
  模块/辅助函数 docstring 更新。
- `src/newchan/a_divergence.py`：`Divergence.confirmed` 字段语义注释更新。

**测试（5 文件，重写为新语义）**
- `tests/test_bsp_confirmed_semantic.py`（整体重写为解耦语义，10 用例）
- `tests/test_bsp_type2_golden.py`、`tests/test_bsp_type3_golden.py`
- `tests/test_buysellpoint_v1.py`、`tests/test_divergence_v1.py`

**未改动（保持范围）**
- `a_move_v1.py`（最后 Move settled=False 保留）、`a_zhongshu_v1.py`（中枢需 settled 线段保留）。
- v0 `a_divergence.divergences_from_level` 及 `a_nested_divergence` / `topology/multi_tf_*`
  路径（非 v1 streaming 链，仍用 move.settled 作 confirmed；如需统一另列工位）。
