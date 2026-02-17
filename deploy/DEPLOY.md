# 元编排 v3 + Everything Claude Code 部署指南

## 前提

- Claude Code CLI v2.1+（`claude --version` 确认）
- Node.js 18+
- Git

## 架构概览（谱系014 卡组分解）

元编排指令不再是单一 SKILL.md 单体（旧版633行），而是**核心卡 + 工位卡组**：

| 组件 | 文件 | 安装位置 | 职责 |
|------|------|----------|------|
| **核心卡** | `skills/meta-orchestration/SKILL.md` | `~/.claude/skills/meta-orchestration/` | 质询序列、概念分离、生成态/结算态 |
| **Lead** | `agents/meta-lead.md` | `~/.claude/agents/` | 中断路由器：收信→判断→转发工位 |
| **谱系维护** | `agents/genealogist.md` | `~/.claude/agents/` | 谱系写入、张力检查、回溯扫描 |
| **质量守卫** | `agents/quality-guard.md` | `~/.claude/agents/` | 结果包检查、代码违规扫描 |
| **源头审计** | `agents/source-auditor.md` | `~/.claude/agents/` | 三级权威链、溯源标签、原文考古 |
| **元规则观测** | `agents/meta-observer.md` | `~/.claude/agents/` | 二阶反馈回路、元编排进化 |
| **拓扑管理** | `agents/topology-manager.md` | `~/.claude/agents/` | 工位扩张/收缩建议 |

**设计原则**：一个 agent 一件事。Lead 只路由不认知。复杂度通过 agent 数量扩展。

## 部署步骤

### 第一步：安装 Everything Claude Code

```bash
# 在 Claude Code 中执行
/plugin marketplace add affaan-m/everything-claude-code
/plugin install everything-claude-code@everything-claude-code
```

ECC 的 plugin 系统不支持自动分发 rules，手动复制：

```bash
git clone https://github.com/affaan-m/everything-claude-code.git /tmp/ecc
cp -r /tmp/ecc/rules/* ~/.claude/rules/
rm -rf /tmp/ecc
```

### 第二步：安装元编排层

```bash
# 解压部署包后，在部署包目录下执行
bash install.sh
```

或手动：

```bash
# rules
cp rules/*.md ~/.claude/rules/

# commands
mkdir -p ~/.claude/commands && cp commands/*.md ~/.claude/commands/

# 核心卡
mkdir -p ~/.claude/skills/meta-orchestration/references
cp skills/meta-orchestration/SKILL.md ~/.claude/skills/meta-orchestration/
cp skills/meta-orchestration/README.md ~/.claude/skills/meta-orchestration/
cp skills/meta-orchestration/references/*.md ~/.claude/skills/meta-orchestration/references/

# 工位卡组
mkdir -p ~/.claude/agents
cp agents/*.md ~/.claude/agents/
```

### 第三步：初始化项目

```bash
cd ~/your-chanlun-project
cp /path/to/deploy/CLAUDE.md.example ./CLAUDE.md
# 编辑 CLAUDE.md，填入项目信息和当前阶段目标

mkdir -p .chanlun/definitions .chanlun/genealogy
```

### 第四步：验证

```bash
claude   # 启动 Claude Code
```

启动后执行：
```
/ceremony
```

看到创世仪式输出（定义列表、生成态矛盾、项目目标、就绪确认）即部署成功。

## 部署包内容

```
deploy/
├── DEPLOY.md                           # 本文件
├── CLAUDE.md.example                   # 项目级配置模板
├── install.sh                          # 一键安装脚本
├── rules/
│   ├── no-workaround.md                # 核心禁令（优先级最高）
│   ├── result-package.md               # 结果包格式强制
│   └── testing-override.md             # 测试规则（加生成态例外）
├── commands/
│   ├── ceremony.md                     # /ceremony 开端（创世仪式）
│   ├── inquire.md                      # /inquire 质询序列
│   ├── escalate.md                     # /escalate 矛盾上浮（含lead建议方案）
│   └── ritual.md                       # /ritual 仪式（概念层收缩的顶点）
├── agents/                             # 蜂群工位卡组（谱系014）
│   ├── meta-lead.md                    # Lead — 中断路由器
│   ├── genealogist.md                  # 谱系维护工位
│   ├── quality-guard.md                # 质量守卫工位
│   ├── source-auditor.md               # 源头审计工位
│   ├── meta-observer.md                # 元规则观测工位
│   └── topology-manager.md             # 拓扑管理工位
└── skills/
    └── meta-orchestration/
        ├── SKILL.md                    # 核心卡（~109行，质询+概念分离+生成态）
        ├── README.md
        └── references/
            ├── methodology-v2.md       # 完整方法论（v2历史参考）
            ├── genealogy-template.md   # 谱系记录模板
            └── result-package-template.md  # 结果包模板
```

## 安装后的目录结构

```
~/.claude/
├── rules/
│   ├── no-workaround.md          ← 元编排
│   ├── result-package.md         ← 元编排
│   ├── testing-override.md       ← 元编排
│   ├── security.md               ← ECC
│   ├── coding-style.md           ← ECC
│   ├── testing.md                ← ECC（被 testing-override 补充）
│   ├── git-workflow.md           ← ECC
│   ├── agents.md                 ← ECC
│   └── performance.md            ← ECC
├── commands/
│   ├── ceremony.md               ← 元编排
│   ├── inquire.md                ← 元编排
│   ├── escalate.md               ← 元编排
│   ├── ritual.md                 ← 元编排
│   ├── tdd.md                    ← ECC
│   ├── plan.md                   ← ECC
│   ├── code-review.md            ← ECC
│   ├── build-fix.md              ← ECC
│   └── refactor-clean.md         ← ECC
├── agents/                       ← 蜂群工位卡组（谱系014 新增）
│   ├── meta-lead.md              ← 元编排
│   ├── genealogist.md            ← 元编排
│   ├── quality-guard.md          ← 元编排
│   ├── source-auditor.md         ← 元编排
│   ├── meta-observer.md          ← 元编排
│   ├── topology-manager.md       ← 元编排
│   ├── planner.md                ← ECC
│   ├── tdd-guide.md              ← ECC
│   ├── code-reviewer.md          ← ECC
│   └── ...                       ← ECC 其他 agents
└── skills/
    └── meta-orchestration/       ← 元编排（核心卡）
        ├── SKILL.md
        └── references/

~/your-project/
├── CLAUDE.md                     ← 项目配置
└── .chanlun/
    ├── definitions/              ← 概念定义仓库
    └── genealogy/                ← 谱系记录
```

## 日常使用

确保已启用 Agent Teams：`export CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`

1. `claude` 启动 → `/ceremony` 创世仪式
2. 用缠论术语描述任务，让lead分配给teammates
3. **系统自转：** Teammates并行工作、互相质询、解决实现层分歧；lead监控、做谱系比对、自主处理工程层矛盾
4. **你不需要全程在线。** 系统在两个概念层转折点之间自主运行
5. 概念层矛盾 → lead附带建议方案上浮 → 你用缠论语言做决断（或确认lead建议）→ `/ritual`
6. 谱系自动写入 `.chanlun/genealogy/`（lead自主结算和编排者决断都会写入）
7. 定义稳定的模块 → `/plan` → `/tdd` → `/code-review` 正常工程流

## 注意事项

- 上下文窗口管理：ECC 建议同时启用的 MCP 不超过 10 个、工具不超过 80 个
- Agent Teams 每个 teammate 消耗独立 token，成本高于单 session，用在需要并行和互相质询的场景
- 元编排的 commands 和 rules 会占用一些上下文空间，但量不大
- 谱系文件增长后，/ceremony 可能需要较多上下文加载，这时只加载摘要
- Teammates 自动加载 rules 和 skills，不需要额外配置
- **v3自转原则：** 如果你不确定是否该介入，让系统继续跑。真正需要你的时候矛盾报告会清晰地告诉你
