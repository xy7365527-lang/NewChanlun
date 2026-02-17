# 元编排 v3 + Everything Claude Code 部署指南

## 前提

- Claude Code CLI v2.1+（`claude --version` 确认）
- Node.js 18+
- Git

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
cp rules/*.md ~/.claude/rules/
mkdir -p ~/.claude/commands && cp commands/*.md ~/.claude/commands/
mkdir -p ~/.claude/skills/meta-orchestration/references
cp skills/meta-orchestration/SKILL.md ~/.claude/skills/meta-orchestration/
cp skills/meta-orchestration/README.md ~/.claude/skills/meta-orchestration/
cp skills/meta-orchestration/references/*.md ~/.claude/skills/meta-orchestration/references/
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
└── skills/
    └── meta-orchestration/
        ├── SKILL.md                    # 元编排主指令
        ├── README.md
        └── references/
            ├── methodology-v2.md       # 完整方法论（v2历史参考，v3变更见SKILL.md末尾）
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
└── skills/
    └── meta-orchestration/       ← 元编排
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
