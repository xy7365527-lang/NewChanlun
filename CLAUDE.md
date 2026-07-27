# CLAUDE.md

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

## Agent skills

### Issue tracker

Issues live in GitHub Issues at `xy7365527-lang/NewChanlun`（private，gh 已认证 keyring，操作走 `gh issue`）. See `docs/agents/issue-tracker.md`.

### Triage labels

Five canonical roles, label string equal to its name: `needs-triage` / `needs-info` / `ready-for-agent` / `ready-for-human` / `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: one `CONTEXT.md` + `docs/adr/` at repo root（文件不存在时静默跳过，由 /domain-modeling 惰性创建）. See `docs/agents/domain.md`.

### 统计结论口径标注与引用审查

归档判决落盘时标口径（轻档）；有人要拿某结论当决策依据时先查过没过异质审查（重档），未标默认按「未审」降级使用，不追溯存量。See `docs/agents/stat-provenance.md`.
