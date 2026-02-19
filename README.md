# NewChanlun

缠论量化分析引擎 — 从原始 K 线到买卖点的全递归形式化实现。

## 概述

NewChanlun 是一个基于**缠中说禅**技术分析理论的量化引擎，将缠论的核心概念（笔、线段、中枢、走势类型、背驰、买卖点）形式化为可计算的事件驱动管线。

**核心特点：**

- **递归级别体系** — 级别 = 递归层级（level_id），从 1 分钟 K 线为底座无限向上构造，不依赖时间周期
- **四层同构管线** — BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine，每层结构相同
- **事件驱动架构** — 所有状态变更通过 DomainEvent 传播，支持增量计算和实时流
- **形式化验证** — 1300+ 测试用例，22 条不变量断言覆盖全管线

## 架构

```
原始 K 线
  │
  ▼
包含处理 (a_inclusion.py)
  │
  ▼
分型识别 (a_fractal.py)
  │
  ▼
笔引擎 (bi_engine.py)              ← BiEngine：事件驱动
  │
  ▼
线段引擎 (core/recursion/)          ← SegmentEngine：特征序列法
  │
  ▼
中枢引擎 (core/recursion/)          ← ZhongshuEngine：三段重叠
  │
  ▼
走势类型 (core/recursion/)          ← MoveEngine：盘整/趋势识别
  │
  ▼
背驰检测 (a_divergence.py)          ← MACD 面积比较
  │
  ▼
买卖点 (a_buysellpoint_v1.py)       ← 一二三类买卖点
```

## 快速开始

### 环境要求

- Python >= 3.10
- Node.js >= 18（前端可视化，可选）

### 安装

```bash
# 克隆仓库
git clone https://github.com/xy7365527-lang/NewChanlun.git
cd NewChanlun

# 创建虚拟环境
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate

# 安装依赖
pip install -e ".[test]"

# 配置环境变量
cp .env.example .env
# 编辑 .env 填写 API 密钥
```

### 数据获取

```bash
# 从 Databento 拉取历史数据
newchan fetch-db --symbol CL --intervals 1min,1day --start 2020-01-01

# 从 Interactive Brokers 拉取实时数据
newchan fetch --source ibkr --symbol ES --interval 1min

# 生成合成标的（价差/比价）
newchan synthetic --a CL --b GC --op ratio
```

### 图表可视化

```bash
# 交互式蜡烛图（TradingView 风格）
newchan plot --symbol CL --display-tf 5m

# 启动 Web 图表服务
newchan chart --port 8765
```

### 前端开发

```bash
cd frontend
npm install
npm run dev
```

### 运行测试

```bash
pytest                    # 全量测试
pytest -x                 # 遇到失败即停
pytest -m "not slow"      # 跳过慢测试
```

## 项目结构

```
src/newchan/
├── a_*.py                 # 纯函数层（A 层）：无状态算法
│   ├── a_inclusion.py     #   包含关系处理
│   ├── a_fractal.py       #   分型识别
│   ├── a_stroke.py        #   笔构造
│   ├── a_segment_v1.py    #   线段（特征序列法）
│   ├── a_zhongshu_v1.py   #   中枢认定
│   ├── a_move_v1.py       #   走势类型实例
│   ├── a_divergence.py    #   背驰检测
│   └── a_buysellpoint_v1.py # 买卖点识别
├── core/recursion/        # 事件驱动引擎层
│   ├── bi_engine.py       #   笔引擎
│   ├── segment_engine.py  #   线段引擎
│   ├── zhongshu_engine.py #   中枢引擎
│   └── move_engine.py     #   走势引擎
├── audit/                 # 不变量检查器（I1-I22）
├── orchestrator/          # 多周期编排
├── events.py              # 领域事件定义
├── types.py               # 核心数据类型
├── cli.py                 # 命令行入口
├── server.py              # FastAPI 后端
├── equivalence.py         # 等价关系验证
├── flow_relation.py       # 流转关系（四矩阵有向流场）
├── ratio_engine.py        # 比价引擎
└── data_*.py              # 数据源适配器（IBKR/Databento/AV）

frontend/                  # React + TypeScript 前端
├── src/
│   ├── primitives/        #   TradingView 图元（K线/笔/段/中枢/事件标记）
│   ├── components/        #   UI 组件
│   ├── hooks/             #   数据钩子
│   └── types/             #   TypeScript 类型定义

tests/                     # 测试套件（1300+ 用例）
docs/spec/                 # 规则规范文档
```

## 数据源

| 数据源 | 用途 | 配置 |
|--------|------|------|
| [Databento](https://databento.com/) | 美股 + 期货历史数据 | `DATABENTO_API_KEY` |
| [Interactive Brokers](https://www.interactivebrokers.com/) | 实时行情 + 交易 | `IB_HOST` / `IB_PORT` |
| [Alpha Vantage](https://www.alphavantage.co/) | 辅助数据 | `ALPHAVANTAGE_API_KEY` |

## 缠论概念映射

| 缠论概念 | 代码实体 | 规范文档 |
|----------|----------|----------|
| 包含关系 | `merge_inclusion()` | — |
| 分型 | `Fractal` | — |
| 笔 | `Stroke` / `BiEngine` | — |
| 线段 | `Segment` / `SegmentEngine` | `segment_rules_v1.md` |
| 中枢 | `Zhongshu` / `ZhongshuEngine` | `zhongshu_rules_v1.md` |
| 走势类型 | `Move` / `MoveEngine` | `move_rules_v1.md` |
| 背驰 | `Divergence` | — |
| 买卖点 | `BuySellPoint` | `maimai_rules_v1.md` |
| 级别递归 | `LevelView` / `LStar` | `level_recursion_interface_v1.md` |

## 形式化方法

本项目采用**概念优先于代码**的形式化方法：

1. **定义基底** — 12 个核心概念的条件-结论条款（`definitions.yaml`）
2. **谱系追踪** — 38 条已结算的概念发现记录（`.chanlun/genealogy/`）
3. **三级权威链** — 缠师原始博文 > 编纂版 > 第三方总结
4. **不变量断言** — 22 条运行时可检查的形式约束（`audit/`）

## 许可证

本项目仅供学习研究使用。缠论相关内容版权归原作者所有。
