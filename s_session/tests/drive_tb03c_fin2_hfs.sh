#!/usr/bin/env bash
# #1672：为 FIN/2 创建 E + 3B 四个独立 HFS+ 载体，运行指定命令后完整回收。
set -euo pipefail

if [[ $# -eq 0 ]]; then
  echo "usage: $0 <command> [args...]" >&2
  exit 64
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "FIN/2 HFS+ acceptance requires macOS; got $(uname -s)" >&2
  exit 65
fi

ROOT="$(mktemp -d "${TMPDIR:-/tmp}/tb03c-fin2-hfs.XXXXXX")"
# macOS 的 /var 是指向 /private/var 的链接，挂载表使用物理路径；先规范化再逐字比对。
ROOT="$(cd "$ROOT" && pwd -P)"
NAMES=(e b0 b1 b2)
MOUNTED=()

cleanup() {
  local rc=$?
  for name in "${MOUNTED[@]}"; do
    echo "cleanup: hdiutil detach $ROOT/$name"
    hdiutil detach "$ROOT/$name" -quiet || true
  done
  echo "cleanup: remove $ROOT"
  rm -rf "$ROOT"
  return "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

DEVICES=""
for name in "${NAMES[@]}"; do
  mkdir -p "$ROOT/$name"
  hdiutil create -size 16m -fs HFS+ -volname "$name" -type UDIF "$ROOT/$name.dmg"
  hdiutil attach -nobrowse -mountpoint "$ROOT/$name" "$ROOT/$name.dmg"
  MOUNTED+=("$name")
  test -w "$ROOT/$name"
  # mount -t 直接按挂载表判定，不依赖 macOS stat 的格式字符。
  mount -t hfs | grep -F " on $ROOT/$name " >/dev/null
  fs_type=hfs
  device="$(stat -f '%d' "$ROOT/$name")"
  case " $DEVICES " in
    *" $device "*) echo "carrier device reused: $device" >&2; exit 67 ;;
  esac
  DEVICES="$DEVICES $device"
  capacity_kib="$(df -k "$ROOT/$name" | awk 'NR==2 {print $2}')"
  echo "carrier name=$name fs=$fs_type device=$device capacity_kib=$capacity_kib mount=$ROOT/$name"
done

export TB03C_FIN2_CARRIER_ROOT="$ROOT"
echo "TB03C_FIN2_CARRIER_ROOT=$TB03C_FIN2_CARRIER_ROOT"
"$@"
