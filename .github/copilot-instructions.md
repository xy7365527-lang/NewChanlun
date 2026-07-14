# GitHub Copilot 仓库指令（桥接文件）

本文件是**桥接**，不是规则本体。规则本体在根目录 `CLAUDE.md`（蜂群基因组）。

## 第零条：语言

一律用**简体中文**回复与产出文档。

## 第一条：先读基因组

任何实质工作开始前，先读根目录 `CLAUDE.md`。它定义了本项目的蜂群方法论（元编排 v2）、原则 0-17、skill 索引。skill 集合在 `.claude/skills/`（Copilot 会自动发现，按需加载），agent 定义在 `.claude/agents/`。

## 第二条：缠论内容的权威链

涉及缠论定义/走势建模时，按三级权威链检索（高层级优先）：

1. 缠师原始博文（最终权威）：`docs/chanlun/text/blog/INDEX.md`
2. 《股市技术理论》编纂版：`docs/chanlun/text/chan99/INDEX.md`
3. 思维导图/第三方总结（仅辅助）：`docs/chanlun/text/mindmaps/INDEX.md`

速查入口：`缠论知识库.md`。总纲领：`docs/ROADMAP.md`。

## 第三条：蜂群工作流入口

- **ceremony 启动序**（进入蜂群循环前执行）：

```bash
python scripts/ceremony_state.py write 1 initial
python scripts/ceremony_scan.py
```

- **Lead/goal 协议**：协议文本在 `.claude/commands/goal.md`，goal 事件账本在 `.chanlun/goals/events.jsonl`。Copilot 无 Claude Code 的 slash 命令机制——需人工把 goal.md 作为指令读入后按协议执行。
- 平台能力差异与降级说明：`docs/copilot-runbook.md`。

## 第四条：090 号无条件严格纪律

严格性是本蜂群的语法规则（CLAUDE.md 原则17 / 谱系 090）：**禁止简化实装、禁止补丁方案、禁止"大致/后续再说"**。严格 = 概念清晰 + 声明与实际能力一致 + 无未定义模糊地带。非严格产出视为不合法产出。

## 工程约束

- Rust 引擎在 `rust/`，改动后跑该目录的测试再声明完成。
- 不要修改 `.claude/`、`.chanlun/` 下的配置与谱系文件，除非任务明确要求。
- commit 信息风格参照 `git log`：`feat/docs/perf(scope): 中文描述`。
