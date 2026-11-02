#!/usr/bin/env bash
# a5s6-0076 WSL 入口（经 nc-remote 在 rust/ 下执行）：run_wsl.sh <label> <binary> <port> [额外 a5s6_run.py 参数...]
# 运行目录 ext4 ~/nc-remote/1488-accept/a5s6/<label>，结束（成败都）整目录拷到 G:/AgentStorage/1488-accept/a5s6/<label>/。
set -uo pipefail
L=$1; BIN=$2; PORT=$3; shift 3
REPO=$(cd .. && pwd); T=$REPO/.chanlun/review-results/1488-acceptance/a5s6/tools
W=$HOME/nc-remote/1488-accept/a5s6/$L; test ! -e "$W" || { echo "已存在 $W"; exit 2; }
mkdir -p "$(dirname "$W")"
echo "label=$L bin=$BIN sha256=$(sha256sum "$BIN" | cut -c1-64) head=$(git -C "$REPO" rev-parse HEAD) node=$(node --version)"
cat /proc/loadavg
rc=0
# final1 实测：功能档客户端 g8 时 node RSS ≈1 GB，大代放宽 V8 堆上限（不改客户端逻辑）
export NODE_OPTIONS=--max-old-space-size=8192
# 本 ssh→wsl 会话无 tty：procps 判屏幕 131072x1 为 bogus，向 stderr 告警；launcher 的 process_identity
# 对已退出 PID 把“非空 stderr”当成“无法确认进程状态”，stop 因而误报（final1/smoke1 实测）。只补行数，不截列宽。
export LINES=50
python3.12 $T/a5s6_run.py --repo "$REPO" --binary "$BIN" --work "$W" --port "$PORT" --socket /tmp/a76-$L.sock \
  --q-resources $T/q3-frozen.json --q-functional $T/q3-functional.json \
  --oracle /mnt/g/AgentStorage/1488-accept/d65/min12d-main1/RUN.json "$@" || rc=$?
cat /proc/loadavg
G=/mnt/g/AgentStorage/1488-accept/a5s6/$L; mkdir -p $G
[ -d "$W" ] && cp -r "$W"/. $G/ 2>/dev/null
echo "$rc" > $G/DRIVER-EXIT
(cd $G && find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS)
echo "driver rc=$rc out=$W g=$G"
exit $rc
