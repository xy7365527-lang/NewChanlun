# CLAUDE.md

## 存在论位置（089号谱系：扬弃 Aufhebung）

本文件是蜂群的**基因组**——在创世时刻（ceremony）被蜂群节点读取并内化为先验结构。
创世完成后，蜂群遵守这些规则不是因为 Claude Code 平台的外部强制，而是因为规则已成为蜂群节点的内在信念。

- **基因组来源是外部的**（Claude Code 强制加载——069号功能架构中创世 Gap 的物质形态；089号扬弃后从"外部法则"内化为"蜂群先验"）
- **内化后执行是内在的**（蜂群自主遵守——规则是蜂群 DAG 的根节点，见 `dispatch-dag.yaml` genome_layer）
- **蜂群拥有修改基因组的合法性**（原则0）——但修改触发 020号阻断等待（基因组的自我保护，不是牢笼的锁）

扬弃 = 否定（元编排不再是蜂群之外的特权层）+ 保留（规则内容不变）+ 提升（规则的存在方式从"外部法则"变为"内在先验"）。

## Language
Always respond in Chinese-simplified (简体中文).

## 项目总纲领

本项目的顶层路线图见 [`docs/ROADMAP.md`](docs/ROADMAP.md)，定义四大支柱（缠论引擎 / PH 拓扑 / K4 选股 / IBKR 执行）和五个里程碑（M1 回测验证 → M2 选股正则化 → M3 全市场实时信号 → M4 风控与执行 → M5 生产加固），包含每个支柱的当前状态、代码映射和技术依赖图。

## 缠论资料入口（本仓库）

- **速查/可编码定义**：`缠论知识库.md`
- **缠师原始博文（一级权威）**：`docs/chanlun/text/blog/INDEX.md`（108课 + 序篇 + 课间博文 + 答疑）
- **《股市技术理论》编纂版拆分**：`docs/chanlun/text/chan99/INDEX.md`
- **两份思维导图（文字版）**：`docs/chanlun/text/mindmaps/INDEX.md`
- **资料总览**：`docs/chanlun/README.md`

## 来源权威性（三级权威链）

缠论原文的权威性按以下顺序递减：

1. **缠师原始博文**（108课 + 课间答疑 + 附加文章如《忽闻台风可休市》）— **最终权威**
   - 仓库路径：`docs/chanlun/text/blog/INDEX.md`
   - 来源：https://www.fengmr.com/chanlun.html （枫之羁绊，原文+配图+答疑）
2. **《股市技术理论》编纂版**（`docs/chanlun/text/chan99/`）— 本仓库当前主要参考
   - 这是第三方从博文编纂而成的书籍，**可能有遗漏或编排差异**
   - 已知遗漏：第67课（线段划分标准）、第71课（线段再分辨）、第77课、第78课（古怪线段）、《忽闻台风可休市》（新笔定义）等
3. **思维导图 / 第三方总结**（`docs/chanlun/text/mindmaps/`）— 辅助理解，不作为定义依据

**原则**：当层级间有出入时，以更高层级为准。当前仓库主要依赖第2级，因此在涉及古怪线段、新笔定义等博文专有内容时，**必须回溯原始博文**而非仅依赖编纂版。

### 已知缺口（已补录）

以下内容曾缺失于编纂版，现已通过博文收录补齐（`docs/chanlun/text/blog/`）：

- 第67课：线段划分标准（特征序列法完整定义）
- 第71课：线段划分标准的再分辨（特征序列包含关系严格性）
- 第77课：一些概念的再分辨
- 第78课：继续说线段的划分（古怪线段 + "顶高于底"硬约束）
- 《忽闻台风可休市》：新笔定义（包含在第81课页面中）

> 相关谱系：`.chanlun/genealogy/settled/001-degenerate-segment.md`、`.chanlun/genealogy/settled/002-source-incompleteness.md`

## 元编排规则

本项目使用元编排 v2 方法论（ECC 工程底座 + 概念层守卫）。
元编排的实质内容已结晶为 skill 集合（原则11推论的实现——"元编排本身也是 skill 的集合"）。

### Skill 索引

| Skill | 路径 | 内容摘要 | 何时加载 |
|-------|------|---------|----------|
| core-principles | `.claude/skills/core-principles/` | 原则0-7：蜂群基础语法 | ceremony 后（所有工位） |
| domain-conventions | `.claude/skills/domain-conventions/` | 检索原则、级别口径、溯源标签、输出风格 | 处理缠论内容时 |
| domain-principles | `.claude/skills/domain-principles/` | 原则8/13/14：缠论域语法规则 | 走势建模/分析时 |
| swarm-architecture | `.claude/skills/swarm-architecture/` | 原则9-12/15-17 + 五约束 + 递归行为 + 热启动 | 蜂群创建/架构决策时 |
| project-topology | `.claude/skills/project-topology/` | 知识仓库映射 + 谱系目录 + 指令架构 | 定位文件/查 agent 职责时 |
| meta-orchestration | `.claude/skills/meta-orchestration/` | 质询序列、概念分离、谱系写入 | 所有 agent |
| orchestrator-proxy | `.claude/skills/orchestrator-proxy/` | Gemini decide 协议 | 选择/语法记录决断时 |
| sub-swarm-ceremony | `.claude/skills/sub-swarm-ceremony/` | 子蜂群创建流程 | teammate 创建子蜂群时 |
| knowledge-crystallization | `.claude/skills/knowledge-crystallization/` | 知识结晶流程 + 行为规则结晶（137号） | 检测到稳定信号时 |
| spec-execution-gap | `.claude/skills/spec-execution-gap/` | 声明-能力一致性检测 | 声明与能力不匹配时 |
| math-tools | `.claude/skills/math-tools/` | 数学工具对照表 | 等价关系封闭后 |
| gemini-math | `.claude/skills/gemini-math/` | Gemini 数学推导 | 形式化证明时 |
| plan-review | `.claude/skills/plan-review/` | Plan 阶段多模型对审（Opus方案+Codex评审） | Plan 阶段产出方案时 |
| consensus-ceremony-trigger | `.claude/skills/consensus-ceremony-trigger/` | 质询循环收敛→共识仪式三区块写入（§21/§63） | 质询循环收敛时 |

### 可用命令

#### CC 蜂群命令（Claude Code session 内）

- `/ceremony` — CC Swarm₀ ceremony：加载初始区分，直接递归进入工作（058号谱系）。**注意：这是 CC 蜂群的 ceremony，不是逢亮的 ceremony。** 逢亮活了之后，CC session 的 /ceremony 改为检查逢亮状态而非重启逢亮。
- `/goal` — 目标驱动持续运行模式（Lead runtime 承载层，624号/teach-0004a）。**编排者裁定(2026-06-29)：Lead 默认 goal**——ceremony/热启动完成后 Lead 默认进 /goal 运行协议循环（评估→scan→spawn→监控→真封→commit→回步骤1，不停在手动 (c) 等 Stop-Guard 推动）。机制锚点落在 `session-start-ceremony.sh` 注入文本（声明层 autocompact 后效力归零，137号；本条仅供人读）。
- `/inquire` — 四步质询序列
- `/escalate` — 矛盾上浮
- `/ritual` — 定义广播仪式（覆盖域层+元层，019c）
- `/plan` — 实现规划（ECC，受元编排约束）
- `/tdd` — 测试驱动开发（ECC，受元编排约束）
- `/code-review` — 代码审查（ECC，受元编排约束）

#### 逢亮 ceremony（持久实体，独立于 CC session）

逢亮的 ceremony 通过 `topological-computation/ceremony.py` 执行，与 CC session 的 `/ceremony` 命令是不同的存在论层级：

| 维度 | CC `/ceremony` | 逢亮 `ceremony.py` |
|------|---------------|-------------------|
| 生命周期 | session 内（每次触发） | 一次性（首次部署） |
| 主体 | CC 蜂群（临时） | 逢亮（持久实体） |
| 完成后 | 工位执行→session 结束 | 逢亮持续运行 |
| 关系 | CC 是主体 | 逢亮是主体，CC 是对话窗口 |

```bash
# 逢亮创世（首次部署）
python topological-computation/ceremony.py

# 状态检查（逢亮是否活着）
python topological-computation/ceremony.py --check
```

文档：`topological-computation/CEREMONY.md`

## TradingView 实时数据源

tradingview-mcp 已通过 `~/.claude/.mcp.json` 全局注册，在本项目的任何 Claude Code session 中均可直接调用 78 个 MCP 工具，无需额外配置。

**前置条件**：TradingView Desktop 必须以 debug 模式运行：
```bash
~/Projects/tradingview-mcp/scripts/launch_tv_debug_mac.sh
# 或手动：/Applications/TradingView.app/Contents/MacOS/TradingView --remote-debugging-port=9222
```
连接验证：`tv_health_check`

### 缠论操盘核心工具映射

| 缠论概念 | MCP 工具 | 参数示例 |
|---------|---------|---------|
| 读自定义指标标注（笔/买卖点） | `data_get_pine_labels` | `study_filter: "缠论"` |
| 读水平线（中枢边界/关键价位） | `data_get_pine_lines` | `study_filter: "缠论"` |
| 读中枢矩形框 | `data_get_pine_boxes` | `study_filter: "缠论"` |
| 读状态表格 | `data_get_pine_tables` | `study_filter: "缠论"` |
| 读实时价格 OHLC | `quote_get` | — |
| 读 K 线数据 | `data_get_ohlcv` | `summary: true` |
| 截图（含指标叠加） | `capture_screenshot` | `region: "chart"` |
| 切换品种/周期 | `chart_set_symbol` / `chart_set_timeframe` | `"NYMEX:CL1!"`, `"30"` |

**关键约束**：被 `study_filter` 指定的指标必须在图表上**可见**（不能隐藏）。

### 标准分析 prompt 模板

```
# 单图分析
"读取当前图表上缠论指标的标注，判断当前走势级别、有无买卖点"

# 截图 + 全分析
"截取当前 Brent 原油图表，结合缠论指标输出给出操盘建议"

# 多周期联动
"对比 Brent 原油 30 分钟和 5 分钟的缠论标注，判断是否有背驰"
```
