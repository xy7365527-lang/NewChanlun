#!/usr/bin/env bash
# #1629 读数脚本：在 nc-remote 的 rust/ 下执行。用法：bench.sh <批次名...>
# 每次运行前后记录宿主负载（loadavg、MemAvailable、CPU 占用前 5 的进程），随收据入日志。
set -euo pipefail
cargo build --release --locked --features backtest_bin --bin issue1588_parse_mem_probe 2>&1 | tail -1
P=$CARGO_TARGET_DIR/release/issue1588_parse_mem_probe
D=$HOME/nc-remote/data/btc_1m_full.json
load() {
  echo "--- host $1: loadavg=$(cut -d' ' -f1-3 /proc/loadavg) nproc=$(nproc) $(grep MemAvailable /proc/meminfo | tr -s ' ')"
  ps -eo pcpu,rss,etimes,comm --sort=-pcpu | head -6 | sed 's/^/    /'
}
run() { load before; env "$@"; load after; }
for batch in "$@"; do
  echo "=== batch $batch"
  case $batch in
    small)
      run ISSUE1588_WINDOW=50000 "$P" "$D" facts 49995
      run ISSUE1588_WINDOW=50000 "$P" "$D" group-evidence 49995 0
      run ISSUE1588_WINDOW=50000 "$P" "$D" group-evidence 20000 500 ;;
    mid)
      run ISSUE1588_WINDOW=150000 ISSUE1588_SKIP_FACTS_DIGEST=1 "$P" "$D" facts 149995 ;;
    p300)
      run ISSUE1588_WINDOW=300000 ISSUE1588_SKIP_FACTS_DIGEST=1 "$P" "$D" facts 299995 ;;
    p300time)
      run ISSUE1588_WINDOW=300000 ISSUE1629_LAYER_ONLY=1 "$P" "$D" facts 299995 ;;
    big)
      for n in 1000000 2000000; do
        run ISSUE1588_WINDOW=$((n + 5)) ISSUE1629_LAYER_ONLY=1 "$P" "$D" facts $n
        run ISSUE1588_WINDOW=$((n + 5)) "$P" "$D" layer $n
      done ;;
    full)
      run ISSUE1588_WINDOW=4613599 ISSUE1629_LAYER_ONLY=1 "$P" "$D" facts 4613594
      run ISSUE1588_WINDOW=4613599 "$P" "$D" layer 4613594 ;;
    *) echo "未知批次 $batch"; exit 2 ;;
  esac
done
