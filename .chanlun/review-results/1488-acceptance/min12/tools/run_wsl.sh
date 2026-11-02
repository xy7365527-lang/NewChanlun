#!/usr/bin/env bash
# WSL 入口（经 nc-remote 在 rust/ 下执行）：run_wsl.sh <label> <port>。库放 ext4 ~/nc-remote/accept-s2/run-<label>，结束后整目录拷到 G: 并写 SHA256SUMS。
# 0055/F7：驱动失败时也拷回 G:、写清单，并以驱动的退出码退出（rc 另记在 G: 的 DRIVER-EXIT）。
set -euo pipefail
L=$1; PORT=$2
REPO=$(cd .. && pwd); T=$REPO/.chanlun/review-results/1488-acceptance/min12
W=$HOME/nc-remote/accept-s2/run-$L; rm -rf "$W.tmp"; test ! -e "$W"
rc=0
python3.12 $T/tools/min12_oracle_run.py --repo "$REPO" --binary /mnt/g/AgentStorage/1488-accept/s2/bin/s_structure_session-6badaa6049 \
  --python /usr/bin/python3.12 --messages $T/input/messages.jsonl --clock $T/input/clock.json \
  --expect-messages-sha d61371c95036eaaaacc94c9780955555271be41a49ad4d5f242424a1c447c6cd \
  --expect-clock-sha 779dcb8539bc9278933bde14d852e9d34493d956cb2c2647cb1db3ead9dfb36f \
  --work "$W" --port $PORT --socket /tmp/s2$L.sock --query-resources $T/tools/query-resources-oracle.json || rc=$?
G=/mnt/g/AgentStorage/1488-accept/s2/run-$L; mkdir -p $G
[ -d "$W" ] && cp -r "$W"/. $G/
echo "$rc" > $G/DRIVER-EXIT
(cd $G && find . -type f ! -name '*.sock' ! -name SHA256SUMS -exec sha256sum {} + | sort -k2 > /tmp/s2-$L-manifest.txt && mv /tmp/s2-$L-manifest.txt $G/SHA256SUMS) || [ $rc -ne 0 ] || rc=$?
echo "driver rc=$rc"
exit $rc
