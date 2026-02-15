#!/bin/bash
# 元编排 v2 + Everything Claude Code 安装脚本
# 用法: bash install.sh

set -e

echo "=== 元编排 v2 部署 ==="
echo ""

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# 检查 Claude Code
if ! command -v claude &> /dev/null; then
    echo "错误: 未找到 claude 命令。请先安装 Claude Code CLI v2.1+"
    echo "  npm install -g @anthropic-ai/claude-code"
    exit 1
fi

echo "Claude Code 版本: $(claude --version 2>/dev/null || echo '无法获取版本')"
echo ""

# 创建目录
echo "[1/4] 创建目录..."
mkdir -p ~/.claude/rules
mkdir -p ~/.claude/commands
mkdir -p ~/.claude/skills/meta-orchestration/references

# 复制 rules
echo "[2/4] 安装 rules..."
cp "$SCRIPT_DIR/rules/no-workaround.md" ~/.claude/rules/
cp "$SCRIPT_DIR/rules/result-package.md" ~/.claude/rules/
cp "$SCRIPT_DIR/rules/testing-override.md" ~/.claude/rules/
echo "  已安装: no-workaround.md, result-package.md, testing-override.md"

# 复制 commands
echo "[3/4] 安装 commands..."
cp "$SCRIPT_DIR/commands/inquire.md" ~/.claude/commands/
cp "$SCRIPT_DIR/commands/escalate.md" ~/.claude/commands/
cp "$SCRIPT_DIR/commands/ceremony.md" ~/.claude/commands/
cp "$SCRIPT_DIR/commands/ritual.md" ~/.claude/commands/
echo "  已安装: /inquire, /escalate, /ceremony, /ritual"

# 复制 skill
echo "[4/4] 安装元编排 skill..."
cp "$SCRIPT_DIR/skills/meta-orchestration/SKILL.md" ~/.claude/skills/meta-orchestration/
cp "$SCRIPT_DIR/skills/meta-orchestration/README.md" ~/.claude/skills/meta-orchestration/
cp "$SCRIPT_DIR/skills/meta-orchestration/references/"*.md ~/.claude/skills/meta-orchestration/references/
echo "  已安装: meta-orchestration skill + 参考文档"

echo "验证安装..."
echo ""

# 验证
MISSING=0
for f in ~/.claude/rules/no-workaround.md \
         ~/.claude/rules/result-package.md \
         ~/.claude/commands/inquire.md \
         ~/.claude/commands/escalate.md \
         ~/.claude/commands/ceremony.md \
         ~/.claude/commands/ritual.md \
         ~/.claude/skills/meta-orchestration/SKILL.md; do
    if [ ! -f "$f" ]; then
        echo "  缺失: $f"
        MISSING=1
    fi
done

if [ "$MISSING" -eq 0 ]; then
    echo "=== 安装完成 ==="
    echo ""
    echo "下一步:"
    echo "  1. 安装 Everything Claude Code（如果还没装）:"
    echo "     在 Claude Code 中执行: /plugin marketplace add affaan-m/everything-claude-code"
    echo "     然后执行: /plugin install everything-claude-code@everything-claude-code"
    echo ""
    echo "  2. 在你的项目根目录:"
    echo "     cp $SCRIPT_DIR/CLAUDE.md.example ./CLAUDE.md"
    echo "     mkdir -p .chanlun/{definitions,genealogy}"
    echo "     然后编辑 CLAUDE.md 填入你的项目信息"
    echo ""
    echo "  3. 启动 Claude Code，执行 /ceremony"
else
    echo "=== 安装不完整，请检查上述缺失文件 ==="
    exit 1
fi
