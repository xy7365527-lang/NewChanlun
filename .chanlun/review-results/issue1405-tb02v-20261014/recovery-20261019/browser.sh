set -e
WT=$(cd .. && pwd)
HEADSHA=$(git -C "$WT" rev-parse HEAD)
cargo build --release --features s_session --bin s_structure_session 2>&1 | tail -2
BIN=$CARGO_TARGET_DIR/release/s_structure_session
ID=$(date +%Y%m%d-%H%M%S)
PLAN=$HOME/tb1405-browser/$ID
mkdir -p $HOME/tb1405-browser
cd "$WT/s_session"
PW=/home/hanju/nc-remote/pw/node_modules/playwright
export TB02B_CHROMIUM_EXECUTABLE=/home/hanju/.cache/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-linux64/chrome-headless-shell
python3 tests/tb02v_runtime.py prepare --plan "$PLAN" --binary "$BIN" --python "$(which python3)" --node "$(which node)" --playwright-module "$PW" --port 18960 > /dev/null
sed -i 's#/private/tmp/#/tmp/#' "$PLAN"/config-*.json
set +e
python3 tests/tb02v_runtime.py session --plan "$PLAN" --node "$(which node)" > "$PLAN/session.stdout" 2>&1; S=$?
python3 - "$PLAN" <<'PY' > "$PLAN/browser.stdout" 2>&1
import json,subprocess,sys
plan=json.load(open(sys.argv[1]+"/PLAN.json"))
sys.exit(subprocess.run(plan["browser_command"]).returncode)
PY
B=$?
python3 tests/tb02v_runtime.py check-browser --plan "$PLAN" > "$PLAN/check.stdout" 2>&1; C=$?
echo "head=$HEADSHA session_rc=$S browser_rc=$B check_rc=$C" | tee "$PLAN/RCS.txt"
sha256sum "$BIN" | tee -a "$PLAN/RCS.txt"
sleep 3; (ss -ltn | grep -E ':1896[01] ' || echo "ports 18960-18961 free") | tee -a "$PLAN/RCS.txt"
(pgrep -af "s_structure_session|chrome-headless|headless_shell" || echo "no leftover processes") | tee -a "$PLAN/RCS.txt"
cd "$PLAN" && tar czf /mnt/g/AgentStorage/tb1405-browser-$ID.tgz --exclude='*.sqlite*' . && echo "archive=/mnt/g/AgentStorage/tb1405-browser-$ID.tgz"
cat "$PLAN/TB02V-SESSION-CHECK.json" 2>/dev/null | head -c 600; echo
cat "$PLAN/TB02V-BROWSER-CHECK.json" 2>/dev/null | head -c 1500
