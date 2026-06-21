---
name: todo-master
description: 量化系统全局待办清单（2026-06-08），按依赖关系排序
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 当前阻塞项（并行进行中）

### A. Databento 1min 10年数据拉取（进行中）
- ES/GC/CL/ZN/DX/BRN 全期货，ohlcv-1m
- CME + ICE Europe 订阅已生效
- 依赖：无 → 是下游所有任务的前置

### B. 引擎性能优化v2（进行中）
- attach_persistence 超线性增长 ~N^1.86 → 目标O(N)或O(N logN)
- 并行化6条边（multiprocessing）
- 比价OHLC构造向量化
- 依赖：无 → 是下游计算任务的前置

## 阻塞解除后的任务队列

### 1. 1min a0 K4配置转换矩阵（依赖A+B）
- 6条边1min比价OHLC → 缠论递归 → σ → Γ → 27态转换矩阵
- 10年数据看配置转换路径、吸收态、平均驻留时间

### 2. 闭合残差缠论递归（依赖A+B）— 验证耗散结构假说
- 4个三角形的走势结构层闭合残差时间序列
- 对每个残差做缠论递归：有结构（趋势/中枢/背驰）= 耗散结构成立
- 2020-2022历史验证：空转/走资/沉没的拓扑签名

### 3. Γ→Δ映射1min版重验（依赖A+B+COT数据）
- 1min Γ + COT持仓数据 → Δ
- 重验H(交叉边|Γ)和ω分辨能力
- 搜索补齐剩余熵的变量

### 4. 跨国K4管线实装验证（依赖A）
- cross_national_pipeline.py已写好
- 用真实1min数据（ES/BRN + EURUSD/USDJPY）验证跨国闭合
- 中国数据仍缺（akshare待拉）

### 5. 递归分解树纵向闭合实测（依赖A+B）
- P顶点（ES）分解为成分股
- 纵向闭合：ES/CL vs 各成分stock_i/CL
- 选股信号：哪些成分驱动宏观趋势

### 6. M1操盘层E版本1min全标的重跑（依赖A+B）
- 用优化后引擎+1min 10年数据
- E版本（背驰定位器）在ES/GC/CL/ZN上的表现
- 和BH对比，看不同宏观周期的alpha分布

### 7. M2选股C路径端到端回测（依赖1+2+6）
- 用1min a0的K4联合读数驱动选股
- 多品种组合：C路径排序→资金配置→E版本操盘
- 验证"排序优于门控"的结论在10年数据上是否稳健

## 独立任务（不阻塞）

### 8. 中国市场数据拉取
- akshare拉IF/AU9999/SC 1min数据
- 或TradingView拉取

### 9. Git全量commit
- 大量未提交的代码和文档
- K4折叠重构、递归分解树设计文档、分析脚本等

### 10. ROADMAP更新
- docs/ROADMAP.md 需要反映当前进展
- M1状态：E版本有alpha（OKLO +932%），降成本/挣股数/完整版待重验
- M2状态：K4折叠重构完成，C路径验证，递归分解树设计中

**Why:** 追踪所有待办，防止遗漏
**How to apply:** 每完成一项打勾，新增任务追加

Related: [[quant-system-progress]], [[alpha-diagnosis]]
