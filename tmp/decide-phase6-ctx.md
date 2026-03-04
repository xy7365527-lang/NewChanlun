# Phase 6 Roadmap Derive Context

## 任务
基于 NewChanlun 项目第五阶段全部完成（4/4 objectives），派生第六阶段 roadmap 目标。
模式：derive（推导新 roadmap 目标）

## 已完成能力清单（第五阶段结束状态）

### A系统（缠论引擎）— 全链路完成
- 包含处理 → 分型 → 笔 → 线段 → 中枢 → 走势类型 → 背驰 → 买卖点 ✅
- 级别递归（RecursiveOrchestrator）✅
- 小转大 ✅ | 嵌套背驰 ✅
- 流转关系（dengjia→bijia→liuzhuan）测试补全 ✅
- 递归中枢严格化（组件完成判定）✅
- beichi T6/T7 结算 ✅

### B系统（可视化）
- b_plot.py / b_chart.py：K线/笔/线段/中枢/买卖点渲染 ✅

### 回测与信号评估（第五阶段新增）
- backtest.py：回测引擎（anti-lookahead-bias, per-BSP-type 统计）✅
- signal_quality_report.py：信号质量统计脚本 ✅

### 实时数据（第五阶段新增）
- live_engine.py：Databento Live → RecursiveOrchestrator 增量驱动 ✅
- gateway.py /ws/live/{symbol}：实时 WebSocket 推送 ✅

### 工程质量（第五阶段新增）
- API 错误处理强化（server.py/gateway.py 统一错误中间件）✅
- 性能基准测试（performance_benchmark.py）✅

### 测试
- 2138 passed, 15 skipped

### 定义与谱系
- 14 个已结算定义
- 217 个谱系已结算

## 定义文件中的未决项（概念层缺口）

### baohan.md
- 初始方向 None → UP 的合理性：原文未明确，默认 UP 是工程选择

### bi.md
- wide vs strict 模式语义：wide（gap>=4 merged）= 旧笔；strict（gap>=5 merged）无原文对应

### level_recursion.md
- 递归链未完成时的候选中枢是"暂态"的（已标注，非阻塞）

## 当前系统缺口分析

### 维度1：回测→实战的缺口
- 回测引擎已有，但缺少：策略组合框架（多信号组合）、风控模块（止损/仓位管理）、滑点/手续费模型
- 信号统计已有，但缺少：多标的横向对比、信号时效性分析

### 维度2：实时系统→生产的缺口
- live_engine 已有，但缺少：断线重连、数据缺失补偿、多标的并发管理
- WebSocket 推送已有，但缺少：客户端认证、连接管理、背压控制

### 维度3：前端交互缺口
- server.py 有静态文件服务，但前端交互能力有限
- 缺少：实时图表更新、交互式回放控制、多标的切换

### 维度4：数据源扩展
- 当前仅 AlphaVantage（美股）+ Databento（期货/实时）
- 缺少：A股数据源、加密货币数据源

### 维度5：概念层剩余缺口
- baohan.md 初始方向（低优先级，不阻塞实战）
- bi.md wide/strict 语义（低优先级）

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
- P1 = 阻塞实战使用的缺口
- P2 = 提升质量但不阻塞

不要重复已完成的目标。
