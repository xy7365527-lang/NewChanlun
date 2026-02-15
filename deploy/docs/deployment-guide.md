# 部署指南：元编排 v2 + Everything Claude Code

## 前提条件

- Claude Code CLI v2.1+（`claude --version` 检查）
- Node.js（ECC的hooks需要）
- Git

## 第一步：安装 Everything Claude Code

```bash
# 方法A：作为plugin安装（推荐）
# 在Claude Code会话中执行：
/plugin marketplace add affaan-m/everything-claude-code
/plugin install everything-claude-code@everything-claude-code

# 方法B：手动安装
git clone https://github.com/affaan-m/everything-claude-code.git /tmp/ecc

# 复制ECC组件到用户级Claude配置
mkdir -p ~/.claude/{agents,rules,commands,skills,hooks}
cp /tmp/ecc/agents/*.md ~/.claude/agents/
cp /tmp/ecc/rules/*.md ~/.claude/rules/
cp /tmp/ecc/commands/*.md ~/.claude/commands/
cp -r /tmp/ecc/skills/* ~/.claude/skills/
cp -r /tmp/ecc/hooks/* ~/.claude/hooks/
```

## 第二步：安装元编排 skill

```bash
# 将 meta-orchestration skill 复制到 Claude Code skills 目录
cp -r meta-orchestration-skill/ ~/.claude/skills/meta-orchestration/
```

确认文件结构：
```
~/.claude/skills/meta-orchestration/
├── SKILL.md
├── README.md
└── references/
    ├── methodology-v2.md
    ├── genealogy-template.md
    └── result-package-template.md
```

## 第三步：覆盖ECC的agent（元编排改版）

用本部署包中的改版agent替换ECC原版：

```bash
# 备份ECC原版
mkdir -p ~/.claude/agents/ecc-originals/
cp ~/.claude/agents/planner.md ~/.claude/agents/ecc-originals/
cp ~/.claude/agents/tdd-guide.md ~/.claude/agents/ecc-originals/
cp ~/.claude/agents/code-reviewer.md ~/.claude/agents/ecc-originals/
cp ~/.claude/agents/architect.md ~/.claude/agents/ecc-originals/

# 覆盖为改版
cp deploy/agents/planner.md ~/.claude/agents/
cp deploy/agents/tdd-guide.md ~/.claude/agents/
cp deploy/agents/code-reviewer.md ~/.claude/agents/
cp deploy/agents/architect.md ~/.claude/agents/
```

## 第四步：安装新增的rules

```bash
cp deploy/rules/no-workaround.md ~/.claude/rules/
cp deploy/rules/result-package.md ~/.claude/rules/
cp deploy/rules/testing-override.md ~/.claude/rules/
```

## 第五步：安装新增的commands

```bash
cp deploy/commands/inquire.md ~/.claude/commands/
cp deploy/commands/escalate.md ~/.claude/commands/
cp deploy/commands/ceremony.md ~/.claude/commands/
cp deploy/commands/ritual.md ~/.claude/commands/
```

## 第六步：初始化项目

```bash
# 创建你的缠论项目
mkdir -p ~/chanlun-quant/{knowledge,genealogy/{settled,pending},src,tests}

# 复制项目级CLAUDE.md
cp deploy/CLAUDE.md ~/chanlun-quant/CLAUDE.md

# 进入项目目录
cd ~/chanlun-quant

# 初始化git
git init
git add .
git commit -m "feat: project initialization with meta-orchestration"
```

## 第七步：验证安装

在项目目录下启动Claude Code：

```bash
cd ~/chanlun-quant
claude
```

验证清单：
1. 输入 `/ceremony` — 应该执行创世仪式，报告knowledge/和genealogy/为空
2. 输入 `/plan "实现K线数据结构"` — 应该先检查定义结算状态
3. 输入 `/inquire` — 应该要求提供结果包
4. 输入 `/escalate` — 应该生成矛盾报告模板

## 第八步：不需要安装的ECC组件

以下ECC组件不安装或安装后不启用：

- `skills/verification-loop/` — 不安装。你的质询循环本身就是验证机制
- `skills/eval-harness/` — 不安装。同上
- `skills/iterative-retrieval/` — 不安装。蜂群传递完整结果包，不需要渐进检索
- `skills/continuous-learning/` — 先不安装。它的自动模式提取逻辑可能与谱系冲突，等实际使用中判断

以下ECC组件直接使用原版：
- `agents/security-reviewer.md`
- `agents/build-error-resolver.md`
- `agents/refactor-cleaner.md`
- `agents/doc-updater.md`
- `hooks/memory-persistence/`
- `hooks/strategic-compact/`
- `rules/security.md`
- `rules/git-workflow.md`
- `rules/coding-style.md`

## 上下文窗口管理

ECC的README警告：不要同时启用太多MCP，200k窗口会缩到70k。

建议：
- 蜂群启动时不挂载任何MCP
- 只在具体需要时临时启用（如数据库MCP、API MCP）
- 保持 active tools < 80

## 开始使用

```bash
cd ~/chanlun-quant
claude

# 第一件事：创世仪式
> /ceremony

# 然后开始你的第一个概念定义
# 用缠论语言描述，例如：
> 定义"笔"：...
```

编排者的工作方式：用缠论术语说话。不需要说代码语言。Claude会翻译。
