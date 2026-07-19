#!/bin/sh
# cargo 授权门（RUSTC_WRAPPER）。哨兵在 → 拒绝编译并给出可行动诊断；哨兵不在 → 逐字透传。
BAN=/tmp/kimi-nest-mainline/.chanlun/locks/CARGO_BAN
if [ -f "$BAN" ]; then
  echo "CARGO_GATE: 编译被授权门拒绝 —— p124 重放收口在途，禁止重建 target/。" >&2
  echo "CARGO_GATE: 详情与退休条件见 $BAN" >&2
  echo "CARGO_GATE: 只读操作（cargo metadata 等不触发 rustc）不受影响。" >&2
  exit 87
fi
exec "$@"
