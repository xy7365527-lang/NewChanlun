#!/usr/bin/env bash
set -euo pipefail
if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
  echo "用法: $0 <非 strip 的 release 二进制>" >&2
  exit 2
fi
bin="$1"
HOOK_RE='observe_for_test|for_testonly|_for_test|_testonly'
BAN_RE='S_CAPTURE_NATIVE_TESTONLY|NativeTestOnly|TestOnly staging|须持有完整候选'
hits=0
while IFS= read -r line; do
  [ -n "$line" ] || continue
  echo "命中[nm]: $line"
  hits=1
done < <(nm -C "$bin" 2>/dev/null | grep -E "$HOOK_RE" || true)
while IFS= read -r line; do
  [ -n "$line" ] || continue
  echo "命中[strings]: $line"
  hits=1
done < <(strings "$bin" | grep -E "$HOOK_RE|$BAN_RE" || true)
if [ "$hits" -ne 0 ]; then
  exit 1
fi
echo "未命中钩子名或禁串: $bin"
