#!/bin/bash
# 重跑 emergence ON vs OFF 两套明确隔离的逐笔数据（现有 data_cache 缓存 ON/OFF 已被覆盖为同一版本，不可用于对比）。
# OFF = T_NO_EMERGENCE=1（消融门关闭涌现升级，退化为纯 BSP 驱动 sink/recover）
# ON  = default（涌现升级开启，核心仓随 level 涌现 relabel 上移）
# 两次跑同一份代码，仅差环境变量 ⟹ A/B 隔离 emergence-upgrade 净效应。
set -e
export PATH="$HOME/.cargo/bin:$PATH"
cd /Users/silencehan/Projects/NewChanlun/rust

OFF_DIR=/tmp/t_off_v2
ON_DIR=/tmp/t_on_v2
rm -rf "$OFF_DIR" "$ON_DIR"
mkdir -p "$OFF_DIR" "$ON_DIR"

echo "=== [OFF] T_NO_EMERGENCE=1 run start $(date) ==="
T_NO_EMERGENCE=1 BT_DUMP_TRADES=1 cargo test --release t_engine_8x3 -- --ignored --nocapture
cp ../analysis/data_cache/t_engine_*_trades.json "$OFF_DIR"/
echo "=== [OFF] copied $(ls "$OFF_DIR" | wc -l) files to $OFF_DIR ==="

echo "=== [ON] default run start $(date) ==="
BT_DUMP_TRADES=1 cargo test --release t_engine_8x3 -- --ignored --nocapture
cp ../analysis/data_cache/t_engine_*_trades.json "$ON_DIR"/
echo "=== [ON] copied $(ls "$ON_DIR" | wc -l) files to $ON_DIR ==="

echo "=== ALL DONE $(date) ==="
