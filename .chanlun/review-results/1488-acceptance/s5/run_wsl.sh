#!/usr/bin/env bash
# #1488 S5（0075）WSL 入口（经 nc-remote 在 rust/ 下执行）：run_wsl.sh <label> <binary> <native|ordinary> [<label> <binary> <mode>]...
# 每臂：~/nc-remote/1488-accept/s5/<label> 新目录新库（已存在即拒绝），跑前后记 /proc/loadavg 与 top 进程，
# 结束后整目录拷到 /mnt/g/AgentStorage/1488-accept/s5/<label>/（库文件也拷，供对拍与 R1 分解复核）。
# 输入一律用 S1 冻结的 s86/（两臂同一份 s-resources.json）。
set -uo pipefail
REPO=$(cd .. && pwd); T=$REPO/.chanlun/review-results/1488-acceptance
rc=0
while [ $# -ge 3 ]; do
  L=$1; BIN=$2; MODE=$3; shift 3
  W=$HOME/nc-remote/1488-accept/s5/$L; G=/mnt/g/AgentStorage/1488-accept/s5/$L
  test ! -e "$W" || { echo "已存在 $W"; rc=2; continue; }
  mkdir -p "$(dirname "$W")" "$G"
  {
    echo "label=$L mode=$MODE bin=$BIN sha256=$(sha256sum "$BIN" | cut -c1-64) head=$(git -C "$REPO" rev-parse HEAD)"
    echo "start=$(date -u +%FT%TZ) loadavg=$(cat /proc/loadavg)"
    ps -eo pid,etime,pcpu,args --sort=-pcpu | head -8 | cut -c1-220
  } > "$G/HOST-BEFORE.txt"
  flag=; [ "$MODE" = native ] && flag=--native
  python3.12 "$T/s5/run_s5.py" --bin "$BIN" --workdir "$W" --inputs "$T/s86" $flag
  r=$?; [ $r -ne 0 ] && rc=$r
  {
    echo "end=$(date -u +%FT%TZ) loadavg=$(cat /proc/loadavg) driver_rc=$r"
    ps -eo pid,etime,pcpu,args --sort=-pcpu | head -8 | cut -c1-220
  } > "$G/HOST-AFTER.txt"
  cp -r "$W"/. "$G"/ 2>/dev/null; rm -f "$G/s.sock"
  echo "$r" > "$G/DRIVER-EXIT"
  echo "arm $L rc=$r out=$W g=$G"
done
exit $rc
