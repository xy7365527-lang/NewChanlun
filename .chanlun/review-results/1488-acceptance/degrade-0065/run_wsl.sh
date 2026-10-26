#!/usr/bin/env bash
# degrade-0065 WSL 入口（经 nc-remote 在 rust/ 下执行）：run_wsl.sh <s86|min12|min12d|min12q> <label> <binary> <native|ordinary> [port]
# 库与运行目录放 ext4 ~/nc-remote/1488-accept/d65/<kind>-<label>，结束后整目录拷到 G:（不含 sqlite 时另记 sha）。
# 回执期限放到 1,800,000 ms：v3 写端在宿主负载≈3 时 op-0046 超过 oracle 用的 120 s（min12-ord1 实测），只是驱动/回执期限，不是资源帽。
# native = S_CAPTURE_NATIVE_TESTONLY=1 + S_MANIFEST_WORKSPACE_DIAGNOSTICS=1；ordinary = 两者都不设。
set -uo pipefail
K=$1; L=$2; BIN=$3; MODE=$4; PORT=${5:-18765}
REPO=$(cd .. && pwd); T=$REPO/.chanlun/review-results/1488-acceptance/degrade-0065
# min12q = 对已有 min12d-<label> 只重做 Q3 查询与 oracle 对拍（写入不重跑）。
if [ "$K" = min12q ]; then W=$HOME/nc-remote/1488-accept/d65/min12d-$L; test -f "$W/INGEST.jsonl" || { echo "缺 $W"; exit 2; }
else W=$HOME/nc-remote/1488-accept/d65/$K-$L; test ! -e "$W" || { echo "已存在 $W"; exit 2; }; fi
unset S_CAPTURE_NATIVE_TESTONLY S_MANIFEST_WORKSPACE_DIAGNOSTICS
if [ "$MODE" = native ]; then export S_CAPTURE_NATIVE_TESTONLY=1 S_MANIFEST_WORKSPACE_DIAGNOSTICS=1; fi
echo "kind=$K label=$L mode=$MODE bin=$BIN sha256=$(sha256sum "$BIN" | cut -c1-64) head=$(git -C "$REPO" rev-parse HEAD)"
rc=0
case $K in
  s86)
    python3.12 $T/s86/run_s86_native.py --bin "$BIN" --workdir "$W" --inputs $T/s86 || rc=$? ;;
  min12)
    python3.12 $T/min12/tools/min12_v3_run.py --repo "$REPO" --binary "$BIN" \
      --python /usr/bin/python3.12 --messages $T/min12/input/messages.jsonl --clock $T/min12/input/clock.json \
      --expect-messages-sha d61371c95036eaaaacc94c9780955555271be41a49ad4d5f242424a1c447c6cd \
      --expect-clock-sha 779dcb8539bc9278933bde14d852e9d34493d956cb2c2647cb1db3ead9dfb36f \
      --reply-timeout-ms 1800000 --work "$W" --port $PORT --socket /tmp/d65$L.sock --query-resources $T/min12/tools/query-resources-q3.json || rc=$? ;;
  min12d)
    python3.12 $T/min12/tools/min12_v3_direct.py --bin "$BIN" --work "$W" --port $PORT || rc=$? ;;
  min12q)
    python3.12 $T/min12/tools/min12_v3_direct.py --bin "$BIN" --work "$W" --port $PORT --query-only || rc=$? ;;
  *) echo "未知 kind $K"; exit 2 ;;
esac
cat /proc/loadavg
G=/mnt/g/AgentStorage/1488-accept/d65/$(basename "$W"); mkdir -p $G
[ -d "$W" ] && cp -r "$W"/. $G/ 2>/dev/null
echo "$rc" > $G/DRIVER-EXIT
echo "driver rc=$rc out=$W g=$G"
exit $rc
