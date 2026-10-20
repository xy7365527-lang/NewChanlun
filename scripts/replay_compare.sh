#!/usr/bin/env bash
# 双跑对拍脚本：D4 回放引擎 vs p123 生产基准
#
# 使用：
#   ./scripts/replay_compare.sh [DATA] [MAX_BARS]
#
# 参数：
#   DATA      - Bar 数据文件路径（默认：analysis/data_cache/btc_1m_full.json）
#   MAX_BARS  - 最大回放 bar 数（默认：100）
#
# 输出：
#   - 通过：exit 0，打印 "✅ 对拍通过：X bars, diff=0"
#   - 失败：exit 1，打印 "❌ 对拍失败：X diffs"
#
# 示例：
#   ./scripts/replay_compare.sh                            # 使用默认参数
#   ./scripts/replay_compare.sh btc_data.json 500         # 自定义参数

set -euo pipefail

# 参数
DATA="${1:-analysis/data_cache/btc_1m_full.json}"
MAX_BARS="${2:-100}"

# 检查数据文件
if [ ! -f "$DATA" ]; then
    echo "❌ 数据文件不存在: $DATA"
    exit 1
fi

# 临时文件
P123_DUMP=$(mktemp)
trap "rm -f '$P123_DUMP'" EXIT

# 1. 跑 p123 dump
echo "▶️  步骤 1/3：运行 p123_fast_replay（生成 dump）"
P116_DUMP="$P123_DUMP" P116_MAX_BARS="$MAX_BARS" \
    cargo run --release --bin p123_fast_replay -- "$DATA" > /dev/null 2>&1

# 检查 dump 是否生成
if [ ! -f "$P123_DUMP" ]; then
    echo "❌ p123 dump 未生成"
    exit 1
fi

DUMP_LINES=$(wc -l < "$P123_DUMP")
echo "✅ p123 dump 生成完成：$DUMP_LINES 行"

# 2. 跑 D4 replay + 对拍
echo "▶️  步骤 2/3：运行 D4 replay 并对拍"
cargo run --release --bin replay_compare -- "$DATA" "$P123_DUMP" "$MAX_BARS"

# 3. 报告（由 replay_compare bin 输出）
echo "▶️  步骤 3/3：对拍完成"
