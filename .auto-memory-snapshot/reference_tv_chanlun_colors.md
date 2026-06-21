---
name: TV Chanlun indicator color mapping
description: TradingView Chan Theory CZSC指标的颜色-级别映射：蓝=笔，橙=线段，紫=趋势
type: reference
originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---
TradingView "Chan Theory - CHANLUN | CZSC" 指标的Box颜色映射：

- **蓝色** = 笔级别中枢（三笔重叠）
- **橙色**（bgColor=872064120）= 线段级别中枢（三段重叠）
- **紫色**（bgColor=856729599）= 趋势级别中枢（多个线段级别中枢构成）

MU日线图加载了两个同名指标实例，参数不同以显示不同级别结构。

Labels格式：
- 买卖点：`" 1卖(趋势) 116"` — 类型+MACD面积
- 纯数字：`"  106"` — 笔/段端点的MACD面积（正=上升，负=下降）
- 背驰判定：后段MACD面积 < 前段 = 背驰

**Why:** 用户直接告知的指标设置，不是推断
**How to apply:** 读TV pine_boxes时按bgColor区分级别，橙色=操作级别中枢，紫色=大级别支撑/阻力
