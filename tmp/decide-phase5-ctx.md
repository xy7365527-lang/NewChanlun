# Phase 5 Roadmap Derive Context

## 任务
基于 NewChanlun 项目第四阶段全部完成（12/12 objectives），派生第五阶段 roadmap 目标。
模式：derive（推导新 roadmap 目标，不是质询已有产出）

## 已完成能力清单（第四阶段结束状态）

### A系统（缠论引擎）
- 包含处理（a_inclusion.py）✅
- 分型识别（a_fractal.py）✅
- 笔引擎（a_stroke.py + bi_engine.py，mode='new' 为默认）✅
- 线段（a_segment_v1.py，特征序列法）✅
- 中枢（a_zhongshu_v1.py，充要条件已验证）✅
- 走势类型（a_trendtype_v0.py + a_move_v1.py）✅
- 背驰（a_divergence_v1.py，T2/T6/T7 OR 逻辑）✅
- 买卖点（a_buysellpoint_v1.py，三类）✅
- 级别递归（a_recursive_engine.py + RecursiveOrchestrator）✅
- 小转大（a_xiaozhuan_da.py）✅
- 嵌套背驰（a_nested_divergence.py）✅

### B系统（可视化）
- b_plot.py：K线/笔/线段/中枢/买卖点渲染 ✅
- b_chart.py：同上 ✅

### 桥接层
- ab_bridge_newchan.py：全链路（含嵌套背驰可选）✅

### 回放引擎
- gateway.py + replay.py：单TF/多TF回放，RecursiveOrchestrator 全链路 ✅
- WsSnapshot：全链路字段（strokes/segments/centers/moves/bsp/lstar）✅

### 端点
- /api/overlay ✅
- /api/nested_divergence ✅
- /api/replay/* ✅

### 测试
- 2044 passed, 15 skipped

### 定义
- 14个已结算定义
- 216个谱系已结算

## 定义文件中的未决项（概念层缺口）

### baohan.md
- 初始方向 None → UP 的合理性：原文未明确，默认 UP 是工程选择，尚无原文依据

### beichi.md
- T6（黄白线高度/DIF峰值）和 T7（柱子伸长高度/HIST峰值）工具函数已实现，但 #2 or/and 关系未结算
- 当前 OR 逻辑：T2 OR T6 OR T7，但 T6/T7 是否应该是 AND 关系（同时满足才算背驰）未从原文确认

### bi.md
- wide vs strict 模式语义：wide（gap>=4 merged）= 旧笔；strict（gap>=5 merged）无原文对应，可能是经验值

### liuzhuan.md
- 流转关系（dengjia→bijia→liuzhuan）定义存在但实现不完整
- flow_relation.py 存在但规范文件 flow_relation_v1.md 待建

### level_recursion.md
- 组件完成判定不严格：用 Segment 直接构造中枢时，未验证 Segment 是否构成完整的次级别走势类型
- 递归链未完成时的候选中枢是"暂态"的

## 关键源文件
- src/newchan/server.py（622行）— FastAPI 服务端
- src/newchan/gateway.py — WebSocket 回放网关
- src/newchan/orchestrator/recursive.py — 递归调度器
- src/newchan/ab_bridge_newchan.py — A→B 桥接
- src/newchan/b_plot.py, b_chart.py — 可视化
- src/newchan/a_divergence_v1.py — 背驰（T2/T6/T7）
- src/newchan/flow_relation.py — 流转关系（不完整）
- src/newchan/data_av.py — AlphaVantage 数据
- src/newchan/data_databento.py — Databento 数据
- src/newchan/data_databento_live.py — 实时数据

## 派生维度

请从以下四个维度推导第五阶段目标：

### 维度1：A系统→实战应用的缺口
- 信号质量评估：买卖点信号的历史胜率统计
- 回测框架：基于买卖点信号的简单回测（入场/出场/收益）
- 实时数据接入：data_databento_live.py 已存在但与 RecursiveOrchestrator 的集成状态未知

### 维度2：B系统→用户体验的缺口
- 前端交互：当前 server.py 有静态文件服务，但前端交互能力未知
- 多标的对比：当前回放是单标的，多标的对比视图缺失
- 实时更新：WebSocket 回放已有，但实时推送（非回放）的集成状态未知

### 维度3：工程质量缺口
- 性能：RecursiveOrchestrator 在大数据集上的性能未测试
- 错误处理：server.py 端点的错误处理完整性
- 监控：无运行时监控/指标收集

### 维度4：概念层缺口
- beichi.md #2：T6/T7 or/and 关系未结算
- liuzhuan.md：流转关系实现不完整
- baohan.md：初始方向合理性未结算

## 输出要求

输出格式：YAML tasks 节，每个目标包含：
- id: snake_case
- title: 简洁中文标题
- priority: P1（阻塞实战）或 P2（提升质量）
- status: active
- source: 来源说明
- description: 详细描述
- constraints: 约束列表
- subtasks: 子任务列表（每个含 id/title/description）
- decomposition: parallel

优先级原则：
- P1 = 阻塞实战使用的缺口（没有它就无法在真实市场中使用）
- P2 = 提升质量但不阻塞

不要重复已完成的目标。
