#!/usr/bin/env bash
# #1527 失败收据：在 windows-5090 WSL 上用已封存 release 二进制与正式 Python CLI 跑两类失败。
set -u
cd .. # nc-remote 在 rust/ 下起步
REPO=$(pwd); HEADSHA=$(git rev-parse HEAD)
B=/mnt/g/AgentStorage/issue1527-binary/s_structure_session-947e05be
OUT=/mnt/g/AgentStorage/issue1527-receipts/$(date +%Y%m%dT%H%M%S); mkdir -p "$OUT"; W=$(mktemp -d /tmp/r1527.XXXX)
echo "repo_head=$HEADSHA out=$OUT work=$W"; echo "binary_sha256=$(sha256sum $B | cut -d' ' -f1)"; python3 --version; uname -a
run() { # name, then command...
  local name=$1; shift
  printf '%q ' "$@" > "$OUT/$name.cmd"; echo >> "$OUT/$name.cmd"
  env | grep -E '^S_CONTROL_|^PATH=' | sed 's/=.*SHA.*/=<redacted>/' > "$OUT/$name.env"
  local t0=$(date +%s%N)
  /usr/bin/time -v -o "$OUT/$name.time" "$@" > "$OUT/$name.stdout" 2> "$OUT/$name.stderr"; local rc=$?
  echo "$rc" > "$OUT/$name.rc"; echo "elapsed_ms=$(( ($(date +%s%N)-t0)/1000000 ))" >> "$OUT/$name.rc"
  echo "== $name rc=$rc"; head -c 1500 "$OUT/$name.stderr"; echo; head -c 800 "$OUT/$name.stdout"; echo
  grep -E "Maximum resident|Exit status|Elapsed" "$OUT/$name.time"
}
CAT=$REPO/s_session/catalog/signed-catalog.json; FX=$REPO/s_session/tests/fixtures/tb01c
# 正式产库（producer init，s-session/3）
run P0-init "$B" init --db "$W/s.sqlite" --session r1527-fail --catalog "$CAT" --profile "$FX/profile.json" --session-generation 1 --clock-plan "$FX/clock-plan.json" --delivery-retain-generations 1 --protocol-revision s-session/3
ls -l "$W"
mkdir -p "$W/state"
cat > "$W/s-resources.json" <<J
{"schema_revision":"s-structure-resources/1","s_resources":{"max_frame_bytes":"1048576","read_timeout_ms":"5000","write_timeout_ms":"5000","response_timeout_ms":"5000","audit_cache_source_bytes":"1048576","queue_capacity":"8","max_connections":"4"}}
J
python3 - "$W" "$B" "$FX" "$CAT" <<'PY'
import json,sys; w,b,fx,cat=sys.argv[1:]
cfg={"schema_revision":"s-launcher/3","db":w+"/s.sqlite","socket":w+"/s.sock","binary":b,"python":sys.executable,
 "catalog":cat,"profile":fx+"/profile.json","clock_plan":fx+"/clock-plan.json","browser":w+"/browser",
 "query_resource_config":fx+"/query-resources.json","session_id":"r1527-fail","session_generation":"1",
 "source_namespace":"r1527","source_epoch":"1","producer_id":"r1527","producer_epoch":"1","writer_epoch":"1",
 "query_epoch":"1","port":"18881","startup_timeout_ms":"10000","delivery_retain_generations":"1",
 "s_resources":{k:"1048576" if "bytes" in k else "5000" if "ms" in k else "8" for k in
  ["max_frame_bytes","read_timeout_ms","write_timeout_ms","response_timeout_ms","audit_cache_source_bytes","queue_capacity","max_connections"]}}
open(w+"/launcher.json","w").write(json.dumps(cfg))
PY
# F1 负控：正式 CLI 对无锚库启动 S —— R4 必拒，且不得自行建锚
run F1-cli-start-s-no-anchor python3 -B "$REPO/s_session/s_service_control.py" start-s --config "$W/launcher.json" --state-dir "$W/state"
echo "post: $(ls -A "$W/state" | tr '\n' ' ') owner=$(ls "$W"/s.sqlite.control-owner.json 2>&1)"
# F1b 负控：Rust 写入口直接以伪造预算文件 recover —— Rust 侧锚核验必拒
printf '{"max_elapsed_ms":"1"}' > "$W/forged-limits.json"; chmod 600 "$W/forged-limits.json"
run F1b-rust-recover-forged-limits "$B" recover --protocol-revision s-session/3 --db "$W/s.sqlite" --new-epoch 2 --clock-plan "$FX/clock-plan.json" --clock-event-id e1 --admission-job-limits "$W/forged-limits.json"
# F2 资源不足（磁盘满模拟）：RLIMIT_FSIZE=64KiB 下 producer init 建库
run F2-init-fsize-64k bash -c "trap '' XFSZ; ulimit -f 64; exec \"$B\" init --db \"$W/full.sqlite\" --session r1527-full --catalog \"$CAT\" --profile \"$FX/profile.json\" --session-generation 1 --clock-plan \"$FX/clock-plan.json\" --delivery-retain-generations 1 --protocol-revision s-session/3"
ls -l "$W"/full.sqlite* 2>&1
# F2b 资源不足（文件描述符耗尽）：RLIMIT_NOFILE=8 下正式 CLI 进入 R4 首读
run F2b-cli-start-s-nofile-8 bash -c "ulimit -n 8; exec python3 -B \"$REPO/s_session/s_service_control.py\" start-s --config \"$W/launcher.json\" --state-dir \"$W/state\""
sha256sum "$W/s.sqlite" > "$OUT/db-sha256.txt"; cp "$W/launcher.json" "$W/s-resources.json" "$OUT/"; ls "$OUT"
