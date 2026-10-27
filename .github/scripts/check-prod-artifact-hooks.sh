#!/usr/bin/env bash
# #1640 §2.3 第 3 层（产物机检，权威层）：对不 strip 的 s_session,s_incremental release 二进制，
#   1) nm -C 符号不得含测试钩子子串（NM_SUBSTRINGS）；
#   2) 二进制原始字节不得含禁串：固定清单（FIXED_LITERALS）＋ gen-prod-banned-literals.py 自动抽取、
#      入仓的 .github/prod-banned-literals.txt。按字节匹配（grep -aF），覆盖 strings 默认 7 位编码看不到的中文字面。
# 合法生产命中只能进 NM_EXEMPT（完整去混淆符号前缀的扩展正则，逐条写理由与票号），不得收窄子串。
set -euo pipefail
if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
  echo "用法: $0 <非 strip 的 release 二进制>" >&2
  exit 2
fi
bin="$1"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
generated="$here/../prod-banned-literals.txt"
if [ ! -s "$generated" ]; then
  echo "缺少自动抽取的禁串清单: $generated" >&2
  exit 2
fi
NM_SUBSTRINGS=(for_test testonly inject allocation_probe b2_source_fault RECOVER_AFTER_WRITE ACCEPT_PROBE)
FIXED_LITERALS=(S_CAPTURE_NATIVE_TESTONLY NativeTestOnly 'TestOnly staging' '须持有完整候选' S_SESSION_PAUSE)
NM_EXEMPT=(
  # theta_v0 ledger 的 C28「ν 单射」判定（injective，数学术语，非注入钩子）；若被链接进本二进制属合法生产符号。
  '^newchan_rust::theta_v0::ledger::separate::nu_injective_on(::h[0-9a-f]{16})?$'
)
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
nm_re="$(IFS='|'; echo "${NM_SUBSTRINGS[*]}")"
exempt_re="$(IFS='|'; echo "${NM_EXEMPT[*]}")"
{ printf '%s\n' "${FIXED_LITERALS[@]}"; grep -v '^#@ ' "$generated" | grep -v '^$'; } >"$work/literals"
hits=0
symbols="$(nm -C "$bin")" || { echo "nm 失败，不能判定" >&2; exit 2; }
[ -n "$symbols" ] || { echo "nm 无符号输出（二进制已 strip？），不能判定" >&2; exit 2; }
while IFS= read -r line; do
  [ -n "$line" ] || continue
  name="$(printf '%s\n' "$line" | sed -E 's/^[0-9a-fA-F]*[[:space:]]*[A-Za-z?][[:space:]]+//')"
  if printf '%s\n' "$name" | grep -Eq "$exempt_re"; then
    echo "豁免[nm]: $name"
    continue
  fi
  echo "命中[nm]: $line"
  hits=1
done < <(printf '%s\n' "$symbols" | grep -E "$nm_re" || true)
rc=0
LC_ALL=C grep -a -o -F -f "$work/literals" "$bin" >"$work/literal-hits" || rc=$?
if [ "$rc" -gt 1 ]; then
  echo "禁串扫描失败（grep 退出码 $rc），不能判定" >&2
  exit 2
fi
while IFS= read -r line; do
  [ -n "$line" ] || continue
  echo "命中[禁串]: $line"
  hits=1
done < <(LC_ALL=C sort "$work/literal-hits" | uniq -c)
echo "nm 子串 ${#NM_SUBSTRINGS[@]} 项；禁串 $(wc -l <"$work/literals" | tr -d ' ') 项（固定 ${#FIXED_LITERALS[@]}）"
if [ "$hits" -ne 0 ]; then
  exit 1
fi
echo "未命中钩子名或禁串: $bin"
