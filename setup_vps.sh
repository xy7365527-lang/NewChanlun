#!/bin/bash
# NewChanlun VPS Setup Script
# 在 Ubuntu VPS 上一键部署 Claude Code + Vicoa + 项目环境
set -e

echo "=== 1/7 系统更新 ==="
apt update && apt upgrade -y

echo "=== 2/7 安装基础工具 ==="
apt install -y python3.11 python3.11-venv python3-pip git curl

echo "=== 3/7 安装 Node.js 20 ==="
curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
apt install -y nodejs

echo "=== 4/7 安装 Claude Code CLI ==="
npm install -g @anthropic-ai/claude-code

echo "=== 5/7 克隆项目 ==="
cd /root
git clone https://github.com/xy7365527-lang/NewChanlun.git
cd NewChanlun

echo "=== 6/7 Python 虚拟环境 + 依赖 ==="
python3.11 -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install -r requirements.txt 2>/dev/null || echo "No requirements.txt found, skipping"
pip install -e ".[dev]" 2>/dev/null || pip install -e . 2>/dev/null || echo "No setup.py/pyproject.toml install, skipping"

echo "=== 7/7 安装 Vicoa ==="
pip install vicoa

echo ""
echo "========================================="
echo "  安装完成！"
echo "========================================="
echo ""
echo "下一步："
echo "  1. 设置 API key:"
echo "     export ANTHROPIC_API_KEY='your-key-here'"
echo ""
echo "  2. 启动方式 A - Vicoa (手机远程):"
echo "     cd /root/NewChanlun"
echo "     source .venv/bin/activate"
echo "     vicoa"
echo ""
echo "  2. 启动方式 B - Claude Code (纯终端):"
echo "     cd /root/NewChanlun"
echo "     claude"
echo ""
echo "  3. 跑测试验证环境:"
echo "     cd /root/NewChanlun"
echo "     source .venv/bin/activate"
echo "     python -m pytest --tb=short -q"
echo ""
