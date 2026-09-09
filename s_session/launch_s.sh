#!/usr/bin/env bash
# #1370 TB-01-A：正式结构会话 S 的正式 launcher（可复制命令）。
#
# 启动链：S init → S.AcceptInput（原始输入 + 具名 profile）→ S.Advance（Begin→同次 Rust parser→Commit）
#         → 只读查询外壳（独立只读进程，S 自有 SQLite mode=ro）→ 浏览器。
# 不启动 E/B/X、不建其库、不加载经济政策。
#
# H4：默认**不删除**既有 S 库——库存在即恢复（跳过 init）；新建只允许不存在的目标。
#     显式 `--reset` 才删除重建（一次性试验）。
# M4：正式入口要求**明确** `--input` 与 `--profile`（无静默默认）；`--testonly` 显式选择具名 TestOnly。
# M5：按实际宿主解析 Python 与运行库路径（不硬编码 Linux 容器路径），并按宿主核构建产物。
#     构建产物路径随 CARGO_TARGET_DIR 解析（stamp 与产物同目录）；显式 `--bin` 只核本平台、
#     不自动构建、不用无关 stamp 证明平台。
#
# 用法：
#   ./s_session/launch_s.sh --input <输入.json> --profile <profile.json> [--db <sqlite路径>] [--port <端口>] [--build]
#   ./s_session/launch_s.sh --testonly [--db <sqlite路径>] [--port <端口>] [--build]   # 显式选择 TestOnly 档案
#   ./s_session/launch_s.sh stop [--port <端口>]
#   覆盖：--bin <s_structure_session 路径> --python <python3 路径>
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
# 实际 cargo 目标目录：尊重 CARGO_TARGET_DIR（可独立目录），默认 rust/target。
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/rust/target}"
BIN_DEFAULT="$TARGET_DIR/debug/s_structure_session"
STAMP="$TARGET_DIR/.s_session_host"
BIN=""
BIN_EXPLICIT=0
DB=""
INPUT=""
PROFILE=""
PORT="${PORT:-8787}"
DO_BUILD=0
DO_RESET=0
CATALOG="$HERE/catalog/signed-catalog.json"
BROWSER="$HERE/browser/index.html"
SESSION_ID="${SESSION_ID:-s-session-testonly-001}"

# 从候选 Python 的 sysconfig 取实际 LIBDIR 与 LDLIBRARY 并核真实文件——兼容 .so / .dylib
# （macOS LDLIBRARY=libpython3.11.dylib，Linux=libpython3.11.so），不写死任一平台路径。
lib_available() {
  local py="$1" libdir ldlib
  libdir="$("$py" -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR') or '')" 2>/dev/null || true)"
  ldlib="$("$py" -c "import sysconfig; print(sysconfig.get_config_var('LDLIBRARY') or '')" 2>/dev/null || true)"
  [[ -n "$libdir" && -n "$ldlib" && -f "$libdir/$ldlib" ]]
}

resolve_python() {
  if [[ -n "${PYO3_PYTHON:-}" ]]; then echo "$PYO3_PYTHON"; return; fi
  # 优先 uv python（自带 libpython，pyo3 构建/运行都可用）；再回落 PATH。
  for CAND in "$HOME"/.local/share/uv/python/*/bin/python3.11; do
    if [[ -x "$CAND" ]] && lib_available "$CAND"; then echo "$CAND"; return; fi
  done
  for CAND in python3 python; do
    if command -v "$CAND" >/dev/null 2>&1; then
      local p
      p="$(command -v "$CAND")"
      if lib_available "$p"; then echo "$p"; return; fi
    fi
  done
  echo ""
}

configure_runtime() {
  # 从选中的 Python 取得匹配运行库目录（同一来源，不固定版本字符串）。
  PY="$(resolve_python)"
  if [[ -z "$PY" ]]; then
    echo "[launcher] 错误：找不到带 libpython3.11（.so/.dylib）的 python（pyo3 构建/运行需要；可用 --python 指定）" >&2
    exit 1
  fi
  export PYO3_PYTHON="$PY"
  LIBDIR="$("$PY" -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR') or '')" 2>/dev/null || true)"
  if [[ -n "$LIBDIR" ]]; then
    export LD_LIBRARY_PATH="$LIBDIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
    export DYLD_LIBRARY_PATH="$LIBDIR${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
  fi
}

host_os() { case "$(uname -s)" in Darwin) echo macos;; Linux) echo linux;; *) echo "$(uname -s)";; esac; }
host_arch() { uname -m; }
host_stamp() { echo "$(host_os)-$(host_arch)"; }

# 用 `file` 有界分类二进制平台（macho / elf / pe / unknown）。
binary_platform() {
  local f="$1" out
  [[ -f "$f" ]] || { echo missing; return 0; }
  out="$(file -b "$f" 2>/dev/null || true)"
  case "$out" in
    *"Mach-O"*) echo macho ;;
    *"ELF"*) echo elf ;;
    *"PE32"*) echo pe ;;
    *) echo unknown ;;
  esac
}

host_binary_platform() {
  case "$(host_os)" in
    macos) echo macho ;;
    linux) echo elf ;;
    *) echo unknown ;;
  esac
}

needs_build() {
  [[ ! -x "$BIN" ]] && return 0
  [[ "$(binary_platform "$BIN")" != "$(host_binary_platform)" ]] && return 0
  [[ ! -f "$STAMP" ]] && return 0
  [[ "$(cat "$STAMP")" != "$(host_stamp)" ]] && return 0
  return 1
}

build() {
  echo "[launcher] 构建 S 二进制（cargo build --features s_session --bin s_structure_session）"
  (cd "$ROOT/rust" && cargo build --features s_session --bin s_structure_session)
  echo "$(host_stamp)" > "$STAMP"
}

# 有界身份核：只按 PID 核对进程命令行是否含本任务只读脚本，避免 PID 复用误杀无关进程。
# 不用 pkill -f（工具 shell 命令文本本身含脚本名，pkill -f 会自匹配并自杀）。
is_readonly_server() {
  local pid="$1" cmd
  if [[ -r "/proc/$pid/cmdline" ]]; then
    cmd="$(tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null || true)"
  else
    cmd="$(ps -p "$pid" -o command= 2>/dev/null || true)"
  fi
  [[ "$cmd" == *"s_readonly_server.py"* ]]
}

stop() {
  if [[ -f "$PIDFILE" ]]; then
    PID="$(cat "$PIDFILE")"
    if is_readonly_server "$PID"; then
      echo "[launcher] 停止只读查询进程 pid=$PID"
      kill "$PID"
    else
      echo "[launcher] PID $PID 不是本任务只读进程（或已退出），跳过 kill"
    fi
    rm -f "$PIDFILE"
  fi
  echo "[launcher] 已停止（S 写进程为一次性命令，无常驻进程）"
}

if [[ "${1:-}" == "stop" ]]; then
  while [[ $# -gt 1 ]]; do
    case "$2" in
      --port) PORT="$3"; shift 2 ;;
      *) shift ;;
    esac
  done
  PIDFILE="/tmp/s_session_readonly_${PORT}.pid"
  stop
  exit 0
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db) DB="$2"; shift 2 ;;
    --input) INPUT="$2"; shift 2 ;;
    --profile) PROFILE="$2"; shift 2 ;;
    --port) PORT="$2"; shift 2 ;;
    --build) DO_BUILD=1; shift ;;
    --reset) DO_RESET=1; shift ;;
    --bin) BIN="$2"; BIN_EXPLICIT=1; shift 2 ;;
    --python) PYO3_PYTHON="$2"; shift 2 ;;
    --testonly)
      INPUT="$HERE/inputs/cc006_four_branch.json"
      PROFILE="$HERE/profiles/testonly_tick_1_1_ohlc.json"
      shift ;;
    *) echo "未知参数 $1"; exit 2 ;;
  esac
done

# M4：正式入口要求明确输入 + 已加载 profile（--testonly 是显式选择，非静默默认）。
if [[ -z "$INPUT" || -z "$PROFILE" ]]; then
  echo "[launcher] 错误：必须显式指定 --input 与 --profile（或 --testonly 显式选择 TestOnly 档案）" >&2
  exit 2
fi
[[ -z "$DB" ]] && DB="/tmp/s_session_testonly.sqlite"
PIDFILE="/tmp/s_session_readonly_${PORT}.pid"

configure_runtime

if [[ "$BIN_EXPLICIT" -eq 1 ]]; then
  # 显式 --bin：核其为本平台原生产物；不自动构建、不用无关 stamp 证明平台。
  if [[ ! -x "$BIN" ]]; then
    echo "[launcher] 错误：--bin $BIN 不存在或不可执行" >&2
    exit 1
  fi
  BP="$(binary_platform "$BIN")"
  HP="$(host_binary_platform)"
  if [[ "$BP" != "$HP" ]]; then
    echo "[launcher] 错误：--bin $BIN 非本平台产物（file=$BP，本平台=$HP）；请提供本平台构建产物" >&2
    exit 1
  fi
else
  BIN="$BIN_DEFAULT"
  [[ "$DO_BUILD" -eq 1 ]] && build
  needs_build && build
fi

if [[ "$DO_RESET" -eq 1 ]]; then
  echo "[launcher] --reset：删除并重建（一次性试验）"
  rm -f "$DB" "$DB-wal" "$DB-shm"
fi

if [[ -f "$DB" ]]; then
  echo "[launcher] 1/5 恢复已有 S 库（不 init、不删除既有持久事实）"
else
  echo "[launcher] 1/5 S init（独立 SQLite/WAL 单写者）"
  "$BIN" init --db "$DB" --session "$SESSION_ID" --catalog "$CATALOG"
fi

echo "[launcher] 2/5 S.AcceptInput（原始逐笔档案 + 具名 profile）"
"$BIN" accept --db "$DB" --input "$INPUT" --profile "$PROFILE"

echo "[launcher] 3/5 S.Advance（Begin→同次 Rust parser→Commit）"
"$BIN" advance --db "$DB"

echo "[launcher] 4/5 启动只读查询外壳（独立只读进程，端口 ${PORT}）"
nohup "$PY" "$HERE/s_readonly_server.py" --db "$DB" --port "$PORT" --browser "$BROWSER" \
  > /tmp/s_session_readonly_${PORT}.log 2>&1 &
SRV_PID=$!
echo "$SRV_PID" > "$PIDFILE"
sleep 1
if ! kill -0 "$SRV_PID" 2>/dev/null; then
  echo "[launcher] 只读查询进程启动失败，日志："
  cat /tmp/s_session_readonly_${PORT}.log
  exit 1
fi

echo "[launcher] 5/5 完成。浏览器 / API："
echo "  浏览器        http://127.0.0.1:${PORT}/"
echo "  目录 API      http://127.0.0.1:${PORT}/api/catalog"
echo "  快照 API      http://127.0.0.1:${PORT}/api/snapshot"
echo "  状态 API      http://127.0.0.1:${PORT}/api/state"
echo "  停止          ./s_session/launch_s.sh stop --port ${PORT}"
echo "  S 数据库      $DB"
echo "  只读日志      /tmp/s_session_readonly_${PORT}.log"
