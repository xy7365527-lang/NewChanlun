#!/usr/bin/env bash
# =============================================================================
# arch_hotzone_monitor.sh —— 架构深化循环「热区触发式」常驻监控（#1197）
#
# 背景：架构深化循环（workflows/architecture-deepening-loop.md）于 #1193 判停，
#       自判停起转为「热区触发式」：不再全仓清道扫描，改为按热区摩擦读数触发
#       新一轮扫描。本脚本就是那段「热区监控」的常驻组件。
#
# 判据（可证伪，三条件缺一不触发）：
#   C1 摩擦：文件 > 2500 行 ∧ 近 30 天改动 ≥ 5 次；或 近 30 天改动 ≥ 12 次；
#   C2 不在裁定锁定名单（默认 .chanlun/arch-hotzone-locklist.txt，可配置）；
#   C3 无 open 本线 wayfinder:map 在飞（本线 = 标题含「架构深化」的 wayfinder:map）。
#
# 动作：命中 → 不自动开图（HITL 闸一保留）；把命中清单 + 摩擦读数写入
#       .chanlun/arch-hotzone-report-<date>.md（仓内工作草稿），等编排者拍板
#       后再跑 /improve-codebase-architecture。
#
# 用法：
#   bash scripts/arch_hotzone_monitor.sh --once       # 单轮扫描（默认）
#   bash scripts/arch_hotzone_monitor.sh --live       # 常驻轮询（launchd 入口）
#   bash scripts/arch_hotzone_monitor.sh --selftest   # 合成实测（命中/不命中样例）
#
# 部署（受管常驻，launchd KeepAlive，与 wayfinder-engine 同款安装模式）：
#   bash .sandcastle/install-arch-hotzone.sh install
#   bash .sandcastle/install-arch-hotzone.sh status
#   bash .sandcastle/install-arch-hotzone.sh uninstall
#
# 输出样例（当前 main，四轮收敛后现状，应输出「无命中」）：
#   [arch_hotzone] 2026-08-23T06:00:00 一轮扫描开始（--once）
#   [arch_hotzone] open 本线 wayfinder:map（标题含「架构深化」）：0 张
#   [arch_hotzone] 候选文件（*.rs）：429 个
#   [arch_hotzone] 近 30 天改动阈值：>2500 行 且 ≥5 次，或 ≥12 次
#   [arch_hotzone] 锁定名单：.chanlun/arch-hotzone-locklist.txt
#   [arch_hotzone] 命中 0 个 → 无命中
#
# 实现说明（bash 3.2 兼容，macOS 宿主同款）：
#   - 三个判据均为纯函数（hotzone_friction / is_locked / open_map_blocks），
#     吃显式入参、无 IO 副作用（is_locked 只读名单文件），--selftest 离线对拍；
#   - 候选面默认 *.rs（架构深化循环的本体 = rust/ 深模块），
#     ARCH_HOTZONE_FILE_GLOB 可覆盖为更宽/更窄的 git pathspec；
#   - 逐文件「git log --since 计数」并行化（xargs -P，自调用 --_eval-file 子进程），
#     计数口径与票面字面一致；ARCH_HOTZONE_PARALLEL 可调并发；
#   - 本脚本不碰 main 交易代码路径：只读 git 历史 + 行数，写报告到 .chanlun/。
# =============================================================================
set -euo pipefail

# ── 顶部常量（环境变量可覆盖）───────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
SCRIPT_PATH="$SCRIPT_DIR/$(basename "${BASH_SOURCE[0]:-$0}")"
REPO="${ARCH_HOTZONE_REPO:-}"
if [ -z "$REPO" ]; then
  # 从脚本所在目录（仓内）解析仓根，不依赖调用时的 cwd。
  REPO="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel 2>/dev/null || true)"
  [ -z "$REPO" ] && REPO="$(cd "$SCRIPT_DIR/.." && pwd)"
fi
BIG_FILE_LINES="${ARCH_HOTZONE_BIG_FILE_LINES:-2500}"        # C1：大文件行数阈值（>）
BIG_FILE_CHANGES="${ARCH_HOTZONE_BIG_FILE_CHANGES:-5}"       # C1：大文件改动阈值（≥）
SMALL_FILE_CHANGES="${ARCH_HOTZONE_SMALL_FILE_CHANGES:-12}"  # C1：高频小文件改动阈值（≥）
SINCE_DAYS="${ARCH_HOTZONE_SINCE_DAYS:-30}"                  # C1：时间窗（天）
FILE_GLOB="${ARCH_HOTZONE_FILE_GLOB:-*.rs}"                  # 候选面（git pathspec）
MAP_TITLE_FILTER="${ARCH_HOTZONE_MAP_TITLE_FILTER:-架构深化}" # C3：本线 wayfinder:map 标题过滤
REPORT_DIR="${ARCH_HOTZONE_REPORT_DIR:-.chanlun}"            # 报告落盘目录（相对 REPO）
INTERVAL_SECONDS="${ARCH_HOTZONE_INTERVAL_SECONDS:-86400}"   # --live 轮询间隔（默认 24h）
PARALLEL="${ARCH_HOTZONE_PARALLEL:-8}"                       # 逐文件 git log 并发度
GH_REPO="${GH_REPO:-xy7365527-lang/NewChanlun}"

cd "$REPO"

# launchd 日志目录（plist StandardOut/ErrorPath 落 .sandcastle/logs/；launchd 不建父目录）。
mkdir -p "$REPO/.sandcastle/logs" 2>/dev/null || true

# ── 锁定名单解析：优先级 ARCH_HOTZONE_LOCKLIST_FILE > 仓内默认 > 内建最小名单 ──
LOCKLIST_PATH="${ARCH_HOTZONE_LOCKLIST_FILE:-}"
if [ -z "$LOCKLIST_PATH" ] || [ ! -f "$LOCKLIST_PATH" ]; then
  LOCKLIST_PATH="$REPO/.chanlun/arch-hotzone-locklist.txt"
fi
if [ ! -f "$LOCKLIST_PATH" ]; then
  # 内建最小名单（票面点名的裁定锁定 + 类别锁定），供缺省文件时兜底。
  LOCKLIST_PATH="$(mktemp)"
  cat > "$LOCKLIST_PATH" <<'LOCKEOF'
rust/src/theta_v0/backtest/fill.rs
rust/src/trading/positional_fusion.rs
rust/src/theta_v0/classifier/nest.rs
rust/src/theta_v0/classifier/nest_index.rs
rust/src/theta_v0/classifier/nest_lifecycle.rs
rust/src/theta_v0/classifier/nest_lifecycle/*
rust/src/recursive_t/*
rust/src/bin/*
*_test.rs
*_tests.rs
*/tests.rs
*/tests/*
*/test_*.rs
LOCKEOF
fi

# ── 纯函数 ────────────────────────────────────────────────────────────────────

# C1 摩擦判据（纯函数）：入参 $1=行数 $2=近 N 天改动次数；命中返回 0，否则 1。
hotzone_friction() {
  local lines=$1 changes=$2
  if [ "$lines" -gt "$BIG_FILE_LINES" ] && [ "$changes" -ge "$BIG_FILE_CHANGES" ]; then
    return 0
  fi
  if [ "$changes" -ge "$SMALL_FILE_CHANGES" ]; then
    return 0
  fi
  return 1
}

# C2 锁定判据（纯函数）：入参 $1=仓库相对路径 $2=名单文件；锁定返回 0，否则 1。
# 名单格式：每行一个 glob 模式（bash case 语义，* 可跨 /），# 开头为注释，空行忽略。
is_locked() {
  local f=$1 listfile=$2 line pat
  while IFS= read -r line || [ -n "$line" ]; do
    pat="${line%%#*}"                             # 剥内联注释
    pat="${pat#"${pat%%[![:space:]]*}"}"          # 去首空白
    pat="${pat%"${pat##*[![:space:]]}"}"          # 去尾空白
    [ -z "$pat" ] && continue
    case "$f" in
      $pat) return 0 ;;                           # $pat 不加引号：glob 语义
    esac
  done < "$listfile"
  return 1
}

# C3 在飞图判据（纯函数）：入参 $1=open 本线 wayfinder:map 数量；>0 即阻断（返回 0）。
open_map_blocks() {
  [ "${1:-0}" -gt 0 ]
}

# 三判据合取（纯函数）：命中返回 0（C1 ∧ C2 ∧ C3 全满足）。
# 入参：$1=行数 $2=改动次数 $3=仓库相对路径 $4=名单文件 $5=open 本线图数量。
evaluate_file() {
  local lines=$1 changes=$2 path=$3 locklist=$4 open_maps=$5
  hotzone_friction "$lines" "$changes" || return 1
  is_locked "$path" "$locklist" && return 1          # 锁定 → 不命中
  open_map_blocks "$open_maps" && return 1           # 在飞图 → 不命中
  return 0
}

# ── 数据采集（非纯函数：读 git 历史 / 行数 / tracker）────────────────────────

# 近 N 天改动次数（git log --since 计数，票面字面口径）。
recent_change_count() {
  git log --since="${SINCE_DAYS} days ago" --oneline -- "$1" | wc -l | tr -d ' '
}

line_count() {
  wc -l < "$1" | tr -d ' '
}

# open 本线 wayfinder:map 数量（gh 查询）；失败返回 1（上层按「无法核验 C3」跳过）。
open_map_count() {
  local out n
  out="$(gh issue list --repo "$GH_REPO" --label wayfinder:map --state open \
        --limit 100 --json number,title 2>/dev/null)" || return 1
  n="$(printf '%s' "$out" | grep -o -F "$MAP_TITLE_FILTER" | wc -l | tr -d ' ' || true)"
  printf '%s' "${n:-0}"
}

# ── 内部子调用：评估单文件（xargs -P 并行，输出命中行到 stdout）──────────────
if [ "${1:-}" = "--_eval-file" ]; then
  f="$2"
  n="$(recent_change_count "$f")"
  lines="$(line_count "$f")"
  if hotzone_friction "$lines" "$n" && ! is_locked "$f" "$LOCKLIST_PATH"; then
    printf 'HIT\t%s\t%s\t%s\n' "$f" "$lines" "$n"
  fi
  exit 0
fi

# ── 扫描与报告 ────────────────────────────────────────────────────────────────

scan_once() {
  local mode=$1 stamp open_maps hits=0
  local total report_date report_path hit_rows=""
  local tmp_files tmp_hits

  stamp="$(date '+%Y-%m-%dT%H:%M:%S')"
  echo "[arch_hotzone] $stamp 一轮扫描开始（$mode）"

  # C3 先判（缺一不触发）：在飞图在 → 整轮不触发；gh 失败 → 无法核验 → 不触发。
  if ! open_maps="$(open_map_count)"; then
    echo "[arch_hotzone] 判据 3 无法核验（gh 查询失败）→ 本轮不触发，跳过（不写报告）" >&2
    return 0
  fi
  echo "[arch_hotzone] open 本线 wayfinder:map（标题含「$MAP_TITLE_FILTER」）：$open_maps 张"
  if open_map_blocks "$open_maps"; then
    echo "[arch_hotzone] 判据 3 不满足（在飞图在）→ 不触发，跳过（不写报告）"
    return 0
  fi

  # 候选文件（NUL 分隔写临时文件，兼容含空格/特殊字符的路径）。
  tmp_files="$(mktemp)"
  git ls-files -z -- "$FILE_GLOB" > "$tmp_files" 2>/dev/null || true
  total="$(tr -cd '\0' < "$tmp_files" | wc -c | tr -d ' ')"
  echo "[arch_hotzone] 候选文件（$FILE_GLOB）：${total:-0} 个"
  echo "[arch_hotzone] 近 ${SINCE_DAYS} 天改动阈值：>${BIG_FILE_LINES} 行 且 ≥${BIG_FILE_CHANGES} 次，或 ≥${SMALL_FILE_CHANGES} 次"
  echo "[arch_hotzone] 锁定名单：$LOCKLIST_PATH"

  # C1 ∧ C2 逐文件判（C3 已全局核过）；并行自调用，结果按路径排序后落盘。
  tmp_hits="$(mktemp)"
  if [ "${total:-0}" -gt 0 ]; then
    xargs -0 -n1 -P "$PARALLEL" bash "$SCRIPT_PATH" --_eval-file < "$tmp_files" > "$tmp_hits" || true
    sort -o "$tmp_hits" "$tmp_hits" 2>/dev/null || true
  fi

  hits=0
  hit_rows=""
  while IFS=$'\t' read -r _tag _f _lines _n; do
    [ "$_tag" = "HIT" ] || continue
    hits=$((hits + 1))
    hit_rows="${hit_rows}| $_f | $_lines | $_n |\n"
  done < "$tmp_hits"
  rm -f "$tmp_files" "$tmp_hits"

  if [ "$hits" -eq 0 ]; then
    echo "[arch_hotzone] 命中 0 个 → 无命中"
    return 0
  fi

  # 命中 → 写报告（不自动开图，HITL 闸一保留）。
  report_date="$(date '+%Y-%m-%d')"
  report_path="$REPORT_DIR/arch-hotzone-report-$report_date.md"
  mkdir -p "$REPORT_DIR"
  {
    echo "# 架构热区报告（$report_date）"
    echo
    echo "> 自动生成：\`scripts/arch_hotzone_monitor.sh\`（#1197）。本报告是仓内工作草稿。"
    echo "> HITL 闸一保留：本监控不自动开图；等编排者拍板后再跑 \`/improve-codebase-architecture\`。"
    echo
    echo "## 命中清单（$hits 个）"
    echo
    echo "| 文件 | 行数 | 近 ${SINCE_DAYS} 天改动 |"
    echo "|---|---|---|"
    printf '%b' "$hit_rows"
    echo
    echo "## 扫描建议"
    echo
    echo "以上文件命中热区判据（大文件+高频 或 高频小文件）且不在裁定锁定名单。"
    echo "建议编排者确认是否值得重启一轮架构深化扫描；确认后再开图。"
  } > "$report_path"
  echo "[arch_hotzone] 命中 $hits 个 → 已写 $report_path（不自动开图）"
}

# ── 合成实测（--selftest）：构造命中/不命中样例各跑一遍 ──────────────────────

selftest() {
  local fails=0 tmp lockfile
  set +e   # 断言过程中允许被断言函数返回非零

  tmp="$(mktemp)"
  lockfile="$tmp.locklist"
  cat > "$lockfile" <<'LOCKEOF'
# 合成名单
rust/locked_a.rs
rust/locked_dir/*
*_test.rs
*/test_*.rs
LOCKEOF

  # 断言辅助：$1=描述 $2=实际退出码 $3=期望退出码（0=真/命中，1=假/不命中）
  assert() {
    local desc=$1 actual=$2 expected=$3
    if [ "$actual" -eq "$expected" ]; then
      echo "  PASS  $desc"
    else
      echo "  FAIL  $desc（实际 $actual，期望 $expected）"
      fails=$((fails + 1))
    fi
  }

  echo "[arch_hotzone] --selftest 合成实测"

  echo "  C1 摩擦判据（hotzone_friction）"
  hotzone_friction 2501 5
  assert "大文件 2501 行 + 5 次 → 命中" $? 0
  hotzone_friction 2500 5
  assert "边界 2500 行（不 >2500）+ 5 次 → 不命中" $? 1
  hotzone_friction 2501 4
  assert "大文件 2501 行 + 4 次 → 不命中" $? 1
  hotzone_friction 100 12
  assert "小文件 100 行 + 12 次 → 命中" $? 0
  hotzone_friction 100 11
  assert "小文件 100 行 + 11 次 → 不命中" $? 1

  echo "  C2 锁定判据（is_locked）"
  is_locked rust/locked_a.rs "$lockfile"
  assert "精确命中 locked_a.rs → 锁定" $? 0
  is_locked rust/locked_dir/foo.rs "$lockfile"
  assert "目录 glob locked_dir/* → 锁定" $? 0
  is_locked rust/free.rs "$lockfile"
  assert "自由文件 free.rs → 不锁定" $? 1
  is_locked rust/foo_test.rs "$lockfile"
  assert "测试后缀 *_test.rs → 锁定" $? 0
  is_locked rust/sub/test_foo.rs "$lockfile"
  assert "测试前缀 */test_*.rs → 锁定" $? 0

  echo "  C3 在飞图判据（open_map_blocks）"
  open_map_blocks 0
  assert "0 张在飞图 → 不阻断" $? 1
  open_map_blocks 1
  assert "1 张在飞图 → 阻断" $? 0

  echo "  三判据合取（evaluate_file）"
  evaluate_file 2501 5 rust/free.rs "$lockfile" 0
  assert "全满足（大文件+高频+不锁+无图）→ 命中" $? 0
  evaluate_file 2501 5 rust/locked_a.rs "$lockfile" 0
  assert "锁定 → 不命中" $? 1
  evaluate_file 2501 5 rust/free.rs "$lockfile" 1
  assert "在飞图 1 张 → 不命中" $? 1
  evaluate_file 100 11 rust/free.rs "$lockfile" 0
  assert "摩擦不足 → 不命中" $? 1

  rm -f "$tmp" "$lockfile"
  set -e
  if [ "$fails" -gt 0 ]; then
    echo "[arch_hotzone] --selftest 失败：$fails 项"
    return 1
  fi
  echo "[arch_hotzone] --selftest 全过（命中/不命中样例各跑一遍）"
  return 0
}

usage() {
  echo "用法: $0 [--once|--live|--selftest]"
  echo "  --once      单轮扫描（默认）"
  echo "  --live      常驻轮询（launchd 入口，每 \${ARCH_HOTZONE_INTERVAL_SECONDS:-86400}s 一轮）"
  echo "  --selftest  合成实测（构造命中/不命中样例各跑一遍）"
}

# ── 入口 ───────────────────────────────────────────────────────────────────────

MODE="--once"
for arg in "$@"; do
  case "$arg" in
    --once) MODE="--once" ;;
    --live) MODE="--live" ;;
    --selftest) MODE="--selftest" ;;
    -h|--help) usage; exit 0 ;;
    *) echo "未知参数: $arg" >&2; usage; exit 2 ;;
  esac
done

case "$MODE" in
  --selftest)
    selftest
    ;;
  --once)
    scan_once "$MODE"
    ;;
  --live)
    echo "[arch_hotzone] --live 常驻启动：每 ${INTERVAL_SECONDS}s 一轮"
    while :; do
      scan_once "$MODE" || echo "[arch_hotzone] 一轮异常（继续下一轮）" >&2
      sleep "$INTERVAL_SECONDS"
    done
    ;;
esac
