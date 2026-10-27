set +e
python3 - <<'PY'
from pathlib import Path
import re
for p in Path("src/bin").rglob("*.rs"):
 s=p.read_text()
 s=re.sub(r'#\[allow\(\s*(?:dead_code|unused_imports|unused_variables),\s*reason = "#1488 再合 #1608：8组合实测无调用方，保留不删"\s*\)\]\s*', '', s)
 p.write_text(s)
PY
failed=0
for pair in S:s_session N:s_session,s_capture_native_testonly J:s_session,s_allocation_json_testonly JN:s_session,s_allocation_json_testonly,s_capture_native_testonly; do
 label=${pair%%:*}; features=${pair#*:}
 for target in P T; do
 echo "=== BEGIN $label$target ==="
 if [ "$target" = P ]; then cargo check --locked --lib --bins --features "$features"; else cargo check --locked --tests --features "$features"; fi
 rc=$?; echo "=== END $label$target RC=$rc ==="; [ "$rc" = 0 ] || failed=1
 done
done
exit $failed
